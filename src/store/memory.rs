// cash — memory store (taught commands)
//
// Stores user-taught command aliases in ~/.cash/memory.db (SQLite).
// The `teach` command writes here. The Resolver reads here.
//
// Schema:
//   memories(id, name, command, created_at, use_count, last_used_at)
//
// Example:
//   teach list files 'ls -la'
//   → name="list files", command="ls -la"

use rusqlite::{Connection, params};
use chrono::Utc;

use super::Store;

#[derive(Debug, Clone)]
pub struct Memory {
    pub id:           i64,
    pub name:         String,   // the natural-language trigger
    pub command:      String,   // the shell command to run
    pub created_at:   String,
    pub use_count:    i64,
    pub last_used_at: Option<String>,
}

pub fn init(store: &Store) -> anyhow::Result<()> {
    let conn = open(store)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS memories (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT    NOT NULL UNIQUE,
            command      TEXT    NOT NULL,
            created_at   TEXT    NOT NULL,
            use_count    INTEGER NOT NULL DEFAULT 0,
            last_used_at TEXT
        );
        CREATE INDEX IF NOT EXISTS memories_name ON memories(name);
    ")?;
    Ok(())
}

/// Teach cash a new command. Overwrites if the name already exists.
pub fn teach(store: &Store, name: &str, command: &str) -> anyhow::Result<()> {
    let name = name.trim().to_lowercase();
    let conn = open(store)?;
    conn.execute(
        "INSERT INTO memories (name, command, created_at)
              VALUES (?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET
              command    = excluded.command,
              created_at = excluded.created_at",
        params![name, command, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Exact lookup by name.
pub fn get(store: &Store, name: &str) -> anyhow::Result<Option<Memory>> {
    let name = name.trim().to_lowercase();
    let conn = open(store)?;
    let result = conn.query_row(
        "SELECT id, name, command, created_at, use_count, last_used_at
           FROM memories WHERE name = ?1",
        params![name],
        row_to_memory,
    );
    match result {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Return all taught commands, alphabetical by name.
pub fn list(store: &Store) -> anyhow::Result<Vec<Memory>> {
    let conn = open(store)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, command, created_at, use_count, last_used_at
           FROM memories ORDER BY name"
    )?;
    let rows = stmt.query_map([], row_to_memory)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Forget a taught command by name.
pub fn forget(store: &Store, name: &str) -> anyhow::Result<bool> {
    let name = name.trim().to_lowercase();
    let conn = open(store)?;
    let changed = conn.execute("DELETE FROM memories WHERE name = ?1", params![name])?;
    Ok(changed > 0)
}

/// Record that a memory was used (increments use_count).
pub fn record_use(store: &Store, name: &str) -> anyhow::Result<()> {
    let name = name.trim().to_lowercase();
    let conn = open(store)?;
    conn.execute(
        "UPDATE memories
            SET use_count    = use_count + 1,
                last_used_at = ?1
          WHERE name = ?2",
        params![Utc::now().to_rfc3339(), name],
    )?;
    Ok(())
}

fn row_to_memory(row: &rusqlite::Row) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id:           row.get(0)?,
        name:         row.get(1)?,
        command:      row.get(2)?,
        created_at:   row.get(3)?,
        use_count:    row.get(4)?,
        last_used_at: row.get(5)?,
    })
}

fn open(store: &Store) -> anyhow::Result<Connection> {
    Ok(Connection::open(store.db_path("memory.db"))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_store() -> (Store, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        init(&store).unwrap();
        (store, dir)
    }

    #[test]
    fn teach_and_get() {
        let (store, _dir) = temp_store();
        teach(&store, "list files", "ls -la").unwrap();
        let m = get(&store, "list files").unwrap().unwrap();
        assert_eq!(m.command, "ls -la");
    }

    #[test]
    fn teach_overwrites() {
        let (store, _dir) = temp_store();
        teach(&store, "list files", "ls -la").unwrap();
        teach(&store, "list files", "ls -lah").unwrap();
        let m = get(&store, "list files").unwrap().unwrap();
        assert_eq!(m.command, "ls -lah");
    }

    #[test]
    fn get_missing_returns_none() {
        let (store, _dir) = temp_store();
        assert!(get(&store, "nonexistent").unwrap().is_none());
    }

    #[test]
    fn list_all() {
        let (store, _dir) = temp_store();
        teach(&store, "list files", "ls -la").unwrap();
        teach(&store, "show processes", "ps aux").unwrap();
        let all = list(&store).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "list files");
        assert_eq!(all[1].name, "show processes");
    }

    #[test]
    fn forget_removes() {
        let (store, _dir) = temp_store();
        teach(&store, "list files", "ls -la").unwrap();
        let removed = forget(&store, "list files").unwrap();
        assert!(removed);
        assert!(get(&store, "list files").unwrap().is_none());
    }

    #[test]
    fn record_use_increments() {
        let (store, _dir) = temp_store();
        teach(&store, "list files", "ls -la").unwrap();
        record_use(&store, "list files").unwrap();
        record_use(&store, "list files").unwrap();
        let m = get(&store, "list files").unwrap().unwrap();
        assert_eq!(m.use_count, 2);
    }

    #[test]
    fn name_is_case_insensitive() {
        let (store, _dir) = temp_store();
        teach(&store, "List Files", "ls -la").unwrap();
        let m = get(&store, "LIST FILES").unwrap().unwrap();
        assert_eq!(m.command, "ls -la");
    }
}
