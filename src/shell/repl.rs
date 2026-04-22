use std::io::{self, Write};
use super::{
    eval::{eval, EvalResult},
    input::{read_line, InputResult},
    parser::{parse, ParseResult},
    prompt,
};
use crate::store::{history, config};

pub fn run(shell: &mut super::Shell) -> anyhow::Result<()> {
    let mut out = io::stdout();
    let cfg = config::load(&shell.store.root);
    loop {
        let mut prompt_buf: Vec<u8> = Vec::new();
        prompt::render(&mut prompt_buf, &shell.cwd)?;
        out.write_all(&prompt_buf)?;
        out.flush()?;
        let prompt_visible = visible_len(&prompt_buf);
        let line = match read_line(prompt_visible as u16)? {
            InputResult::Line(l) => l,
            InputResult::Interrupted => continue,
            InputResult::Eof => { println!("exit"); break; }
        };
        let pipeline = match parse(&line) {
            ParseResult::Pipeline(p) => p,
            ParseResult::Empty => continue,
            ParseResult::Error(e) => { eprintln!("cash: parse error: {}", e); continue; }
        };
        let cwd_str = shell.cwd.display().to_string();
        match eval(&pipeline, &shell.cwd, &shell.store, &cfg) {
            EvalResult::Ok => {
                let _ = history::push(&shell.store, &line, &cwd_str, 0, cfg.history_dedup);
                let _ = history::trim(&shell.store, cfg.history_limit);
            }
            EvalResult::ChangeDir(new_cwd) => {
                shell.cwd = new_cwd;
                let _ = history::push(&shell.store, &line, &cwd_str, 0, cfg.history_dedup);
            }
            EvalResult::Exit(code) => {
                let _ = history::push(&shell.store, &line, &cwd_str, code, cfg.history_dedup);
                if code != 0 { std::process::exit(code); }
                break;
            }
            EvalResult::Error(e) => {
                if !e.is_empty() { eprintln!("cash: {}", e); }
                let _ = history::push(&shell.store, &line, &cwd_str, 1, cfg.history_dedup);
            }
            EvalResult::NeedsConfirmation { suggestion, command, confidence } => {
                let pct = (confidence * 100.0) as u32;
                eprint!("cash: did you mean '{}' → {} ({}% match)? [y/N] ", suggestion, command, pct);
                io::stderr().flush().ok();
                let mut answer = String::new();
                io::stdin().read_line(&mut answer).ok();
                if answer.trim().eq_ignore_ascii_case("y") {
                    match parse(&command) {
                        ParseResult::Pipeline(p) => {
                            match eval(&p, &shell.cwd, &shell.store, &cfg) {
                                EvalResult::Ok => { let _ = history::push(&shell.store, &line, &cwd_str, 0, cfg.history_dedup); }
                                EvalResult::ChangeDir(new_cwd) => { shell.cwd = new_cwd; }
                                EvalResult::Error(e) => { if !e.is_empty() { eprintln!("cash: {}", e); } }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                } else {
                    println!("cash: cancelled");
                }
            }
        }
    }
    Ok(())
}

fn visible_len(buf: &[u8]) -> usize {
    let s = String::from_utf8_lossy(buf);
    let mut count = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if ch == '\x1b' { in_escape = true; }
        else if in_escape { if ch.is_ascii_alphabetic() { in_escape = false; } }
        else { count += if (ch as u32) > 0x2E80 { 2 } else { 1 }; }
    }
    count
}
