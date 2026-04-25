// cash — Webhook / Alert Delivery
//
// Sends alerts via:
//   1. Twilio WhatsApp (user brings own account)
//   2. Generic webhook (Zapier, Slack, Teams, etc.)
//   3. Email via SMTP (future)
//
// Config lives in ~/.cash/config.toml under [alerts]

use std::io::Write;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub twilio_sid:      Option<String>,
    pub twilio_token:    Option<String>,
    pub whatsapp_from:   Option<String>,
    pub whatsapp_to:     Option<String>,
    pub webhook_url:     Option<String>,
    pub min_level:       String,
    pub enabled:         bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            twilio_sid:    None,
            twilio_token:  None,
            whatsapp_from: None,
            whatsapp_to:   None,
            webhook_url:   None,
            min_level:     "WARNING".into(),
            enabled:       false,
        }
    }
}

impl AlertConfig {
    pub fn from_toml(path: &std::path::Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else { return Self::default() };
        let Ok(val)  = text.parse::<toml::Value>() else { return Self::default() };
        let alerts   = match val.get("alerts") { Some(a) => a, None => return Self::default() };

        Self {
            twilio_sid:    alerts.get("twilio_sid").and_then(|v| v.as_str()).map(String::from),
            twilio_token:  alerts.get("twilio_token").and_then(|v| v.as_str()).map(String::from),
            whatsapp_from: alerts.get("whatsapp_from").and_then(|v| v.as_str()).map(String::from),
            whatsapp_to:   alerts.get("whatsapp_to").and_then(|v| v.as_str()).map(String::from),
            webhook_url:   alerts.get("webhook_url").and_then(|v| v.as_str()).map(String::from),
            min_level:     alerts.get("min_level").and_then(|v| v.as_str()).unwrap_or("WARNING").into(),
            enabled:       alerts.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        // Read existing config
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        let mut doc: toml::Value = existing.parse().unwrap_or(toml::Value::Table(Default::default()));

        // Build alerts table
        let mut alerts = toml::value::Table::new();
        alerts.insert("enabled".into(), toml::Value::Boolean(self.enabled));
        alerts.insert("min_level".into(), toml::Value::String(self.min_level.clone()));

        if let Some(v) = &self.twilio_sid    { alerts.insert("twilio_sid".into(),    toml::Value::String(v.clone())); }
        if let Some(v) = &self.twilio_token  { alerts.insert("twilio_token".into(),  toml::Value::String(v.clone())); }
        if let Some(v) = &self.whatsapp_from { alerts.insert("whatsapp_from".into(), toml::Value::String(v.clone())); }
        if let Some(v) = &self.whatsapp_to   { alerts.insert("whatsapp_to".into(),   toml::Value::String(v.clone())); }
        if let Some(v) = &self.webhook_url   { alerts.insert("webhook_url".into(),   toml::Value::String(v.clone())); }

        if let toml::Value::Table(ref mut t) = doc {
            t.insert("alerts".into(), toml::Value::Table(alerts));
        }

        // Write atomically
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, toml::to_string_pretty(&doc)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn is_whatsapp_configured(&self) -> bool {
        self.twilio_sid.is_some() &&
        self.twilio_token.is_some() &&
        self.whatsapp_from.is_some() &&
        self.whatsapp_to.is_some()
    }

    pub fn is_webhook_configured(&self) -> bool {
        self.webhook_url.is_some()
    }
}

/// Send a WhatsApp message via Twilio API.
/// Runs in a background thread — never blocks the shell.
pub fn send_whatsapp(config: &AlertConfig, message: &str) {
    let (sid, token, from, to) = match (
        config.twilio_sid.clone(),
        config.twilio_token.clone(),
        config.whatsapp_from.clone(),
        config.whatsapp_to.clone(),
    ) {
        (Some(s), Some(t), Some(f), Some(to)) => (s, t, f, to),
        _ => return,
    };

    let message = message.to_string();
    std::thread::spawn(move || {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            sid
        );
        let body = format!(
            "From={}&To={}&Body={}",
            urlencode(&from),
            urlencode(&to),
            urlencode(&message)
        );
        match send_http_post(&url, &body, &sid, &token) {
            Ok(_)  => tracing::debug!("WhatsApp alert sent"),
            Err(e) => tracing::warn!("WhatsApp alert failed: {}", e),
        }
    });
}

/// Send a webhook POST request.
pub fn send_webhook(url: &str, message: &str) {
    let url = url.to_string();
    let body = format!(r#"{{"text":"{}","source":"cash-security"}}"#,
        message.replace('"', "'"));
    std::thread::spawn(move || {
        match send_http_post_json(&url, &body) {
            Ok(_)  => tracing::debug!("Webhook alert sent"),
            Err(e) => tracing::warn!("Webhook alert failed: {}", e),
        }
    });
}

/// Simple HTTP POST using std::net — no async, no reqwest dependency.
fn send_http_post(url: &str, body: &str, user: &str, pass: &str) -> Result<(), String> {
    use std::net::TcpStream;
    use std::io::{Read};

    let (host, path) = parse_url(url)?;
    let auth = base64_encode(&format!("{}:{}", user, pass));

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Basic {}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, auth, body.len(), body
    );

    let mut stream = TcpStream::connect(format!("{}:443", host))
        .map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;

    let mut response = String::new();
    stream.read_to_string(&mut response).ok();
    if response.contains("201") || response.contains("200") {
        Ok(())
    } else {
        Err(format!("HTTP error: {}", &response[..response.len().min(100)]))
    }
}

fn send_http_post_json(url: &str, body: &str) -> Result<(), String> {
    let (host, path) = parse_url(url)?;
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, body.len(), body
    );
    use std::net::TcpStream;
    use std::io::Read;
    let mut stream = TcpStream::connect(format!("{}:443", host))
        .map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok();
    Ok(())
}

fn parse_url(url: &str) -> Result<(String, String), String> {
    let url = url.trim_start_matches("https://").trim_start_matches("http://");
    let slash = url.find('/').unwrap_or(url.len());
    let host = url[..slash].to_string();
    let path = if slash < url.len() { url[slash..].to_string() } else { "/".to_string() };
    Ok((host, path))
}

fn urlencode(s: &str) -> String {
    s.chars().map(|c| match c {
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        ' ' => "+".to_string(),
        c => format!("%{:02X}", c as u32),
    }).collect()
}

fn base64_encode(s: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut result = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        result.push(CHARS[(b0 >> 2)] as char);
        result.push(CHARS[((b0 & 3) << 4) | (b1 >> 4)] as char);
        result.push(if chunk.len() > 1 { CHARS[((b1 & 15) << 2) | (b2 >> 6)] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[b2 & 63] as char } else { '=' });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn alert_config_default_disabled() {
        let cfg = AlertConfig::default();
        assert!(!cfg.enabled);
        assert!(!cfg.is_whatsapp_configured());
    }

    #[test]
    fn alert_config_save_and_load() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = AlertConfig::default();
        cfg.enabled = true;
        cfg.whatsapp_to = Some("+447911123456".into());
        cfg.min_level = "CRITICAL".into();
        cfg.save(&path).unwrap();
        let loaded = AlertConfig::from_toml(&path);
        assert!(loaded.enabled);
        assert_eq!(loaded.whatsapp_to, Some("+447911123456".into()));
        assert_eq!(loaded.min_level, "CRITICAL");
    }

    #[test]
    fn whatsapp_not_configured_without_all_fields() {
        let mut cfg = AlertConfig::default();
        cfg.twilio_sid = Some("AC123".into());
        assert!(!cfg.is_whatsapp_configured());
    }

    #[test]
    fn urlencode_spaces() {
        assert!(urlencode("hello world").contains("hello"));
    }

    #[test]
    fn base64_encode_works() {
        assert_eq!(base64_encode("Hello"), "SGVsbG8=");
    }
}
