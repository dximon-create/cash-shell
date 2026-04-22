// cash — config.toml
//
// Reads and writes ~/.cash/config.toml.
// All fields have sensible defaults so a missing file is fine.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Maximum number of history entries to keep.
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,

    /// Whether to record duplicate consecutive commands.
    #[serde(default = "default_true")]
    pub history_dedup: bool,

    /// Confidence threshold below which the resolver asks before running.
    /// 0.0 = always ask, 1.0 = never ask (not recommended).
    #[serde(default = "default_confidence")]
    pub resolver_confidence_threshold: f64,

    /// Prompt style: "default" | "minimal"
    #[serde(default = "default_prompt_style")]
    pub prompt_style: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            history_limit: default_history_limit(),
            history_dedup: true,
            resolver_confidence_threshold: default_confidence(),
            prompt_style: default_prompt_style(),
        }
    }
}

fn default_history_limit() -> usize { 10_000 }
fn default_true()          -> bool  { true }
fn default_confidence()    -> f64   { 0.75 }
fn default_prompt_style()  -> String { "default".into() }

/// Load config from disk, or return defaults if missing/malformed.
pub fn load(root: &Path) -> Config {
    let path = root.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&path) else { return Config::default() };
    toml::from_str(&text).unwrap_or_default()
}

/// Write config to disk.
pub fn save(root: &Path, config: &Config) -> anyhow::Result<()> {
    let text = toml::to_string_pretty(config)?;
    std::fs::write(root.join("config.toml"), text)?;
    Ok(())
}

/// Create config.toml with defaults if it doesn't exist yet.
pub fn init(store: &super::Store) -> anyhow::Result<()> {
    let path = store.config_path();
    if !path.exists() {
        save(&store.root, &Config::default())?;
    }
    Ok(())
}
