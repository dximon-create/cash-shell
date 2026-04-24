// cash — Agent Verifier
//
// Verifies agent packages before installation.
// Checks SHA-256 checksum and validates manifest.

use sha2::{Sha256, Digest};
use hex::encode as hex_encode;

pub struct Verifier;

impl Verifier {
    /// Compute SHA-256 checksum of bytes.
    pub fn checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex_encode(hasher.finalize())
    }

    /// Verify data matches expected checksum.
    pub fn verify(data: &[u8], expected: &str) -> bool {
        Self::checksum(data) == expected
    }

    /// Validate an agent manifest has required fields.
    pub fn validate_manifest(manifest: &AgentManifest) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if manifest.id.is_empty()      { errors.push("id is required".into()); }
        if manifest.name.is_empty()    { errors.push("name is required".into()); }
        if manifest.version.is_empty() { errors.push("version is required".into()); }
        if manifest.entry.is_empty()   { errors.push("entry point is required".into()); }
        if manifest.id.contains(' ')   { errors.push("id cannot contain spaces".into()); }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

#[derive(Debug, Clone)]
pub struct AgentManifest {
    pub id:          String,
    pub name:        String,
    pub version:     String,
    pub description: String,
    pub entry:       String,
    pub permissions: Vec<String>,
    pub checksum:    String,
}

impl AgentManifest {
    pub fn new(id: &str, name: &str, version: &str, entry: &str) -> Self {
        Self {
            id:          id.to_string(),
            name:        name.to_string(),
            version:     version.to_string(),
            description: String::new(),
            entry:       entry.to_string(),
            permissions: Vec::new(),
            checksum:    String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_deterministic() {
        let data = b"hello world";
        assert_eq!(Verifier::checksum(data), Verifier::checksum(data));
    }

    #[test]
    fn verify_correct_checksum() {
        let data = b"agent package data";
        let cs = Verifier::checksum(data);
        assert!(Verifier::verify(data, &cs));
    }

    #[test]
    fn verify_wrong_checksum_fails() {
        let data = b"agent package data";
        assert!(!Verifier::verify(data, "wrongchecksum"));
    }

    #[test]
    fn valid_manifest_passes() {
        let m = AgentManifest::new("my-agent", "My Agent", "1.0.0", "main.rs");
        assert!(Verifier::validate_manifest(&m).is_ok());
    }

    #[test]
    fn manifest_missing_id_fails() {
        let m = AgentManifest::new("", "My Agent", "1.0.0", "main.rs");
        let errs = Verifier::validate_manifest(&m).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("id")));
    }

    #[test]
    fn manifest_id_with_space_fails() {
        let m = AgentManifest::new("my agent", "My Agent", "1.0.0", "main.rs");
        let errs = Verifier::validate_manifest(&m).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("spaces")));
    }
}
