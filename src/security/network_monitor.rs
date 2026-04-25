// cash — Network Monitor
//
// Watches /proc/net/tcp and /proc/net/tcp6 on Linux.
// Records every outbound connection: destination IP, port, process.
// Detects connections to blocked domains/IPs.
// Writes all events to ~/.cash/network_events.db

use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, params};
use chrono::Utc;

use crate::store::Store;

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkConnection {
    pub local_ip:    String,
    pub local_port:  u16,
    pub remote_ip:   String,
    pub remote_port: u16,
    pub state:       ConnectionState,
    pub uid:         u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Established,
    Listen,
    TimeWait,
    CloseWait,
    Other(String),
}

impl ConnectionState {
    fn from_hex(s: &str) -> Self {
        match s {
            "01" => Self::Established,
            "0A" => Self::Listen,
            "06" => Self::TimeWait,
            "08" => Self::CloseWait,
            other => Self::Other(other.to_string()),
        }
    }
}

pub struct NetworkMonitor {
    store:      Store,
    known:      HashSet<String>, // connection fingerprints
    blocklist:  Arc<Mutex<Vec<String>>>,
}

impl NetworkMonitor {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        let mut monitor = Self {
            store,
            known: HashSet::new(),
            blocklist: Arc::new(Mutex::new(Vec::new())),
        };
        monitor.init_db()?;
        // Take initial snapshot.
        let current = monitor.read_connections();
        for conn in &current {
            monitor.known.insert(conn_fingerprint(conn));
        }
        Ok(monitor)
    }

    pub fn add_to_blocklist(&self, ip_or_domain: &str) {
        self.blocklist.lock().unwrap().push(ip_or_domain.to_lowercase());
    }

    pub fn is_blocked(&self, ip: &str) -> bool {
        self.blocklist.lock().unwrap()
            .iter()
            .any(|b| ip.contains(b.as_str()))
    }

    /// Scan for new connections. Returns new ones since last scan.
    pub fn scan(&mut self) -> Vec<NetworkConnection> {
        let current = self.read_connections();
        let mut new_connections = Vec::new();

        for conn in &current {
            let fp = conn_fingerprint(conn);
            if !self.known.contains(&fp) {
                self.known.insert(fp);
                self.record_connection(conn);

                // Check blocklist.
                if self.is_blocked(&conn.remote_ip) {
                    self.alert_blocked(conn);
                }

                if conn.state == ConnectionState::Established {
                    new_connections.push(conn.clone());
                }
            }
        }

        // Clean up known set — remove connections that no longer exist.
        let current_fps: HashSet<String> = current.iter().map(conn_fingerprint).collect();
        self.known.retain(|fp| current_fps.contains(fp));

        new_connections
    }

    pub fn recent_connections(&self, limit: usize) -> anyhow::Result<Vec<NetworkConnection>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT local_ip, local_port, remote_ip, remote_port, state, uid
               FROM network_events ORDER BY id DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(NetworkConnection {
                local_ip:    row.get(0)?,
                local_port:  row.get::<_, i64>(1)? as u16,
                remote_ip:   row.get(2)?,
                remote_port: row.get::<_, i64>(3)? as u16,
                state:       ConnectionState::Other(row.get(4)?),
                uid:         row.get::<_, i64>(5)? as u32,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn read_connections(&self) -> Vec<NetworkConnection> {
        let mut connections = Vec::new();

        #[cfg(target_os = "linux")]
        {
            for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
                if let Ok(content) = std::fs::read_to_string(path) {
                    for line in content.lines().skip(1) {
                        if let Some(conn) = parse_proc_net_line(line) {
                            connections.push(conn);
                        }
                    }
                }
            }
        }

        connections
    }

    fn init_db(&self) -> anyhow::Result<()> {
        let conn = self.conn()?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS network_events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                local_ip    TEXT NOT NULL,
                local_port  INTEGER NOT NULL,
                remote_ip   TEXT NOT NULL,
                remote_port INTEGER NOT NULL,
                state       TEXT NOT NULL,
                uid         INTEGER NOT NULL,
                blocked     INTEGER NOT NULL DEFAULT 0,
                recorded_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS ne_recorded_at ON network_events(recorded_at DESC);
            CREATE INDEX IF NOT EXISTS ne_remote_ip ON network_events(remote_ip);
        ")?;
        Ok(())
    }

    fn record_connection(&self, nc: &NetworkConnection) {
        let Ok(conn) = self.conn() else { return };
        let state_str = match &nc.state {
            ConnectionState::Established => "ESTABLISHED",
            ConnectionState::Listen      => "LISTEN",
            ConnectionState::TimeWait    => "TIME_WAIT",
            ConnectionState::CloseWait   => "CLOSE_WAIT",
            ConnectionState::Other(s)    => s.as_str(),
        };
        let _ = conn.execute(
            "INSERT INTO network_events (local_ip,local_port,remote_ip,remote_port,state,uid,recorded_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                nc.local_ip, nc.local_port as i64,
                nc.remote_ip, nc.remote_port as i64,
                state_str, nc.uid as i64,
                Utc::now().to_rfc3339(),
            ],
        );
    }

    fn alert_blocked(&self, nc: &NetworkConnection) {
        eprintln!(
            "[cash security] BLOCKED CONNECTION: {}:{} → {}:{}",
            nc.local_ip, nc.local_port, nc.remote_ip, nc.remote_port
        );
        self.write_alert(&format!(
            "blocked connection to {}:{}",
            nc.remote_ip, nc.remote_port
        ));
    }

    fn write_alert(&self, msg: &str) {
        use std::io::Write;
        let path = self.store.root.join("security_alerts.log");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{} [NETWORK] {}", Utc::now().to_rfc3339(), msg);
        }
    }

    fn conn(&self) -> anyhow::Result<Connection> {
        let c = Connection::open(self.store.db_path("network_events.db"))?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(c)
    }
}

/// Parse a line from /proc/net/tcp.
fn parse_proc_net_line(line: &str) -> Option<NetworkConnection> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 8 { return None; }

    let (local_ip, local_port) = parse_addr(fields[1])?;
    let (remote_ip, remote_port) = parse_addr(fields[2])?;
    let state = ConnectionState::from_hex(fields[3]);
    let uid: u32 = fields[7].parse().ok()?;

    Some(NetworkConnection { local_ip, local_port, remote_ip, remote_port, state, uid })
}

/// Parse hex address:port from /proc/net/tcp format.
fn parse_addr(s: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return None; }

    let ip_hex = parts[0];
    let port = u16::from_str_radix(parts[1], 16).ok()?;

    // IPv4: 4 bytes little-endian hex
    if ip_hex.len() == 8 {
        let n = u32::from_str_radix(ip_hex, 16).ok()?;
        let ip = Ipv4Addr::from(n.swap_bytes());
        Some((ip.to_string(), port))
    } else {
        // IPv6 — just return raw for now
        Some((ip_hex.to_string(), port))
    }
}

fn conn_fingerprint(nc: &NetworkConnection) -> String {
    format!("{}:{}-{}:{}", nc.local_ip, nc.local_port, nc.remote_ip, nc.remote_port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (NetworkMonitor, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Store { root: dir.path().to_path_buf() };
        (NetworkMonitor::new(store).unwrap(), dir)
    }

    #[test]
    fn monitor_initialises() {
        let (_m, _dir) = setup();
    }

    #[test]
    fn blocklist_works() {
        let (m, _dir) = setup();
        m.add_to_blocklist("192.168.1.100");
        assert!(m.is_blocked("192.168.1.100"));
        assert!(!m.is_blocked("192.168.1.200"));
    }

    #[test]
    fn parse_addr_ipv4() {
        let result = parse_addr("0100007F:0050");
        assert!(result.is_some());
        let (ip, port) = result.unwrap();
        assert_eq!(port, 80);
    }

    #[test]
    fn parse_proc_net_line_valid() {
        let line = "  0: 0100007F:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 12345 1 0000000000000000 100 0 0 10 0";
        let result = parse_proc_net_line(line);
        assert!(result.is_some());
    }

    #[test]
    fn recent_connections_empty_at_start() {
        let (m, _dir) = setup();
        assert!(m.recent_connections(10).unwrap().is_empty());
    }
}
