// cash — Store
//
// Owns everything under ~/.cash/:
//   config.toml   — user preferences
//   history.db    — command history (SQLite)
//   memory.db     — taught commands (SQLite)

pub mod config;
pub mod history;
pub mod memory;

use std::path::PathBuf;

pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn open() -> anyhow::Result<Self> {
        let root = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("cannot resolve home directory"))?
            .join(".cash");

        std::fs::create_dir_all(&root)?;

        let store = Self { root };

        // Initialise sub-systems in order.
        config::init(&store)?;
        history::init(&store)?;
        memory::init(&store)?;

        Ok(store)
    }

    pub fn db_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }
}
