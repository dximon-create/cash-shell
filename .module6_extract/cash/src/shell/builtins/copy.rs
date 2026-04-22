// cash built-in: copy
//
// Copy files or directories. Never overwrites without saying so.
//
// Usage:
//   copy <src> <dst>        copy file or dir
//   copy -r <src> <dst>     recursive (required for directories)

use std::path::{Path, PathBuf};

use super::super::pipeline::Stage;
use super::BuiltinResult;

pub fn run(stage: &Stage, cwd: &Path) -> BuiltinResult {
    let mut recursive = false;
    let mut paths: Vec<PathBuf> = Vec::new();

    for arg in &stage.args {
        match arg.as_str() {
            "-r" | "--recursive" => recursive = true,
            p => paths.push(resolve(p, cwd)),
        }
    }

    if paths.len() < 2 {
        eprintln!("copy: usage: copy [-r] <source> <destination>");
        return BuiltinResult::Err;
    }

    let dst = paths.pop().unwrap();
    let sources = paths;

    for src in &sources {
        if !src.exists() {
            eprintln!("copy: {}: no such file or directory", src.display());
            return BuiltinResult::Err;
        }

        if src.is_dir() && !recursive {
            eprintln!(
                "copy: {}: is a directory — use 'copy -r' to copy directories",
                src.display()
            );
            return BuiltinResult::Err;
        }

        let actual_dst = if dst.is_dir() {
            dst.join(src.file_name().unwrap())
        } else {
            dst.clone()
        };

        if let Err(e) = do_copy(src, &actual_dst, recursive) {
            eprintln!("copy: {}", e);
            return BuiltinResult::Err;
        }
    }

    BuiltinResult::Ok
}

fn do_copy(src: &Path, dst: &Path, recursive: bool) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let target = dst.join(entry.file_name());
            do_copy(&entry.path(), &target, recursive)?;
        }
    } else {
        std::fs::copy(src, dst)?;
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
