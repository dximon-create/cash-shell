// cash — Tool Manager
//
// Installs, registers, wraps, and manages external security tools.
// Detects OS and uses the right package manager automatically.
//
// Supported package managers:
//   apt     — Ubuntu, Debian, Kali
//   brew    — macOS
//   pacman  — Arch Linux
//
// Tools are registered in ~/.cash/tools.db after install.
// Installed tools get natural language wrappers in cash.

pub mod detector;
pub mod installer;
pub mod registry;
pub mod runner;
pub mod formatter;

pub use installer::ToolInstaller;
pub use registry::{ToolRegistry, ToolRecord};
pub use runner::ToolRunner;
pub use formatter::OutputFormatter;
