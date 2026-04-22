// cash — command line parser / tokeniser
//
// Converts a raw input string into a Pipeline.
//
// Handles:
//   Pipes           cmd1 | cmd2 | cmd3
//   Redirects       < file   > file   >> file   2> file
//   Quoted strings  'hello world'   "hello world"
//   Escapes         \n  \\  \"  (inside double quotes)
//   Env pairs       VAR=value cmd   (leading tokens before command name)
//   Comments        lines starting with # (after whitespace)

use super::pipeline::{Pipeline, Redirect, RedirectKind, Stage};

#[derive(Debug, Clone, PartialEq)]
pub enum ParseResult {
    Pipeline(Pipeline),
    Empty,
    Error(String),
}

pub fn parse(line: &str) -> ParseResult {
    let raw = line.to_string();
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with('#') {
        return ParseResult::Empty;
    }

    // Split into pipe segments on unquoted '|'
    let segments = match split_on_pipe(trimmed) {
        Ok(s) => s,
        Err(e) => return ParseResult::Error(e),
    };

    let mut stages: Vec<Stage> = Vec::new();

    for seg in &segments {
        match parse_stage(seg.trim()) {
            Ok(Some(stage)) => stages.push(stage),
            Ok(None) => {} // empty segment — ignore (e.g. trailing pipe)
            Err(e) => return ParseResult::Error(e),
        }
    }

    if stages.is_empty() {
        return ParseResult::Empty;
    }

    ParseResult::Pipeline(Pipeline { stages, raw })
}

// ---------------------------------------------------------------------------
// Split a line on unquoted '|'
// ---------------------------------------------------------------------------

fn split_on_pipe(s: &str) -> Result<Vec<String>, String> {
    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    loop {
        match chars.next() {
            None => break,
            Some('\'') if !in_double => {
                in_single = !in_single;
                current.push('\'');
            }
            Some('"') if !in_single => {
                in_double = !in_double;
                current.push('"');
            }
            Some('\\') if in_double => {
                current.push('\\');
                if let Some(ch) = chars.next() {
                    current.push(ch);
                }
            }
            Some('|') if !in_single && !in_double => {
                segments.push(current.trim().to_string());
                current = String::new();
            }
            Some(ch) => current.push(ch),
        }
    }

    if in_single {
        return Err("unterminated single quote".into());
    }
    if in_double {
        return Err("unterminated double quote".into());
    }

    segments.push(current.trim().to_string());
    Ok(segments)
}

// ---------------------------------------------------------------------------
// Parse a single pipeline stage into a Stage struct
// ---------------------------------------------------------------------------

fn parse_stage(s: &str) -> Result<Option<Stage>, String> {
    if s.is_empty() {
        return Ok(None);
    }

    let mut tokens = tokenise(s)?;
    if tokens.is_empty() {
        return Ok(None);
    }

    // Separate redirect tokens from argument tokens.
    // We walk through tokens and pull out redirect pairs.
    let mut redirects: Vec<Redirect> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i].as_str() {
            "<" => {
                i += 1;
                let target = tokens.get(i)
                    .ok_or("expected filename after '<'")?
                    .clone();
                redirects.push(Redirect { kind: RedirectKind::Stdin, target });
            }
            ">" => {
                i += 1;
                let target = tokens.get(i)
                    .ok_or("expected filename after '>'")?
                    .clone();
                redirects.push(Redirect { kind: RedirectKind::Stdout, target });
            }
            ">>" => {
                i += 1;
                let target = tokens.get(i)
                    .ok_or("expected filename after '>>'")?
                    .clone();
                redirects.push(Redirect { kind: RedirectKind::Append, target });
            }
            "2>" => {
                i += 1;
                let target = tokens.get(i)
                    .ok_or("expected filename after '2>'")?
                    .clone();
                redirects.push(Redirect { kind: RedirectKind::Stderr, target });
            }
            _ => {
                args.push(tokens.remove(i));
                continue;
            }
        }
        i += 1;
    }

    if args.is_empty() {
        return Ok(None);
    }

    // Peel leading VAR=value env pairs.
    let mut env: Vec<(String, String)> = Vec::new();
    while !args.is_empty() {
        if let Some(eq) = args[0].find('=') {
            let key = args[0][..eq].to_string();
            if is_valid_identifier(&key) && args.len() > 1 {
                let val = args[0][eq + 1..].to_string();
                env.push((key, val));
                args.remove(0);
                continue;
            }
        }
        break;
    }

    if args.is_empty() {
        return Ok(None);
    }

    Ok(Some(Stage {
        name: args[0].clone(),
        args: args[1..].to_vec(),
        env,
        redirects,
    }))
}

// ---------------------------------------------------------------------------
// Tokenise a stage string into raw tokens
// ---------------------------------------------------------------------------

fn tokenise(s: &str) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut chars = s.chars().peekable();

    loop {
        // skip whitespace
        while chars.peek().map(|c| c.is_whitespace()) == Some(true) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        // Check for redirect operators first (before general token reading)
        if let Some(&c) = chars.peek() {
            if c == '>' || c == '<' || (c == '2' && s.contains("2>")) {
                if let Some(op) = try_read_redirect_op(&mut chars) {
                    tokens.push(op);
                    continue;
                }
            }
        }

        match read_token(&mut chars)? {
            Some(tok) => tokens.push(tok),
            None => break,
        }
    }
    Ok(tokens)
}

/// Try to read a redirect operator: >  >>  <  2>
fn try_read_redirect_op(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    match chars.peek() {
        Some(&'>') => {
            chars.next();
            if chars.peek() == Some(&'>') {
                chars.next();
                Some(">>".into())
            } else {
                Some(">".into())
            }
        }
        Some(&'<') => {
            chars.next();
            Some("<".into())
        }
        // 2> — only if the next char is '>' (not a plain '2' argument)
        _ => None,
    }
}

fn read_token(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Option<String>, String> {
    if chars.peek().is_none() {
        return Ok(None);
    }

    let mut token = String::new();

    loop {
        match chars.peek() {
            None => break,
            Some(&c) if c.is_whitespace() => break,
            // Redirect operators terminate the current token
            Some(&'>') | Some(&'<') => break,
            Some(&'\'') => {
                chars.next();
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => token.push(ch),
                        None => return Err("unterminated single quote".into()),
                    }
                }
            }
            Some(&'"') => {
                chars.next();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => token.push('\n'),
                            Some('t') => token.push('\t'),
                            Some('\\') => token.push('\\'),
                            Some('"') => token.push('"'),
                            Some(ch) => { token.push('\\'); token.push(ch); }
                            None => return Err("unterminated escape in double quote".into()),
                        },
                        Some(ch) => token.push(ch),
                        None => return Err("unterminated double quote".into()),
                    }
                }
            }
            Some(&'\\') => {
                chars.next();
                match chars.next() {
                    Some(ch) => token.push(ch),
                    None => return Err("trailing backslash".into()),
                }
            }
            Some(_) => {
                // Check for 2> redirect
                let ch = chars.next().unwrap();
                if ch == '2' && chars.peek() == Some(&'>') {
                    if !token.is_empty() {
                        // Put back by returning what we have; the '2' is tricky.
                        // Simplification: if token already has content, the '2'
                        // is part of an argument (e.g. fd2>file is unusual).
                        token.push(ch);
                    } else {
                        chars.next(); // consume '>'
                        return Ok(Some("2>".into()));
                    }
                } else {
                    token.push(ch);
                }
            }
        }
    }

    if token.is_empty() { Ok(None) } else { Ok(Some(token)) }
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::pipeline::RedirectKind;

    fn pipeline(line: &str) -> Pipeline {
        match parse(line) {
            ParseResult::Pipeline(p) => p,
            other => panic!("expected Pipeline, got {:?}", other),
        }
    }

    #[test]
    fn simple_command() {
        let p = pipeline("ls -la /tmp");
        assert_eq!(p.stages.len(), 1);
        assert_eq!(p.stages[0].name, "ls");
        assert_eq!(p.stages[0].args, vec!["-la", "/tmp"]);
    }

    #[test]
    fn pipe_two_stages() {
        let p = pipeline("ls | grep foo");
        assert_eq!(p.stages.len(), 2);
        assert_eq!(p.stages[0].name, "ls");
        assert_eq!(p.stages[1].name, "grep");
        assert_eq!(p.stages[1].args, vec!["foo"]);
    }

    #[test]
    fn pipe_three_stages() {
        let p = pipeline("cat file | sort | uniq -c");
        assert_eq!(p.stages.len(), 3);
        assert_eq!(p.stages[2].name, "uniq");
    }

    #[test]
    fn redirect_stdout() {
        let p = pipeline("echo hello > out.txt");
        assert_eq!(p.stages[0].redirects.len(), 1);
        assert_eq!(p.stages[0].redirects[0].kind, RedirectKind::Stdout);
        assert_eq!(p.stages[0].redirects[0].target, "out.txt");
    }

    #[test]
    fn redirect_append() {
        let p = pipeline("echo hi >> log.txt");
        assert_eq!(p.stages[0].redirects[0].kind, RedirectKind::Append);
    }

    #[test]
    fn redirect_stdin() {
        let p = pipeline("sort < input.txt");
        assert_eq!(p.stages[0].redirects[0].kind, RedirectKind::Stdin);
    }

    #[test]
    fn env_vars() {
        let p = pipeline("FOO=bar BAZ=qux env");
        assert_eq!(p.stages[0].env, vec![
            ("FOO".into(), "bar".into()),
            ("BAZ".into(), "qux".into()),
        ]);
        assert_eq!(p.stages[0].name, "env");
    }

    #[test]
    fn single_quotes_in_pipe() {
        let p = pipeline("echo 'hello | world' | cat");
        assert_eq!(p.stages.len(), 2);
        assert_eq!(p.stages[0].args[0], "hello | world");
    }

    #[test]
    fn empty_and_comment() {
        assert_eq!(parse(""), ParseResult::Empty);
        assert_eq!(parse("   "), ParseResult::Empty);
        assert_eq!(parse("# comment"), ParseResult::Empty);
    }

    #[test]
    fn unterminated_quote() {
        assert!(matches!(parse("echo 'oops"), ParseResult::Error(_)));
    }
}
