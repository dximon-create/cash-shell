// cash — Marketplace Registry
//
// Local database of installed agents in ~/.cash/marketplace.db

use rusqlite::{Connection, params};
use chrono::Utc;
use crate::store::Store;

#[derive(Debug, Clone)]
pub struct InstalledAgent {
    pub id:           String,
    pub name:         String,
    pub version:      String,
    pub description:  String,
    pub permissions:  Vec<String>,
    pub checksum:     String,
    pub installed_at: String,
    pub enabled:      bool,
}

pub struct MarketplaceRegistry {
    store: Store,
}

impl MarketplaceRegistry {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        let reg = Self { store };
        reg.init()?;
        Ok(reg)
    }

    fn init(&self) -> anyhow::Result<()> {
        let conn = self.conn()?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS installed_agents (
                id           TEXT PRIMARY KEY,
                name         TEXT NOT NULL,
                version      TEXT NOT NULL,
                description  TEXT NOT NULL DEFAULT '',
                permissions  TEXT NOT NULL DEFAULT '[]',
                checksum     TEXT NOT NULL,
                installed_at TEXT NOT NULL,
                enabled      INTEGER NOT NULL DEFAULT 1
            );
        ")?;
        Ok(())
    }

    pub fn install(&self, agent: &InstalledAgent) -> anyhow::Result<()> {
        let conn = self.conn()?;
        let perms = serde_json::to_string(&agent.permissions)?;
        conn.execute(
            "INSERT INTO installed_agents
             (id, name, version, description, permissions, checksum, installed_at, enabled)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(id) DO UPDATE SET
             version=excluded.version, checksum=excluded.checksum,
             installed_at=excluded.installed_at",
            params![
                agent.id, agent.name, agent.version, agent.description,
                perms, agent.checksum, Utc::now().to_rfc3339(), agent.enabled as i32
            ],
        )?;
        Ok(())
    }

    pub fn uninstall(&self, id: &str) -> anyhow::Result<bool> {
        let conn = self.conn()?;
        let changed = conn.execute("DELETE FROM installed_agents WHERE id=?1", params![id])?;
        Ok(changed > 0)
    }

    pub fn get(&self, id: &str) -> anyhow::Result<Option<InstalledAgent>> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id,name,version,description,permissions,checksum,installed_at,enabled
               FROM installed_agents WHERE id=?1",
            params![id],
            row_to_agent,
        );
        match result {
            Ok(a) => Ok(Some(a)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self) -> anyhow::Result<Vec<InstalledAgent>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id,name,version,description,permissions,checksum,installed_at,enabled
               FROM installed_agents ORDER BY name"
        )?;
        let rows = stmt.query_map([], row_to_agent)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn enable(&self, id: &str, enabled: bool) -> anyhow::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE installed_agents SET enabled=?1 WHERE id=?2",
            params![enabled as i32, id],
        )?;
        Ok(())
    }

    fn conn(&self) -> anyhow::Result<Connection> {
        Ok(Connection::open(self.store.db_path("marketplace.db"))?)
    }
}

fn row_to_agent(row: &rusqlite::Row) -> rusqlite::Result<InstalledAgent> {
    let perms_str: String = row.get(4)?;
    let permissions = serde_json::from_str(&perms_str).unwrap_or_default();
    Ok(InstalledAgent {
        id:           row.get(0)?,
        name:         row.get(1)?,
        version:      row.get(2)?,
        description:  row.get(3)?,
        permissions,
        checksum:     row.get(5)?,
        installed_at: row.get(6)?,
        enabled:      row.get::<_, i32>(7)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (MarketplaceRegistry, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        let reg = MarketplaceRegistry::new(store).unwrap();
        (reg, dir)
    }

    fn agent(id: &str) -> InstalledAgent {
        InstalledAgent {
            id:           id.to_string(),
            name:         format!("Agent {}", id),
            version:      "1.0.0".into(),
            description:  "Test agent".into(),
            permissions:  vec!["read".into(), "write".into()],
            checksum:     "abc123".into(),
            installed_at: Utc::now().to_rfc3339(),
            enabled:      true,
        }
    }

    #[test]
    fn install_and_get() {
        let (reg, _dir) = setup();
        reg.install(&agent("test-agent")).unwrap();
        let a = reg.get("test-agent").unwrap().unwrap();
        assert_eq!(a.name, "Agent test-agent");
        assert!(a.permissions.contains(&"read".to_string()));
    }

    #[test]
    fn uninstall_removes() {
        let (reg, _dir) = setup();
        reg.install(&agent("test-agent")).unwrap();
        assert!(reg.uninstall("test-agent").unwrap());
        assert!(reg.get("test-agent").unwrap().is_none());
    }

    #[test]
    fn list_all() {
        let (reg, _dir) = setup();
        reg.install(&agent("agent-a")).unwrap();
        reg.install(&agent("agent-b")).unwrap();
        assert_eq!(reg.list().unwrap().len(), 2);
    }

    #[test]
    fn enable_disable() {
        let (reg, _dir) = setup();
        reg.install(&agent("test-agent")).unwrap();
        reg.enable("test-agent", false).unwrap();
        let a = reg.get("test-agent").unwrap().unwrap();
        assert!(!a.enabled);
    }
}
