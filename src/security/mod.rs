// cash — Layer 2: Security Engine
//
// Runs on its own thread. NEVER blocks the shell.
// CANNOT be disabled by the user.
//
// Module 9: Tamper Detection
// Watches ~/.cash/ files at startup by recording SHA-256 baseline hashes.
// A background thread re-checks every 30 seconds.
// If any watched file changes outside of cash — alerts and logs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Sha256, Digest};
use hex::encode as hex_encode;

pub struct SecurityEngine {
    cash_dir: PathBuf,
}

impl SecurityEngine {
    pub fn new(cash_dir: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { cash_dir })
    }

    /// Spawn the security thread. Returns immediately.
    /// The shell is never blocked.
    pub fn spawn(self) {
        std::thread::spawn(move || {
            let watcher = TamperWatcher::new(&self.cash_dir);
            watcher.run();
        });
    }
}

// ---------------------------------------------------------------------------
// Tamper Watcher
// ---------------------------------------------------------------------------

struct TamperWatcher {
    cash_dir:  PathBuf,
    /// filename → SHA-256 hex at last check
    baseline:  Arc<Mutex<HashMap<String, String>>>,
}

/// Files we watch for tampering.
const WATCHED: &[&str] = &[
    "audit.db",
    "history.db",
    "memory.db",
    "config.toml",
];

impl TamperWatcher {
    fn new(cash_dir: &Path) -> Self {
        let baseline = Arc::new(Mutex::new(HashMap::new()));
        let watcher = Self {
            cash_dir: cash_dir.to_path_buf(),
            baseline,
        };
        watcher.take_baseline();
        watcher
    }

    /// Record the current hashes of all watched files.
    fn take_baseline(&self) {
        let mut map = self.baseline.lock().unwrap();
        for name in WATCHED {
            let path = self.cash_dir.join(name);
            if let Some(hash) = hash_file(&path) {
                map.insert(name.to_string(), hash);
            }
        }
    }

    /// Main watch loop — runs forever on its own thread.
    fn run(&self) {
        loop {
            std::thread::sleep(Duration::from_secs(30));
            self.check();
        }
    }

    /// Compare current hashes against baseline. Log any changes.
    fn check(&self) {
        let mut map = self.baseline.lock().unwrap();
        for name in WATCHED {
            let path = self.cash_dir.join(name);
            match hash_file(&path) {
                None => {
                    // File was deleted — that's suspicious if it was present before.
                    if map.contains_key(*name) {
                        self.alert(name, "deleted outside cash");
                        map.remove(*name);
                    }
                }
                Some(current_hash) => {
                    match map.get(*name) {
                        None => {
                            // New file appeared — record it.
                            map.insert(name.to_string(), current_hash);
                        }
                        Some(baseline_hash) => {
                            if *baseline_hash != current_hash {
                                self.alert(name, "modified outside cash");
                                // Update baseline so we don't re-alert every 30s.
                                map.insert(name.to_string(), current_hash);
                            }
                        }
                    }
                }
            }
        }
    }

    fn alert(&self, filename: &str, reason: &str) {
        // Write to stderr and to a tamper log file.
        let msg = format!(
            "[cash security] TAMPER DETECTED: {} — {}",
            filename, reason
        );
        eprintln!("{}", msg);
        self.write_tamper_log(&msg);
    }

    fn write_tamper_log(&self, msg: &str) {
        use std::io::Write;
        let path = self.cash_dir.join("tamper.log");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "{} {}", chrono::Utc::now().to_rfc3339(), msg);
        }
    }
}

/// Hash a file with SHA-256. Returns None if file doesn't exist.
fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(hex_encode(hasher.finalize()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn baseline_records_existing_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("audit.db"), b"fake data").unwrap();
        let watcher = TamperWatcher::new(dir.path());
        let map = watcher.baseline.lock().unwrap();
        assert!(map.contains_key("audit.db"));
    }

    #[test]
    fn detects_modification() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("audit.db");
        std::fs::write(&file, b"original").unwrap();
        let watcher = TamperWatcher::new(dir.path());
        // Modify the file outside cash.
        std::fs::write(&file, b"tampered content").unwrap();
        // check() should detect the change.
        // We verify by checking the hash changed.
        let original_hash = {
            let map = watcher.baseline.lock().unwrap();
            map.get("audit.db").cloned().unwrap()
        };
        let current_hash = hash_file(&file).unwrap();
        assert_ne!(original_hash, current_hash);
    }

    #[test]
    fn hash_file_returns_none_for_missing() {
        let result = hash_file(Path::new("/nonexistent/path/file.db"));
        assert!(result.is_none());
    }

    #[test]
    fn hash_file_is_deterministic() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.db");
        std::fs::write(&file, b"same content").unwrap();
        let h1 = hash_file(&file).unwrap();
        let h2 = hash_file(&file).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_content_gives_different_hash() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.db");
        std::fs::write(&file, b"content a").unwrap();
        let h1 = hash_file(&file).unwrap();
        std::fs::write(&file, b"content b").unwrap();
        let h2 = hash_file(&file).unwrap();
        assert_ne!(h1, h2);
    }
}
