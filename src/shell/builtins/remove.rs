// cash built-in: remove
//
// Delete files or directories. ALWAYS confirms before deleting a directory
// or multiple files. Never fails silently.
//
// Usage:
//   remove <file>            delete a single file (no prompt)
//   remove <file1> <file2>   delete multiple files (confirms)
//   remove -r <dir>          delete directory recursively (confirms)
//   remove -f <file>         skip confirmation (force)

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::super::pipeline::Stage;
use super::BuiltinResult;

pub fn run(stage: &Stage, cwd: &Path) -> BuiltinResult {
    let mut force = false;
    let mut recursive = false;
    let mut targets: Vec<PathBuf> = Vec::new();

    for arg in &stage.args {
        match arg.as_str() {
            "-f" | "--force" => force = true,
            "-r" | "--recursive" => recursive = true,
            "-rf" | "-fr" => {
                force = true;
                recursive = true;
            }
            p => targets.push(resolve(p, cwd)),
        }
    }

    if targets.is_empty() {
        eprintln!("remove: no targets specified");
        return BuiltinResult::Err;
    }

    // Validate before touching anything.
    for t in &targets {
        if !t.exists() {
            eprintln!("remove: {}: no such file or directory", t.display());
            return BuiltinResult::Err;
        }
        if t.is_dir() && !recursive {
            eprintln!(
                "remove: {}: is a directory — use 'remove -r' to delete directories",
                t.display()
            );
            return BuiltinResult::Err;
        }
    }

    // Confirm if: multiple targets, or any directory, or not forced.
    let needs_confirm = !force && (targets.len() > 1 || targets.iter().any(|t| t.is_dir()));

    if needs_confirm {
        let names: Vec<String> = targets.iter().map(|t| t.display().to_string()).collect();
        eprint!("remove: delete {}? [y/N] ", names.join(", "));
        io::stderr().flush().ok();

        let mut answer = String::new();
        if io::stdin().read_line(&mut answer).is_err() || !answer.trim().eq_ignore_ascii_case("y") {
            println!("remove: cancelled");
            return BuiltinResult::Ok;
        }
    }

    for target in &targets {
        let result = if target.is_dir() {
            std::fs::remove_dir_all(target)
        } else {
            std::fs::remove_file(target)
        };

        if let Err(e) = result {
            eprintln!("remove: {}: {}", target.display(), e);
            return BuiltinResult::Err;
        }
    }

    BuiltinResult::Ok
}

fn resolve(p: &str, cwd: &Path) -> PathBuf {
    if Path::new(p).is_absolute() {
        PathBuf::from(p)
    } else {
        cwd.join(p)
    }
}
