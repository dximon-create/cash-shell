// cash — Syntax Highlighting
//
// Highlights as you type:
//   Green  — valid command (found in PATH or built-ins)
//   Red    — unknown command
//   Cyan   — arguments and flags
//   Yellow — strings (quoted)
//   Blue   — pipes and redirects
//   White  — everything else

use crossterm::style::{Color, SetForegroundColor, ResetColor};
use crossterm::QueueableCommand;
use std::io::Write;

const BUILTINS: &[&str] = &[
    "show", "go", "copy", "move", "remove", "teach", "help", "exit",
    "scan", "trace", "dns", "whois", "arp", "lab", "learn", "cash",
];

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    ValidCommand,
    UnknownCommand,
    Argument,
    Flag,
    String,
    Pipe,
    Redirect,
    EnvVar,
}

#[derive(Debug, Clone)]
pub struct HighlightedToken {
    pub text:  String,
    pub kind:  TokenKind,
}

/// Tokenise and classify a command line for highlighting.
pub fn highlight(line: &str) -> Vec<HighlightedToken> {
    let mut tokens = Vec::new();
    let mut first_token = true;
    let mut after_pipe = false;

    for part in split_for_highlight(line) {
        let kind = classify(&part, first_token || after_pipe);

        if part == "|" || part == "||" {
            after_pipe = true;
            first_token = false;
        } else if matches!(part.as_str(), ">" | ">>" | "<" | "2>") {
            after_pipe = false;
            first_token = false;
        } else {
            after_pipe = false;
            first_token = false;
        }

        tokens.push(HighlightedToken { text: part, kind });
    }

    tokens
}

fn classify(token: &str, is_command: bool) -> TokenKind {
    if token == "|" || token == "||" {
        return TokenKind::Pipe;
    }
    if matches!(token, ">" | ">>" | "<" | "2>") {
        return TokenKind::Redirect;
    }
    if (token.starts_with('"') && token.ends_with('"')) ||
       (token.starts_with('\'') && token.ends_with('\'')) {
        return TokenKind::String;
    }
    if token.starts_with('-') {
        return TokenKind::Flag;
    }
    if token.contains('=') && !token.starts_with('-') {
        let key = token.split('=').next().unwrap_or("");
        if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return TokenKind::EnvVar;
        }
    }
    if is_command {
        if BUILTINS.contains(&token) || command_exists(token) {
            return TokenKind::ValidCommand;
        }
        return TokenKind::UnknownCommand;
    }
    TokenKind::Argument
}

fn command_exists(cmd: &str) -> bool {
    if cmd.contains('/') {
        return std::path::Path::new(cmd).exists();
    }
    let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
    path.split(':').any(|dir| std::path::Path::new(dir).join(cmd).exists())
}

/// Split line into tokens preserving whitespace structure.
fn split_for_highlight(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => { in_single = !in_single; current.push(ch); }
            '"'  if !in_single => { in_double = !in_double; current.push(ch); }
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() { tokens.push(current.clone()); current.clear(); }
            }
            '|' if !in_single && !in_double => {
                if !current.is_empty() { tokens.push(current.clone()); current.clear(); }
                tokens.push("|".to_string());
            }
            '>' if !in_single && !in_double => {
                if !current.is_empty() { tokens.push(current.clone()); current.clear(); }
                if chars.peek() == Some(&'>') { chars.next(); tokens.push(">>".to_string()); }
                else { tokens.push(">".to_string()); }
            }
            '<' if !in_single && !in_double => {
                if !current.is_empty() { tokens.push(current.clone()); current.clear(); }
                tokens.push("<".to_string());
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() { tokens.push(current); }
    tokens
}

/// Write highlighted line to stdout.
pub fn print_highlighted<W: Write>(out: &mut W, tokens: &[HighlightedToken]) -> std::io::Result<()> {
    for (i, token) in tokens.iter().enumerate() {
        let color = match token.kind {
            TokenKind::ValidCommand   => Color::Green,
            TokenKind::UnknownCommand => Color::Red,
            TokenKind::Argument       => Color::White,
            TokenKind::Flag           => Color::Cyan,
            TokenKind::String         => Color::Yellow,
            TokenKind::Pipe           => Color::Blue,
            TokenKind::Redirect       => Color::Blue,
            TokenKind::EnvVar         => Color::Magenta,
        };
        out.queue(SetForegroundColor(color))?;
        write!(out, "{}", token.text)?;
        if i < tokens.len() - 1 { write!(out, " ")?; }
    }
    out.queue(ResetColor)?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_valid_command() {
        let tokens = highlight("ls -la");
        assert_eq!(tokens[0].kind, TokenKind::ValidCommand);
        assert_eq!(tokens[1].kind, TokenKind::Flag);
    }

    #[test]
    fn highlights_unknown_command() {
        let tokens = highlight("zzz_fake_cmd arg");
        assert_eq!(tokens[0].kind, TokenKind::UnknownCommand);
    }

    #[test]
    fn highlights_pipe() {
        let tokens = highlight("ls | grep foo");
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Pipe));
    }

    #[test]
    fn highlights_redirect() {
        let tokens = highlight("echo hi > out.txt");
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Redirect));
    }

    #[test]
    fn highlights_string() {
        let tokens = highlight("echo 'hello world'");
        assert!(tokens.iter().any(|t| t.kind == TokenKind::String));
    }

    #[test]
    fn highlights_env_var() {
        let tokens = highlight("FOO=bar ls");
        assert_eq!(tokens[0].kind, TokenKind::EnvVar);
    }

    #[test]
    fn builtin_is_valid() {
        let tokens = highlight("show /tmp");
        assert_eq!(tokens[0].kind, TokenKind::ValidCommand);
    }
}
