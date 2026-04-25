// cash — Tool Runner
//
// Runs installed tools with natural language commands.
// Wraps raw output through the colour formatter.
// Every run is logged to the audit trail.

use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

use super::formatter::OutputFormatter;
use super::registry::ToolRegistry;
use crate::store::Store;

pub struct ToolRunner {
    registry: ToolRegistry,
}

impl ToolRunner {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        Ok(Self { registry: ToolRegistry::new(store)? })
    }

    /// Run nmap scan — `scan <target> [--ports p1,p2]`
    pub fn scan(&self, target: &str, ports: Option<&str>) {
        if !self.check_installed("nmap", "scan") { return; }

        OutputFormatter::header(&format!("Scanning {}", target));
        OutputFormatter::info("Tool: nmap");
        println!();

        let mut args = vec!["-sV", "--open", "-T4", target];
        let port_arg;
        if let Some(p) = ports {
            port_arg = format!("-p{}", p);
            args.push(&port_arg);
        }

        self.run_streaming("nmap", &args);
    }

    /// Run nikto web vulnerability scan — `vuln <url>`
    pub fn vuln(&self, target: &str) {
        if !self.check_installed("nikto", "vuln") { return; }

        OutputFormatter::header(&format!("Vulnerability scan: {}", target));
        OutputFormatter::info("Tool: nikto");
        OutputFormatter::warning("This may take several minutes...");
        println!();

        self.run_streaming("nikto", &["-h", target]);
    }

    /// Run gobuster directory scan — `dirs <url>`
    pub fn dirs(&self, target: &str, wordlist: Option<&str>) {
        if !self.check_installed("gobuster", "dirs") { return; }

        let wl = wordlist.unwrap_or("/usr/share/wordlists/dirb/common.txt");
        OutputFormatter::header(&format!("Directory scan: {}", target));
        OutputFormatter::info(&format!("Tool: gobuster | Wordlist: {}", wl));
        println!();

        self.run_streaming("gobuster", &["dir", "-u", target, "-w", wl]);
    }

    /// Run curl HTTP header inspection — `http <url>`
    pub fn http(&self, url: &str) {
        if !self.check_installed("curl", "http") { return; }

        OutputFormatter::header(&format!("HTTP headers: {}", url));
        println!();

        self.run_streaming("curl", &["-I", "-L", "--max-time", "10", url]);
    }

    /// Run SSL certificate check — `ssl <host>`
    pub fn ssl(&self, host: &str) {
        if !self.check_installed("openssl", "ssl") { return; }

        OutputFormatter::header(&format!("SSL certificate: {}", host));
        println!();

        let cmd = format!("echo | openssl s_client -connect {}:443 -servername {} 2>/dev/null | openssl x509 -noout -text", host, host);
        let _ = Command::new("sh").arg("-c").arg(&cmd).status();
    }

    /// Run whois lookup.
    pub fn whois_run(&self, target: &str) {
        if !self.check_installed("whois", "whois") { return; }

        OutputFormatter::header(&format!("WHOIS: {}", target));
        println!();

        self.run_streaming("whois", &[target]);
    }

    /// Check if a tool is installed, print helpful message if not.
    fn check_installed(&self, tool: &str, command: &str) -> bool {
        if self.registry.is_installed(tool) {
            return true;
        }
        OutputFormatter::error(&format!("{} is not installed", tool));
        OutputFormatter::info(&format!("Install it with: cash install {}", tool));
        false
    }

    /// Run a command and stream output line by line through formatter.
    fn run_streaming(&self, binary: &str, args: &[&str]) {
        let mut child = match Command::new(binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                OutputFormatter::error(&format!("failed to run {}: {}", binary, e));
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().filter_map(|l| l.ok()) {
                OutputFormatter::raw_output(&line, binary);
            }
        }

        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn runner_initialises() {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        let _runner = ToolRunner::new(store).unwrap();
    }

    #[test]
    fn check_installed_false_when_not_registered() {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        let runner = ToolRunner::new(store).unwrap();
        assert!(!runner.check_installed("nmap", "scan"));
    }
}
