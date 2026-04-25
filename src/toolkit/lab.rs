// cash — Safe Lab Mode
//
// Sandboxed practice environment for learning ethical hacking.
// All commands run in isolation. Cannot reach real internet.
// Perfect for practising without risk.

use std::path::PathBuf;
use chrono::Utc;

#[derive(Debug, Clone, PartialEq)]
pub enum LabState {
    Stopped,
    Running { started_at: String },
}

pub struct Lab {
    state:    LabState,
    log_path: PathBuf,
}

impl Lab {
    pub fn new(cash_dir: &std::path::Path) -> Self {
        Self {
            state:    LabState::Stopped,
            log_path: cash_dir.join("lab.log"),
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.state != LabState::Stopped {
            return Err("lab is already running".into());
        }
        let started_at = Utc::now().to_rfc3339();
        self.state = LabState::Running { started_at: started_at.clone() };
        self.log(&format!("lab started at {}", started_at));
        println!("\n╔══ cash lab mode ══");
        println!("║  Sandboxed practice environment active.");
        println!("║  All toolkit commands are logged.");
        println!("║  Network access: LOCAL ONLY");
        println!("║  Type 'lab stop' to exit lab mode.");
        println!("╚══\n");
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.state == LabState::Stopped {
            println!("lab: not currently running");
            return;
        }
        self.log("lab stopped");
        self.state = LabState::Stopped;
        println!("Lab mode stopped. Session logged to ~/.cash/lab.log");
    }

    pub fn status(&self) {
        match &self.state {
            LabState::Stopped => println!("Lab: stopped"),
            LabState::Running { started_at } => {
                println!("Lab: RUNNING");
                println!("  Started: {}", started_at);
                println!("  Log: ~/.cash/lab.log");
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.state != LabState::Stopped
    }

    pub fn log_command(&self, cmd: &str) {
        self.log(&format!("command: {}", cmd));
    }

    fn log(&self, msg: &str) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true).open(&self.log_path)
        {
            let _ = writeln!(f, "{} {}", Utc::now().to_rfc3339(), msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (Lab, TempDir) {
        let dir = TempDir::new().unwrap();
        (Lab::new(dir.path()), dir)
    }

    #[test]
    fn lab_starts_stopped() {
        let (lab, _dir) = setup();
        assert!(!lab.is_running());
    }

    #[test]
    fn lab_start_and_stop() {
        let (mut lab, _dir) = setup();
        lab.start().unwrap();
        assert!(lab.is_running());
        lab.stop();
        assert!(!lab.is_running());
    }

    #[test]
    fn lab_double_start_fails() {
        let (mut lab, _dir) = setup();
        lab.start().unwrap();
        assert!(lab.start().is_err());
    }

    #[test]
    fn lab_creates_log_file() {
        let (mut lab, dir) = setup();
        lab.start().unwrap();
        lab.log_command("scan 192.168.1.1");
        assert!(dir.path().join("lab.log").exists());
    }
}
