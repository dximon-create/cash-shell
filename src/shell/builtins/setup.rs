// cash — setup wizard
//
// First-run wizard for configuring cash.
// Runs when user types: cash setup
//
// Configures:
//   - WhatsApp alerts via Twilio
//   - Webhook alerts
//   - Alert level threshold

use std::io::{self, Write};
use crate::store::Store;
use crate::security::webhook::AlertConfig;

const RESET:  &str = "\x1b[0m";
const BOLD:   &str = "\x1b[1m";
const GREEN:  &str = "\x1b[32m";
const CYAN:   &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const DIM:    &str = "\x1b[2m";

pub fn run(store: &Store) {
    println!("\n{}{}╔══ cash setup ══╗{}", BOLD, CYAN, RESET);
    println!("{}Welcome to cash — Conscious Adaptive Secure Host{}", DIM, RESET);
    println!("{}This wizard configures your security alerts.\n{}", DIM, RESET);

    let config_path = store.root.join("config.toml");
    let mut config = AlertConfig::from_toml(&config_path);

    // --- WhatsApp alerts ---
    println!("{}{}1. WhatsApp Alerts{}", BOLD, CYAN, RESET);
    println!("{}Get a WhatsApp message when cash detects a security threat.{}", DIM, RESET);
    println!("{}Requires a free Twilio account: twilio.com{}\n", DIM, RESET);

    if ask_yes_no("  Enable WhatsApp alerts?") {
        println!();
        println!("{}  Sign up at twilio.com, then enter your credentials:{}", DIM, RESET);

        let sid = ask_input("  Twilio Account SID (AC...): ");
        let token = ask_input("  Twilio Auth Token: ");
        let from = ask_input("  WhatsApp from number (whatsapp:+14155238886): ");
        let to = ask_input("  Your WhatsApp number (whatsapp:+447...): ");

        if !sid.is_empty() && !token.is_empty() && !from.is_empty() && !to.is_empty() {
            config.twilio_sid    = Some(sid);
            config.twilio_token  = Some(token);
            config.whatsapp_from = Some(from);
            config.whatsapp_to   = Some(to);
            config.enabled       = true;
            println!("\n  {}✓ WhatsApp configured.{}", GREEN, RESET);
        } else {
            println!("\n  {}Skipped — you can configure later by editing ~/.cash/config.toml{}", YELLOW, RESET);
        }
    }

    println!();

    // --- Webhook alerts ---
    println!("{}{}2. Webhook Alerts{}", BOLD, CYAN, RESET);
    println!("{}Works with Zapier, Slack, Microsoft Teams, or any HTTP endpoint.{}", DIM, RESET);
    println!("{}Zapier → WhatsApp is the easiest setup if you don't have Twilio.{}\n", DIM, RESET);

    if ask_yes_no("  Enable webhook alerts?") {
        let url = ask_input("  Webhook URL: ");
        if !url.is_empty() {
            config.webhook_url = Some(url);
            config.enabled     = true;
            println!("\n  {}✓ Webhook configured.{}", GREEN, RESET);
        }
    }

    println!();

    // --- Alert level ---
    println!("{}{}3. Alert Level{}", BOLD, CYAN, RESET);
    println!("  {}WARNING{}  — alerts on suspicious activity (recommended)", YELLOW, RESET);
    println!("  {}CRITICAL{} — alerts only on serious threats", "\x1b[31m", RESET);
    println!("  {}INFO{}     — alerts on everything (noisy)\n", DIM, RESET);

    let level = ask_input("  Alert level [WARNING]: ");
    let level = if level.is_empty() { "WARNING".to_string() } else { level.to_uppercase() };
    config.min_level = level.clone();

    println!();

    // --- Save ---
    match config.save(&config_path) {
        Ok(_) => {
            println!("{}{}✓ Setup complete!{}", BOLD, GREEN, RESET);
            println!("{}Config saved to: ~/.cash/config.toml{}", DIM, RESET);

            if config.is_whatsapp_configured() {
                println!("\n{}{}Testing WhatsApp...{}", DIM, CYAN, RESET);
                crate::security::webhook::send_whatsapp(
                    &config,
                    "cash security alert system is now active on your machine."
                );
                println!("{}✓ Test message sent. Check your WhatsApp.{}", GREEN, RESET);
            }

            if config.is_webhook_configured() {
                if let Some(url) = &config.webhook_url {
                    crate::security::webhook::send_webhook(url,
                        "cash security alert system is now active.");
                    println!("{}✓ Webhook test sent.{}", GREEN, RESET);
                }
            }
        }
        Err(e) => {
            eprintln!("{}✗ Setup failed: {}{}", "\x1b[31m", e, RESET);
        }
    }

    println!();
}

/// Show current alert configuration.
pub fn show_config(store: &Store) {
    let config_path = store.root.join("config.toml");
    let config = AlertConfig::from_toml(&config_path);

    println!("\n{}{}cash alerts config:{}", BOLD, CYAN, RESET);
    println!("  enabled:    {}", if config.enabled { "yes" } else { "no" });
    println!("  min_level:  {}", config.min_level);
    println!("  whatsapp:   {}", if config.is_whatsapp_configured() { "configured" } else { "not configured" });
    println!("  webhook:    {}", if config.is_webhook_configured() { "configured" } else { "not configured" });
    println!("\n{}Run 'cash setup' to configure.{}\n", DIM, RESET);
}

fn ask_yes_no(prompt: &str) -> bool {
    print!("{} [y/N] ", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let t = input.trim().to_lowercase();
    t == "y" || t == "yes"
}

fn ask_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}
