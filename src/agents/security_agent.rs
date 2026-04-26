// cash — Security Agent
//
// The first real AI agent in cash.
// Thinks via Anthropic API. Acts via CLI tools.
// Monitors the system, detects threats, explains findings.
//
// Usage inside cash:
//   agent start security    — starts the security agent
//   agent stop security     — stops it
//   agent ask "is my system safe?"  — ask a question

use std::process::Command;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct SecurityAgent {
    pub name:    String,
    pub running: bool,
    api_key:     Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub message:  String,
    pub actions:  Vec<String>,
    pub risk:     String,
}

impl SecurityAgent {
    pub fn new() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok()
            .or_else(|| read_key_from_vault());
        Self {
            name:    "security-agent".into(),
            running: false,
            api_key,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Ask the security agent a question.
    /// Gathers system context first, then asks Claude.
    pub fn ask(&self, question: &str) -> AgentResponse {
        if !self.is_configured() {
            return AgentResponse {
                message: "Security agent needs an API key. Run: agent configure".into(),
                actions: vec![],
                risk:    "UNKNOWN".into(),
            };
        }

        // Gather system context
        let context = self.gather_context();

        // Build prompt
        let prompt = format!(
            "You are a security agent monitoring a Linux system called cash.\n\
             You have access to the following system information:\n\n\
             {}\n\n\
             The user asks: {}\n\n\
             Respond concisely. If there are security concerns, list them clearly.\n\
             Format: RISK: [LOW/MEDIUM/HIGH/CRITICAL]\nAnswer: [your answer]\nActions: [comma-separated list of recommended actions, or 'none']",
            context, question
        );

        match self.call_api(&prompt) {
            Ok(response) => parse_agent_response(&response),
            Err(e) => AgentResponse {
                message: format!("Agent error: {}", e),
                actions: vec![],
                risk:    "UNKNOWN".into(),
            },
        }
    }

    /// Scan the system and report threats.
    pub fn scan(&self) -> AgentResponse {
        self.ask("Scan this system for security threats. Check for suspicious processes, unusual network connections, and any signs of compromise. Be specific about what you find.")
    }

    /// Gather system context for the agent.
    fn gather_context(&self) -> String {
        let mut ctx = String::new();

        // Running processes
        if let Ok(out) = Command::new("ps").args(["aux", "--no-headers"]).output() {
            let procs = String::from_utf8_lossy(&out.stdout);
            let count = procs.lines().count();
            ctx.push_str(&format!("Running processes: {} total\n", count));

            // Flag suspicious ones
            let suspicious: Vec<&str> = procs.lines()
                .filter(|l| l.contains("nmap") || l.contains("metasploit") ||
                            l.contains("netcat") || l.contains("wireshark") ||
                            l.contains("tcpdump"))
                .collect();
            if !suspicious.is_empty() {
                ctx.push_str(&format!("⚠ Suspicious processes: {}\n", suspicious.join(", ")));
            }
        }

        // Network connections
        if let Ok(out) = Command::new("ss").args(["-tuln"]).output() {
            let conns = String::from_utf8_lossy(&out.stdout);
            let listen_count = conns.lines().filter(|l| l.contains("LISTEN")).count();
            ctx.push_str(&format!("Listening ports: {}\n", listen_count));
        }

        // Disk usage
        if let Ok(out) = Command::new("df").args(["-h", "/"]).output() {
            let df = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = df.lines().nth(1) {
                ctx.push_str(&format!("Disk: {}\n", line.trim()));
            }
        }

        // Recent auth log (last 5 lines)
        if let Ok(out) = Command::new("tail").args(["-5", "/var/log/auth.log"]).output() {
            let log = String::from_utf8_lossy(&out.stdout);
            if !log.trim().is_empty() {
                ctx.push_str(&format!("Recent auth log:\n{}\n", log.trim()));
            }
        }

        // Failed login attempts
        if let Ok(out) = Command::new("grep")
            .args(["-c", "Failed", "/var/log/auth.log"])
            .output() {
            let count = String::from_utf8_lossy(&out.stdout).trim().to_string();
            ctx.push_str(&format!("Failed login attempts: {}\n", count));
        }

        ctx.push_str(&format!("Timestamp: {}\n", Utc::now().to_rfc3339()));
        ctx
    }

    /// Call Anthropic API with a prompt.
    fn call_api(&self, prompt: &str) -> Result<String, String> {
        let key = self.api_key.as_ref().ok_or("no API key")?;

        // Build JSON body
        let body = format!(
            r#"{{"model":"claude-haiku-4-5-20251001","max_tokens":500,"messages":[{{"role":"user","content":{}}}]}}"#,
            serde_json::to_string(prompt).map_err(|e| e.to_string())?
        );

        // HTTP POST via Command (curl) — no async needed
        let output = Command::new("curl")
            .args([
                "-s",
                "-X", "POST",
                "https://api.anthropic.com/v1/messages",
                "-H", "Content-Type: application/json",
                "-H", &format!("x-api-key: {}", key),
                "-H", "anthropic-version: 2023-06-01",
                "-d", &body,
            ])
            .output()
            .map_err(|e| e.to_string())?;

        let response = String::from_utf8_lossy(&output.stdout).to_string();

        // Parse response — extract text content
        let val: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| format!("JSON parse error: {} — response: {}", e, &response[..response.len().min(200)]))?;

        val["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("unexpected API response: {}", &response[..response.len().min(200)]))
    }
}

fn parse_agent_response(raw: &str) -> AgentResponse {
    let mut risk    = "LOW".to_string();
    let mut message = raw.to_string();
    let mut actions = vec![];

    for line in raw.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("risk:") {
            risk = line.split(':').nth(1).unwrap_or("LOW").trim().to_uppercase();
        } else if lower.starts_with("answer:") {
            message = line.split(':').nth(1).unwrap_or(raw).trim().to_string();
        } else if lower.starts_with("actions:") {
            let acts = line.split(':').nth(1).unwrap_or("").trim();
            if acts != "none" && !acts.is_empty() {
                actions = acts.split(',').map(|a| a.trim().to_string()).collect();
            }
        }
    }

    AgentResponse { message, actions, risk }
}

fn read_key_from_vault() -> Option<String> {
    // Try to read from ~/.cash/vault.db via cash vault
    // For now, fall back to config file
    let config_path = dirs::home_dir()?.join(".cash").join("config.toml");
    let text = std::fs::read_to_string(config_path).ok()?;
    let val: toml::Value = text.parse().ok()?;
    val.get("agents")?.get("anthropic_key")?.as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_initialises() {
        let agent = SecurityAgent::new();
        assert_eq!(agent.name, "security-agent");
    }

    #[test]
    fn parse_response_extracts_fields() {
        let raw = "RISK: HIGH\nAnswer: Found suspicious process\nActions: kill process, check logs";
        let r = parse_agent_response(raw);
        assert_eq!(r.risk, "HIGH");
        assert!(r.message.contains("suspicious"));
        assert_eq!(r.actions.len(), 2);
    }

    #[test]
    fn parse_response_handles_plain_text() {
        let raw = "System looks clean.";
        let r = parse_agent_response(raw);
        assert_eq!(r.message, "System looks clean.");
    }

    #[test]
    fn unconfigured_agent_returns_helpful_message() {
        // Agent without API key
        let agent = SecurityAgent { name: "test".into(), running: false, api_key: None };
        let r = agent.ask("is my system safe?");
        assert!(r.message.contains("API key"));
    }
}
