use rusqlite::{Connection, params};
use chrono::Utc;
use sha2::{Sha256, Digest};
use hex::encode as hex_encode;
use super::Store;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub id: i64,
    pub session_id: String,
    pub command: String,
    pub cwd: String,
    pub exit_code: i32,
    pub kind: String,
    pub hash: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandKind { Builtin, Resolved, System, Unknown }

impl CommandKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin  => "builtin",
            Self::Resolved => "resolved",
            Self::System   => "system",
            Self::Unknown  => "unknown",
        }
    }
}

pub fn init(store: &Store) -> anyhow::Result<()> {
    let conn = open(store)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            command TEXT NOT NULL,
            cwd TEXT NOT NULL DEFAULT '',
            exit_code INTEGER NOT NULL DEFAULT 0,
            kind TEXT NOT NULL DEFAULT 'unknown',
            hash TEXT NOT NULL,
            recorded_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS audit_session ON audit(session_id);
        CREATE INDEX IF NOT EXISTS audit_recorded_at ON audit(recorded_at DESC);
    ")?;
    Ok(())
}

pub fn record(store: &Store, session_id: &str, command: &str, cwd: &str, exit_code: i32, kind: CommandKind) -> anyhow::Result<()> {
    let recorded_at = Utc::now().to_rfc3339();
    let hash = compute_hash(session_id, command, cwd, &recorded_at);
    let conn = open(store)?;
    conn.execute(
        "INSERT INTO audit (session_id, command, cwd, exit_code, kind, hash, recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![session_id, command, cwd, exit_code, kind.as_str(), hash, recorded_at],
    )?;
    Ok(())
}

pub fn recent(store: &Store, limit: usize) -> anyhow::Result<Vec<AuditEntry>> {
    let conn = open(store)?;
    let mut stmt = conn.prepare("SELECT id,session_id,command,cwd,exit_code,kind,hash,recorded_at FROM audit ORDER BY id DESC LIMIT ?1")?;
    let rows = stmt.query_map(params![limit as i64], row_to_entry)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn by_session(store: &Store, session_id: &str) -> anyhow::Result<Vec<AuditEntry>> {
    let conn = open(store)?;
    let mut stmt = conn.prepare("SELECT id,session_id,command,cwd,exit_code,kind,hash,recorded_at FROM audit WHERE session_id=?1 ORDER BY id ASC")?;
    let rows = stmt.query_map(params![session_id], row_to_entry)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn verify_entry(entry: &AuditEntry) -> bool {
    compute_hash(&entry.session_id, &entry.command, &entry.cwd, &entry.recorded_at) == entry.hash
}

pub fn verify_all(store: &Store) -> anyhow::Result<Vec<i64>> {
    let conn = open(store)?;
    let mut stmt = conn.prepare("SELECT id,session_id,command,cwd,exit_code,kind,hash,recorded_at FROM audit")?;
    let rows = stmt.query_map([], row_to_entry)?;
    Ok(rows.filter_map(|r| r.ok()).filter(|e| !verify_entry(e)).map(|e| e.id).collect())
}

fn compute_hash(session_id: &str, command: &str, cwd: &str, recorded_at: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_id.as_bytes());
    hasher.update(b"|");
    hasher.update(command.as_bytes());
    hasher.update(b"|");
    hasher.update(cwd.as_bytes());
    hasher.update(b"|");
    hasher.update(recorded_at.as_bytes());
    hex_encode(hasher.finalize())
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<AuditEntry> {
    Ok(AuditEntry { id: row.get(0)?, session_id: row.get(1)?, command: row.get(2)?, cwd: row.get(3)?, exit_code: row.get(4)?, kind: row.get(5)?, hash: row.get(6)?, recorded_at: row.get(7)? })
}

fn open(store: &Store) -> anyhow::Result<Connection> {
    Ok(Connection::open(store.db_path("audit.db"))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use tempfile::TempDir;

    fn setup() -> (Store, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        init(&store).unwrap();
        (store, dir)
    }

    #[test]
    fn records_and_retrieves() {
        let (store, _dir) = setup();
        record(&store, "sess1", "ls -la", "/tmp", 0, CommandKind::System).unwrap();
        record(&store, "sess1", "cd /", "/tmp", 0, CommandKind::Builtin).unwrap();
        let entries = recent(&store, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "cd /");
    }

    #[test]
    fn by_session_filters_correctly() {
        let (store, _dir) = setup();
        record(&store, "sess1", "ls", "/", 0, CommandKind::System).unwrap();
        record(&store, "sess2", "pwd", "/", 0, CommandKind::System).unwrap();
        let sess1 = by_session(&store, "sess1").unwrap();
        assert_eq!(sess1.len(), 1);
        assert_eq!(sess1[0].command, "ls");
    }

    #[test]
    fn hash_verification_passes_for_valid_entry() {
        let (store, _dir) = setup();
        record(&store, "sess1", "echo hello", "/home", 0, CommandKind::System).unwrap();
        let entries = recent(&store, 1).unwrap();
        assert!(verify_entry(&entries[0]));
    }

    #[test]
    fn hash_verification_fails_for_tampered_entry() {
        let (store, _dir) = setup();
        record(&store, "sess1", "echo hello", "/home", 0, CommandKind::System).unwrap();
        let mut entry = recent(&store, 1).unwrap().remove(0);
        entry.command = "rm -rf /".to_string();
        assert!(!verify_entry(&entry));
    }

    #[test]
    fn verify_all_clean_log() {
        let (store, _dir) = setup();
        record(&store, "s1", "ls", "/", 0, CommandKind::System).unwrap();
        let tampered = verify_all(&store).unwrap();
        assert!(tampered.is_empty());
    }
}
