// cash — Policy Engine
//
// Every action passes through here before execution.
// Nothing runs without a policy decision.
//
// Decision outcomes:
//   Allow  — low risk, execute automatically
//   Warn   — safe but surprising, show warning then execute
//   Approve — user must confirm before execution
//   Block  — forbidden, never execute

pub mod action;
pub mod engine;
pub mod risk;
pub mod approval;

pub use action::{Action, ActionKind};
pub use engine::{PolicyEngine, Decision};
pub use risk::RiskScorer;
pub use approval::ApprovalPrompt;
