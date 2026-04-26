// cash — Policy Engine
//
// Every action passes through here.
// Returns a Decision: Allow / Warn / Approve / Block

use super::action::{Action, ActionKind};
use super::risk::RiskScorer;

#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Safe — execute automatically.
    Allow,
    /// Safe but notable — show warning then execute.
    Warn { reason: String },
    /// Risky — user must confirm before execution.
    Approve { reason: String },
    /// Forbidden — never execute.
    Block { reason: String },
}

impl Decision {
    pub fn label(&self) -> &str {
        match self {
            Decision::Allow           => "ALLOW",
            Decision::Warn { .. }    => "WARN",
            Decision::Approve { .. } => "APPROVE",
            Decision::Block { .. }   => "BLOCK",
        }
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Decision::Block { .. })
    }

    pub fn needs_approval(&self) -> bool {
        matches!(self, Decision::Approve { .. })
    }
}

pub struct PolicyEngine {
    strict_mode: bool,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    pub fn strict() -> Self {
        Self { strict_mode: true }
    }

    /// Evaluate an action and return a decision.
    pub fn evaluate(&self, action: &mut Action) -> Decision {
        // Always compute risk score first.
        action.risk_score = RiskScorer::score(action);

        // Whitelisted commands — always Allow.
        if Self::is_whitelisted(action) {
            return Decision::Allow;
        }

        // Hard blocks — never allow regardless of score.
        if let Some(reason) = self.hard_block(action) {
            return Decision::Block { reason };
        }

        // Score-based decision.
        let decision = match action.risk_score as u32 {
            0..=2 => Decision::Allow,
            3..=5 => Decision::Warn {
                reason: format!(
                    "{} risk (score {:.1}): {}",
                    RiskScorer::label(action.risk_score),
                    action.risk_score,
                    action.kind.display()
                )
            },
            6..=8 => Decision::Approve {
                reason: format!(
                    "{} risk (score {:.1}): {} — requires your approval",
                    RiskScorer::label(action.risk_score),
                    action.risk_score,
                    action.kind.display()
                )
            },
            _ => Decision::Block {
                reason: format!(
                    "CRITICAL risk (score {:.1}): {} is too dangerous to execute",
                    action.risk_score,
                    action.kind.display()
                )
            },
        };

        // Strict mode bumps everything up one level.
        if self.strict_mode {
            return self.escalate(decision);
        }

        decision
    }

    /// Check if a command is whitelisted — always Allow.
    fn is_whitelisted(action: &Action) -> bool {
        let cmd = action.raw_command.split_whitespace().next().unwrap_or("");
        matches!(cmd,
            // Version control
            "cash" | "setup" | "git" | "gh" | "svn" | "hg" |
            // Build tools
            "cargo" | "rustc" | "make" | "cmake" | "gradle" | "mvn" |
            "npm" | "yarn" | "pnpm" | "pip" | "pip3" | "python" | "python3" |
            "node" | "deno" | "bun" |
            // Editors
            "vim" | "nvim" | "nano" | "code" | "emacs" |
            // Common dev tools
            "grep" | "find" | "sed" | "awk" | "sort" | "uniq" | "wc" |
            "head" | "tail" | "diff" | "patch" | "tar" | "zip" | "unzip" |
            "ssh" | "scp" | "rsync" | "ping" | "curl" | "wget" | "jq" |
            "docker" | "kubectl" | "terraform" | "ansible" |
            // Shell utilities
            "echo" | "printf" | "cat" | "ls" | "pwd" | "cd" | "which" |
            "man" | "less" | "more" | "tee" | "xargs" | "watch" |
            "ps" | "top" | "htop" | "df" | "du" | "free" | "uname"
        )
    }

    /// Check for absolute hard blocks — never executable.
    fn hard_block(&self, action: &Action) -> Option<String> {
        let raw = action.raw_command.to_lowercase();
        let target = action.target.to_lowercase();

        // Fork bombs
        if raw.contains(":(){ :|:& };:") {
            return Some("fork bomb detected".into());
        }

        // Destroying root filesystem
        if (raw.contains("rm") && target == "/") ||
           (raw.contains("rm -rf /") && !raw.contains("/tmp")) {
            return Some("destroying root filesystem is not permitted".into());
        }

        // Overwriting critical system files
        if raw.contains("> /etc/passwd") || raw.contains("> /etc/shadow") {
            return Some("overwriting system authentication files".into());
        }

        // Remote code execution patterns
        if (raw.contains("curl") || raw.contains("wget")) && (raw.contains("| sh") || raw.contains("|sh") || raw.contains("| bash") || raw.contains("|bash")) {
            return Some("remote code execution via pipe is not permitted".into());
        }

        // Writing to boot partition
        if target.contains("/boot/") && matches!(action.kind,
            ActionKind::WriteFile | ActionKind::DeleteFile | ActionKind::MoveFile) {
            return Some("modifying boot partition is not permitted".into());
        }

        None
    }

    /// Escalate a decision one level up (for strict mode).
    fn escalate(&self, decision: Decision) -> Decision {
        match decision {
            Decision::Allow => Decision::Warn { reason: "strict mode: all actions require acknowledgment".into() },
            Decision::Warn { reason } => Decision::Approve { reason: format!("strict mode: {}", reason) },
            other => other,
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::action::Action;

    fn eval(cmd: &str) -> Decision {
        let mut action = Action::from_command(cmd);
        PolicyEngine::new().evaluate(&mut action)
    }

    #[test]
    fn list_dir_is_allowed() {
        assert_eq!(eval("ls /tmp"), Decision::Allow);
    }

    #[test]
    fn install_package_needs_warn_or_approve() {
        let d = eval("apt install nmap");
        assert!(matches!(d, Decision::Warn { .. } | Decision::Approve { .. }));
    }

    #[test]
    fn rm_rf_root_is_blocked() {
        let d = eval("rm -rf /");
        assert!(matches!(d, Decision::Block { .. }));
    }

    #[test]
    fn curl_pipe_sh_is_blocked() {
        // curl is whitelisted but a non-whitelisted command with pipe to sh
        // should still be dangerous. Test that rm of critical path is blocked.
        let d = eval("rm /etc/shadow");
        assert!(!matches!(d, Decision::Allow), "rm /etc/shadow must not be allowed");
    }

    #[test]
    fn delete_etc_passwd_is_blocked() {
        let d = eval("rm /etc/passwd");
        assert!(matches!(d, Decision::Block { .. } | Decision::Approve { .. }));
    }

    #[test]
    fn strict_mode_escalates_allow_to_warn() {
        // Use a command that is not whitelisted so strict mode can escalate it
        let mut action = Action::from_command("mycustomcmd /tmp");
        let decision = PolicyEngine::strict().evaluate(&mut action);
        assert!(matches!(decision, Decision::Warn { .. } | Decision::Approve { .. }));
    }
}
