// cash — Shared Memory
//
// A thread-safe key/value store shared between all agents in a session.
// Agents with SharedMemoryRead can read.
// Agents with SharedMemoryWrite can write.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct SharedMemory {
    store: Arc<RwLock<HashMap<String, String>>>,
}

impl SharedMemory {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set(&self, key: &str, value: &str) {
        self.store.write().unwrap().insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.store.read().unwrap().get(key).cloned()
    }

    pub fn delete(&self, key: &str) -> bool {
        self.store.write().unwrap().remove(key).is_some()
    }

    pub fn keys(&self) -> Vec<String> {
        self.store.read().unwrap().keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.store.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.read().unwrap().is_empty()
    }
}

impl Default for SharedMemory {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mem = SharedMemory::new();
        mem.set("key1", "value1");
        assert_eq!(mem.get("key1"), Some("value1".to_string()));
    }

    #[test]
    fn get_missing_returns_none() {
        let mem = SharedMemory::new();
        assert_eq!(mem.get("missing"), None);
    }

    #[test]
    fn delete_removes_key() {
        let mem = SharedMemory::new();
        mem.set("key1", "value1");
        assert!(mem.delete("key1"));
        assert_eq!(mem.get("key1"), None);
    }

    #[test]
    fn delete_missing_returns_false() {
        let mem = SharedMemory::new();
        assert!(!mem.delete("nonexistent"));
    }

    #[test]
    fn shared_across_clones() {
        let mem1 = SharedMemory::new();
        let mem2 = mem1.clone();
        mem1.set("shared", "data");
        assert_eq!(mem2.get("shared"), Some("data".to_string()));
    }

    #[test]
    fn concurrent_access() {
        use std::thread;
        let mem = SharedMemory::new();
        let mut handles = vec![];
        for i in 0..10 {
            let m = mem.clone();
            handles.push(thread::spawn(move || {
                m.set(&format!("key{}", i), &format!("val{}", i));
            }));
        }
        for h in handles { h.join().unwrap(); }
        assert_eq!(mem.len(), 10);
    }
}
