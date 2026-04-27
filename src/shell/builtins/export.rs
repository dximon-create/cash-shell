
use super::super::pipeline::Stage;
use crate::store::Store;
use std::io::Write;

pub fn run(stage: &Stage, store: &Store) {
    let fmt = stage.args.first().map(|s| s.as_str()).unwrap_or("csv");
    let out_path = store.db_path("history.db")
        .parent().unwrap().join(format!("export.{}", fmt));

    match fmt {
        "csv"  => export_csv(store, &out_path),
        "json" => export_json(store, &out_path),
        _ => {
            println!("Usage: export [csv|json]");
            println!("Default: csv");
        }
    }
}

fn export_csv(store: &Store, path: &std::path::Path) {
    match read_history(store) {
        Ok(rows) => {
            let mut f = match std::fs::File::create(path) {
                Ok(f) => f, Err(e) => { eprintln!("cash: {}", e); return; }
            };
            writeln!(f, "command,cwd,exit_code,ran_at").ok();
            for (cmd, cwd, exit, ran) in &rows {
                writeln!(f, "{},{},{},{}", csv_esc(cmd), csv_esc(cwd), exit, ran).ok();
            }
            println!("\n  ✓ Exported {} rows to {}\n", rows.len(), path.display());
        }
        Err(e) => eprintln!("cash: {}", e),
    }
}

fn export_json(store: &Store, path: &std::path::Path) {
    match read_history(store) {
        Ok(rows) => {
            let mut f = match std::fs::File::create(path) {
                Ok(f) => f, Err(e) => { eprintln!("cash: {}", e); return; }
            };
            writeln!(f, "[").ok();
            let last = rows.len().saturating_sub(1);
            for (i, (cmd, cwd, exit, ran)) in rows.iter().enumerate() {
                let comma = if i == last { "" } else { "," };
                writeln!(f, "  {{\"command\":{},\"cwd\":{},\"exit_code\":{},\"ran_at\":{}}}{}", 
                    js(cmd), js(cwd), exit, js(ran), comma).ok();
            }
            writeln!(f, "]").ok();
            println!("\n  ✓ Exported {} rows to {}\n", rows.len(), path.display());
        }
        Err(e) => eprintln!("cash: {}", e),
    }
}

fn read_history(store: &Store) -> anyhow::Result<Vec<(String,String,i32,String)>> {
    let conn = rusqlite::Connection::open(store.db_path("history.db"))?;
    let mut stmt = conn.prepare(
        "SELECT command, cwd, exit_code, ran_at FROM history ORDER BY id"
    )?;
    let rows = stmt.query_map([], |r| Ok((
        r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?
    )))?.flatten().collect();
    Ok(rows)
}

fn csv_esc(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else { s.to_string() }
}

fn js(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
