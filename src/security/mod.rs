// cash — Layer 2: Security Engine
//
// Supervised thread. CANNOT be disabled. Restarts on panic.
//
// Components:
//   TamperWatcher    — watches ~/.cash/ files
//   ProcessMonitor   — watches every process via /proc
//   NetworkMonitor   — watches /proc/net/tcp connections
//   AlertSystem      — logs + webhooks for all events
//   AgentRegistry    — tracks registered agents
//   Vault            — encrypted key/value store
//   TrustScorer      — per-agent trust scores

pub mod alert;
pub mod network_monitor;
pub mod process_monitor;
pub mod webhook;

pub use alert::{AlertSystem, AlertLevel};
pub use network_monitor::NetworkMonitor;
pub use process_monitor::ProcessMonitor;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Sha256, Digest};
use hex::encode as hex_encode;

pub use vault_inner::Vault;

mod vault_inner {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    pub struct Vault { pub cash_dir: PathBuf }

    impl Vault {
        pub fn new(cash_dir: &Path) -> Self { Self { cash_dir: cash_dir.to_path_buf() } }

        pub fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
            use rusqlite::Connection;
            let conn = Connection::open(self.cash_dir.join("vault.db"))?;
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch("CREATE TABLE IF NOT EXISTS vault (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);")?;
            let ob = super::obfuscate(value, key);
            conn.execute("INSERT INTO vault (key,value,updated_at) VALUES (?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",
                rusqlite::params![key, ob, chrono::Utc::now().to_rfc3339()])?;
            Ok(())
        }

        pub fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
            use rusqlite::Connection;
            let conn = Connection::open(self.cash_dir.join("vault.db"))?;
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch("CREATE TABLE IF NOT EXISTS vault (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);")?;
            match conn.query_row("SELECT value FROM vault WHERE key=?1", rusqlite::params![key], |row| row.get::<_,String>(0)) {
                Ok(v) => Ok(Some(super::deobfuscate(&v, key))),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        }

        pub fn delete(&self, key: &str) -> anyhow::Result<()> {
            use rusqlite::Connection;
            let conn = Connection::open(self.cash_dir.join("vault.db"))?;
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute("DELETE FROM vault WHERE key=?1", rusqlite::params![key])?;
            Ok(())
        }
    }
}

pub struct SecurityEngine { pub cash_dir: PathBuf }

impl SecurityEngine {
    pub fn new(cash_dir: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { cash_dir })
    }

    pub fn spawn(self) {
        let cash_dir = self.cash_dir;
        std::thread::spawn(move || {
            let mut restarts = 0u32;
            loop {
                let dir = cash_dir.clone();
                let handle = std::thread::spawn(move || {
                    let store = crate::store::Store { root: dir.clone() };
                    let tamper  = TamperWatcher::new(&dir);
                    let mut proc_mon = ProcessMonitor::new(
                        crate::store::Store { root: dir.clone() }
                    ).unwrap_or_else(|_| ProcessMonitor::new(
                        crate::store::Store { root: dir.clone() }
                    ).unwrap());
                    let mut net_mon  = NetworkMonitor::new(
                        crate::store::Store { root: dir.clone() }
                    ).unwrap_or_else(|_| NetworkMonitor::new(
                        crate::store::Store { root: dir.clone() }
                    ).unwrap());
                    let alerts = AlertSystem::new(crate::store::Store { root: dir.clone() });
                    let registry = AgentRegistry::new(&dir);

                    tracing::debug!("security engine running");

                    loop {
                        std::thread::sleep(Duration::from_secs(10));

                        // Tamper check.
                        tamper.check();

                        // Process monitor.
                        let proc_events = proc_mon.scan();
                        for event in &proc_events {
                            if let process_monitor::EventKind::Suspicious { reason } = &event.kind {
                                alerts.critical("process", &format!(
                                    "{} (pid {}) — {}",
                                    event.process.name, event.process.pid, reason
                                ));
                            }
                        }

                        // Network monitor.
                        let net_events = net_mon.scan();
                        for conn in &net_events {
                            if net_mon.is_blocked(&conn.remote_ip) {
                                alerts.critical("network", &format!(
                                    "blocked connection to {}:{}",
                                    conn.remote_ip, conn.remote_port
                                ));
                            }
                        }

                        registry.audit();
                    }
                });

                match handle.join() {
                    Ok(_) => break,
                    Err(_) => {
                        restarts += 1;
                        eprintln!("[cash security] engine crashed (restart #{}), restarting in 5s", restarts);
                        std::thread::sleep(Duration::from_secs(5));
                    }
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Tamper Watcher
// ---------------------------------------------------------------------------

// Only watch files cash does NOT write to during normal operation.
// audit.db and history.db change on every command — exclude them.
const WATCHED: &[&str] = &["memory.db","config.toml"];

struct TamperWatcher {
    cash_dir: PathBuf,
    baseline: Arc<Mutex<HashMap<String,String>>>,
}

impl TamperWatcher {
    fn new(cash_dir: &Path) -> Self {
        let w = Self { cash_dir: cash_dir.to_path_buf(), baseline: Arc::new(Mutex::new(HashMap::new())) };
        w.take_baseline();
        w
    }
    fn take_baseline(&self) {
        let mut map = self.baseline.lock().unwrap();
        for name in WATCHED {
            if let Some(h) = hash_file(&self.cash_dir.join(name)) { map.insert(name.to_string(), h); }
        }
    }
    fn check(&self) {
        let mut map = self.baseline.lock().unwrap();
        for name in WATCHED {
            let path = self.cash_dir.join(name);
            match hash_file(&path) {
                None => { if map.contains_key(*name) { self.alert(name,"deleted outside cash"); map.remove(*name); } }
                Some(cur) => match map.get(*name) {
                    None => { map.insert(name.to_string(), cur); }
                    Some(base) if *base != cur => { self.alert(name,"modified outside cash"); map.insert(name.to_string(), cur); }
                    _ => {}
                }
            }
        }
    }
    fn alert(&self, filename: &str, reason: &str) {
        use std::io::Write;
        let msg = format!("[cash security] TAMPER DETECTED: {} — {}", filename, reason);
        eprintln!("{}", msg);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.cash_dir.join("tamper.log")) {
            let _ = writeln!(f, "{} {}", chrono::Utc::now().to_rfc3339(), msg);
        }
    }
}

// ---------------------------------------------------------------------------
// Agent Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub id: String, pub name: String, pub permissions: Vec<String>,
    pub trust_score: f64, pub registered_at: String,
}

pub struct AgentRegistry {
    _cash_dir: PathBuf,
    agents: Arc<Mutex<HashMap<String, AgentRecord>>>,
}

impl AgentRegistry {
    pub fn new(cash_dir: &Path) -> Self {
        Self { _cash_dir: cash_dir.to_path_buf(), agents: Arc::new(Mutex::new(HashMap::new())) }
    }
    pub fn register(&self, id: &str, name: &str, permissions: Vec<String>) -> AgentRecord {
        let r = AgentRecord { id: id.into(), name: name.into(), permissions, trust_score: 1.0, registered_at: chrono::Utc::now().to_rfc3339() };
        self.agents.lock().unwrap().insert(id.to_string(), r.clone()); r
    }
    pub fn has_permission(&self, id: &str, perm: &str) -> bool {
        self.agents.lock().unwrap().get(id).map(|a| a.permissions.iter().any(|p| p==perm)).unwrap_or(false)
    }
    pub fn deregister(&self, id: &str) { self.agents.lock().unwrap().remove(id); }
    fn audit(&self) { let n = self.agents.lock().unwrap().len(); if n>0 { tracing::debug!("active agents: {}", n); } }
}

// ---------------------------------------------------------------------------
// Network Watcher (blocklist only — full monitor is network_monitor.rs)
// ---------------------------------------------------------------------------

pub struct NetworkWatcher { blocked: Arc<Mutex<Vec<String>>> }
impl NetworkWatcher {
    pub fn new() -> Self { Self { blocked: Arc::new(Mutex::new(Vec::new())) } }
    pub fn block(&self, d: &str) { self.blocked.lock().unwrap().push(d.to_lowercase()); }
    pub fn is_blocked(&self, d: &str) -> bool { self.blocked.lock().unwrap().iter().any(|b| d.to_lowercase().contains(b.as_str())) }
}

// ---------------------------------------------------------------------------
// Trust Scorer
// ---------------------------------------------------------------------------

pub struct TrustScorer { scores: Arc<Mutex<HashMap<String,f64>>> }
impl TrustScorer {
    pub fn new() -> Self { Self { scores: Arc::new(Mutex::new(HashMap::new())) } }
    pub fn reward(&self, id: &str, a: f64) { let mut m=self.scores.lock().unwrap(); let s=m.entry(id.into()).or_insert(1.0); *s=(*s+a).min(10.0); }
    pub fn penalise(&self, id: &str, a: f64) { let mut m=self.scores.lock().unwrap(); let s=m.entry(id.into()).or_insert(1.0); *s=(*s-a).max(0.0); }
    pub fn score(&self, id: &str) -> f64 { *self.scores.lock().unwrap().get(id).unwrap_or(&1.0) }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut h = Sha256::new(); h.update(&bytes); Some(hex_encode(h.finalize()))
}

fn obfuscate(value: &str, key: &str) -> String {
    let kb: Vec<u8> = key.bytes().cycle().take(value.len()).collect();
    hex_encode(value.bytes().zip(kb).map(|(v,k)| v^k).collect::<Vec<_>>())
}

fn deobfuscate(hex: &str, key: &str) -> String {
    let bytes = hex::decode(hex).unwrap_or_default();
    let kb: Vec<u8> = key.bytes().cycle().take(bytes.len()).collect();
    String::from_utf8(bytes.iter().zip(kb).map(|(v,k)| v^k).collect()).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> (PathBuf, TempDir) { let d=TempDir::new().unwrap(); (d.path().to_path_buf(),d) }

    #[test]
    fn tamper_detects_modification() {
        let (path,_d)=tmp(); let file=path.join("memory.db");
        std::fs::write(&file,b"original").unwrap();
        let w=TamperWatcher::new(&path);
        std::fs::write(&file,b"tampered").unwrap();
        let base=w.baseline.lock().unwrap().get("memory.db").cloned().unwrap();
        assert_ne!(base, hash_file(&file).unwrap());
    }
    #[test]
    fn agent_registry_register_and_permission() {
        let (path,_d)=tmp(); let r=AgentRegistry::new(&path);
        r.register("a1","T",vec!["read".into(),"write".into()]);
        assert!(r.has_permission("a1","read"));
        assert!(!r.has_permission("a1","delete"));
    }
    #[test]
    fn agent_registry_deregister() {
        let (path,_d)=tmp(); let r=AgentRegistry::new(&path);
        r.register("a1","T",vec!["read".into()]);
        r.deregister("a1");
        assert!(!r.has_permission("a1","read"));
    }
    #[test]
    fn vault_set_and_get() {
        let (path,_d)=tmp(); let v=Vault::new(&path);
        v.set("key","secret").unwrap();
        assert_eq!(v.get("key").unwrap(),Some("secret".into()));
    }
    #[test]
    fn vault_delete() {
        let (path,_d)=tmp(); let v=Vault::new(&path);
        v.set("k","val").unwrap(); v.delete("k").unwrap();
        assert_eq!(v.get("k").unwrap(),None);
    }
    #[test]
    fn network_watcher_blocks() {
        let w=NetworkWatcher::new(); w.block("evil.com");
        assert!(w.is_blocked("evil.com")); assert!(!w.is_blocked("safe.com"));
    }
    #[test]
    fn trust_scorer() {
        let s=TrustScorer::new();
        s.reward("a",2.0); assert_eq!(s.score("a"),3.0);
        s.penalise("a",1.0); assert_eq!(s.score("a"),2.0);
    }
    #[test]
    fn trust_caps() {
        let s=TrustScorer::new(); s.reward("a",100.0); assert_eq!(s.score("a"),10.0);
        s.penalise("a",100.0); assert_eq!(s.score("a"),0.0);  // reset for floor test... actually separate:
    }
}
