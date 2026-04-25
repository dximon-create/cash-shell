// cash — Tool Registry
//
// Tracks all tools installed through cash in ~/.cash/tools.db

use rusqlite::{Connection, params};
use chrono::Utc;
use crate::store::Store;

#[derive(Debug, Clone)]
pub struct ToolRecord {
    pub id:           String,
    pub name:         String,
    pub version:      String,
    pub binary:       String,
    pub description:  String,
    pub cash_command: String,  // the cash natural-language command
    pub installed_at: String,
    pub enabled:      bool,
}

pub struct ToolRegistry { store: Store }

impl ToolRegistry {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        let reg = Self { store };
        reg.init()?;
        Ok(reg)
    }

    fn init(&self) -> anyhow::Result<()> {
        let conn = self.conn()?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS tools (
                id           TEXT PRIMARY KEY,
                name         TEXT NOT NULL,
                version      TEXT NOT NULL DEFAULT 'unknown',
                binary       TEXT NOT NULL,
                description  TEXT NOT NULL DEFAULT '',
                cash_command TEXT NOT NULL DEFAULT '',
                installed_at TEXT NOT NULL,
                enabled      INTEGER NOT NULL DEFAULT 1
            );
        ")?;
        Ok(())
    }

    pub fn register(&self, tool: &ToolRecord) -> anyhow::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO tools (id,name,version,binary,description,cash_command,installed_at,enabled)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(id) DO UPDATE SET
             version=excluded.version, installed_at=excluded.installed_at",
            params![
                tool.id, tool.name, tool.version, tool.binary,
                tool.description, tool.cash_command,
                Utc::now().to_rfc3339(), tool.enabled as i32
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> anyhow::Result<Option<ToolRecord>> {
        let conn = self.conn()?;
        match conn.query_row(
            "SELECT id,name,version,binary,description,cash_command,installed_at,enabled
               FROM tools WHERE id=?1",
            params![id], row_to_tool,
        ) {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self) -> anyhow::Result<Vec<ToolRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id,name,version,binary,description,cash_command,installed_at,enabled
               FROM tools ORDER BY name"
        )?;
        let rows = stmt.query_map([], row_to_tool)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn remove(&self, id: &str) -> anyhow::Result<bool> {
        let conn = self.conn()?;
        Ok(conn.execute("DELETE FROM tools WHERE id=?1", params![id])? > 0)
    }

    pub fn is_installed(&self, id: &str) -> bool {
        self.get(id).ok().flatten().is_some()
    }

    fn conn(&self) -> anyhow::Result<Connection> {
        let c = Connection::open(self.store.db_path("tools.db"))?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(c)
    }
}

fn row_to_tool(row: &rusqlite::Row) -> rusqlite::Result<ToolRecord> {
    Ok(ToolRecord {
        id:           row.get(0)?,
        name:         row.get(1)?,
        version:      row.get(2)?,
        binary:       row.get(3)?,
        description:  row.get(4)?,
        cash_command: row.get(5)?,
        installed_at: row.get(6)?,
        enabled:      row.get::<_, i32>(7)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (ToolRegistry, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        (ToolRegistry::new(store).unwrap(), dir)
    }

    fn tool(id: &str) -> ToolRecord {
        ToolRecord {
            id: id.into(), name: format!("Tool {}", id),
            version: "1.0".into(), binary: id.into(),
            description: "test tool".into(),
            cash_command: "scan".into(),
            installed_at: Utc::now().to_rfc3339(),
            enabled: true,
        }
    }

    #[test]
    fn register_and_get() {
        let (reg, _dir) = setup();
        reg.register(&tool("nmap")).unwrap();
        let t = reg.get("nmap").unwrap().unwrap();
        assert_eq!(t.name, "Tool nmap");
    }

    #[test]
    fn list_tools() {
        let (reg, _dir) = setup();
        reg.register(&tool("nmap")).unwrap();
        reg.register(&tool("nikto")).unwrap();
        assert_eq!(reg.list().unwrap().len(), 2);
    }

    #[test]
    fn remove_tool() {
        let (reg, _dir) = setup();
        reg.register(&tool("nmap")).unwrap();
        assert!(reg.remove("nmap").unwrap());
        assert!(!reg.is_installed("nmap"));
    }

    #[test]
    fn is_installed_false_for_unknown() {
        let (reg, _dir) = setup();
        assert!(!reg.is_installed("nonexistent"));
    }
}
