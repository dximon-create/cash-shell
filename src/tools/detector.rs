// cash — OS and Package Manager Detector

#[derive(Debug, Clone, PartialEq)]
pub enum Os {
    Ubuntu,
    Kali,
    Debian,
    Arch,
    MacOs,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PackageManager {
    Apt,
    Brew,
    Pacman,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os:              Os,
    pub package_manager: PackageManager,
    pub is_wsl:          bool,
    pub is_root:         bool,
}

impl SystemInfo {
    pub fn detect() -> Self {
        let os = detect_os();
        let package_manager = detect_package_manager(&os);
        let is_wsl = detect_wsl();
        let is_root = detect_root();

        Self { os, package_manager, is_wsl, is_root }
    }

    pub fn install_prefix(&self) -> Vec<String> {
        match self.is_root {
            true  => vec![],
            false => vec!["sudo".to_string()],
        }
    }

    pub fn install_command(&self, tool: &str) -> Option<Vec<String>> {
        match &self.package_manager {
            PackageManager::Apt => {
                let mut cmd = self.install_prefix();
                cmd.extend(["apt".into(), "install".into(), "-y".into(), tool.into()]);
                Some(cmd)
            }
            PackageManager::Brew => {
                Some(vec!["brew".into(), "install".into(), tool.into()])
            }
            PackageManager::Pacman => {
                let mut cmd = self.install_prefix();
                cmd.extend(["pacman".into(), "-S".into(), "--noconfirm".into(), tool.into()]);
                Some(cmd)
            }
            PackageManager::Unknown => None,
        }
    }

    pub fn describe(&self) -> String {
        let os_str = match &self.os {
            Os::Ubuntu  => "Ubuntu",
            Os::Kali    => "Kali Linux",
            Os::Debian  => "Debian",
            Os::Arch    => "Arch Linux",
            Os::MacOs   => "macOS",
            Os::Unknown(s) => s.as_str(),
        };
        let wsl = if self.is_wsl { " (WSL)" } else { "" };
        format!("{}{}", os_str, wsl)
    }
}

fn detect_os() -> Os {
    // Check /etc/os-release on Linux
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        let lower = content.to_lowercase();
        if lower.contains("kali")   { return Os::Kali; }
        if lower.contains("ubuntu") { return Os::Ubuntu; }
        if lower.contains("debian") { return Os::Debian; }
        if lower.contains("arch")   { return Os::Arch; }
    }
    // macOS
    if cfg!(target_os = "macos") { return Os::MacOs; }
    Os::Unknown("unknown".into())
}

fn detect_package_manager(os: &Os) -> PackageManager {
    match os {
        Os::Ubuntu | Os::Kali | Os::Debian => PackageManager::Apt,
        Os::Arch    => PackageManager::Pacman,
        Os::MacOs   => PackageManager::Brew,
        Os::Unknown(_) => {
            // Try to find a package manager
            if cmd_exists("apt")    { return PackageManager::Apt; }
            if cmd_exists("brew")   { return PackageManager::Brew; }
            if cmd_exists("pacman") { return PackageManager::Pacman; }
            PackageManager::Unknown
        }
    }
}

fn detect_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

fn detect_root() -> bool {
    #[cfg(unix)]
    {
        // Check effective UID via /proc/self/status
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("Uid:") {
                    return line.split_whitespace().nth(1).map(|s| s == "0").unwrap_or(false);
                }
            }
        }
        // Fallback: check if sudo is needed
        std::env::var("USER").map(|u| u == "root").unwrap_or(false)
    }
    #[cfg(not(unix))]
    { false }
}

fn cmd_exists(cmd: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
    path.split(':').any(|dir| std::path::Path::new(dir).join(cmd).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_info_detects() {
        let info = SystemInfo::detect();
        // Just verify it doesn't panic
        let _ = info.describe();
    }

    #[test]
    fn apt_install_command() {
        let info = SystemInfo {
            os: Os::Ubuntu,
            package_manager: PackageManager::Apt,
            is_wsl: true,
            is_root: false,
        };
        let cmd = info.install_command("nmap").unwrap();
        assert!(cmd.contains(&"apt".to_string()));
        assert!(cmd.contains(&"nmap".to_string()));
    }

    #[test]
    fn brew_install_command() {
        let info = SystemInfo {
            os: Os::MacOs,
            package_manager: PackageManager::Brew,
            is_wsl: false,
            is_root: false,
        };
        let cmd = info.install_command("nmap").unwrap();
        assert!(cmd.contains(&"brew".to_string()));
    }

    #[test]
    fn unknown_pm_returns_none() {
        let info = SystemInfo {
            os: Os::Unknown("test".into()),
            package_manager: PackageManager::Unknown,
            is_wsl: false,
            is_root: false,
        };
        assert!(info.install_command("nmap").is_none());
    }
}
