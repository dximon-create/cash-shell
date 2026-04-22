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
    pub cwd: std::path::PathBuf,
    pub store: Store,
}

impl Shell {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        Ok(Self { cwd, store })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        repl::run(self)
    }
}
