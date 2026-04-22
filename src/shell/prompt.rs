// cash — prompt renderer
//
// Renders the shell prompt. Always shows the current working directory,
// shortened if it falls inside $HOME (replaced with ~).
// Colour is applied via crossterm. Falls back to plain text if the
// terminal does not support colour.

use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::QueueableCommand;
use std::io::Write;

const ARROW: &str = "❯";

/// Write the prompt to `out`. Caller must flush.
pub fn render<W: Write>(out: &mut W, cwd: &std::path::Path) -> std::io::Result<()> {
    let home = dirs::home_dir();
    let display = match &home {
        Some(h) => {
            if let Ok(rel) = cwd.strip_prefix(h) {
                if rel.as_os_str().is_empty() {
                    "~".to_string()
                } else {
                    format!("~/{}", rel.display())
                }
            } else {
                cwd.display().to_string()
            }
        }
        None => cwd.display().to_string(),
    };

    // cwd in cyan
    out.queue(SetForegroundColor(Color::Cyan))?;
    out.queue(SetAttribute(Attribute::Bold))?;
    write!(out, " {}", display)?;

    // arrow in green
    out.queue(SetForegroundColor(Color::Green))?;
    out.queue(SetAttribute(Attribute::Reset))?;
    write!(out, " {} ", ARROW)?;

    out.queue(ResetColor)?;
    Ok(())
}
