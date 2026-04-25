// cash — AI Intent Engine
//
// Understands natural language and maps it to actions.
// "set up my router"  → router_setup intent → step sequence
// "check for hackers" → security_audit intent → tool sequence
// "install nmap"      → tool_install intent → installer
//
// Uses a local intent classifier first (fast, no API needed).
// Falls back to AI API for complex/unknown intents.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    // Network
    ScanNetwork { target: String },
    ScanPorts   { target: String, ports: Option<String> },
    TraceRoute  { target: String },
    DnsLookup   { target: String },
    CheckSsl    { target: String },
    HttpInspect { target: String },

    // Router
    RouterSetup,
    RouterScan,
    RouterAudit,

    // Security
    SecurityAudit,
    CheckVulnerabilities { target: String },
    MonitorProcesses,
    CheckConnections,

    // Tools
    InstallTool  { name: String },
    ListTools,
    UpdateTools,

    // Shell
    ListFiles    { path: Option<String> },
    ChangeDir    { path: String },
    ShowHelp     { topic: Option<String> },
    TeachCommand { name: String, command: String },

    // Unknown — needs AI API
    Unknown { input: String },
}

#[derive(Debug, Clone)]
pub struct IntentResult {
    pub intent:     Intent,
    pub confidence: f64,
    pub steps:      Vec<String>,
    pub suggestion: Option<String>,
}

pub struct IntentEngine {
    patterns: Vec<IntentPattern>,
}

struct IntentPattern {
    keywords: Vec<&'static str>,
    intent:   fn(&str) -> Intent,
    steps:    Vec<&'static str>,
}

impl IntentEngine {
    pub fn new() -> Self {
        Self { patterns: build_patterns() }
    }

    /// Classify input and return intent with action steps.
    pub fn classify(&self, input: &str) -> IntentResult {
        let lower = input.trim().to_lowercase();

        // Try each pattern.
        for pattern in &self.patterns {
            let matches = pattern.keywords.iter()
                .filter(|&&kw| lower.contains(kw))
                .count();

            if matches > 0 {
                let confidence = matches as f64 / pattern.keywords.len() as f64;
                let intent = (pattern.intent)(input);
                let steps = pattern.steps.iter().map(|s| s.to_string()).collect();
                return IntentResult { intent, confidence, steps, suggestion: None };
            }
        }

        // No pattern matched — return Unknown for AI API.
        IntentResult {
            intent:     Intent::Unknown { input: input.to_string() },
            confidence: 0.0,
            steps:      vec![],
            suggestion: Some(format!("Try: learn scan | cash install nmap | teach {} '<command>'", input)),
        }
    }

    /// Execute an intent — print steps and run actions.
    pub fn explain(&self, result: &IntentResult) {
        match &result.intent {
            Intent::Unknown { input } => {
                println!("cash: I don't know how to '{}'", input);
                if let Some(s) = &result.suggestion {
                    println!("cash: {}", s);
                }
            }
            _ => {
                println!("cash: I understand — here's what I'll do:\n");
                for (i, step) in result.steps.iter().enumerate() {
                    println!("  {}. {}", i + 1, step);
                }
                println!();
            }
        }
    }
}

fn build_patterns() -> Vec<IntentPattern> {
    vec![
        IntentPattern {
            keywords: vec!["scan", "network", "my network"],
            intent:   |_| Intent::ScanNetwork { target: "192.168.1.0/24".into() },
            steps:    vec![
                "Find your local network range",
                "Run nmap to discover live hosts",
                "Show open ports on each host",
                "Identify running services",
            ],
        },
        IntentPattern {
            keywords: vec!["router", "setup", "configure"],
            intent:   |_| Intent::RouterSetup,
            steps:    vec![
                "Find router IP on your network",
                "Check router admin panel is accessible",
                "Scan router for open ports",
                "Check for default credentials",
                "Check firmware version",
            ],
        },
        IntentPattern {
            keywords: vec!["router", "audit", "secure", "check"],
            intent:   |_| Intent::RouterAudit,
            steps:    vec![
                "Scan router ports with nmap",
                "Check for exposed admin interfaces",
                "Test for default passwords",
                "Check UPnP exposure",
                "Report security issues found",
            ],
        },
        IntentPattern {
            keywords: vec!["hacker", "intrusion", "breach", "attack", "check for"],
            intent:   |_| Intent::SecurityAudit,
            steps:    vec![
                "Check process monitor for suspicious activity",
                "Review network connections log",
                "Check audit.db for unusual commands",
                "Scan for unexpected open ports",
                "Review tamper detection log",
            ],
        },
        IntentPattern {
            keywords: vec!["install", "nmap"],
            intent:   |_| Intent::InstallTool { name: "nmap".into() },
            steps:    vec!["Detect package manager", "Install nmap", "Verify binary", "Register in cash"],
        },
        IntentPattern {
            keywords: vec!["install", "nikto"],
            intent:   |_| Intent::InstallTool { name: "nikto".into() },
            steps:    vec!["Detect package manager", "Install nikto", "Verify binary", "Register in cash"],
        },
        IntentPattern {
            keywords: vec!["vulnerability", "vuln", "cve"],
            intent:   |input| {
                let target = extract_target(input).unwrap_or("localhost".into());
                Intent::CheckVulnerabilities { target }
            },
            steps:    vec![
                "Run nikto web vulnerability scan",
                "Check for known CVEs",
                "Test common misconfigurations",
                "Report findings with severity",
            ],
        },
        IntentPattern {
            keywords: vec!["ssl", "certificate", "https", "tls"],
            intent:   |input| {
                let target = extract_target(input).unwrap_or("localhost".into());
                Intent::CheckSsl { target }
            },
            steps:    vec![
                "Connect to target on port 443",
                "Inspect certificate details",
                "Check expiry date",
                "Check cipher strength",
            ],
        },
        IntentPattern {
            keywords: vec!["list", "tools", "installed"],
            intent:   |_| Intent::ListTools,
            steps:    vec!["Query tools registry", "Show installed tools and versions"],
        },
        IntentPattern {
            keywords: vec!["monitor", "process", "watch"],
            intent:   |_| Intent::MonitorProcesses,
            steps:    vec![
                "Start process monitor",
                "Show all running processes",
                "Flag suspicious activity",
            ],
        },
    ]
}

fn extract_target(input: &str) -> Option<String> {
    // Simple extraction — last token that looks like a host/IP
    input.split_whitespace()
        .filter(|w| w.contains('.') || w.contains(':'))
        .last()
        .map(|s| s.to_string())
}

impl Default for IntentEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> IntentEngine { IntentEngine::new() }

    #[test]
    fn classifies_scan_network() {
        let e = engine();
        let r = e.classify("scan my network");
        assert!(matches!(r.intent, Intent::ScanNetwork { .. }));
        assert!(r.confidence > 0.0);
    }

    #[test]
    fn classifies_router_setup() {
        let e = engine();
        let r = e.classify("set up my router");
        assert!(matches!(r.intent, Intent::RouterSetup));
    }

    #[test]
    fn classifies_security_audit() {
        let e = engine();
        let r = e.classify("hacker intrusion detected");
        assert!(matches!(r.intent, Intent::SecurityAudit));
    }

    #[test]
    fn classifies_install_nmap() {
        let e = engine();
        let r = e.classify("install nmap");
        assert!(matches!(r.intent, Intent::InstallTool { .. }));
    }

    #[test]
    fn unknown_intent_has_suggestion() {
        let e = engine();
        let r = e.classify("make me a sandwich");
        assert!(matches!(r.intent, Intent::Unknown { .. }));
        assert!(r.suggestion.is_some());
    }

    #[test]
    fn intent_has_steps() {
        let e = engine();
        let r = e.classify("scan my network");
        assert!(!r.steps.is_empty());
    }

    #[test]
    fn router_audit_has_steps() {
        let e = engine();
        let r = e.classify("audit my router");
        assert!(!r.steps.is_empty());
    }
}
