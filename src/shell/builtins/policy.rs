// cash — policy profiles builtin
use super::super::pipeline::Stage;
use crate::store::Store;

pub fn run(stage: &Stage, store: &Store) {
    let sub = stage.args.first().map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list"   => cmd_list(store),
        "create" => cmd_create(stage, store),
        "apply"  => cmd_apply(stage, store),
        "show"   => cmd_show(stage, store),
        "delete" => cmd_delete(stage, store),
        _ => {
            println!();
            println!("  cash policy commands:");
            println!("    policy list              — list all profiles");
            println!("    policy create <name>     — create a new profile");
            println!("    policy apply  <name>     — switch to a profile");
            println!("    policy show   <name>     — show profile rules");
            println!("    policy delete <name>     — delete a profile");
            println!();
            println!("  Built-in profiles: default, strict, developer");
            println!();
        }
    }
}

fn cmd_list(store: &Store) {
    ensure_table(store);
    let active = active_profile(store);
    println!();
    println!("  ── Policy Profiles ─────────────────────────────");
    println!();
    for p in builtin_profiles() {
        let marker = if p.name == active { " ◀ active" } else { "" };
        println!("  {}  {:<16} {}{}", if p.name == active { "●" } else { "○" }, p.name, p.description, marker);
    }
    match load_user_profiles(store) {
        Ok(profiles) => {
            for p in &profiles {
                let marker = if p.name == active { " ◀ active" } else { "" };
                println!("  {}  {:<16} {}{}", if p.name == active { "●" } else { "○" }, p.name, p.description, marker);
            }
            if profiles.is_empty() {
                println!();
                println!("  No custom profiles yet. Use \'policy create <name>\' to add one.");
            }
        }
        Err(e) => eprintln!("  cash: {}", e),
    }
    println!();
}

fn cmd_create(stage: &Stage, store: &Store) {
    let name = match stage.args.get(1) {
        Some(n) => n.clone(),
        None => { println!("Usage: policy create <name>"); return; }
    };
    if builtin_profiles().iter().any(|p| p.name == name) {
        println!("cash: \'{}\' is a built-in profile and cannot be overwritten.", name);
        return;
    }
    ensure_table(store);
    let desc = prompt(&format!("Description for \'{}\': ", name));
    let block = prompt("Block commands above risk score [0-10, default 7]: ");
    let block: f64 = block.trim().parse().unwrap_or(7.0_f64).max(0.0).min(10.0);
    let warn = prompt("Warn on commands above risk score [0-10, default 4]: ");
    let warn: f64 = warn.trim().parse().unwrap_or(4.0_f64).max(0.0).min(10.0);
    let rules = format!("block_above={},warn_above={}", block, warn);
    match store_profile(store, &name, &desc, &rules) {
        Ok(_) => { println!(); println!("  ✓ Profile \'{}\' created.", name); println!("    Run \'policy apply {}\' to activate it.", name); println!(); }
        Err(e) => eprintln!("  cash: {}", e),
    }
}

fn cmd_apply(stage: &Stage, store: &Store) {
    let name = match stage.args.get(1) {
        Some(n) => n.as_str(),
        None => { println!("Usage: policy apply <name>"); return; }
    };
    let valid = builtin_profiles().iter().any(|p| p.name == name)
        || load_user_profiles(store).unwrap_or_default().iter().any(|p| p.name == name);
    if !valid {
        println!("cash: profile \'{}\' not found. Run \'policy list\' to see options.", name);
        return;
    }
    match set_active(store, name) {
        Ok(_) => {
            println!();
            println!("  ✓ Policy \'{}\' is now active.", name);
            if let Some(p) = builtin_profiles().into_iter().find(|p| p.name == name) {
                println!();
                for rule in &p.rules { println!("    • {}", rule); }
            }
            println!();
        }
        Err(e) => eprintln!("  cash: {}", e),
    }
}

fn cmd_show(stage: &Stage, store: &Store) {
    let name = match stage.args.get(1) {
        Some(n) => n.as_str(),
        None => { println!("Usage: policy show <name>"); return; }
    };
    println!();
    if let Some(p) = builtin_profiles().into_iter().find(|p| p.name == name) {
        let active = active_profile(store);
        println!("  Profile    : {} {}", p.name, if p.name == active { "(active)" } else { "" });
        println!("  Type       : built-in");
        println!("  Description: {}", p.description);
        println!();
        println!("  Rules:");
        for rule in &p.rules { println!("    • {}", rule); }
        println!();
        return;
    }
    match load_user_profiles(store) {
        Ok(profiles) => {
            if let Some(p) = profiles.iter().find(|p| p.name == name) {
                println!("  Profile    : {}", p.name);
                println!("  Type       : custom");
                println!("  Description: {}", p.description);
                println!();
            } else {
                println!("  cash: profile \'{}\' not found.", name);
            }
        }
        Err(e) => eprintln!("  cash: {}", e),
    }
    println!();
}

fn cmd_delete(stage: &Stage, store: &Store) {
    let name = match stage.args.get(1) {
        Some(n) => n.as_str(),
        None => { println!("Usage: policy delete <name>"); return; }
    };
    if builtin_profiles().iter().any(|p| p.name == name) {
        println!("cash: built-in profiles cannot be deleted.");
        return;
    }
    if active_profile(store) == name {
        println!("cash: cannot delete the active profile. Apply another first.");
        return;
    }
    match delete_profile(store, name) {
        Ok(_) => println!("  ✓ Profile \'{}\' deleted.", name),
        Err(e) => eprintln!("  cash: {}", e),
    }
}

struct Profile { name: &'static str, description: &'static str, rules: Vec<&'static str> }

fn builtin_profiles() -> Vec<Profile> {
    vec![
        Profile { name: "default", description: "Balanced — warns on medium risk, blocks critical", rules: vec!["Allow risk 0-3 silently", "Warn on risk 4-6", "Approve risk 7-8", "Block risk 9-10"] },
        Profile { name: "strict",  description: "Maximum security — approves everything above LOW",  rules: vec!["Allow risk 0-2 silently", "Approve risk 3+", "Block risk 8-10", "sudo always blocked"] },
        Profile { name: "developer", description: "Relaxed for development — fewer interruptions",  rules: vec!["Allow risk 0-5 silently", "Warn risk 6-7", "Approve risk 8", "Block risk 9-10", "git/cargo/npm always allowed"] },
    ]
}

struct UserProfile { name: String, description: String }

fn ensure_table(store: &Store) {
    if let Ok(conn) = rusqlite::Connection::open(store.db_path("cash.db")) {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS policy_profiles (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, description TEXT NOT NULL DEFAULT \'\', rules TEXT NOT NULL DEFAULT \'\', created_at TEXT NOT NULL DEFAULT (datetime(\'now\')));").ok();
    }
}

fn load_user_profiles(store: &Store) -> anyhow::Result<Vec<UserProfile>> {
    let conn = rusqlite::Connection::open(store.db_path("cash.db"))?;
    let _ = conn.execute_batch("CREATE TABLE IF NOT EXISTS policy_profiles (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, description TEXT NOT NULL DEFAULT \'\', rules TEXT NOT NULL DEFAULT \'\', created_at TEXT NOT NULL DEFAULT (datetime(\'now\')));");
    let mut stmt = conn.prepare("SELECT name, description FROM policy_profiles ORDER BY created_at")?;
    let rows = stmt.query_map([], |row| Ok(UserProfile { name: row.get(0)?, description: row.get(1)? }))?;
    Ok(rows.flatten().collect())
}

fn store_profile(store: &Store, name: &str, desc: &str, rules: &str) -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open(store.db_path("cash.db"))?;
    conn.execute("INSERT OR REPLACE INTO policy_profiles (name, description, rules) VALUES (?1, ?2, ?3)", rusqlite::params![name, desc, rules])?;
    Ok(())
}

fn delete_profile(store: &Store, name: &str) -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open(store.db_path("cash.db"))?;
    conn.execute("DELETE FROM policy_profiles WHERE name = ?1", rusqlite::params![name])?;
    Ok(())
}

fn active_profile(store: &Store) -> String {
    if let Ok(conn) = rusqlite::Connection::open(store.db_path("cash.db")) {
        let _ = conn.execute_batch("CREATE TABLE IF NOT EXISTS cash_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);");
        if let Ok(val) = conn.query_row("SELECT value FROM cash_settings WHERE key = \'active_policy\'", [], |r| r.get::<_,String>(0)) {
            return val;
        }
    }
    "default".to_string()
}

fn set_active(store: &Store, name: &str) -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open(store.db_path("cash.db"))?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS cash_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
    conn.execute("INSERT OR REPLACE INTO cash_settings (key, value) VALUES (\'active_policy\', ?1)", rusqlite::params![name])?;
    Ok(())
}

fn prompt(label: &str) -> String {
    use std::io::{self, Write};
    print!("{}", label);
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    buf.trim().to_string()
}
