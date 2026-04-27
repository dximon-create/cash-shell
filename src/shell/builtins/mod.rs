// cash — built-in commands

pub mod agent;
pub mod copy;
pub mod go;
pub mod help;
pub mod setup;
pub mod explain;
pub mod remove;
pub mod show;
pub mod mv;
pub mod teach;

use super::pipeline::Stage;
use crate::store::Store;

#[derive(Debug)]
pub enum BuiltinResult {
    Ok,
    ChangeDir(std::path::PathBuf),
    Exit(i32),
    Err,
}

/// Dispatch a stage to the correct built-in.
/// Returns None if the command is not a built-in.
pub fn dispatch(stage: &Stage, cwd: &std::path::Path, store: &Store) -> Option<BuiltinResult> {
    match stage.name.as_str() {
        "show"   => Some(show::run(stage, cwd)),
        "go"     => Some(go::run(stage, cwd)),
        "copy"   => Some(copy::run(stage, cwd)),
        "move"   => Some(mv::run(stage, cwd)),
        "remove" => Some(remove::run(stage, cwd)),
        "teach"  => Some(teach::run(stage, store)),
        "help"    => Some(help::run(stage)),
        "explain" => { explain::run(stage); Some(BuiltinResult::Ok) }
        "setup"  => { setup::run(store); Some(BuiltinResult::Ok) }
        "agent"  => { agent::run(&stage.args, store); Some(BuiltinResult::Ok) }
        "cash"   => {
            // cash <subcommand> — dispatch subcommands
            match stage.args.first().map(|s| s.as_str()) {
                Some("setup")  => { setup::run(store); Some(BuiltinResult::Ok) }
                Some("alerts") => { setup::show_config(store); Some(BuiltinResult::Ok) }
                _ => {
                    println!("cash commands: setup, alerts");
                    Some(BuiltinResult::Ok)
                }
            }
        }
        "exit" | "quit" => {
            let code = stage.args.first()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            Some(BuiltinResult::Exit(code))
        }
        _ => None,
    }
}
