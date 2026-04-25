// cash — evaluator
//
// Routes a Pipeline through these layers in order:
//   1. Policy gate  — every command is risk-scored before execution
//   2. Built-ins    — show, go, copy, move, remove, teach, help, exit
//   3. Resolver     — exact → pattern → fuzzy match against taught commands
//   4. Executor     — fork/exec for everything else
//   5. Suggester    — when executor returns 127 (command not found)
//
// Nothing executes without passing through the policy gate first.
// Recursive depth is capped to prevent infinite loops.

use super::builtins::{self, BuiltinResult};
use super::executor;
use super::pipeline::Pipeline;
use super::resolver::{self, MatchKind, ResolveResult};
use super::suggester;
use crate::store::Store;
use crate::store::config::Config;
use crate::policy::{Action, PolicyEngine, Decision};
use crate::policy::approval::ApprovalPrompt;

const MAX_EVAL_DEPTH: u32 = 5;

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
    eval_depth(pipeline, cwd, store, config, 0)
}

fn eval_depth(
    pipeline: &Pipeline,
    cwd:      &std::path::Path,
    store:    &Store,
    config:   &Config,
    depth:    u32,
) -> EvalResult {
    // Guard against infinite recursion from taught commands.
    if depth >= MAX_EVAL_DEPTH {
        return EvalResult::Error(
            "cash: command depth limit reached (possible infinite loop in taught commands)".into()
        );
    }

    // Build raw command string for policy evaluation.
    let raw_command = pipeline_to_string(pipeline);

    // --- Policy Gate ---
    // Every command passes through here before execution.
    // Built-ins are exempt from the gate (they're cash's own safe commands).
    let is_builtin = pipeline.stages.len() == 1 && {
        let name = pipeline.stages[0].name.as_str();
        matches!(name, "show"|"go"|"copy"|"move"|"remove"|"teach"|"help"|"exit")
    };

    if !is_builtin && !raw_command.is_empty() {
        let mut action = Action::from_command(&raw_command);
        let engine = PolicyEngine::new();
        let decision = engine.evaluate(&mut action);

        match &decision {
            Decision::Block { reason } => {
                eprintln!("\x1b[1m\x1b[31m✗ blocked:\x1b[0m {}", reason);
                eprintln!("  command: \x1b[2m{}\x1b[0m", raw_command);
                return EvalResult::Error(String::new());
            }
            Decision::Approve { reason } => {
                let prompt = ApprovalPrompt::new();
                let approved = prompt.ask_action(&action, &decision);
                if !approved {
                    return EvalResult::Error("cash: cancelled".into());
                }
            }
            Decision::Warn { reason } => {
                eprintln!("\x1b[33m⚠  {}\x1b[0m", reason);
            }
            Decision::Allow => {
                // Safe — proceed silently.
            }
        }
    }

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
                return run_command_string(&command, cwd, store, config, depth + 1);
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

/// Convert a pipeline back to a command string for policy evaluation.
fn pipeline_to_string(pipeline: &Pipeline) -> String {
    pipeline.stages.iter()
        .map(|s| {
            if s.args.is_empty() {
                s.name.clone()
            } else {
                format!("{} {}", s.name, s.args.join(" "))
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn run_command_string(
    command: &str,
    cwd:     &std::path::Path,
    store:   &Store,
    config:  &Config,
    depth:   u32,
) -> EvalResult {
    use super::parser::{parse, ParseResult};
    match parse(command) {
        ParseResult::Pipeline(p) => eval_depth(&p, cwd, store, config, depth),
        ParseResult::Empty       => EvalResult::Ok,
        ParseResult::Error(e)   => EvalResult::Error(format!("taught command parse error: {}", e)),
    }
}
