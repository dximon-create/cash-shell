// cash — Marketplace
//
// Install, verify, and manage agents from the cash marketplace.
// All agents are verified before installation.
// Permissions are declared upfront and cannot be expanded at runtime.
//
// Components:
//   Registry    — local database of installed agents
//   Installer   — download, verify, install
//   Verifier    — SHA-256 checksum + signature check
//   Permissions — permission declaration and wall enforcement

pub mod installer;
pub mod registry;
pub mod verifier;

pub use registry::{MarketplaceRegistry, InstalledAgent};
pub use installer::Installer;
pub use verifier::Verifier;
