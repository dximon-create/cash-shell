// cash — raw terminal line reader
//
// On Unix: uses crossterm raw mode for full line editing.
// On Windows native: falls back to simple stdin readline.
//                    Windows users should use WSL for full features.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, ClearType},
    ExecutableCommand, QueueableCommand,
};
use std::io::{self, Write};

#[derive(Debug)]
pub enum InputResult {
    Line(String),
    Interrupted,
    Eof,
}

/// Detect if we are running on Windows native (not WSL).
fn is_windows_native() -> bool {
    cfg!(target_os = "windows")
}

/// Read a single line — raw mode on Unix, simple readline on Windows.
pub fn read_line(prompt_len: u16) -> io::Result<InputResult> {
    // Windows native: skip raw mode — it causes double-typing.
    if is_windows_native() {
        return read_line_simple();
    }

    terminal::enable_raw_mode()?;
    let result = read_inner(prompt_len);
    terminal::disable_raw_mode()?;
    let mut out = io::stdout();
    out.execute(cursor::MoveToNextLine(1))?;
    result
}

/// Simple fallback for Windows native — just read a line from stdin.
fn read_line_simple() -> io::Result<InputResult> {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => Ok(InputResult::Eof),
        Ok(_) => {
            let trimmed = line.trim_end_matches('\n')
                              .trim_end_matches('\r')
                              .to_string();
            Ok(InputResult::Line(trimmed))
        }
        Err(e) => Err(e),
    }
}

fn read_inner(prompt_len: u16) -> io::Result<InputResult> {
    let mut out = io::stdout();
    let mut buf: Vec<char> = Vec::new();
    let mut cursor_pos: usize = 0;

    loop {
        match event::read()? {
            Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                let line: String = buf.iter().collect();
                return Ok(InputResult::Line(line));
            }

            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                if buf.is_empty() {
                    return Ok(InputResult::Eof);
                }
            }

            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                out.queue(terminal::Clear(ClearType::CurrentLine))?;
                out.queue(cursor::MoveToColumn(0))?;
                writeln!(out, "^C")?;
                out.flush()?;
                return Ok(InputResult::Interrupted);
            }

            Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                if cursor_pos > 0 {
                    cursor_pos -= 1;
                    buf.remove(cursor_pos);
                    redraw(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            Event::Key(KeyEvent { code: KeyCode::Delete, .. }) => {
                if cursor_pos < buf.len() {
                    buf.remove(cursor_pos);
                    redraw(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            Event::Key(KeyEvent { code: KeyCode::Left, .. }) => {
                if cursor_pos > 0 {
                    cursor_pos -= 1;
                    let col = prompt_len + cursor_pos as u16;
                    out.queue(cursor::MoveToColumn(col))?;
                    out.flush()?;
                }
            }

            Event::Key(KeyEvent { code: KeyCode::Right, .. }) => {
                if cursor_pos < buf.len() {
                    cursor_pos += 1;
                    let col = prompt_len + cursor_pos as u16;
                    out.queue(cursor::MoveToColumn(col))?;
                    out.flush()?;
                }
            }

            Event::Key(KeyEvent { code: KeyCode::Home, .. }) => {
                cursor_pos = 0;
                out.queue(cursor::MoveToColumn(prompt_len))?;
                out.flush()?;
            }

            Event::Key(KeyEvent { code: KeyCode::End, .. }) => {
                cursor_pos = buf.len();
                let col = prompt_len + buf.len() as u16;
                out.queue(cursor::MoveToColumn(col))?;
                out.flush()?;
            }

            Event::Key(KeyEvent { code: KeyCode::Up, .. }) => {}
            Event::Key(KeyEvent { code: KeyCode::Down, .. }) => {}

            Event::Key(KeyEvent {
                code: KeyCode::Char(ch),
                modifiers,
                ..
            }) if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
                buf.insert(cursor_pos, ch);
                cursor_pos += 1;
                redraw(&mut out, &buf, cursor_pos, prompt_len)?;
            }

            _ => {}
        }
    }
}

fn redraw<W: Write>(
    out: &mut W,
    buf: &[char],
    cursor_pos: usize,
    prompt_len: u16,
) -> io::Result<()> {
    out.queue(cursor::MoveToColumn(prompt_len))?;
    out.queue(terminal::Clear(ClearType::UntilNewLine))?;
    let s: String = buf.iter().collect();
    write!(out, "{}", s)?;
    let col = prompt_len + cursor_pos as u16;
    out.queue(cursor::MoveToColumn(col))?;
    out.flush()
}
