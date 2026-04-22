// cash built-in: teach
//
// Teach cash a new natural-language command alias.
// Stored in ~/.cash/memory.db via the memory store.
//
// Usage:
//   teach <name> <command>
//   teach list files 'ls -la'
//   teach show processes 'ps aux | sort -k3 -rn'
//   teach list            → lists all taught commands
//   teach forget <name>   → removes a taught command

use super::super::pipeline::Stage;
use super::BuiltinResult;
use crate::store::Store;

pub fn run(stage: &Stage, store: &Store) -> BuiltinResult {
    match stage.args.first().map(|s| s.as_str()) {
        None => {
            // No args: list everything
            return run_list(store);
        }
        Some("list") if stage.args.len() == 1 => {
            return run_list(store);
        }
        Some("forget") => {
            let name = stage.args[1..].join(" ");
            if name.is_empty() {
                eprintln!("teach: usage: teach forget <name>");
                return BuiltinResult::Err;
            }
            return run_forget(store, &name);
        }
        _ => {}
    }

    // teach <name...> <command>
    // The last argument is the command. Everything before it is the name.
    if stage.args.len() < 2 {
        eprintln!("teach: usage: teach <name> <command>");
        eprintln!("       teach list files 'ls -la'");
        return BuiltinResult::Err;
    }

    let command = stage.args.last().unwrap().clone();
    let name = stage.args[..stage.args.len() - 1].join(" ");

    match crate::store::memory::teach(store, &name, &command) {
        Ok(()) => {
            println!("taught: '{}' → {}", name, command);
            BuiltinResult::Ok
        }
        Err(e) => {
            eprintln!("teach: {}", e);
            BuiltinResult::Err
        }
    }
}

fn run_list(store: &Store) -> BuiltinResult {
    match crate::store::memory::list(store) {
        Ok(memories) if memories.is_empty() => {
            println!("No taught commands yet.");
            println!("Try: teach list files 'ls -la'");
        }
        Ok(memories) => {
            println!("{} taught command(s):\n", memories.len());
            for m in &memories {
                println!("  {:30}  →  {}", m.name, m.command);
            }
        }
        Err(e) => {
            eprintln!("teach: {}", e);
            return BuiltinResult::Err;
        }
    }
    BuiltinResult::Ok
}

fn run_forget(store: &Store, name: &str) -> BuiltinResult {
    match crate::store::memory::forget(store, name) {
        Ok(true) => {
            println!("forgotten: '{}'", name);
            BuiltinResult::Ok
        }
        Ok(false) => {
            eprintln!("teach: '{}' not found", name);
            BuiltinResult::Err
        }
        Err(e) => {
            eprintln!("teach: {}", e);
            BuiltinResult::Err
        }
    }
}
