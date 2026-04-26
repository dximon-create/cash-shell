pub mod security_agent;
// cash — Layer 3: Agent Platform
//
// Native runtime for AI agents. Agents are first-class citizens in cash.
// They use CLI tools to act and AI APIs only to think.
//
// Components:
//   Agent          — identity, permissions, state machine
//   AgentRuntime   — spawns, manages, terminates agents
//   SharedMemory   — key/value space shared between agents
//   MessageBus     — Unix domain socket messaging between agents
//   AgentHandle    — reference to a running agent

pub mod agent;
pub mod bus;
pub mod memory;
pub mod runtime;

pub use agent::{Agent, AgentState, AgentPermission};
pub use runtime::AgentRuntime;
pub use memory::SharedMemory;
pub use bus::MessageBus;
