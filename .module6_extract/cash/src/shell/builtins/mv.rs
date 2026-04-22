// cash built-in: move
//
// Move or rename a file or directory.
// Works across filesystems by falling back to copy+delete.
//
// Usage:
//   move <src> <dst>

use std::path::{Path, PathBuf};

use super::super::pipeline::Stage;
use super::BuiltinResult;

pub fn run(stage: &Stage, cwd: &Path) -> BuiltinResult {
    if stage.args.len() < 2 {
        eprintln!("move: usage: move <source> <destination>");
        return BuiltinResult::Err;
    }

    let src = resolve(&stage.args[0], cwd);
    let dst_arg = resolve(&stage.args[1], cwd);

    if !src.exists() {
        eprintln!("move: {}: no such file or directory", src.display());
        return BuiltinResult::Err;
    }

    // If dst is an existing directory, move src inside it.
    let dst = if dst_arg.is_dir() {
        dst_arg.join(src.file_name().unwrap())
    } else {
        dst_arg
    };

    // Try a simple rename first (same filesystem, instant).
    if let Err(_) = std::fs::rename(&src, &dst) {
        // Cross-filesystem: copy then delete.
        if src.is_dir() {
            if let Err(e) = copy_dir(&src, &dst) {
                eprintln!("move: {}", e);
                return BuiltinResult::Err;
            }
            if let Err(e) = std::fs::remove_dir_all(&src) {
                eprintln!("move: cleanup failed: {}", e);
                return BuiltinResult::Err;
            }
        } else {
            if let Err(e) = std::fs::copy(&src, &dst) {
                eprintln!("move: {}", e);
                return BuiltinResult::Err;
            }
            if let Err(e) = std::fs::remove_file(&src) {
                eprintln!("move: cleanup failed: {}", e);
                return BuiltinResult::Err;
            }
        }
    }

    BuiltinResult::Ok
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn resolve(p: &str, cwd: &Path) -> PathBuf {
    if Path::new(p).is_absolute() {
        PathBuf::from(p)
    } else {
        cwd.join(p)
    }
}
