// cash — Output Formatter
//
// Formats tool output in colour with clear structure.
// Every tool's output goes through here before printing.

use crossterm::style::{Color, SetForegroundColor, ResetColor, Attribute, SetAttribute};
use crossterm::QueueableCommand;
use std::io::Write;

pub struct OutputFormatter;

impl OutputFormatter {
    /// Print a success line in green.
    pub fn success(msg: &str) {
        let mut out = std::io::stdout();
        let _ = out.queue(SetForegroundColor(Color::Green));
        let _ = out.queue(SetAttribute(Attribute::Bold));
        print!("  ✓ ");
        let _ = out.queue(ResetColor);
        println!("{}", msg);
        let _ = out.flush();
    }

    /// Print a warning line in yellow.
    pub fn warning(msg: &str) {
        let mut out = std::io::stdout();
        let _ = out.queue(SetForegroundColor(Color::Yellow));
        let _ = out.queue(SetAttribute(Attribute::Bold));
        print!("  ⚠ ");
        let _ = out.queue(ResetColor);
        println!("{}", msg);
        let _ = out.flush();
    }

    /// Print an error line in red.
    pub fn error(msg: &str) {
        let mut out = std::io::stderr();
        let _ = out.queue(SetForegroundColor(Color::Red));
        let _ = out.queue(SetAttribute(Attribute::Bold));
        eprint!("  ✗ ");
        let _ = out.queue(ResetColor);
        eprintln!("{}", msg);
        let _ = out.flush();
    }

    /// Print an info line in cyan.
    pub fn info(msg: &str) {
        let mut out = std::io::stdout();
        let _ = out.queue(SetForegroundColor(Color::Cyan));
        print!("  → ");
        let _ = out.queue(ResetColor);
        println!("{}", msg);
        let _ = out.flush();
    }

    /// Print a section header.
    pub fn header(title: &str) {
        let mut out = std::io::stdout();
        let _ = out.queue(SetForegroundColor(Color::Blue));
        let _ = out.queue(SetAttribute(Attribute::Bold));
        println!("\n  {}", title);
        println!("  {}", "─".repeat(title.len()));
        let _ = out.queue(ResetColor);
        let _ = out.flush();
    }

    /// Print an open port line.
    pub fn open_port(port: u16, service: &str) {
        let mut out = std::io::stdout();
        let _ = out.queue(SetForegroundColor(Color::Green));
        print!("  {:6}", port);
        let _ = out.queue(ResetColor);
        print!("  open   ");
        let _ = out.queue(SetForegroundColor(Color::Cyan));
        println!("{}", service);
        let _ = out.queue(ResetColor);
        let _ = out.flush();
    }

    /// Print a host discovery line.
    pub fn host_found(ip: &str, hostname: &str) {
        let mut out = std::io::stdout();
        let _ = out.queue(SetForegroundColor(Color::Green));
        print!("  ● ");
        let _ = out.queue(ResetColor);
        let _ = out.queue(SetAttribute(Attribute::Bold));
        print!("{}", ip);
        let _ = out.queue(SetAttribute(Attribute::Reset));
        if !hostname.is_empty() && hostname != ip {
            print!("  ({})", hostname);
        }
        println!();
        let _ = out.flush();
    }

    /// Print a vulnerability finding.
    pub fn vulnerability(severity: &str, title: &str, detail: &str) {
        let color = match severity.to_uppercase().as_str() {
            "HIGH"   | "CRITICAL" => Color::Red,
            "MEDIUM"              => Color::Yellow,
            _                     => Color::Cyan,
        };
        let mut out = std::io::stdout();
        let _ = out.queue(SetForegroundColor(color));
        let _ = out.queue(SetAttribute(Attribute::Bold));
        print!("  [{}]", severity.to_uppercase());
        let _ = out.queue(ResetColor);
        print!(" {}", title);
        if !detail.is_empty() {
            print!(" — {}", detail);
        }
        println!();
        let _ = out.flush();
    }

    /// Format raw tool output line by line with basic colouring.
    pub fn raw_output(output: &str, tool: &str) {
        for line in output.lines() {
            let lower = line.to_lowercase();
            if lower.contains("open") || lower.contains("found") || lower.contains("success") {
                Self::success(line.trim());
            } else if lower.contains("error") || lower.contains("failed") || lower.contains("denied") {
                Self::error(line.trim());
            } else if lower.contains("warning") || lower.contains("warn") {
                Self::warning(line.trim());
            } else if !line.trim().is_empty() {
                println!("  {}", line.trim());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatter_methods_dont_panic() {
        // Just verify none of these panic
        OutputFormatter::info("test info");
        OutputFormatter::warning("test warning");
        OutputFormatter::header("Test Section");
        OutputFormatter::open_port(80, "HTTP");
        OutputFormatter::host_found("192.168.1.1", "router.local");
        OutputFormatter::vulnerability("HIGH", "SQL Injection", "login form");
    }
}
