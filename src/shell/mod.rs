// cash — Layer 1: The Shell

pub mod builtins;
pub mod eval;
pub mod executor;
pub mod input;
pub mod parser;
pub mod pipeline;
pub mod prompt;
pub mod repl;
pub mod resolver;
pub mod suggester;

use crate::store::Store;

pub struct Shell {
    pub cwd:        std::path::PathBuf,
    pub store:      Store,
    pub session_id: String,
}

impl Shell {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let session_id = generate_session_id();
        Ok(Self { cwd, store, session_id })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        repl::run(self)
    }
}

/// Generate a unique session ID using timestamp + process ID.
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    format!("{}-{}", ts, pid)
}
