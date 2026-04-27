use super::super::pipeline::Stage;

pub fn run(stage: &Stage) {
    let cmd = match stage.args.first() {
        Some(c) => c.as_str(),
        None => {
            println!("Usage: explain <command>");
            println!("Example: explain rm");
            return;
        }
    };
    println!();
    explain_command(cmd);
    println!();
}

fn explain_command(cmd: &str) {
    match find_entry(cmd) {
        Some(e) => {
            println!("  Command     : {}", e.name);
            println!("  What it does: {}", e.description);
            println!("  Risk level  : {}", risk_label(e.risk));
            println!("  cash policy : {}", policy_label(e.risk));
            if let Some(w) = e.warning {
                println!();
                println!("  ⚠  {}", w);
            }
        }
        None => {
            println!("  Command     : {}", cmd);
            println!("  What it does: Unknown — not in cash knowledge base.");
            println!("  Risk level  : UNKNOWN");
            println!("  cash policy : Will ask for approval before running.");
            println!();
            println!("  Tip: teach {} = <real command>", cmd);
        }
    }
}

fn find_entry(cmd: &str) -> Option<Entry> {
    knowledge_base().into_iter().find(|e| e.name == cmd)
}

fn risk_label(score: u8) -> &'static str {
    match score {
        0..=2 => "LOW      (safe)",
        3..=5 => "MEDIUM   (use with care)",
        6..=8 => "HIGH     (cash will warn you)",
        _     => "CRITICAL (blocked by default)",
    }
}

fn policy_label(score: u8) -> &'static str {
    match score {
        0..=2 => "✓ ALLOW   — runs silently",
        3..=5 => "⚠ WARN    — runs with a warning",
        6..=8 => "⚠ APPROVE — cash will ask first",
        _     => "✗ BLOCK   — requires explicit approval",
    }
}

struct Entry {
    name: &'static str,
    description: &'static str,
    risk: u8,
    warning: Option<&'static str>,
}

fn knowledge_base() -> Vec<Entry> {
    vec![
        Entry { name: "rm",       description: "Remove files or directories.",                          risk: 6,  warning: Some("rm -rf / is permanently blocked.") },
        Entry { name: "ls",       description: "List directory contents.",                              risk: 0,  warning: None },
        Entry { name: "cd",       description: "Change the current working directory.",                 risk: 0,  warning: None },
        Entry { name: "cat",      description: "Print file contents to the terminal.",                  risk: 1,  warning: None },
        Entry { name: "curl",     description: "Transfer data from a URL.",                             risk: 5,  warning: Some("curl | sh is permanently blocked.") },
        Entry { name: "wget",     description: "Download files from the internet.",                     risk: 5,  warning: None },
        Entry { name: "chmod",    description: "Change file permissions.",                              risk: 7,  warning: Some("chmod 777 on system files is blocked.") },
        Entry { name: "sudo",     description: "Run a command as superuser.",                           risk: 9,  warning: Some("All sudo commands require approval.") },
        Entry { name: "ssh",      description: "Connect to a remote machine securely.",                 risk: 4,  warning: None },
        Entry { name: "git",      description: "Version control system.",                               risk: 1,  warning: None },
        Entry { name: "cargo",    description: "Rust build tool and package manager.",                  risk: 1,  warning: None },
        Entry { name: "docker",   description: "Run and manage containers.",                            risk: 5,  warning: Some("Review volume mounts carefully.") },
        Entry { name: "kill",     description: "Send a signal to a process.",                           risk: 6,  warning: Some("kill -9 force-kills with no cleanup.") },
        Entry { name: "dd",       description: "Low-level raw data copy. Can write to disk devices.",   risk: 10, warning: Some("dd pointed at /dev/sda will wipe your disk.") },
        Entry { name: "nmap",     description: "Network scanner — discovers hosts and open ports.",     risk: 7,  warning: Some("Scanning networks you don't own may be illegal.") },
        Entry { name: "nc",       description: "Netcat — reads/writes across network connections.",     risk: 8,  warning: Some("Can open reverse shells.") },
        Entry { name: "grep",     description: "Search for text patterns in files.",                    risk: 0,  warning: None },
        Entry { name: "find",     description: "Search for files and directories.",                     risk: 1,  warning: None },
        Entry { name: "cp",       description: "Copy files and directories.",                           risk: 1,  warning: None },
        Entry { name: "mv",       description: "Move or rename files.",                                 risk: 2,  warning: None },
        Entry { name: "tar",      description: "Archive and compress files.",                           risk: 1,  warning: None },
        Entry { name: "apt",      description: "Package manager — install/remove software.",            risk: 6,  warning: Some("Modifies system software. Requires sudo.") },
        Entry { name: "systemctl",description: "Control system services.",                              risk: 7,  warning: Some("Stopping critical services can break your machine.") },
        Entry { name: "python3",  description: "Run Python 3 scripts.",                                 risk: 3,  warning: None },
        Entry { name: "node",     description: "Run JavaScript with Node.js.",                          risk: 3,  warning: None },
        Entry { name: "ping",     description: "Check network connectivity to a host.",                 risk: 0,  warning: None },
        Entry { name: "ps",       description: "Show running processes.",                               risk: 0,  warning: None },
        Entry { name: "top",      description: "Interactive view of processes and resources.",          risk: 0,  warning: None },
        Entry { name: "echo",     description: "Print text to the terminal.",                           risk: 0,  warning: None },
        Entry { name: "mkdir",    description: "Create a new directory.",                               risk: 0,  warning: None },
        Entry { name: "whoami",   description: "Print the current logged-in username.",                 risk: 0,  warning: None },
        Entry { name: "man",      description: "Show the manual page for a command.",                   risk: 0,  warning: None },
        Entry { name: "history",  description: "Show previously run commands.",                         risk: 0,  warning: None },
        Entry { name: "uname",    description: "Print system information.",                             risk: 0,  warning: None },
        Entry { name: "df",       description: "Show disk space usage.",                                risk: 0,  warning: None },
        Entry { name: "du",       description: "Show disk usage of files and directories.",             risk: 0,  warning: None },
        Entry { name: "awk",      description: "Pattern scanning and text processing.",                 risk: 2,  warning: None },
        Entry { name: "sed",      description: "Find and replace text in files or output.",             risk: 2,  warning: None },
        Entry { name: "xargs",    description: "Build and run commands from standard input.",           risk: 4,  warning: Some("xargs rm can delete many files — check output first.") },
    ]
}
