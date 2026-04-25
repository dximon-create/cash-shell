// cash — Ctrl+R History Search
//
// Reverse history search — press Ctrl+R, type to filter.
// Results come from history.db — the full audit trail.

use crate::store::{Store, history};

pub struct HistorySearch {
    store:   Store,
    query:   String,
    results: Vec<String>,
    index:   usize,
}

impl HistorySearch {
    pub fn new(store: Store) -> Self {
        Self { store, query: String::new(), results: Vec::new(), index: 0 }
    }

    /// Add a character to the search query and refresh results.
    pub fn push(&mut self, ch: char) {
        self.query.push(ch);
        self.refresh();
    }

    /// Remove last character from query.
    pub fn pop(&mut self) {
        self.query.pop();
        self.refresh();
    }

    /// Move to next (older) match.
    pub fn next(&mut self) {
        if self.index + 1 < self.results.len() {
            self.index += 1;
        }
    }

    /// Move to previous (newer) match.
    pub fn prev(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    /// Current selected result.
    pub fn current(&self) -> Option<&str> {
        self.results.get(self.index).map(|s| s.as_str())
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn prompt(&self) -> String {
        let result = self.current().unwrap_or("");
        format!("(reverse-i-search)`{}': {}", self.query, result)
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.results.clear();
        self.index = 0;
    }

    fn refresh(&mut self) {
        self.index = 0;
        if self.query.is_empty() {
            self.results.clear();
            return;
        }
        self.results = history::search(&self.store, &self.query, 50)
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.command)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::store::history;

    fn setup() -> (HistorySearch, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        history::init(&store).unwrap();
        history::push(&store, "ls -la /tmp", "/", 0, false).unwrap();
        history::push(&store, "git status", "/", 0, false).unwrap();
        history::push(&store, "ls -la /home", "/", 0, false).unwrap();
        let search = HistorySearch::new(store);
        (search, dir)
    }

    #[test]
    fn search_finds_matching_commands() {
        let (mut s, _dir) = setup();
        s.push('l'); s.push('s');
        assert!(s.current().is_some());
        assert!(s.current().unwrap().contains("ls"));
    }

    #[test]
    fn empty_query_returns_none() {
        let (s, _dir) = setup();
        assert!(s.current().is_none());
    }

    #[test]
    fn next_cycles_results() {
        let (mut s, _dir) = setup();
        s.push('l'); s.push('s');
        let first = s.current().map(|s| s.to_string());
        s.next();
        let second = s.current().map(|s| s.to_string());
        // If multiple results, they should differ
        if s.results.len() > 1 {
            assert_ne!(first, second);
        }
    }

    #[test]
    fn pop_removes_character() {
        let (mut s, _dir) = setup();
        s.push('l'); s.push('s');
        s.pop();
        assert_eq!(s.query(), "l");
    }

    #[test]
    fn clear_resets_state() {
        let (mut s, _dir) = setup();
        s.push('l');
        s.clear();
        assert_eq!(s.query(), "");
        assert!(s.current().is_none());
    }
}
