// cash — agent built-in command
//
// Usage:
//   agent start security      — start the security agent
//   agent stop security       — stop it
//   agent ask "question"      — ask the security agent
//   agent scan                — scan system for threats
//   agent configure           — set up API key
//   agent status              — show running agents

use crate::store::Store;
use crate::agents::security_agent::SecurityAgent;

const RESET:  &str = "\x1b[0m";
const BOLD:   &str = "\x1b[1m";
const GREEN:  &str = "\x1b[32m";
const RED:    &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN:   &str = "\x1b[36m";
const DIM:    &str = "\x1b[2m";

pub fn run(args: &[String], store: &Store) {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("help");

    match sub {
        "start" => {
            let name = args.get(1).map(|s| s.as_str()).unwrap_or("security");
            start_agent(name);
        }
        "stop" => {
            let name = args.get(1).map(|s| s.as_str()).unwrap_or("security");
            println!("{}agent:{} {} stopped.", DIM, RESET, name);
        }
        "ask" => {
            let question = args[1..].join(" ");
            if question.is_empty() {
                println!("Usage: agent ask \"your question\"");
                return;
            }
            ask_agent(&question);
        }
        "scan" => {
            run_scan();
        }
        "configure" => {
            configure(store);
        }
        "status" => {
            show_status();
        }
        "help" | _ => {
            show_help();
        }
    }
}

fn start_agent(name: &str) {
    match name {
        "security" => {
            let agent = SecurityAgent::new();
            if !agent.is_configured() {
                println!("{}⚠  Security agent needs an API key.{}", YELLOW, RESET);
                println!("{}Run: agent configure{}", DIM, RESET);
                return;
            }
            println!("{}{}✓ Security agent started.{}", BOLD, GREEN, RESET);
            println!("{}Monitoring system — type 'agent ask <question>' to query.{}", DIM, RESET);
            println!("{}Type 'agent scan' for a full security scan.{}\n", DIM, RESET);
        }
        _ => {
            println!("Unknown agent: {}. Available: security", name);
        }
    }
}

fn ask_agent(question: &str) {
    let agent = SecurityAgent::new();

    if !agent.is_configured() {
        println!("{}⚠  No API key configured.{}", YELLOW, RESET);
        println!("{}Run: agent configure{}\n", DIM, RESET);
        return;
    }

    println!("{}{}Asking security agent...{}", DIM, CYAN, RESET);
    let response = agent.ask(question);

    let risk_color = match response.risk.as_str() {
        "CRITICAL" => "\x1b[35m",
        "HIGH"     => RED,
        "MEDIUM"   => YELLOW,
        _          => GREEN,
    };

    println!("\n{}{}Risk:{} {}{}{}", BOLD, DIM, RESET, risk_color, response.risk, RESET);
    println!("{}{}{}\n", BOLD, response.message, RESET);

    if !response.actions.is_empty() {
        println!("{}Recommended actions:{}", DIM, RESET);
        for action in &response.actions {
            println!("  {}→{} {}", CYAN, RESET, action);
        }
        println!();
    }
}

fn run_scan() {
    let agent = SecurityAgent::new();

    if !agent.is_configured() {
        println!("{}⚠  No API key configured.{}", YELLOW, RESET);
        println!("{}Run: agent configure{}\n", DIM, RESET);
        return;
    }

    println!("{}{}🔍 Security scan starting...{}", BOLD, CYAN, RESET);
    println!("{}Gathering system information...{}\n", DIM, RESET);

    let response = agent.scan();

    let risk_color = match response.risk.as_str() {
        "CRITICAL" => "\x1b[35m",
        "HIGH"     => RED,
        "MEDIUM"   => YELLOW,
        _          => GREEN,
    };

    println!("{}{}Scan complete:{}", BOLD, CYAN, RESET);
    println!("{}Risk level:{} {}{}{}\n", DIM, RESET, risk_color, response.risk, RESET);
    println!("{}", response.message);

    if !response.actions.is_empty() {
        println!("\n{}Recommended actions:{}", DIM, RESET);
        for action in &response.actions {
            println!("  {}→{} {}", CYAN, RESET, action);
        }
    }
    println!();
}

fn configure(store: &Store) {
    use std::io::{self, Write};

    println!("\n{}{}agent configure{}", BOLD, CYAN, RESET);
    println!("{}Set up your Anthropic API key for AI agents.\n{}", DIM, RESET);
    println!("{}Get a free API key at: console.anthropic.com{}\n", DIM, RESET);

    print!("  Anthropic API key (sk-ant-...): ");
    io::stdout().flush().ok();
    let mut key = String::new();
    io::stdin().read_line(&mut key).ok();
    let key = key.trim().to_string();

    if key.is_empty() {
        println!("{}Skipped.{}", DIM, RESET);
        return;
    }

    // Save to config.toml
    let config_path = store.root.join("config.toml");
    let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
    let mut doc: toml::Value = existing.parse()
        .unwrap_or(toml::Value::Table(Default::default()));

    let mut agents_table = toml::value::Table::new();
    agents_table.insert("anthropic_key".into(), toml::Value::String(key));

    if let toml::Value::Table(ref mut t) = doc {
        t.insert("agents".into(), toml::Value::Table(agents_table));
    }

    if let Ok(text) = toml::to_string_pretty(&doc) {
        if std::fs::write(&config_path, text).is_ok() {
            println!("{}✓ API key saved to ~/.cash/config.toml{}", GREEN, RESET);
            println!("{}Run: agent start security{}\n", DIM, RESET);
        }
    }
}

fn show_status() {
    let agent = SecurityAgent::new();
    println!("\n{}{}agent status{}", BOLD, CYAN, RESET);
    println!("  security agent: {}", if agent.is_configured() {
        format!("{}configured{}", GREEN, RESET)
    } else {
        format!("{}not configured — run: agent configure{}", YELLOW, RESET)
    });
    println!();
}

fn show_help() {
    println!("\n{}{}cash agents{}", BOLD, CYAN, RESET);
    println!("{}AI agents that think and act on your behalf.{}\n", DIM, RESET);
    println!("  {}agent start security{}   — start the security agent", BOLD, RESET);
    println!("  {}agent scan{}             — scan system for threats", BOLD, RESET);
    println!("  {}agent ask \"question\"{}   — ask the security agent anything", BOLD, RESET);
    println!("  {}agent configure{}        — set up Anthropic API key", BOLD, RESET);
    println!("  {}agent status{}           — show agent status\n", BOLD, RESET);
}
