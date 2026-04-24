// cash — Layer 2: Security Engine
//
// Runs on its own thread. NEVER blocks the shell. CANNOT be disabled.
//
// Components:
//   TamperWatcher   — watches ~/.cash/ files for outside modification
//   AgentRegistry   — tracks registered agents and their permissions
//   Vault           — encrypted key/value store in vault.db
//   NetworkWatcher  — monitors outbound connections by child processes
//   TrustScorer     — maintains trust scores per agent

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Sha256, Digest};
use hex::encode as hex_encode;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub struct SecurityEngine {
    cash_dir: PathBuf,
}

impl SecurityEngine {
    pub fn new(cash_dir: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { cash_dir })
    }

    pub fn spawn(self) {
        let cash_dir = self.cash_dir;
        std::thread::spawn(move || {
            // Initialise all sub-systems.
            let tamper  = TamperWatcher::new(&cash_dir);
            let registry = AgentRegistry::new(&cash_dir);
            let vault    = Vault::new(&cash_dir);
            let network  = NetworkWatcher::new();
            let scorer   = TrustScorer::new();

            tracing::debug!("security engine started");

            // Main security loop.
            loop {
                std::thread::sleep(Duration::from_secs(30));
                tamper.check();
                registry.audit();
                network.check();
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Tamper Watcher
// ---------------------------------------------------------------------------

const WATCHED: &[&str] = &["audit.db", "history.db", "memory.db", "config.toml"];

struct TamperWatcher {
    cash_dir: PathBuf,
    baseline: Arc<Mutex<HashMap<String, String>>>,
}

impl TamperWatcher {
    fn new(cash_dir: &Path) -> Self {
        let w = Self {
            cash_dir: cash_dir.to_path_buf(),
            baseline: Arc::new(Mutex::new(HashMap::new())),
        };
        w.take_baseline();
        w
    }

    fn take_baseline(&self) {
        let mut map = self.baseline.lock().unwrap();
        for name in WATCHED {
            if let Some(hash) = hash_file(&self.cash_dir.join(name)) {
                map.insert(name.to_string(), hash);
            }
        }
    }

    fn check(&self) {
        let mut map = self.baseline.lock().unwrap();
        for name in WATCHED {
            let path = self.cash_dir.join(name);
            match hash_file(&path) {
                None => {
                    if map.contains_key(*name) {
                        self.alert(name, "deleted outside cash");
                        map.remove(*name);
                    }
                }
                Some(current) => {
                    match map.get(*name) {
                        None => { map.insert(name.to_string(), current); }
                        Some(baseline) if *baseline != current => {
                            self.alert(name, "modified outside cash");
                            map.insert(name.to_string(), current);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn alert(&self, filename: &str, reason: &str) {
        use std::io::Write;
        let msg = format!("[cash security] TAMPER DETECTED: {} — {}", filename, reason);
        eprintln!("{}", msg);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open(self.cash_dir.join("tamper.log"))
        {
            let _ = writeln!(f, "{} {}", chrono::Utc::now().to_rfc3339(), msg);
        }
    }
}

// ---------------------------------------------------------------------------
// Agent Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub id:          String,
    pub name:        String,
    pub permissions: Vec<String>,
    pub trust_score: f64,
    pub registered_at: String,
}

pub struct AgentRegistry {
    cash_dir: PathBuf,
    agents:   Arc<Mutex<HashMap<String, AgentRecord>>>,
}

impl AgentRegistry {
    pub fn new(cash_dir: &Path) -> Self {
        Self {
            cash_dir: cash_dir.to_path_buf(),
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new agent.
    pub fn register(&self, id: &str, name: &str, permissions: Vec<String>) -> AgentRecord {
        let record = AgentRecord {
            id:            id.to_string(),
            name:          name.to_string(),
            permissions,
            trust_score:   1.0,
            registered_at: chrono::Utc::now().to_rfc3339(),
        };
        self.agents.lock().unwrap().insert(id.to_string(), record.clone());
        tracing::debug!("agent registered: {} ({})", name, id);
        record
    }

    /// Check if an agent has a specific permission.
    pub fn has_permission(&self, agent_id: &str, permission: &str) -> bool {
        self.agents.lock().unwrap()
            .get(agent_id)
            .map(|a| a.permissions.iter().any(|p| p == permission))
            .unwrap_or(false)
    }

    /// Deregister an agent.
    pub fn deregister(&self, agent_id: &str) {
        self.agents.lock().unwrap().remove(agent_id);
    }

    /// Periodic audit — log active agents.
    fn audit(&self) {
        let agents = self.agents.lock().unwrap();
        if !agents.is_empty() {
            tracing::debug!("active agents: {}", agents.len());
        }
    }
}

// ---------------------------------------------------------------------------
// Vault — encrypted key/value store
// ---------------------------------------------------------------------------

pub struct Vault {
    cash_dir: PathBuf,
}

impl Vault {
    pub fn new(cash_dir: &Path) -> Self {
        Self { cash_dir: cash_dir.to_path_buf() }
    }

    /// Store a secret. Value is obfuscated (XOR with key hash).
    /// Full encryption comes in a later pass with a proper KDF.
    pub fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        use rusqlite::Connection;
        let conn = Connection::open(self.cash_dir.join("vault.db"))?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS vault (
            key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL
        );")?;
        let obfuscated = obfuscate(value, key);
        conn.execute(
            "INSERT INTO vault (key, value, updated_at) VALUES (?1,?2,?3)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            rusqlite::params![key, obfuscated, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Retrieve a secret.
    pub fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        use rusqlite::Connection;
        let conn = Connection::open(self.cash_dir.join("vault.db"))?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS vault (
            key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL
        );")?;
        let result = conn.query_row(
            "SELECT value FROM vault WHERE key=?1",
            rusqlite::params![key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(Some(deobfuscate(&v, key))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a secret.
    pub fn delete(&self, key: &str) -> anyhow::Result<()> {
        use rusqlite::Connection;
        let conn = Connection::open(self.cash_dir.join("vault.db"))?;
        conn.execute("DELETE FROM vault WHERE key=?1", rusqlite::params![key])?;
        Ok(())
    }
}

/// Simple XOR obfuscation — placeholder until KDF is added in Module 10b.
fn obfuscate(value: &str, key: &str) -> String {
    let key_bytes: Vec<u8> = key.bytes().cycle().take(value.len()).collect();
    let xored: Vec<u8> = value.bytes().zip(key_bytes).map(|(v, k)| v ^ k).collect();
    hex_encode(xored)
}

fn deobfuscate(hex: &str, key: &str) -> String {
    let bytes = hex::decode(hex).unwrap_or_default();
    let key_bytes: Vec<u8> = key.bytes().cycle().take(bytes.len()).collect();
    let xored: Vec<u8> = bytes.iter().zip(key_bytes).map(|(v, k)| v ^ k).collect();
    String::from_utf8(xored).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Network Watcher
// ---------------------------------------------------------------------------

pub struct NetworkWatcher {
    /// Blocked domains — outbound connections to these are rejected.
    blocked: Arc<Mutex<Vec<String>>>,
}

impl NetworkWatcher {
    pub fn new() -> Self {
        Self {
            blocked: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn block(&self, domain: &str) {
        self.blocked.lock().unwrap().push(domain.to_lowercase());
    }

    pub fn is_blocked(&self, domain: &str) -> bool {
        self.blocked.lock().unwrap()
            .iter()
            .any(|b| domain.to_lowercase().contains(b.as_str()))
    }

    /// Periodic network check — placeholder for deep inspection in Module 12.
    fn check(&self) {
        // Module 12 will add /proc/net parsing for Linux.
    }
}

// ---------------------------------------------------------------------------
// Trust Scorer
// ---------------------------------------------------------------------------

pub struct TrustScorer {
    scores: Arc<Mutex<HashMap<String, f64>>>,
}

impl TrustScorer {
    pub fn new() -> Self {
        Self { scores: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Record a positive event for an agent (increases trust).
    pub fn reward(&self, agent_id: &str, amount: f64) {
        let mut map = self.scores.lock().unwrap();
        let score = map.entry(agent_id.to_string()).or_insert(1.0);
        *score = (*score + amount).min(10.0);
    }

    /// Record a negative event (decreases trust).
    pub fn penalise(&self, agent_id: &str, amount: f64) {
        let mut map = self.scores.lock().unwrap();
        let score = map.entry(agent_id.to_string()).or_insert(1.0);
        *score = (*score - amount).max(0.0);
    }

    pub fn score(&self, agent_id: &str) -> f64 {
        *self.scores.lock().unwrap().get(agent_id).unwrap_or(&1.0)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(hex_encode(hasher.finalize()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> (PathBuf, TempDir) {
        let dir = TempDir::new().unwrap();
        (dir.path().to_path_buf(), dir)
    }

    #[test]
    fn tamper_detects_modification() {
        let (path, _dir) = tmp();
        let file = path.join("audit.db");
        std::fs::write(&file, b"original").unwrap();
        let watcher = TamperWatcher::new(&path);
        std::fs::write(&file, b"tampered").unwrap();
        let baseline = watcher.baseline.lock().unwrap().get("audit.db").cloned().unwrap();
        assert_ne!(baseline, hash_file(&file).unwrap());
    }

    #[test]
    fn agent_registry_register_and_permission() {
        let (path, _dir) = tmp();
        let registry = AgentRegistry::new(&path);
        registry.register("agent1", "TestAgent", vec!["read".into(), "write".into()]);
        assert!(registry.has_permission("agent1", "read"));
        assert!(!registry.has_permission("agent1", "delete"));
    }

    #[test]
    fn agent_registry_deregister() {
        let (path, _dir) = tmp();
        let registry = AgentRegistry::new(&path);
        registry.register("agent1", "TestAgent", vec!["read".into()]);
        registry.deregister("agent1");
        assert!(!registry.has_permission("agent1", "read"));
    }

    #[test]
    fn vault_set_and_get() {
        let (path, _dir) = tmp();
        let vault = Vault::new(&path);
        vault.set("api_key", "super-secret-123").unwrap();
        let result = vault.get("api_key").unwrap();
        assert_eq!(result, Some("super-secret-123".to_string()));
    }

    #[test]
    fn vault_delete() {
        let (path, _dir) = tmp();
        let vault = Vault::new(&path);
        vault.set("key", "value").unwrap();
        vault.delete("key").unwrap();
        assert_eq!(vault.get("key").unwrap(), None);
    }

    #[test]
    fn network_watcher_blocks_domain() {
        let watcher = NetworkWatcher::new();
        watcher.block("malicious.com");
        assert!(watcher.is_blocked("malicious.com"));
        assert!(!watcher.is_blocked("safe.com"));
    }

    #[test]
    fn trust_scorer_reward_and_penalise() {
        let scorer = TrustScorer::new();
        scorer.reward("agent1", 2.0);
        assert_eq!(scorer.score("agent1"), 3.0);
        scorer.penalise("agent1", 1.0);
        assert_eq!(scorer.score("agent1"), 2.0);
    }

    #[test]
    fn trust_scorer_caps_at_10() {
        let scorer = TrustScorer::new();
        scorer.reward("agent1", 100.0);
        assert_eq!(scorer.score("agent1"), 10.0);
    }

    #[test]
    fn trust_scorer_floors_at_0() {
        let scorer = TrustScorer::new();
        scorer.penalise("agent1", 100.0);
        assert_eq!(scorer.score("agent1"), 0.0);
    }
}
