// cash — Approval UI
//
// Shows the user what is about to happen.
// Asks for confirmation before risky actions.
// Dry-run mode shows plan without executing.

use std::io::{self, Write};
use super::action::{Action, Plan};
use super::engine::Decision;
use super::risk::RiskScorer;

const RESET:  &str = "\x1b[0m";
const BOLD:   &str = "\x1b[1m";
const GREEN:  &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED:    &str = "\x1b[31m";
const CYAN:   &str = "\x1b[36m";
const DIM:    &str = "\x1b[2m";

pub struct ApprovalPrompt {
    pub dry_run: bool,
}

impl ApprovalPrompt {
    pub fn new() -> Self { Self { dry_run: false } }
    pub fn dry_run() -> Self { Self { dry_run: true } }

    /// Show a single action with its decision.
    /// Returns true if execution should proceed.
    pub fn ask_action(&self, action: &Action, decision: &Decision) -> bool {
        if self.dry_run {
            self.show_dry_run(action, decision);
            return false;
        }

        match decision {
            Decision::Allow => true,
            Decision::Warn { reason } => {
                self.show_warning(action, reason);
                true
            }
            Decision::Approve { reason } => {
                self.show_approval_prompt(action, reason)
            }
            Decision::Block { reason } => {
                self.show_block(action, reason);
                false
            }
        }
    }

    /// Show a full plan and ask for overall approval.
    pub fn ask_plan(&self, plan: &Plan, decisions: &[(Action, Decision)]) -> bool {
        println!("\n{}{}cash plan:{} {}{}{}\n",
            BOLD, CYAN, RESET, BOLD, plan.intent, RESET);

        let mut blocked = false;
        let mut needs_approval = false;

        for (i, (action, decision)) in decisions.iter().enumerate() {
            let colour = RiskScorer::colour(action.risk_score);
            let label  = RiskScorer::label(action.risk_score);
            let icon   = match decision {
                Decision::Allow           => format!("{}✓{}", GREEN, RESET),
                Decision::Warn { .. }    => format!("{}⚠{}", YELLOW, RESET),
                Decision::Approve { .. } => format!("{}●{}", RED, RESET),
                Decision::Block { .. }   => format!("{}✗{}", RED, RESET),
            };
            println!("  {}. {} {}{}{} — {} {}[{}{}{}]{}\n     {}>{} {}",
                i + 1, icon,
                BOLD, action.kind.display(), RESET,
                action.target,
                DIM, colour, label, RESET, RESET,
                DIM, RESET,
                action.raw_command
            );

            if matches!(decision, Decision::Block { .. }) { blocked = true; }
            if matches!(decision, Decision::Approve { .. }) { needs_approval = true; }
        }

        if self.dry_run {
            println!("\n{}{}[dry-run] No actions were executed.{}", DIM, YELLOW, RESET);
            return false;
        }

        if blocked {
            println!("\n{}{}✗ Plan blocked:{} one or more actions are not permitted.\n",
                BOLD, RED, RESET);
            for (action, decision) in decisions {
                if let Decision::Block { reason } = decision {
                    println!("  {} {}: {}", RED, action.raw_command, reason);
                    println!("{}", RESET);
                }
            }
            return false;
        }

        if needs_approval {
            return self.confirm_plan();
        }

        // All warnings — show and proceed
        if decisions.iter().any(|(_, d)| matches!(d, Decision::Warn { .. })) {
            println!("\n{}{}⚠  Some actions carry medium risk.{}", BOLD, YELLOW, RESET);
            println!("{}   Proceeding automatically. Use --approve to require confirmation.{}\n", DIM, RESET);
        }

        true
    }

    fn show_dry_run(&self, action: &Action, decision: &Decision) {
        let colour = RiskScorer::colour(action.risk_score);
        let label  = RiskScorer::label(action.risk_score);
        println!("\n{}[dry-run]{} {} {} {}[{}{}{} risk]{}\n  command: {}",
            YELLOW, RESET,
            action.kind.display(),
            action.target,
            DIM, colour, label, RESET, RESET,
            action.raw_command
        );
        if let Decision::Block { reason } = decision {
            println!("  {}✗ would be blocked:{} {}", RED, RESET, reason);
        }
    }

    fn show_warning(&self, action: &Action, reason: &str) {
        println!("\n{}{}⚠  Warning:{} {}", BOLD, YELLOW, RESET, reason);
        println!("{}   {}: {}{}", DIM, action.kind.display(), action.target, RESET);
    }

    fn show_block(&self, action: &Action, reason: &str) {
        println!("\n{}{}✗  Blocked:{} {}", BOLD, RED, RESET, reason);
        println!("{}   command: {}{}", DIM, action.raw_command, RESET);
    }

    fn show_approval_prompt(&self, action: &Action, reason: &str) -> bool {
        let colour = RiskScorer::colour(action.risk_score);
        println!("\n{}{}● Approval required:{} {}", BOLD, RED, RESET, reason);
        println!("  action:  {}{}{}", BOLD, action.kind.display(), RESET);
        println!("  target:  {}", action.target);
        println!("  command: {}{}{}", DIM, action.raw_command, RESET);
        println!("  risk:    {}{}{:.1}/10.0{}\n",
            colour, BOLD, action.risk_score, RESET);
        self.read_yes("  Proceed? [y/N] ")
    }

    fn confirm_plan(&self) -> bool {
        println!("\n{}{}● This plan requires your approval.{}", BOLD, RED, RESET);
        self.read_yes("  Execute all approved actions? [y/N] ")
    }

    fn read_yes(&self, prompt: &str) -> bool {
        print!("{}", prompt);
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim().to_lowercase();
            return trimmed == "y" || trimmed == "yes";
        }
        false
    }
}

impl Default for ApprovalPrompt {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::action::Action;

    #[test]
    fn dry_run_returns_false() {
        let prompt = ApprovalPrompt::dry_run();
        let action = Action::from_command("ls /tmp");
        assert!(!prompt.ask_action(&action, &Decision::Allow));
    }

    #[test]
    fn allow_returns_true() {
        let prompt = ApprovalPrompt::new();
        let action = Action::from_command("ls /tmp");
        assert!(prompt.ask_action(&action, &Decision::Allow));
    }

    #[test]
    fn block_returns_false() {
        let prompt = ApprovalPrompt::new();
        let action = Action::from_command("rm -rf /");
        assert!(!prompt.ask_action(&action, &Decision::Block {
            reason: "test block".into()
        }));
    }

    #[test]
    fn warn_returns_true_no_tty() {
        let prompt = ApprovalPrompt::new();
        let action = Action::from_command("apt install nmap");
        // Warn should proceed automatically
        assert!(prompt.ask_action(&action, &Decision::Warn {
            reason: "medium risk".into()
        }));
    }
}
