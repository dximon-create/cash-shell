// cash — Resolver
//
// Three-pass command resolution. Runs before the executor on every command.
//
// Pass 1 — Exact match:   "list files" → memory.db lookup → run it
// Pass 2 — Pattern match: "list" → all memories starting with "list"
// Pass 3 — Fuzzy match:   "lsit feles" → scored match against all memories
//
// If no match is found, returns NotFound so the Suggestion Engine can handle it.
// If a fuzzy match scores below the confidence threshold, returns BelowThreshold
// so the shell can ask the user before running.
//
// The Resolver NEVER executes below the confidence threshold. Ever.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::store::{memory, Store};
use crate::store::config::Config;

#[derive(Debug, Clone)]
pub enum ResolveResult {
    /// Exact or high-confidence match. Contains the command to run.
    Resolved {
        original_name: String,
        command:       String,
        confidence:    f64,
        match_kind:    MatchKind,
    },
    /// A match was found but confidence is below threshold.
    /// Shell must ask the user before proceeding.
    BelowThreshold {
        original_name: String,
        command:       String,
        confidence:    f64,
        suggestion:    String,
    },
    /// No match found at all. Hand to Suggestion Engine.
    NotFound {
        input: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchKind {
    Exact,
    Pattern,
    Fuzzy,
}

/// Resolve a command name against the memory store.
pub fn resolve(input: &str, store: &Store, config: &Config) -> ResolveResult {
    let input_lower = input.trim().to_lowercase();

    // --- Pass 1: Exact match ---
    if let Ok(Some(memory)) = memory::get(store, &input_lower) {
        return ResolveResult::Resolved {
            original_name: input.to_string(),
            command:       memory.command,
            confidence:    1.0,
            match_kind:    MatchKind::Exact,
        };
    }

    // Load all memories for pattern and fuzzy passes.
    let memories = match memory::list(store) {
        Ok(m) => m,
        Err(_) => return ResolveResult::NotFound { input: input.to_string() },
    };

    if memories.is_empty() {
        return ResolveResult::NotFound { input: input.to_string() };
    }

    // --- Pass 2: Pattern match (prefix or contains) ---
    let pattern_matches: Vec<_> = memories.iter()
        .filter(|m| {
            m.name.starts_with(&input_lower) || m.name.contains(&input_lower)
        })
        .collect();

    if pattern_matches.len() == 1 {
        let m = pattern_matches[0];
        let confidence = if m.name.starts_with(&input_lower) { 0.9 } else { 0.8 };
        if confidence >= config.resolver_confidence_threshold {
            return ResolveResult::Resolved {
                original_name: input.to_string(),
                command:       m.command.clone(),
                confidence,
                match_kind:    MatchKind::Pattern,
            };
        } else {
            return ResolveResult::BelowThreshold {
                original_name: input.to_string(),
                command:       m.command.clone(),
                confidence,
                suggestion:    m.name.clone(),
            };
        }
    }

    // Multiple pattern matches — fall through to fuzzy to pick the best.

    // --- Pass 3: Fuzzy match ---
    let matcher = SkimMatcherV2::default();
    let mut best_score: i64 = 0;
    let mut best_memory = None;

    for m in &memories {
        if let Some(score) = matcher.fuzzy_match(&m.name, &input_lower) {
            if score > best_score {
                best_score = score;
                best_memory = Some(m);
            }
        }
    }

    if let Some(m) = best_memory {
        // Normalise score to 0.0-1.0 range.
        // SkimMatcher scores vary widely; we use a sigmoid-like normalisation.
        let confidence = normalise_score(best_score, input_lower.len());

        if confidence >= config.resolver_confidence_threshold {
            return ResolveResult::Resolved {
                original_name: input.to_string(),
                command:       m.command.clone(),
                confidence,
                match_kind:    MatchKind::Fuzzy,
            };
        } else {
            return ResolveResult::BelowThreshold {
                original_name: input.to_string(),
                command:       m.command.clone(),
                confidence,
                suggestion:    m.name.clone(),
            };
        }
    }

    ResolveResult::NotFound { input: input.to_string() }
}

/// Normalise a fuzzy score to roughly 0.0–1.0.
fn normalise_score(score: i64, input_len: usize) -> f64 {
    if input_len == 0 { return 0.0; }
    // Each matching character contributes ~score/input_len points.
    // Empirically skim scores ~16 per character for a good match.
    let per_char = 16.0 * input_len as f64;
    let raw = score as f64 / per_char;
    raw.min(1.0).max(0.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Store, memory};
    use crate::store::config::Config;
    use tempfile::TempDir;

    fn setup() -> (Store, TempDir, Config) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        memory::init(&store).unwrap();
        let config = Config::default();
        (store, dir, config)
    }

    #[test]
    fn exact_match() {
        let (store, _dir, config) = setup();
        memory::teach(&store, "list files", "ls -la").unwrap();
        match resolve("list files", &store, &config) {
            ResolveResult::Resolved { match_kind, command, .. } => {
                assert_eq!(match_kind, MatchKind::Exact);
                assert_eq!(command, "ls -la");
            }
            other => panic!("expected Resolved, got {:?}", other),
        }
    }

    #[test]
    fn pattern_match_prefix() {
        let (store, _dir, config) = setup();
        memory::teach(&store, "list files", "ls -la").unwrap();
        match resolve("list", &store, &config) {
            ResolveResult::Resolved { match_kind, .. } => {
                assert_eq!(match_kind, MatchKind::Pattern);
            }
            other => panic!("expected Resolved, got {:?}", other),
        }
    }

    #[test]
    fn fuzzy_match() {
        let (store, _dir, config) = setup();
        memory::teach(&store, "list files", "ls -la").unwrap();
        // "lst fls" is a fuzzy match for "list files"
        match resolve("lst fls", &store, &config) {
            ResolveResult::Resolved { .. } |
            ResolveResult::BelowThreshold { .. } => {
                // Either resolved or below threshold — both mean fuzzy found something
            }
            ResolveResult::NotFound { .. } => panic!("expected a match"),
        }
    }

    #[test]
    fn not_found_when_empty() {
        let (store, _dir, config) = setup();
        match resolve("anything", &store, &config) {
            ResolveResult::NotFound { .. } => {}
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn case_insensitive() {
        let (store, _dir, config) = setup();
        memory::teach(&store, "list files", "ls -la").unwrap();
        match resolve("LIST FILES", &store, &config) {
            ResolveResult::Resolved { match_kind, .. } => {
                assert_eq!(match_kind, MatchKind::Exact);
            }
            other => panic!("expected Resolved, got {:?}", other),
        }
    }

    #[test]
    fn below_threshold_does_not_resolve() {
        let (store, _dir, mut config) = setup();
        config.resolver_confidence_threshold = 1.0; // impossible to reach
        memory::teach(&store, "list files", "ls -la").unwrap();
        match resolve("xyz_no_match_at_all", &store, &config) {
            ResolveResult::BelowThreshold { .. } | ResolveResult::NotFound { .. } => {}
            ResolveResult::Resolved { .. } => panic!("should not resolve at threshold 1.0"),
        }
    }
}
