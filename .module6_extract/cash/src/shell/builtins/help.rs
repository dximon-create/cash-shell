// cash built-in: help
//
// Contextual help. With no args shows the full command reference.
// With a command name shows detailed help for that command.

use super::super::pipeline::Stage;
use super::BuiltinResult;

pub fn run(stage: &Stage) -> BuiltinResult {
    match stage.args.first().map(|s| s.as_str()) {
        None | Some("help") => print_overview(),
        Some("show")        => print_show(),
        Some("go")          => print_go(),
        Some("copy")        => print_copy(),
        Some("move")        => print_move(),
        Some("remove")      => print_remove(),
        Some("teach")       => print_teach(),
        Some(other) => {
            println!("help: no help entry for '{}'", other);
            println!("Type 'help' for the full command list.");
        }
    }
    BuiltinResult::Ok
}

fn print_overview() {
    println!("cash v{} — Conscious Adaptive Secure Host", env!("CARGO_PKG_VERSION"));
    println!("© Personal Studio Limited\n");
    println!("BUILT-IN COMMANDS");
    println!("  show   [path]           List directory contents");
    println!("  go     [path]           Change directory  (go - for previous)");
    println!("  copy   [-r] <src> <dst> Copy file or directory");
    println!("  move   <src> <dst>      Move or rename");
    println!("  remove [-rf] <path...>  Delete (confirms destructive ops)");
    println!("  teach  <name> <cmd>     Teach cash a new command alias");
    println!("  help   [command]        Show this message or command detail");
    println!("  exit   [code]           Exit the shell\n");
    println!("SHELL FEATURES");
    println!("  Pipes       cmd1 | cmd2 | cmd3");
    println!("  Redirects   > file   >> file   < file   2> file");
    println!("  Env vars    VAR=value command\n");
    println!("Type 'help <command>' for detailed usage.");
}

fn print_show() {
    println!("show — list directory contents\n");
    println!("USAGE");
    println!("  show              list current directory");
    println!("  show <path>       list given path");
    println!("  show -a           include hidden files (dotfiles)\n");
    println!("Output columns: kind  size  name");
    println!("Directories are listed first, sorted alphabetically.");
}

fn print_go() {
    println!("go — change directory\n");
    println!("USAGE");
    println!("  go              go to home directory");
    println!("  go <path>       go to path");
    println!("  go ~            go to home directory");
    println!("  go -            go to previous directory");
    println!("  go ..           go up one level");
}

fn print_copy() {
    println!("copy — copy files or directories\n");
    println!("USAGE");
    println!("  copy <src> <dst>       copy a file");
    println!("  copy -r <src> <dst>    copy a directory recursively");
}

fn print_move() {
    println!("move — move or rename files or directories\n");
    println!("USAGE");
    println!("  move <src> <dst>    move src to dst");
    println!("                      works across filesystems");
}

fn print_remove() {
    println!("remove — delete files or directories\n");
    println!("USAGE");
    println!("  remove <file>          delete a file");
    println!("  remove -r <dir>        delete a directory recursively");
    println!("  remove -f <targets>    skip confirmation prompt");
    println!("  remove -rf <dir>       force-delete directory\n");
    println!("SAFETY");
    println!("  Deleting a directory or multiple files always prompts");
    println!("  unless -f is given. There is no undo.");
}

fn print_teach() {
    println!("teach — teach cash a new command\n");
    println!("USAGE");
    println!("  teach <name> <command>");
    println!("  teach list files 'ls -la'    teach 'list files' to run ls -la\n");
    println!("Taught commands are stored in ~/.cash/ and survive restarts.");
    println!("See Module 5 (Memory Store) for full details.");
}
