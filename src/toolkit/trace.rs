// cash — Traceroute wrapper

use std::process::Command;

pub fn traceroute(host: &str) {
    // Use system traceroute/tracepath command.
    let cmd = if which_exists("traceroute") { "traceroute" }
              else if which_exists("tracepath") { "tracepath" }
              else { eprintln!("trace: install traceroute or tracepath"); return; };

    println!("Tracing route to {}...\n", host);
    let _ = Command::new(cmd).arg(host).status();
}

fn which_exists(cmd: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
    path.split(':').any(|dir| std::path::Path::new(dir).join(cmd).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_exists_for_known_commands() {
        // ls should exist on any Unix system
        assert!(which_exists("ls"));
    }

    #[test]
    fn which_exists_false_for_nonexistent() {
        assert!(!which_exists("zzz_nonexistent_cmd_zzz"));
    }
}
