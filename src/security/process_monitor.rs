// cash — Process Monitor
//
// Watches /proc on Linux to detect every process spawn.
// Runs on the security thread. Checks every 2 seconds.
//
// Records:
//   - PID, PPID, command, args, user, start time
//   - Whether process is new since last check
//   - Whether process is suspicious (privilege escalation, etc.)
//
// Writes all events to ~/.cash/process_events.db

use std::collections::HashMap;
use std::path::Path;
use rusqlite::{Connection, params};
use chrono::Utc;

use crate::store::Store;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid:     u32,
    pub ppid:    u32,
    pub name:    String,
    pub cmdline: String,
    pub uid:     u32,
    pub seen_at: String,
}

#[derive(Debug, Clone)]
pub struct ProcessEvent {
    pub kind:    EventKind,
    pub process: ProcessInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// New process appeared since last scan.
    Spawned,
    /// Process disappeared since last scan.
    Exited,
    /// Process matches a suspicious pattern.
    Suspicious { reason: String },
}

pub struct ProcessMonitor {
    store:    Store,
    known:    HashMap<u32, ProcessInfo>,
    patterns: Vec<SuspiciousPattern>,
}

#[derive(Debug, Clone)]
struct SuspiciousPattern {
    name:   String,
    reason: String,
}

impl ProcessMonitor {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        let mut monitor = Self {
            store,
            known: HashMap::new(),
            patterns: default_suspicious_patterns(),
        };
        monitor.init_db()?;
        // Take initial snapshot so we don't alert on existing processes.
        monitor.known = scan_proc();
        Ok(monitor)
    }

    /// Scan for changes. Returns new events.
    pub fn scan(&mut self) -> Vec<ProcessEvent> {
        let current = scan_proc();
        let mut events = Vec::new();

        // Find new processes.
        for (pid, info) in &current {
            if !self.known.contains_key(pid) {
                let event = ProcessEvent {
                    kind:    EventKind::Spawned,
                    process: info.clone(),
                };
                events.push(event.clone());
                self.record_event(&event);

                // Check against suspicious patterns.
                for pattern in &self.patterns {
                    if info.cmdline.contains(&pattern.name) || info.name.contains(&pattern.name) {
                        let sus = ProcessEvent {
                            kind: EventKind::Suspicious { reason: pattern.reason.clone() },
                            process: info.clone(),
                        };
                        events.push(sus.clone());
                        self.record_event(&sus);
                    }
                }
            }
        }

        // Find exited processes.
        for (pid, info) in &self.known {
            if !current.contains_key(pid) {
                let event = ProcessEvent {
                    kind:    EventKind::Exited,
                    process: info.clone(),
                };
                events.push(event.clone());
                self.record_event(&event);
            }
        }

        self.known = current;
        events
    }

    fn init_db(&self) -> anyhow::Result<()> {
        let conn = self.conn()?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS process_events (
                id       INTEGER PRIMARY KEY AUTOINCREMENT,
                kind     TEXT NOT NULL,
                pid      INTEGER NOT NULL,
                ppid     INTEGER NOT NULL,
                name     TEXT NOT NULL,
                cmdline  TEXT NOT NULL,
                uid      INTEGER NOT NULL,
                reason   TEXT,
                recorded_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS pe_recorded_at ON process_events(recorded_at DESC);
            CREATE INDEX IF NOT EXISTS pe_pid ON process_events(pid);
        ")?;
        Ok(())
    }

    fn record_event(&self, event: &ProcessEvent) {
        let Ok(conn) = self.conn() else { return };
        let (kind, reason) = match &event.kind {
            EventKind::Spawned               => ("spawned", None),
            EventKind::Exited                => ("exited",  None),
            EventKind::Suspicious { reason } => ("suspicious", Some(reason.as_str())),
        };
        let _ = conn.execute(
            "INSERT INTO process_events (kind, pid, ppid, name, cmdline, uid, reason, recorded_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                kind,
                event.process.pid,
                event.process.ppid,
                event.process.name,
                event.process.cmdline,
                event.process.uid,
                reason,
                Utc::now().to_rfc3339(),
            ],
        );
    }

    pub fn recent_events(&self, limit: usize) -> anyhow::Result<Vec<ProcessEvent>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT kind, pid, ppid, name, cmdline, uid, reason, recorded_at
               FROM process_events ORDER BY id DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let kind_str: String = row.get(0)?;
            let reason: Option<String> = row.get(6)?;
            let kind = match kind_str.as_str() {
                "spawned"    => EventKind::Spawned,
                "exited"     => EventKind::Exited,
                _            => EventKind::Suspicious {
                    reason: reason.unwrap_or_default()
                },
            };
            Ok(ProcessEvent {
                kind,
                process: ProcessInfo {
                    pid:     row.get::<_, i64>(1)? as u32,
                    ppid:    row.get::<_, i64>(2)? as u32,
                    name:    row.get(3)?,
                    cmdline: row.get(4)?,
                    uid:     row.get::<_, i64>(5)? as u32,
                    seen_at: row.get(7)?,
                },
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn conn(&self) -> anyhow::Result<Connection> {
        let c = Connection::open(self.store.db_path("process_events.db"))?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(c)
    }
}

/// Read all processes from /proc.
fn scan_proc() -> HashMap<u32, ProcessInfo> {
    let mut map = HashMap::new();

    #[cfg(target_os = "linux")]
    {
        let Ok(entries) = std::fs::read_dir("/proc") else { return map };
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let Ok(pid) = name_str.parse::<u32>() else { continue };

            let proc_dir = entry.path();
            let cmdline = read_proc_file(&proc_dir.join("cmdline"))
                .replace('\0', " ")
                .trim()
                .to_string();

            let stat = read_proc_file(&proc_dir.join("stat"));
            let ppid = parse_ppid(&stat);
            let proc_name = parse_proc_name(&stat);
            let uid = read_uid(&proc_dir.join("status"));

            map.insert(pid, ProcessInfo {
                pid,
                ppid,
                name: proc_name,
                cmdline: if cmdline.is_empty() { format!("[{}]", pid) } else { cmdline },
                uid,
                seen_at: Utc::now().to_rfc3339(),
            });
        }
    }

    map
}

fn read_proc_file(path: &Path) -> String {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

fn parse_ppid(stat: &str) -> u32 {
    // stat format: pid (name) state ppid ...
    let fields: Vec<&str> = stat.split_whitespace().collect();
    fields.get(3).and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn parse_proc_name(stat: &str) -> String {
    // Name is between first ( and first )
    if let (Some(start), Some(end)) = (stat.find('('), stat.find(')')) {
        stat[start + 1..end].to_string()
    } else {
        "unknown".to_string()
    }
}

fn read_uid(status_path: &Path) -> u32 {
    let content = read_proc_file(status_path);
    for line in content.lines() {
        if line.starts_with("Uid:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(uid_str) = parts.get(1) {
                return uid_str.parse().unwrap_or(0);
            }
        }
    }
    0
}

fn default_suspicious_patterns() -> Vec<SuspiciousPattern> {
    vec![
        SuspiciousPattern { name: "nmap".into(),       reason: "network scanner detected".into() },
        SuspiciousPattern { name: "masscan".into(),    reason: "mass port scanner detected".into() },
        SuspiciousPattern { name: "metasploit".into(), reason: "exploitation framework detected".into() },
        SuspiciousPattern { name: "msfconsole".into(), reason: "metasploit console detected".into() },
        SuspiciousPattern { name: "netcat".into(),     reason: "netcat detected".into() },
        SuspiciousPattern { name: " nc ".into(),       reason: "netcat detected".into() },
        SuspiciousPattern { name: "tcpdump".into(),    reason: "packet capture detected".into() },
        SuspiciousPattern { name: "wireshark".into(),  reason: "packet capture detected".into() },
        SuspiciousPattern { name: "chmod 777".into(),  reason: "dangerous permission change".into() },
        SuspiciousPattern { name: "chmod +s".into(),   reason: "setuid bit change detected".into() },
        SuspiciousPattern { name: "/etc/passwd".into(),reason: "passwd file access detected".into() },
        SuspiciousPattern { name: "/etc/shadow".into(),reason: "shadow file access detected".into() },
        SuspiciousPattern { name: "base64 -d".into(),  reason: "base64 decode — possible payload".into() },
        SuspiciousPattern { name: "curl | sh".into(),  reason: "remote code execution pattern".into() },
        SuspiciousPattern { name: "wget | sh".into(),  reason: "remote code execution pattern".into() },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (ProcessMonitor, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        let monitor = ProcessMonitor::new(store).unwrap();
        (monitor, dir)
    }

    #[test]
    fn monitor_initialises() {
        let (_monitor, _dir) = setup();
        // Just verify it doesn't panic on init
    }

    #[test]
    fn scan_returns_events_vec() {
        let (mut monitor, _dir) = setup();
        let events = monitor.scan();
        // On Linux we get real processes; on other OS returns empty
        let _ = events;
    }

    #[test]
    fn suspicious_pattern_matches_nmap() {
        let patterns = default_suspicious_patterns();
        let found = patterns.iter().any(|p| p.name == "nmap");
        assert!(found);
    }

    #[test]
    fn recent_events_empty_at_start() {
        let (monitor, _dir) = setup();
        let events = monitor.recent_events(10).unwrap();
        // No events recorded yet (initial snapshot doesn't record)
        assert!(events.is_empty());
    }

    #[test]
    fn parse_ppid_from_stat() {
        let stat = "1234 (bash) S 5678 1234 1234 0";
        assert_eq!(parse_ppid(stat), 5678);
    }

    #[test]
    fn parse_proc_name_from_stat() {
        let stat = "1234 (bash) S 5678";
        assert_eq!(parse_proc_name(stat), "bash");
    }
}
