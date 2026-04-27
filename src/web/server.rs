// cash — web dashboard
// Starts a local HTTP server on port 8080 and serves the audit/history dashboard.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

pub struct DashboardServer {
    cash_dir: Arc<PathBuf>,
    port: u16,
}

impl DashboardServer {
    pub fn new(cash_dir: PathBuf, port: u16) -> Self {
        Self { cash_dir: Arc::new(cash_dir), port }
    }

    pub fn start(&self) {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("cash dashboard: could not bind to {}: {}", addr, e);
                return;
            }
        };

        println!();
        println!("  ╔══════════════════════════════════════╗");
        println!("  ║   cash Dashboard                     ║");
        println!("  ║   http://localhost:{}               ║", self.port);
        println!("  ║   Press Ctrl+C to stop               ║");
        println!("  ╚══════════════════════════════════════╝");
        println!();

        // Try to open browser
        let url = format!("http://localhost:{}", self.port);
        open_browser(&url);

        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let cash_dir = Arc::clone(&self.cash_dir);
                    thread::spawn(move || handle_connection(s, &cash_dir));
                }
                Err(_) => break,
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, cash_dir: &PathBuf) {
    let mut buf = [0u8; 2048];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");

    let (status, content_type, body) = match path {
        "/api/history" => {
            let json = read_history_json(cash_dir);
            ("200 OK", "application/json", json)
        }
        "/api/alerts" => {
            let json = read_alerts_json(cash_dir);
            ("200 OK", "application/json", json)
        }
        "/api/stats" => {
            let json = read_stats_json(cash_dir);
            ("200 OK", "application/json", json)
        }
        _ => {
            ("200 OK", "text/html; charset=utf-8", dashboard_html())
        }
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status, content_type, body.len(), body
    );

    stream.write_all(response.as_bytes()).ok();
}

// ── Data readers ─────────────────────────────────────────────────────────────

fn read_history_json(cash_dir: &PathBuf) -> String {
    let db_path = cash_dir.join("history.db");
    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            let result = conn.prepare(
                "SELECT command, cwd, exit_code, ran_at FROM history ORDER BY id DESC LIMIT 100"
            ).and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                        row.get::<_, i32>(2).unwrap_or(0),
                        row.get::<_, String>(3).unwrap_or_default(),
                    ))
                })?;
                let mut entries = Vec::new();
                for row in rows.flatten() {
                    entries.push(format!(
                        "{{\"command\":{},\"cwd\":{},\"exit_code\":{},\"ran_at\":{}}}",
                        json_str(&row.0), json_str(&row.1), row.2, json_str(&row.3)
                    ));
                }
                Ok(entries)
            });
            match result {
                Ok(entries) => format!("[{}]", entries.join(",")),
                Err(_) => "[]".to_string(),
            }
        }
        Err(_) => "[]".to_string(),
    }
}

fn read_alerts_json(cash_dir: &PathBuf) -> String {
    let db_path = cash_dir.join("history.db");
    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            // Read from audit log — commands with high risk scores
            let result = conn.prepare(
                "SELECT command, cwd, exit_code, ran_at FROM history WHERE exit_code != 0 OR command LIKE '%sudo%' OR command LIKE '%rm -rf%' OR command LIKE '%chmod%' ORDER BY id DESC LIMIT 50"
            ).and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                        row.get::<_, i32>(2).unwrap_or(0),
                        row.get::<_, String>(3).unwrap_or_default(),
                    ))
                })?;
                let mut entries = Vec::new();
                for row in rows.flatten() {
                    let level = risk_level_for(&row.0);
                    entries.push(format!(
                        "{{\"command\":{},\"cwd\":{},\"level\":{},\"ran_at\":{}}}",
                        json_str(&row.0), json_str(&row.1), json_str(level), json_str(&row.3)
                    ));
                }
                Ok(entries)
            });
            match result {
                Ok(entries) => format!("[{}]", entries.join(",")),
                Err(_) => "[]".to_string(),
            }
        }
        Err(_) => "[]".to_string(),
    }
}

fn read_stats_json(cash_dir: &PathBuf) -> String {
    let db_path = cash_dir.join("history.db");
    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            let total: i64 = conn
                .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
                .unwrap_or(0);
            let today: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM history WHERE ran_at >= date('now')",
                    [], |r| r.get(0),
                )
                .unwrap_or(0);
            let errors: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM history WHERE exit_code != 0",
                    [], |r| r.get(0),
                )
                .unwrap_or(0);
            format!(
                "{{\"total\":{},\"today\":{},\"errors\":{}}}",
                total, today, errors
            )
        }
        Err(_) => "{\"total\":0,\"today\":0,\"errors\":0}".to_string(),
    }
}

fn risk_level_for(cmd: &str) -> &'static str {
    let cmd = cmd.to_lowercase();
    if cmd.contains("rm -rf") || cmd.contains("dd ") || cmd.contains("mkfs") {
        "CRITICAL"
    } else if cmd.contains("sudo") || cmd.contains("chmod") || cmd.contains("iptables") {
        "HIGH"
    } else if cmd.contains("curl") || cmd.contains("wget") || cmd.contains("ssh") {
        "MEDIUM"
    } else {
        "LOW"
    }
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

// ── Browser opener ────────────────────────────────────────────────────────────

fn open_browser(url: &str) {
    #[cfg(target_os = "linux")]
    {
        // WSL: try wslview first, then xdg-open
        if std::process::Command::new("wslview").arg(url).spawn().is_err() {
            std::process::Command::new("xdg-open").arg(url).spawn().ok();
        }
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().ok();
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd").args(["/C", "start", url]).spawn().ok();
    }
}

// ── HTML ──────────────────────────────────────────────────────────────────────

fn dashboard_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>cash — Dashboard</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: #0d0d0d;
    color: #f5f5f5;
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 15px;
    min-height: 100vh;
  }
  header {
    background: #111;
    border-bottom: 1px solid #222;
    padding: 18px 32px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  header h1 {
    font-size: 22px;
    font-weight: 900;
    color: #00ff88;
    letter-spacing: 3px;
  }
  header span {
    font-size: 12px;
    color: #ffffff;
  }
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 1px;
  }
  .badge-live { background: #003322; color: #00ff88; border: 1px solid #00ff88; }
  main { padding: 24px 32px; max-width: 1400px; margin: 0 auto; }
  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    margin-bottom: 28px;
  }
  .stat-card {
    background: #111;
    border: 1px solid #222;
    border-radius: 6px;
    padding: 20px 24px;
  }
  .stat-card .label { font-size: 11px; color: #999; letter-spacing: 2px; text-transform: uppercase; margin-bottom: 8px; }
  .stat-card .value { font-size: 36px; font-weight: 700; color: #00ff88; }
  .stat-card .value.red { color: #ff4444; }
  .stat-card .value.yellow { color: #ffaa00; }
  .section { margin-bottom: 28px; }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .section-title {
    font-size: 12px;
    letter-spacing: 2px;
    text-transform: uppercase;
    color: #555;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    background: #111;
    border: 1px solid #222;
    border-radius: 6px;
    overflow: hidden;
  }
  th {
    text-align: left;
    padding: 10px 16px;
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: #444;
    border-bottom: 1px solid #1e1e1e;
    background: #0d0d0d;
  }
  td {
    padding: 10px 16px;
    border-bottom: 1px solid #191919;
    color: #f0f0f0;
    font-size: 14px;
    font-weight: 500;
  }
  tr:last-child td { border-bottom: none; }
  tr:hover td { background: #161616; }
  .cmd { color: #00ff88; font-weight: 600; }
  .cwd { color: #999; font-size: 13px; }
  .time { color: #888; font-size: 13px; }
  .exit-ok { color: #00ff88; }
  .exit-err { color: #ff4444; }
  .level-critical { color: #ff2222; font-weight: 700; }
  .level-high     { color: #ff6600; font-weight: 700; }
  .level-medium   { color: #ffaa00; }
  .level-low      { color: #00ff88; }
  .empty { padding: 32px; text-align: center; color: #666; }
  .refresh-note { font-size: 12px; color: #ffffff; font-weight: 600; }
</style>
</head>
<body>
<header>
  <h1>cash</h1>
  <div style="display:flex;align-items:center;gap:16px;">
    <span class="refresh-note" id="last-refresh">Loading...</span>
    <span class="badge badge-live">● LIVE</span>
  </div>
</header>
<main>
  <div class="stats">
    <div class="stat-card">
      <div class="label">Total Commands</div>
      <div class="value" id="stat-total">—</div>
    </div>
    <div class="stat-card">
      <div class="label">Today</div>
      <div class="value yellow" id="stat-today">—</div>
    </div>
    <div class="stat-card">
      <div class="label">Errors</div>
      <div class="value red" id="stat-errors">—</div>
    </div>
  </div>

  <div class="section">
    <div class="section-header">
      <span class="section-title">Security Alerts</span>
    </div>
    <table>
      <thead>
        <tr>
          <th>Level</th>
          <th>Command</th>
          <th>Directory</th>
          <th>Time</th>
        </tr>
      </thead>
      <tbody id="alerts-body">
        <tr><td colspan="4" class="empty">Loading alerts...</td></tr>
      </tbody>
    </table>
  </div>

  <div class="section">
    <div class="section-header">
      <span class="section-title">Command History</span>
    </div>
    <table>
      <thead>
        <tr>
          <th>Command</th>
          <th>Directory</th>
          <th>Exit</th>
          <th>Time</th>
        </tr>
      </thead>
      <tbody id="history-body">
        <tr><td colspan="4" class="empty">Loading history...</td></tr>
      </tbody>
    </table>
  </div>
</main>

<script>
  function fmt(dt) {
    if (!dt) return '—';
    try { return new Date(dt).toLocaleString(); } catch { return dt; }
  }

  function levelClass(l) {
    const m = { CRITICAL: 'level-critical', HIGH: 'level-high', MEDIUM: 'level-medium', LOW: 'level-low' };
    return m[l] || 'level-low';
  }

  async function loadStats() {
    try {
      const r = await fetch('/api/stats');
      const d = await r.json();
      document.getElementById('stat-total').textContent  = d.total  ?? '—';
      document.getElementById('stat-today').textContent  = d.today  ?? '—';
      document.getElementById('stat-errors').textContent = d.errors ?? '—';
    } catch {}
  }

  async function loadHistory() {
    try {
      const r = await fetch('/api/history');
      const rows = await r.json();
      const tbody = document.getElementById('history-body');
      if (!rows.length) {
        tbody.innerHTML = '<tr><td colspan="4" class="empty">No history yet.</td></tr>';
        return;
      }
      tbody.innerHTML = rows.map(r => `
        <tr>
          <td class="cmd">${esc(r.command)}</td>
          <td class="cwd">${esc(r.cwd)}</td>
          <td class="${r.exit_code === 0 ? 'exit-ok' : 'exit-err'}">${r.exit_code === 0 ? '✓' : '✗ ' + r.exit_code}</td>
          <td class="time">${fmt(r.ran_at)}</td>
        </tr>`).join('');
    } catch {}
  }

  async function loadAlerts() {
    try {
      const r = await fetch('/api/alerts');
      const rows = await r.json();
      const tbody = document.getElementById('alerts-body');
      if (!rows.length) {
        tbody.innerHTML = '<tr><td colspan="4" class="empty">No alerts. All clear.</td></tr>';
        return;
      }
      tbody.innerHTML = rows.map(r => `
        <tr>
          <td class="${levelClass(r.level)}">${esc(r.level)}</td>
          <td class="cmd">${esc(r.command)}</td>
          <td class="cwd">${esc(r.cwd)}</td>
          <td class="time">${fmt(r.ran_at)}</td>
        </tr>`).join('');
    } catch {}
  }

  function esc(s) {
    return String(s ?? '').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
  }

  function refresh() {
    loadStats();
    loadHistory();
    loadAlerts();
    document.getElementById('last-refresh').textContent =
      'Last updated: ' + new Date().toLocaleTimeString();
  }

  refresh();
  setInterval(refresh, 5000);
</script>
</body>
</html>"#.to_string()
}
