// cash — Typed Action Model
//
// The AI never produces raw shell strings.
// It produces typed actions that pass through policy.
//
// Every action has:
//   - A kind (what type of operation)
//   - A target (what it operates on)
//   - Parameters (how it operates)
//   - A risk score (computed by RiskScorer)

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    // File operations
    ReadFile,
    WriteFile,
    DeleteFile,
    MoveFile,
    CopyFile,
    CreateDir,
    DeleteDir,
    ListDir,

    // Process operations
    RunCommand,
    KillProcess,
    StartService,
    StopService,

    // Network operations
    HttpGet,
    HttpPost,
    OpenPort,
    ClosePort,
    DnsLookup,
    PortScan,

    // Install operations
    InstallPackage,
    UninstallPackage,
    UpdatePackage,
    InstallAgent,

    // System operations
    ChangePermissions,
    ChangeOwner,
    EditConfig,
    ReadEnvVars,
    WriteEnvVars,

    // Security operations
    ReadAuditLog,
    ScanNetwork,
    CheckVulnerabilities,
    RunInSandbox,

    // Shell operations
    SetEnvVar,
    ExportVar,
    SourceFile,

    // Unknown — requires extra scrutiny
    Unknown,
}

impl ActionKind {
    pub fn from_command(cmd: &str) -> Self {
        let cmd = cmd.trim().to_lowercase();
        let first = cmd.split_whitespace().next().unwrap_or("");

        match first {
            "rm" | "remove"                          => ActionKind::DeleteFile,
            "rmdir"                                  => ActionKind::DeleteDir,
            "mv" | "move"                            => ActionKind::MoveFile,
            "cp" | "copy"                            => ActionKind::CopyFile,
            "cat" | "less" | "head" | "tail"         => ActionKind::ReadFile,
            "echo" | "tee"                           => ActionKind::WriteFile,
            "mkdir"                                  => ActionKind::CreateDir,
            "ls" | "show" | "dir"                    => ActionKind::ListDir,
            "curl" | "wget"                          => ActionKind::HttpGet,
            "chmod"                                  => ActionKind::ChangePermissions,
            "chown"                                  => ActionKind::ChangeOwner,
            "apt" | "apt-get" | "brew" | "pacman"   => ActionKind::InstallPackage,
            "kill" | "pkill"                         => ActionKind::KillProcess,
            "systemctl" | "service"                  => ActionKind::StartService,
            "nmap" | "scan"                          => ActionKind::PortScan,
            "export"                                 => ActionKind::ExportVar,
            "source" | "."                           => ActionKind::SourceFile,
            "ssh" | "scp"                            => ActionKind::HttpPost,
            _                                        => ActionKind::RunCommand,
        }
    }

    pub fn display(&self) -> &str {
        match self {
            ActionKind::ReadFile           => "read file",
            ActionKind::WriteFile          => "write file",
            ActionKind::DeleteFile         => "delete file",
            ActionKind::MoveFile           => "move file",
            ActionKind::CopyFile           => "copy file",
            ActionKind::CreateDir          => "create directory",
            ActionKind::DeleteDir          => "delete directory",
            ActionKind::ListDir            => "list directory",
            ActionKind::RunCommand         => "run command",
            ActionKind::KillProcess        => "kill process",
            ActionKind::StartService       => "start service",
            ActionKind::StopService        => "stop service",
            ActionKind::HttpGet            => "network request",
            ActionKind::HttpPost           => "network request",
            ActionKind::OpenPort           => "open port",
            ActionKind::ClosePort          => "close port",
            ActionKind::DnsLookup          => "DNS lookup",
            ActionKind::PortScan           => "port scan",
            ActionKind::InstallPackage     => "install package",
            ActionKind::UninstallPackage   => "uninstall package",
            ActionKind::UpdatePackage      => "update package",
            ActionKind::InstallAgent       => "install agent",
            ActionKind::ChangePermissions  => "change permissions",
            ActionKind::ChangeOwner        => "change owner",
            ActionKind::EditConfig         => "edit config",
            ActionKind::ReadEnvVars        => "read env vars",
            ActionKind::WriteEnvVars       => "write env vars",
            ActionKind::ReadAuditLog       => "read audit log",
            ActionKind::ScanNetwork        => "scan network",
            ActionKind::CheckVulnerabilities => "check vulnerabilities",
            ActionKind::RunInSandbox       => "run in sandbox",
            ActionKind::SetEnvVar          => "set env var",
            ActionKind::ExportVar          => "export var",
            ActionKind::SourceFile         => "source file",
            ActionKind::Unknown            => "unknown operation",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub id:          String,
    pub kind:        ActionKind,
    pub target:      String,
    pub params:      HashMap<String, String>,
    pub raw_command: String,
    pub risk_score:  f64,
    pub description: String,
}

impl Action {
    pub fn new(kind: ActionKind, target: &str, raw_command: &str) -> Self {
        let id = format!("act-{}", chrono::Utc::now().timestamp_millis());
        Self {
            id,
            kind,
            target: target.to_string(),
            params: HashMap::new(),
            raw_command: raw_command.to_string(),
            risk_score: 0.0,
            description: String::new(),
        }
    }

    pub fn from_command(cmd: &str) -> Self {
        let kind = ActionKind::from_command(cmd);
        let target = extract_target(cmd);
        let mut action = Self::new(kind, &target, cmd);
        action.description = format!("{}: {}", action.kind.display(), target);
        action
    }

    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }
}

fn extract_target(cmd: &str) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    // Skip flags, take first non-flag argument after command
    parts.iter().skip(1)
         .find(|p| !p.starts_with('-'))
         .unwrap_or(&"")
         .to_string()
}

/// A plan is a sequence of typed actions.
#[derive(Debug, Clone)]
pub struct Plan {
    pub id:          String,
    pub intent:      String,
    pub actions:     Vec<Action>,
    pub created_at:  String,
}

impl Plan {
    pub fn new(intent: &str, actions: Vec<Action>) -> Self {
        Self {
            id:         format!("plan-{}", chrono::Utc::now().timestamp_millis()),
            intent:     intent.to_string(),
            actions,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn max_risk(&self) -> f64 {
        self.actions.iter().map(|a| a.risk_score).fold(0.0_f64, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_kind_from_rm() {
        assert_eq!(ActionKind::from_command("rm -rf /tmp/test"), ActionKind::DeleteFile);
    }

    #[test]
    fn action_kind_from_ls() {
        assert_eq!(ActionKind::from_command("ls -la"), ActionKind::ListDir);
    }

    #[test]
    fn action_kind_from_apt() {
        assert_eq!(ActionKind::from_command("apt install nmap"), ActionKind::InstallPackage);
    }

    #[test]
    fn action_from_command_extracts_target() {
        let action = Action::from_command("rm /tmp/file.txt");
        assert_eq!(action.target, "/tmp/file.txt");
        assert_eq!(action.kind, ActionKind::DeleteFile);
    }

    #[test]
    fn plan_max_risk() {
        let mut a1 = Action::from_command("ls");
        let mut a2 = Action::from_command("rm file");
        a1.risk_score = 1.0;
        a2.risk_score = 8.0;
        let plan = Plan::new("test", vec![a1, a2]);
        assert_eq!(plan.max_risk(), 8.0);
    }
}
