// cash — raw terminal line reader
//
// Full featured input with:
//   Tab         — completion
//   Ctrl+R      — reverse history search
//   Syntax      — highlights as you type (green=valid, red=unknown)
//   Up/Down     — history navigation
//   Left/Right  — cursor movement
//   Home/End    — jump to start/end
//   Ctrl+C      — clear line
//   Ctrl+D      — EOF on empty line
//   Backspace   — delete left
//   Delete      — delete right

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Color, SetForegroundColor, ResetColor},
    terminal::{self, ClearType},
    ExecutableCommand, QueueableCommand,
};
use std::io::{self, Write};
use crate::store::Store;

#[derive(Debug)]
pub enum InputResult {
    Line(String),
    Interrupted,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    HistorySearch,
}

fn is_windows_native() -> bool {
    cfg!(target_os = "windows")
}

pub fn read_line(prompt_len: u16, store: &Store) -> io::Result<InputResult> {
    if is_windows_native() {
        return read_line_simple();
    }
    terminal::enable_raw_mode()?;
    let result = read_inner(prompt_len, store);
    terminal::disable_raw_mode()?;
    let mut out = io::stdout();
    out.execute(cursor::MoveToNextLine(1))?;
    result
}

fn read_line_simple() -> io::Result<InputResult> {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => Ok(InputResult::Eof),
        Ok(_) => Ok(InputResult::Line(
            line.trim_end_matches('\n').trim_end_matches('\r').to_string()
        )),
        Err(e) => Err(e),
    }
}

fn read_inner(prompt_len: u16, store: &Store) -> io::Result<InputResult> {
    let mut out = io::stdout();
    let mut buf: Vec<char> = Vec::new();
    let mut cursor_pos: usize = 0;
    let mut mode = InputMode::Normal;
    let mut search_query = String::new();
    let mut history_idx: Option<usize> = None;

    // Load history for up/down navigation
    let history = load_history(store);

    loop {
        match event::read()? {

            // --- Enter ---
            Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                // Clear search mode display
                if mode == InputMode::HistorySearch {
                    redraw_normal(&mut out, &buf, cursor_pos, prompt_len)?;
                }
                let line: String = buf.iter().collect();
                return Ok(InputResult::Line(line));
            }

            // --- Ctrl+D ---
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL, ..
            }) => {
                if buf.is_empty() { return Ok(InputResult::Eof); }
            }

            // --- Ctrl+C ---
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL, ..
            }) => {
                buf.clear();
                cursor_pos = 0;
                mode = InputMode::Normal;
                search_query.clear();
                out.queue(terminal::Clear(ClearType::CurrentLine))?;
                out.queue(cursor::MoveToColumn(0))?;
                writeln!(out, "^C")?;
                out.flush()?;
                return Ok(InputResult::Interrupted);
            }

            // --- Ctrl+R — toggle history search ---
            Event::Key(KeyEvent {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::CONTROL, ..
            }) => {
                mode = InputMode::HistorySearch;
                search_query.clear();
                redraw_search(&mut out, &search_query, "", prompt_len)?;
            }

            // --- Tab — completion ---
            Event::Key(KeyEvent { code: KeyCode::Tab, .. }) => {
                let line_so_far: String = buf[..cursor_pos].iter().collect();
                let completions = get_completions(&line_so_far, store);
                if completions.len() == 1 {
                    // Single completion — apply it
                    let word_start = find_word_start(&line_so_far);
                    buf.drain(word_start..cursor_pos);
                    let completed: Vec<char> = completions[0].chars().collect();
                    let insert_pos = word_start;
                    for (i, ch) in completed.iter().enumerate() {
                        buf.insert(insert_pos + i, *ch);
                    }
                    cursor_pos = insert_pos + completed.len();
                    redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
                } else if completions.len() > 1 {
                    // Multiple completions — show them below
                    show_completions(&mut out, &completions)?;
                    redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            // --- Backspace ---
            Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                if mode == InputMode::HistorySearch {
                    search_query.pop();
                    let result = search_history(&history, &search_query);
                    redraw_search(&mut out, &search_query, &result, prompt_len)?;
                    if let Some(r) = history.iter().find(|h| h.contains(&search_query)) {
                        buf = r.chars().collect();
                        cursor_pos = buf.len();
                    }
                } else if cursor_pos > 0 {
                    cursor_pos -= 1;
                    buf.remove(cursor_pos);
                    redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            // --- Delete ---
            Event::Key(KeyEvent { code: KeyCode::Delete, .. }) => {
                if cursor_pos < buf.len() {
                    buf.remove(cursor_pos);
                    redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            // --- Left ---
            Event::Key(KeyEvent { code: KeyCode::Left, .. }) => {
                if cursor_pos > 0 {
                    cursor_pos -= 1;
                    out.queue(cursor::MoveToColumn(prompt_len + cursor_pos as u16))?;
                    out.flush()?;
                }
            }

            // --- Right ---
            Event::Key(KeyEvent { code: KeyCode::Right, .. }) => {
                if cursor_pos < buf.len() {
                    cursor_pos += 1;
                    out.queue(cursor::MoveToColumn(prompt_len + cursor_pos as u16))?;
                    out.flush()?;
                }
            }

            // --- Home ---
            Event::Key(KeyEvent { code: KeyCode::Home, .. }) => {
                cursor_pos = 0;
                out.queue(cursor::MoveToColumn(prompt_len))?;
                out.flush()?;
            }

            // --- End ---
            Event::Key(KeyEvent { code: KeyCode::End, .. }) => {
                cursor_pos = buf.len();
                out.queue(cursor::MoveToColumn(prompt_len + buf.len() as u16))?;
                out.flush()?;
            }

            // --- Up — history prev ---
            Event::Key(KeyEvent { code: KeyCode::Up, .. }) => {
                if history.is_empty() { continue; }
                let idx = match history_idx {
                    None => history.len() - 1,
                    Some(i) => i.saturating_sub(1),
                };
                history_idx = Some(idx);
                buf = history[idx].chars().collect();
                cursor_pos = buf.len();
                redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
            }

            // --- Down — history next ---
            Event::Key(KeyEvent { code: KeyCode::Down, .. }) => {
                match history_idx {
                    None => {}
                    Some(i) => {
                        if i + 1 < history.len() {
                            history_idx = Some(i + 1);
                            buf = history[i + 1].chars().collect();
                        } else {
                            history_idx = None;
                            buf.clear();
                        }
                        cursor_pos = buf.len();
                        redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
                    }
                }
            }

            // --- Printable characters ---
            Event::Key(KeyEvent {
                code: KeyCode::Char(ch),
                modifiers,
                ..
            }) if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
                if mode == InputMode::HistorySearch {
                    search_query.push(ch);
                    let result = search_history(&history, &search_query);
                    redraw_search(&mut out, &search_query, &result, prompt_len)?;
                    if !result.is_empty() {
                        buf = result.chars().collect();
                        cursor_pos = buf.len();
                    }
                } else {
                    // Exit history search if active
                    mode = InputMode::Normal;
                    buf.insert(cursor_pos, ch);
                    cursor_pos += 1;
                    redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            // --- Escape — cancel search ---
            Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                if mode == InputMode::HistorySearch {
                    mode = InputMode::Normal;
                    search_query.clear();
                    redraw_highlighted(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Syntax highlighting
// ---------------------------------------------------------------------------

fn redraw_highlighted<W: Write>(
    out: &mut W,
    buf: &[char],
    cursor_pos: usize,
    prompt_len: u16,
) -> io::Result<()> {
    out.queue(cursor::MoveToColumn(prompt_len))?;
    out.queue(terminal::Clear(ClearType::UntilNewLine))?;

    if !buf.is_empty() {
        let line: String = buf.iter().collect();
        write_highlighted(out, &line)?;
    }

    let col = prompt_len + cursor_pos as u16;
    out.queue(cursor::MoveToColumn(col))?;
    out.flush()
}

fn write_highlighted<W: Write>(out: &mut W, line: &str) -> io::Result<()> {
    let tokens: Vec<&str> = line.splitn(2, ' ').collect();
    let cmd = tokens[0];
    let rest = tokens.get(1).copied().unwrap_or("");

    // Command colour: green if valid, red if unknown
    let cmd_color = if is_valid_command(cmd) { Color::Green } else { Color::Red };
    out.queue(SetForegroundColor(cmd_color))?;
    write!(out, "{}", cmd)?;

    if !rest.is_empty() {
        write!(out, " ")?;
        // Colour arguments
        for (i, token) in rest.split_whitespace().enumerate() {
            if i > 0 { write!(out, " ")?; }
            if token == "|" || token == ">" || token == ">>" || token == "<" {
                out.queue(SetForegroundColor(Color::Blue))?;
            } else if token.starts_with('-') {
                out.queue(SetForegroundColor(Color::Cyan))?;
            } else if (token.starts_with('"') && token.ends_with('"')) ||
                      (token.starts_with('\'') && token.ends_with('\'')) {
                out.queue(SetForegroundColor(Color::Yellow))?;
            } else {
                out.queue(SetForegroundColor(Color::White))?;
            }
            write!(out, "{}", token)?;
        }
    }

    out.queue(ResetColor)?;
    Ok(())
}

fn redraw_normal<W: Write>(out: &mut W, buf: &[char], cursor_pos: usize, prompt_len: u16) -> io::Result<()> {
    redraw_highlighted(out, buf, cursor_pos, prompt_len)
}

// ---------------------------------------------------------------------------
// History search
// ---------------------------------------------------------------------------

fn redraw_search<W: Write>(
    out: &mut W,
    query: &str,
    result: &str,
    prompt_len: u16,
) -> io::Result<()> {
    out.queue(cursor::MoveToColumn(0))?;
    out.queue(terminal::Clear(ClearType::CurrentLine))?;
    out.queue(SetForegroundColor(Color::Cyan))?;
    write!(out, "(search)`{}': {}", query, result)?;
    out.queue(ResetColor)?;
    out.flush()
}

fn search_history(history: &[String], query: &str) -> String {
    if query.is_empty() { return String::new(); }
    history.iter().rev()
        .find(|h| h.contains(query))
        .cloned()
        .unwrap_or_default()
}

fn load_history(store: &Store) -> Vec<String> {
    crate::store::history::recent(store, 500)
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.command)
        .rev()
        .collect()
}

// ---------------------------------------------------------------------------
// Tab completion
// ---------------------------------------------------------------------------

fn get_completions(line: &str, store: &Store) -> Vec<String> {
    let word = line.split_whitespace().last().unwrap_or(line);
    let is_first = line.trim().split_whitespace().count() <= 1;

    let mut completions = Vec::new();

    if is_first {
        // Complete commands
        let builtins = ["show","go","copy","move","remove","teach","help","exit",
                        "scan","trace","dns","whois","arp","lab","learn","cash"];
        for b in &builtins {
            if b.starts_with(word) { completions.push(b.to_string()); }
        }
        // Taught commands
        if let Ok(memories) = crate::store::memory::list(store) {
            for m in memories {
                if m.name.starts_with(word) { completions.push(m.name); }
            }
        }
        // PATH binaries
        let path = std::env::var("PATH").unwrap_or_default();
        let mut binaries: Vec<String> = path.split(':')
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with(word))
            .collect();
        binaries.sort();
        binaries.dedup();
        completions.extend(binaries.into_iter().take(20));
    } else {
        // Complete paths
        let (dir, prefix) = split_path_prefix(word);
        let search_dir = if dir.is_empty() { ".".to_string() } else { dir.clone() };
        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            let mut paths: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let suffix = if e.path().is_dir() { "/" } else { "" };
                    if dir.is_empty() { format!("{}{}", name, suffix) }
                    else { format!("{}/{}{}", dir, name, suffix) }
                })
                .collect();
            paths.sort();
            completions.extend(paths);
        }
    }
    completions
}

fn show_completions<W: Write>(out: &mut W, completions: &[String]) -> io::Result<()> {
    writeln!(out)?;
    out.queue(SetForegroundColor(Color::Cyan))?;
    let line: String = completions.iter().take(10)
        .map(|c| format!("  {}", c))
        .collect::<Vec<_>>()
        .join("  ");
    write!(out, "{}", line)?;
    if completions.len() > 10 {
        write!(out, "  ... ({} more)", completions.len() - 10)?;
    }
    out.queue(ResetColor)?;
    writeln!(out)?;
    out.flush()
}

fn find_word_start(line: &str) -> usize {
    line.rfind(' ').map(|i| i + 1).unwrap_or(0)
}

fn split_path_prefix(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(i) => (path[..i].to_string(), path[i+1..].to_string()),
        None    => (String::new(), path.to_string()),
    }
}

fn is_valid_command(cmd: &str) -> bool {
    if cmd.is_empty() { return false; }
    let builtins = ["show","go","copy","move","remove","teach","help","exit",
                    "scan","trace","dns","whois","arp","lab","learn","cash"];
    if builtins.contains(&cmd) { return true; }
    let path = std::env::var("PATH").unwrap_or_default();
    path.split(':').any(|dir| std::path::Path::new(dir).join(cmd).exists())
}
