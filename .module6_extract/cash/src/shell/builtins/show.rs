// cash built-in: show
//
// List directory contents. Human-readable by default.
// Never silently fails — always explains what went wrong.
//
// Usage:
//   show              → list current directory
//   show <path>       → list given path
//   show -a <path>    → include hidden files (dotfiles)

use std::path::{Path, PathBuf};
use std::fs;

use super::super::pipeline::Stage;
use super::BuiltinResult;

pub fn run(stage: &Stage, cwd: &Path) -> BuiltinResult {
    let mut show_hidden = false;
    let mut target: Option<PathBuf> = None;

    for arg in &stage.args {
        match arg.as_str() {
            "-a" | "--all" => show_hidden = true,
            other => {
                target = Some(if Path::new(other).is_absolute() {
                    PathBuf::from(other)
                } else {
                    cwd.join(other)
                });
            }
        }
    }

    let dir = target.unwrap_or_else(|| cwd.to_path_buf());

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("show: {}: {}", dir.display(), err);
            return BuiltinResult::Err;
        }
    };

    let mut items: Vec<DirItem> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| DirItem::from_entry(&e).ok())
        .filter(|item| show_hidden || !item.name.starts_with('.'))
        .collect();

    items.sort_by(|a, b| {
        // Directories first, then alphabetical.
        b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    if items.is_empty() {
        println!("(empty)");
        return BuiltinResult::Ok;
    }

    let name_width = items.iter().map(|i| i.name.len()).max().unwrap_or(0).min(40);

    for item in &items {
        let kind    = if item.is_dir { "dir " } else { "file" };
        let size    = format_size(item.size);
        let name    = if item.is_dir {
            format!("{}/", item.name)
        } else {
            item.name.clone()
        };
        println!("  {}  {:>8}  {}", kind, size, name);
    }

    BuiltinResult::Ok
}

struct DirItem {
    name:   String,
    is_dir: bool,
    size:   u64,
}

impl DirItem {
    fn from_entry(entry: &fs::DirEntry) -> std::io::Result<Self> {
        let meta = entry.metadata()?;
        Ok(Self {
            name:   entry.file_name().to_string_lossy().into_owned(),
            is_dir: meta.is_dir(),
            size:   meta.len(),
        })
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    match bytes {
        0 => "-".into(),
        b if b < KB => format!("{}B", b),
        b if b < MB => format!("{:.1}K", b as f64 / KB as f64),
        b if b < GB => format!("{:.1}M", b as f64 / MB as f64),
        b           => format!("{:.1}G", b as f64 / GB as f64),
    }
}
