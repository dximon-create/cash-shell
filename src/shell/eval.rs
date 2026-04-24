// cash — evaluator
//
// Routes a Pipeline through three layers:
//   1. Built-ins   — show, go, copy, move, remove, teach, help, exit
//   2. Resolver    — exact → pattern → fuzzy match against taught commands
//   3. Executor    — fork/exec for everything else
//   4. Suggester   — when executor returns 127 (command not found)

use super::builtins::{self, BuiltinResult};
use super::executor;
use super::pipeline::Pipeline;
use super::resolver::{self, MatchKind, ResolveResult};
use super::suggester;
use crate::store::Store;
use crate::store::config::Config;

#[derive(Debug)]
pub enum EvalResult {
    Ok,
    Exit(i32),
    Error(String),
    ChangeDir(std::path::PathBuf),
    NeedsConfirmation {
        suggestion: String,
        command:    String,
        confidence: f64,
    },
}

pub fn eval(
    pipeline: &Pipeline,
    cwd:      &std::path::Path,
    store:    &Store,
    config:   &Config,
) -> EvalResult {
    // --- Built-ins ---
    if pipeline.stages.len() == 1 {
        let stage = &pipeline.stages[0];
        if let Some(result) = builtins::dispatch(stage, cwd, store) {
            return match result {
                BuiltinResult::Ok           => EvalResult::Ok,
                BuiltinResult::ChangeDir(p) => EvalResult::ChangeDir(p),
                BuiltinResult::Exit(code)   => EvalResult::Exit(code),
                BuiltinResult::Err          => EvalResult::Error(String::new()),
            };
        }

        // --- Resolver ---
        let full_input = if stage.args.is_empty() {
            stage.name.clone()
        } else {
            format!("{} {}", stage.name, stage.args.join(" "))
        };

        match resolver::resolve(&full_input, store, config) {
            ResolveResult::Resolved { command, match_kind, .. } => {
                if match_kind != MatchKind::Exact {
                    eprintln!("cash: running '{}'", command);
                }
                let _ = crate::store::memory::record_use(store, &full_input);
                return run_command_string(&command, cwd, store, config);
            }
            ResolveResult::BelowThreshold { suggestion, command, confidence, .. } => {
                return EvalResult::NeedsConfirmation { suggestion, command, confidence };
            }
            ResolveResult::NotFound { .. } => {
                // Fall through to executor.
            }
        }
    }

    // --- Executor ---
    match executor::run(pipeline) {
        Ok(status) => {
            if status.code == 127 {
                // Command not found — ask the Suggestion Engine.
                let input = &pipeline.stages[0].name;
                let suggestion = suggester::suggest(input, store);
                suggester::print_suggestion(&suggestion);
                EvalResult::Error(String::new())
            } else {
                EvalResult::Ok
            }
        }
        Err(e) => EvalResult::Error(e.to_string()),
    }
}

fn run_command_string(
    command: &str,
    cwd:     &std::path::Path,
    store:   &Store,
    config:  &Config,
) -> EvalResult {
    use super::parser::{parse, ParseResult};
    match parse(command) {
        ParseResult::Pipeline(p) => eval(&p, cwd, store, config),
        ParseResult::Empty       => EvalResult::Ok,
        ParseResult::Error(e)   => EvalResult::Error(format!("taught command parse error: {}", e)),
    }
}
