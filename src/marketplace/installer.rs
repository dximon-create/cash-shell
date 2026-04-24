// cash — Agent Installer
//
// Handles the install/uninstall flow with full verification.

use crate::store::Store;
use super::registry::{MarketplaceRegistry, InstalledAgent};
use super::verifier::{Verifier, AgentManifest};

pub struct Installer {
    registry: MarketplaceRegistry,
}

impl Installer {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        Ok(Self { registry: MarketplaceRegistry::new(store)? })
    }

    /// Install an agent from a manifest and package bytes.
    /// Verifies checksum before installing.
    pub fn install(&self, manifest: &AgentManifest, data: &[u8]) -> Result<(), String> {
        // Validate manifest.
        Verifier::validate_manifest(manifest)
            .map_err(|errs| format!("invalid manifest: {}", errs.join(", ")))?;

        // Verify checksum.
        if !manifest.checksum.is_empty() && !Verifier::verify(data, &manifest.checksum) {
            return Err("checksum verification failed".into());
        }

        let agent = InstalledAgent {
            id:           manifest.id.clone(),
            name:         manifest.name.clone(),
            version:      manifest.version.clone(),
            description:  manifest.description.clone(),
            permissions:  manifest.permissions.clone(),
            checksum:     if manifest.checksum.is_empty() {
                              Verifier::checksum(data)
                          } else {
                              manifest.checksum.clone()
                          },
            installed_at: chrono::Utc::now().to_rfc3339(),
            enabled:      true,
        };

        self.registry.install(&agent).map_err(|e| e.to_string())?;
        println!("installed: {} v{}", manifest.name, manifest.version);
        Ok(())
    }

    /// Uninstall an agent by ID.
    pub fn uninstall(&self, id: &str) -> Result<(), String> {
        match self.registry.uninstall(id) {
            Ok(true)  => { println!("uninstalled: {}", id); Ok(()) }
            Ok(false) => Err(format!("agent '{}' not found", id)),
            Err(e)    => Err(e.to_string()),
        }
    }

    pub fn list(&self) -> anyhow::Result<Vec<InstalledAgent>> {
        self.registry.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (Installer, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        (Installer::new(store).unwrap(), dir)
    }

    #[test]
    fn install_valid_agent() {
        let (installer, _dir) = setup();
        let data = b"fake agent binary";
        let mut manifest = AgentManifest::new("my-agent", "My Agent", "1.0.0", "main");
        manifest.checksum = Verifier::checksum(data);
        assert!(installer.install(&manifest, data).is_ok());
        assert_eq!(installer.list().unwrap().len(), 1);
    }

    #[test]
    fn install_bad_checksum_fails() {
        let (installer, _dir) = setup();
        let mut manifest = AgentManifest::new("my-agent", "My Agent", "1.0.0", "main");
        manifest.checksum = "badchecksum".into();
        assert!(installer.install(&manifest, b"data").is_err());
    }

    #[test]
    fn uninstall_installed_agent() {
        let (installer, _dir) = setup();
        let data = b"agent data";
        let manifest = AgentManifest::new("my-agent", "My Agent", "1.0.0", "main");
        installer.install(&manifest, data).unwrap();
        assert!(installer.uninstall("my-agent").is_ok());
        assert_eq!(installer.list().unwrap().len(), 0);
    }

    #[test]
    fn uninstall_missing_agent_fails() {
        let (installer, _dir) = setup();
        assert!(installer.uninstall("nonexistent").is_err());
    }
}
