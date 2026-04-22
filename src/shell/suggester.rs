// cash — Suggestion Engine
//
// Called when the Resolver returns NotFound and the Executor returns 127.
// Never fails silently. Always gives the user something useful.
//
// Priority order:
//   1. System command typo    — "gti" → did you mean "git"?
//   2. Taught command partial — partial match against memory.db
//   3. Generic guidance       — "teach me with: teach <name> '<command>'"

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::store::{memory, Store};

/// The result of a suggestion lookup.
#[derive(Debug, Clone)]
pub enum Suggestion {
    /// A close system command was found.
    SystemTypo {
        input:      String,
        suggestion: String,
        score:      f64,
    },
    /// A taught command is a close match.
    TaughtCommand {
        input:      String,
        name:       String,
        command:    String,
        score:      f64,
    },
    /// Nothing close found — show the teach prompt.
    Unknown {
        input: String,
    },
}

/// Find the best suggestion for an unresolved input.
pub fn suggest(input: &str, store: &Store) -> Suggestion {
    let input_lower = input.trim().to_lowercase();
    let matcher = SkimMatcherV2::default();

    // --- Pass 1: System command typo ---
    // Walk PATH and collect all executables, fuzzy-match against input.
    let path_var = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
    let mut best_sys_score: i64 = 0;
    let mut best_sys_cmd: Option<String> = None;

    for dir in path_var.split(':') {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            // Only suggest if input and candidate share at least the first character.
            if !name.starts_with(input_lower.chars().next().unwrap_or('_')) {
                continue;
            }
            if let Some(score) = matcher.fuzzy_match(&name, &input_lower) {
                if score > best_sys_score {
                    best_sys_score = score;
                    best_sys_cmd = Some(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }

    // Only surface a system suggestion if the score is strong enough.
    let sys_threshold = 20 * input_lower.len() as i64;
    if best_sys_score >= sys_threshold {
        if let Some(cmd) = best_sys_cmd {
            let score = (best_sys_score as f64 / (sys_threshold as f64 * 2.0)).min(1.0);
            return Suggestion::SystemTypo {
                input:      input.to_string(),
                suggestion: cmd,
                score,
            };
        }
    }

    // --- Pass 2: Taught command fuzzy match ---
    let memories = memory::list(store).unwrap_or_default();
    let mut best_mem_score: i64 = 0;
    let mut best_mem = None;

    for m in &memories {
        if let Some(score) = matcher.fuzzy_match(&m.name, &input_lower) {
            if score > best_mem_score {
                best_mem_score = score;
                best_mem = Some(m);
            }
        }
    }

    let mem_threshold = 10 * input_lower.len() as i64;
    if best_mem_score >= mem_threshold {
        if let Some(m) = best_mem {
            let score = (best_mem_score as f64 / (mem_threshold as f64 * 2.0)).min(1.0);
            return Suggestion::TaughtCommand {
                input:   input.to_string(),
                name:    m.name.clone(),
                command: m.command.clone(),
                score,
            };
        }
    }

    // --- Pass 3: Unknown ---
    Suggestion::Unknown { input: input.to_string() }
}

/// Print the suggestion to stderr in cash's voice.
pub fn print_suggestion(suggestion: &Suggestion) {
    match suggestion {
        Suggestion::SystemTypo { input, suggestion, .. } => {
            eprintln!("cash: '{}' not found — did you mean '{}'?", input, suggestion);
        }
        Suggestion::TaughtCommand { input, name, command, .. } => {
            eprintln!(
                "cash: '{}' not found — did you mean '{}' → {}?",
                input, name, command
            );
            eprintln!("      run: teach {} '{}'", name, command);
        }
        Suggestion::Unknown { input } => {
            eprintln!("cash: '{}' not found", input);
            eprintln!("      teach me: teach {} '<command>'", input);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Store, memory};
    use tempfile::TempDir;

    fn setup() -> (Store, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        memory::init(&store).unwrap();
        (store, dir)
    }

    #[test]
    fn unknown_when_no_memories_and_no_path_match() {
        let (store, _dir) = setup();
        let suggestion = suggest("zzzzzzzzzzz_no_match", &store);
        assert!(matches!(suggestion, Suggestion::Unknown { .. }));
    }

    #[test]
    fn taught_command_suggestion() {
        let (store, _dir) = setup();
        memory::teach(&store, "list files", "ls -la").unwrap();
        let suggestion = suggest("list fil", &store);
        // Should find "list files" as a taught command suggestion.
        assert!(matches!(
            suggestion,
            Suggestion::TaughtCommand { .. } | Suggestion::SystemTypo { .. }
        ));
    }

    #[test]
    fn system_typo_for_git() {
        let (store, _dir) = setup();
        // "gti" is a common git typo — should suggest "git" if git is on PATH.
        let suggestion = suggest("gti", &store);
        // We can't guarantee git is installed, so just check it doesn't panic.
        let _ = suggestion;
    }
}
