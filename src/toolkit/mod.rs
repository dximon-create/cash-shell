// cash — Ethical Hacking Toolkit
//
// Network reconnaissance and learning tools.
// Every tool:
//   1. Logs to audit.db
//   2. Explains what it does before running
//   3. States the ethical/legal rule
//   4. Only runs on permitted networks
//
// Install via: cash install ethical-toolkit
// All tools available as built-in commands after install.

pub mod arp;
pub mod dns;
pub mod learn;
pub mod scan;
pub mod trace;
pub mod lab;

pub use learn::LearningMode;
