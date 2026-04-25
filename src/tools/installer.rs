// cash — Tool Installer
//
// Installs tools via the system package manager.
// Verifies the binary exists after install.
// Registers with ToolRegistry.

use std::process::Command;
use crate::store::Store;
use super::detector::SystemInfo;
use super::registry::{ToolRegistry, ToolRecord};

pub struct ToolInstaller {
    registry:    ToolRegistry,
    system_info: SystemInfo,
}

/// Known tools cash can install with their metadata.
pub struct KnownTool {
    pub id:           &'static str,
    pub apt_name:     &'static str,
    pub brew_name:    &'static str,
    pub binary:       &'static str,
    pub description:  &'static str,
    pub cash_command: &'static str,
}

pub const KNOWN_TOOLS: &[KnownTool] = &[
    KnownTool { id: "nmap",      apt_name: "nmap",       brew_name: "nmap",
                binary: "nmap",      description: "Network scanner — finds hosts and open ports",
                cash_command: "scan" },
    KnownTool { id: "nikto",     apt_name: "nikto",      brew_name: "nikto",
                binary: "nikto",     description: "Web vulnerability scanner",
                cash_command: "vuln" },
    KnownTool { id: "gobuster",  apt_name: "gobuster",   brew_name: "gobuster",
                binary: "gobuster",  description: "Directory and file brute-forcer",
                cash_command: "dirs" },
    KnownTool { id: "curl",      apt_name: "curl",       brew_name: "curl",
                binary: "curl",      description: "HTTP client — inspect headers, download files",
                cash_command: "http" },
    KnownTool { id: "whois",     apt_name: "whois",      brew_name: "whois",
                binary: "whois",     description: "Domain registration lookup",
                cash_command: "whois" },
    KnownTool { id: "netcat",    apt_name: "netcat",     brew_name: "netcat",
                binary: "nc",        description: "Network swiss army knife",
                cash_command: "nc" },
    KnownTool { id: "traceroute",apt_name: "traceroute", brew_name: "traceroute",
                binary: "traceroute",description: "Trace network path to destination",
                cash_command: "trace" },
    KnownTool { id: "sqlmap",    apt_name: "sqlmap",     brew_name: "sqlmap",
                binary: "sqlmap",    description: "SQL injection detection and exploitation",
                cash_command: "sqli" },
    KnownTool { id: "hydra",     apt_name: "hydra",      brew_name: "hydra",
                binary: "hydra",     description: "Password brute-forcer",
                cash_command: "brute" },
    KnownTool { id: "openssl",   apt_name: "openssl",    brew_name: "openssl",
                binary: "openssl",   description: "SSL/TLS inspector and certificate tool",
                cash_command: "ssl" },
];

impl ToolInstaller {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        Ok(Self {
            registry:    ToolRegistry::new(store)?,
            system_info: SystemInfo::detect(),
        })
    }

    /// Install a tool by name.
    pub fn install(&self, name: &str) -> Result<(), String> {
        println!("cash: Detected system: {}", self.system_info.describe());

        let known = self.find_known(name)
            .ok_or_else(|| format!(
                "cash: '{}' is not in the cash tool registry.\nKnown tools: {}",
                name,
                KNOWN_TOOLS.iter().map(|t| t.id).collect::<Vec<_>>().join(", ")
            ))?;

        if self.registry.is_installed(known.id) {
            println!("cash: {} is already installed.", known.id);
            return Ok(());
        }

        // Get install command for this OS.
        let pkg_name = match &self.system_info.package_manager {
            super::detector::PackageManager::Brew => known.brew_name,
            _ => known.apt_name,
        };

        let cmd_parts = self.system_info.install_command(pkg_name)
            .ok_or_else(|| "cash: cannot detect package manager. Install manually.".to_string())?;

        println!("cash: Installing {} — {}...", known.id, known.description);
        println!("cash: Running: {}", cmd_parts.join(" "));

        let status = Command::new(&cmd_parts[0])
            .args(&cmd_parts[1..])
            .status()
            .map_err(|e| format!("cash: install failed: {}", e))?;

        if !status.success() {
            return Err(format!("cash: {} installation failed. Try manually: {}", known.id, cmd_parts.join(" ")));
        }

        // Verify binary exists.
        if !binary_exists(known.binary) {
            return Err(format!("cash: {} installed but binary '{}' not found in PATH", known.id, known.binary));
        }

        // Register.
        let version = get_version(known.binary);
        self.registry.register(&ToolRecord {
            id:           known.id.to_string(),
            name:         known.id.to_string(),
            version,
            binary:       known.binary.to_string(),
            description:  known.description.to_string(),
            cash_command: known.cash_command.to_string(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            enabled:      true,
        }).map_err(|e| e.to_string())?;

        println!("cash: ✓ {} installed. Use: {}", known.id, known.cash_command);
        Ok(())
    }

    /// Uninstall a tool.
    pub fn uninstall(&self, name: &str) -> Result<(), String> {
        if !self.registry.is_installed(name) {
            return Err(format!("cash: '{}' is not installed through cash", name));
        }
        self.registry.remove(name).map_err(|e| e.to_string())?;
        println!("cash: {} removed from cash registry.", name);
        println!("cash: To fully uninstall: sudo apt remove {}", name);
        Ok(())
    }

    /// List all installed tools.
    pub fn list(&self) {
        let tools = self.registry.list().unwrap_or_default();
        if tools.is_empty() {
            println!("No tools installed through cash.");
            println!("Try: cash install nmap");
            return;
        }
        println!("\n{:<12} {:<10} {:<12} {}", "TOOL", "VERSION", "COMMAND", "DESCRIPTION");
        println!("{}", "─".repeat(65));
        for t in &tools {
            println!("{:<12} {:<10} {:<12} {}", t.name, t.version, t.cash_command, t.description);
        }
        println!("\n{} tool(s) installed.", tools.len());
    }

    /// Show all available tools that can be installed.
    pub fn available(&self) {
        println!("\ncash tool catalogue:\n");
        println!("{:<12} {:<12} {}", "TOOL", "COMMAND", "DESCRIPTION");
        println!("{}", "─".repeat(60));
        for t in KNOWN_TOOLS {
            let installed = if self.registry.is_installed(t.id) { "✓" } else { " " };
            println!("{} {:<12} {:<12} {}", installed, t.id, t.cash_command, t.description);
        }
        println!("\n✓ = installed. Install with: cash install <tool>");
    }

    fn find_known(&self, name: &str) -> Option<&KnownTool> {
        KNOWN_TOOLS.iter().find(|t| t.id == name || t.cash_command == name)
    }
}

fn binary_exists(binary: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
    path.split(':').any(|dir| std::path::Path::new(dir).join(binary).exists())
}

fn get_version(binary: &str) -> String {
    Command::new(binary)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tools_have_required_fields() {
        for tool in KNOWN_TOOLS {
            assert!(!tool.id.is_empty());
            assert!(!tool.binary.is_empty());
            assert!(!tool.description.is_empty());
            assert!(!tool.cash_command.is_empty());
        }
    }

    #[test]
    fn binary_exists_for_ls() {
        assert!(binary_exists("ls"));
    }

    #[test]
    fn binary_not_exists_for_fake() {
        assert!(!binary_exists("zzz_fake_tool_zzz"));
    }
}
