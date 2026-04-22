// cash — command history
//
// Stores every executed command in ~/.cash/history.db (SQLite).
// Up/down arrows in input.rs query this.
//
// Schema:
//   history(id, command, cwd, exit_code, ran_at)

use chrono::Utc;
use rusqlite::{params, Connection};

use super::Store;

/// A single history entry.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub command: String,
    pub cwd: String,
    pub exit_code: i32,
    pub ran_at: String, // ISO-8601
}

/// Create the history table if it does not exist.
pub fn init(store: &Store) -> anyhow::Result<()> {
    let conn = open(store)?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS history (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            command   TEXT    NOT NULL,
            cwd       TEXT    NOT NULL DEFAULT '',
            exit_code INTEGER NOT NULL DEFAULT 0,
            ran_at    TEXT    NOT NULL
        );
        CREATE INDEX IF NOT EXISTS history_ran_at ON history(ran_at DESC);
    ",
    )?;
    Ok(())
}

/// Append a command to history.
/// Skips empty commands and, if dedup is on, consecutive duplicates.
pub fn push(
    store: &Store,
    command: &str,
    cwd: &str,
    exit_code: i32,
    dedup: bool,
) -> anyhow::Result<()> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let conn = open(store)?;

    if dedup {
        let last: Option<String> = conn
            .query_row(
                "SELECT command FROM history ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();
        if last.as_deref() == Some(trimmed) {
            return Ok(());
        }
    }

    conn.execute(
        "INSERT INTO history (command, cwd, exit_code, ran_at) VALUES (?1, ?2, ?3, ?4)",
        params![trimmed, cwd, exit_code, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Return the N most recent history entries (newest first).
pub fn recent(store: &Store, limit: usize) -> anyhow::Result<Vec<HistoryEntry>> {
    let conn = open(store)?;
    let mut stmt = conn.prepare(
        "SELECT id, command, cwd, exit_code, ran_at
           FROM history
          ORDER BY id DESC
          LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            command: row.get(1)?,
            cwd: row.get(2)?,
            exit_code: row.get(3)?,
            ran_at: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Search history for entries containing `query` (newest first).
pub fn search(store: &Store, query: &str, limit: usize) -> anyhow::Result<Vec<HistoryEntry>> {
    let conn = open(store)?;
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id, command, cwd, exit_code, ran_at
           FROM history
          WHERE command LIKE ?1
          ORDER BY id DESC
          LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![pattern, limit as i64], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            command: row.get(1)?,
            cwd: row.get(2)?,
            exit_code: row.get(3)?,
            ran_at: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Trim history to at most `limit` entries (deletes oldest first).
pub fn trim(store: &Store, limit: usize) -> anyhow::Result<()> {
    let conn = open(store)?;
    conn.execute(
        "DELETE FROM history
          WHERE id NOT IN (
              SELECT id FROM history ORDER BY id DESC LIMIT ?1
          )",
        params![limit as i64],
    )?;
    Ok(())
}

fn open(store: &Store) -> anyhow::Result<Connection> {
    Ok(Connection::open(store.db_path("history.db"))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_store() -> (Store, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store {
            root: dir.path().to_path_buf(),
        };
        init(&store).unwrap();
        (store, dir)
    }

    #[test]
    fn push_and_recent() {
        let (store, _dir) = temp_store();
        push(&store, "ls -la", "/tmp", 0, false).unwrap();
        push(&store, "echo hello", "/tmp", 0, false).unwrap();
        let entries = recent(&store, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "echo hello");
        assert_eq!(entries[1].command, "ls -la");
    }

    #[test]
    fn dedup_skips_consecutive() {
        let (store, _dir) = temp_store();
        push(&store, "ls", "/tmp", 0, true).unwrap();
        push(&store, "ls", "/tmp", 0, true).unwrap();
        push(&store, "ls", "/tmp", 0, true).unwrap();
        let entries = recent(&store, 10).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn dedup_allows_non_consecutive() {
        let (store, _dir) = temp_store();
        push(&store, "ls", "/tmp", 0, true).unwrap();
        push(&store, "cd /", "/tmp", 0, true).unwrap();
        push(&store, "ls", "/", 0, true).unwrap();
        let entries = recent(&store, 10).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn search_finds_match() {
        let (store, _dir) = temp_store();
        push(&store, "ls -la /tmp", "/", 0, false).unwrap();
        push(&store, "echo hello", "/", 0, false).unwrap();
        let results = search(&store, "ls", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].command, "ls -la /tmp");
    }

    #[test]
    fn trim_removes_oldest() {
        let (store, _dir) = temp_store();
        for i in 0..10 {
            push(&store, &format!("cmd {}", i), "/", 0, false).unwrap();
        }
        trim(&store, 5).unwrap();
        let entries = recent(&store, 20).unwrap();
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].command, "cmd 9");
    }
}
