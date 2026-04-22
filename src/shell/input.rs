// cash — raw terminal line reader
//
// Reads one line from stdin using crossterm raw mode so we control every
// keypress ourselves. Supports:
//   Printable characters  — insert at cursor
//   Backspace / Delete    — erase character to the left
//   Left / Right arrows   — move cursor within the line
//   Home / End            — jump to start / end
//   Ctrl+C                — clear the current line, return InputResult::Interrupted
//   Ctrl+D on empty line  — return InputResult::Eof
//   Enter                 — confirm and return the line
//
// History (up/down arrows) is a stub — Module 5 (Memory Store) will wire
// it up to history.db once that exists.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, ClearType},
    ExecutableCommand, QueueableCommand,
};
use std::io::{self, Write};

#[derive(Debug)]
pub enum InputResult {
    /// User pressed Enter. Contains the completed line.
    Line(String),
    /// User pressed Ctrl+C — line was cleared.
    Interrupted,
    /// User pressed Ctrl+D on an empty line — exit signal.
    Eof,
}

/// Read a single line with raw-mode editing.
/// `prompt_len` is the visible width of the prompt so we can redraw correctly.
pub fn read_line(prompt_len: u16) -> io::Result<InputResult> {
    terminal::enable_raw_mode()?;
    let result = read_inner(prompt_len);
    terminal::disable_raw_mode()?;
    // Always move to a fresh line after raw mode ends
    let mut out = io::stdout();
    out.execute(cursor::MoveToNextLine(1))?;
    result
}

fn read_inner(prompt_len: u16) -> io::Result<InputResult> {
    let mut out = io::stdout();
    let mut buf: Vec<char> = Vec::new();
    let mut cursor_pos: usize = 0; // index within buf

    loop {
        match event::read()? {
            // --- Confirm ---
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) => {
                let line: String = buf.iter().collect();
                return Ok(InputResult::Line(line));
            }

            // --- Ctrl+D ---
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                if buf.is_empty() {
                    return Ok(InputResult::Eof);
                }
            }

            // --- Ctrl+C ---
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                // Print ^C and clear the line
                out.queue(terminal::Clear(ClearType::CurrentLine))?;
                out.queue(cursor::MoveToColumn(0))?;
                writeln!(out, "^C")?;
                out.flush()?;
                return Ok(InputResult::Interrupted);
            }

            // --- Backspace ---
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => {
                if cursor_pos > 0 {
                    cursor_pos -= 1;
                    buf.remove(cursor_pos);
                    redraw(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            // --- Delete ---
            Event::Key(KeyEvent {
                code: KeyCode::Delete,
                ..
            }) => {
                if cursor_pos < buf.len() {
                    buf.remove(cursor_pos);
                    redraw(&mut out, &buf, cursor_pos, prompt_len)?;
                }
            }

            // --- Left arrow ---
            Event::Key(KeyEvent {
                code: KeyCode::Left,
                ..
            }) => {
                if cursor_pos > 0 {
                    cursor_pos -= 1;
                    let col = prompt_len + cursor_pos as u16;
                    out.queue(cursor::MoveToColumn(col))?;
                    out.flush()?;
                }
            }

            // --- Right arrow ---
            Event::Key(KeyEvent {
                code: KeyCode::Right,
                ..
            }) => {
                if cursor_pos < buf.len() {
                    cursor_pos += 1;
                    let col = prompt_len + cursor_pos as u16;
                    out.queue(cursor::MoveToColumn(col))?;
                    out.flush()?;
                }
            }

            // --- Home ---
            Event::Key(KeyEvent {
                code: KeyCode::Home,
                ..
            }) => {
                cursor_pos = 0;
                out.queue(cursor::MoveToColumn(prompt_len))?;
                out.flush()?;
            }

            // --- End ---
            Event::Key(KeyEvent {
                code: KeyCode::End, ..
            }) => {
                cursor_pos = buf.len();
                let col = prompt_len + buf.len() as u16;
                out.queue(cursor::MoveToColumn(col))?;
                out.flush()?;
            }

            // --- Up / Down — history stub ---
            Event::Key(KeyEvent {
                code: KeyCode::Up, ..
            }) => {
                // Module 5: will load previous history entry
            }
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                ..
            }) => {
                // Module 5: will load next history entry
            }

            // --- Printable characters ---
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

/// Redraw the current line content and reposition the cursor.
fn redraw<W: Write>(
    out: &mut W,
    buf: &[char],
    cursor_pos: usize,
    prompt_len: u16,
) -> io::Result<()> {
    // Move to the character right after the prompt, clear to end of line.
    out.queue(cursor::MoveToColumn(prompt_len))?;
    out.queue(terminal::Clear(ClearType::UntilNewLine))?;
    // Write the buffer
    let s: String = buf.iter().collect();
    write!(out, "{}", s)?;
    // Reposition cursor
    let col = prompt_len + cursor_pos as u16;
    out.queue(cursor::MoveToColumn(col))?;
    out.flush()
}
