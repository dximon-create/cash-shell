// cash built-in: go
//
// Change the current working directory.
// Understands:  go           → home directory
//               go ~         → home directory
//               go -         → previous directory
//               go <path>    → given path
//               go ..        → parent directory

use std::path::{Path, PathBuf};

use super::super::pipeline::Stage;
use super::BuiltinResult;

// Thread-local previous directory for `go -`
std::thread_local! {
    static PREV_DIR: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
}

pub fn run(stage: &Stage, cwd: &Path) -> BuiltinResult {
    let target_str = stage.args.first().map(|s| s.as_str()).unwrap_or("~");

    let target: PathBuf = match target_str {
        "" | "~" => dirs::home_dir().unwrap_or_else(|| cwd.to_path_buf()),
        "-" => {
            let prev = PREV_DIR.with(|p| p.borrow().clone());
            match prev {
                Some(p) => p,
                None => {
                    eprintln!("go: no previous directory");
                    return BuiltinResult::Err;
                }
            }
        }
        s if s.starts_with("~/") => {
            let home = dirs::home_dir().unwrap_or_else(|| cwd.to_path_buf());
            home.join(&s[2..])
        }
        s => {
            if Path::new(s).is_absolute() {
                PathBuf::from(s)
            } else {
                cwd.join(s)
            }
        }
    };

    let canonical = match std::fs::canonicalize(&target) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("go: {}: {}", target.display(), e);
            return BuiltinResult::Err;
        }
    };

    if let Err(e) = std::env::set_current_dir(&canonical) {
        eprintln!("go: {}", e);
        return BuiltinResult::Err;
    }

    // Store previous directory.
    PREV_DIR.with(|p| {
        *p.borrow_mut() = Some(cwd.to_path_buf());
    });

    BuiltinResult::ChangeDir(canonical)
}
