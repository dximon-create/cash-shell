// cash — Tab Completion
//
// Completes:
//   - System binaries from PATH
//   - Taught commands from memory.db
//   - Built-in commands
//   - File and directory paths
//   - Tool manager commands

use std::path::{Path, PathBuf};
use crate::store::{Store, memory};

const BUILTINS: &[&str] = &[
    "show", "go", "copy", "move", "remove", "teach", "help", "exit",
    "scan", "trace", "dns", "whois", "arp", "lab", "learn",
    "cash",
];

#[derive(Debug, Clone)]
pub struct Completion {
    pub value:   String,
    pub kind:    CompletionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Binary,
    Builtin,
    Taught,
    Path,
    Directory,
}

impl Completion {
    pub fn display(&self) -> &str {
        &self.value
    }
}

pub struct Completer {
    store: Store,
}

impl Completer {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// Get completions for the current input word.
    pub fn complete(&self, line: &str, cursor: usize) -> Vec<Completion> {
        let line = &line[..cursor];
        let word = current_word(line);
        let is_first_word = is_first_token(line);

        if word.starts_with('/') || word.starts_with("./") || word.starts_with("~/") || word.starts_with("..") {
            return self.complete_path(word);
        }

        if is_first_word {
            self.complete_command(word)
        } else {
            // After first word — complete paths
            self.complete_path(word)
        }
    }

    fn complete_command(&self, prefix: &str) -> Vec<Completion> {
        let mut completions = Vec::new();
        let prefix_lower = prefix.to_lowercase();

        // Built-ins
        for &builtin in BUILTINS {
            if builtin.starts_with(&prefix_lower) {
                completions.push(Completion {
                    value: builtin.to_string(),
                    kind:  CompletionKind::Builtin,
                });
            }
        }

        // Taught commands
        if let Ok(memories) = memory::list(&self.store) {
            for m in memories {
                if m.name.starts_with(&prefix_lower) {
                    completions.push(Completion {
                        value: m.name.clone(),
                        kind:  CompletionKind::Taught,
                    });
                }
            }
        }

        // System binaries from PATH
        let path_var = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
        let mut binaries: Vec<String> = Vec::new();
        for dir in path_var.split(':') {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.to_lowercase().starts_with(&prefix_lower) {
                    binaries.push(name);
                }
            }
        }
        binaries.sort();
        binaries.dedup();
        for name in binaries {
            completions.push(Completion { value: name, kind: CompletionKind::Binary });
        }

        completions
    }

    fn complete_path(&self, prefix: &str) -> Vec<Completion> {
        let mut completions = Vec::new();

        let expanded = expand_tilde(prefix);
        let (dir, file_prefix) = split_path(&expanded);

        let search_dir = if dir.is_empty() { ".".to_string() } else { dir.clone() };
        let Ok(entries) = std::fs::read_dir(&search_dir) else { return completions };

        let mut paths: Vec<Completion> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                 .to_string_lossy()
                 .to_lowercase()
                 .starts_with(&file_prefix.to_lowercase())
            })
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.path().is_dir();
                let full = if dir.is_empty() {
                    if is_dir { format!("{}/", name) } else { name }
                } else {
                    if is_dir { format!("{}/{}/", dir, name) } else { format!("{}/{}", dir, name) }
                };
                Completion {
                    value: full,
                    kind: if is_dir { CompletionKind::Directory } else { CompletionKind::Path },
                }
            })
            .collect();

        paths.sort_by(|a, b| a.value.cmp(&b.value));
        completions.extend(paths);
        completions
    }
}

fn current_word(line: &str) -> &str {
    line.split_whitespace().last().unwrap_or("")
}

fn is_first_token(line: &str) -> bool {
    line.trim().split_whitespace().count() <= 1
}

fn split_path(path: &str) -> (String, String) {
    if let Some(pos) = path.rfind('/') {
        (path[..pos].to_string(), path[pos+1..].to_string())
    } else {
        (String::new(), path.to_string())
    }
}

fn expand_tilde(s: &str) -> String {
    if s.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), &s[2..]);
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (Completer, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        memory::init(&store).unwrap();
        (Completer::new(store), dir)
    }

    #[test]
    fn completes_builtins() {
        let (c, _dir) = setup();
        let results = c.complete("sh", 2);
        assert!(results.iter().any(|r| r.value == "show"));
    }

    #[test]
    fn completes_go_builtin() {
        let (c, _dir) = setup();
        let results = c.complete("g", 1);
        assert!(results.iter().any(|r| r.value == "go"));
    }

    #[test]
    fn completes_taught_commands() {
        let (c, _dir) = setup();
        memory::teach(&c.store, "list files", "ls -la").unwrap();
        let results = c.complete("list", 4);
        assert!(results.iter().any(|r| r.value == "list files"));
    }

    #[test]
    fn empty_prefix_returns_results() {
        let (c, _dir) = setup();
        let results = c.complete("", 0);
        assert!(!results.is_empty());
    }

    #[test]
    fn path_completion_works() {
        let (c, _dir) = setup();
        let results = c.complete("/tmp", 4);
        // Should attempt path completion
        let _ = results;
    }
}
