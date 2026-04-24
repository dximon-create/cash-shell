// cash — Agent definition
//
// An Agent is an autonomous process that runs inside cash.
// It has an identity, a permission set, and a state machine.

use std::collections::HashSet;
use chrono::Utc;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    /// Agent is registered but not yet started.
    Idle,
    /// Agent is actively running.
    Running,
    /// Agent is paused — can be resumed.
    Paused,
    /// Agent has finished cleanly.
    Completed,
    /// Agent was stopped due to error or policy violation.
    Terminated { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentPermission {
    /// Can read files.
    ReadFiles,
    /// Can write files.
    WriteFiles,
    /// Can run shell commands.
    RunCommands,
    /// Can make network requests.
    Network,
    /// Can read from shared memory.
    SharedMemoryRead,
    /// Can write to shared memory.
    SharedMemoryWrite,
    /// Can send messages to other agents.
    Messaging,
    /// Can spawn child agents.
    SpawnAgents,
}

#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique agent ID.
    pub id:           String,
    /// Human-readable name.
    pub name:         String,
    /// Version string.
    pub version:      String,
    /// What this agent is allowed to do.
    pub permissions:  HashSet<AgentPermission>,
    /// Current lifecycle state.
    pub state:        AgentState,
    /// When this agent was registered.
    pub registered_at: String,
    /// Trust score (0.0 = untrusted, 10.0 = fully trusted).
    pub trust_score:  f64,
    /// Agent-specific metadata (key/value).
    pub metadata:     std::collections::HashMap<String, String>,
}

impl Agent {
    pub fn new(id: &str, name: &str, version: &str, permissions: Vec<AgentPermission>) -> Self {
        Self {
            id:            id.to_string(),
            name:          name.to_string(),
            version:       version.to_string(),
            permissions:   permissions.into_iter().collect(),
            state:         AgentState::Idle,
            registered_at: Utc::now().to_rfc3339(),
            trust_score:   5.0, // Start at mid-range
            metadata:      std::collections::HashMap::new(),
        }
    }

    pub fn has_permission(&self, perm: &AgentPermission) -> bool {
        self.permissions.contains(perm)
    }

    pub fn is_running(&self) -> bool {
        self.state == AgentState::Running
    }

    pub fn start(&mut self) -> Result<(), String> {
        match self.state {
            AgentState::Idle | AgentState::Paused => {
                self.state = AgentState::Running;
                Ok(())
            }
            _ => Err(format!("cannot start agent in state {:?}", self.state)),
        }
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if self.state == AgentState::Running {
            self.state = AgentState::Paused;
            Ok(())
        } else {
            Err(format!("cannot pause agent in state {:?}", self.state))
        }
    }

    pub fn terminate(&mut self, reason: &str) {
        self.state = AgentState::Terminated { reason: reason.to_string() };
    }

    pub fn complete(&mut self) {
        self.state = AgentState::Completed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_agent() -> Agent {
        Agent::new("test-1", "TestAgent", "0.1.0", vec![
            AgentPermission::ReadFiles,
            AgentPermission::RunCommands,
        ])
    }

    #[test]
    fn new_agent_starts_idle() {
        let agent = basic_agent();
        assert_eq!(agent.state, AgentState::Idle);
    }

    #[test]
    fn agent_has_correct_permissions() {
        let agent = basic_agent();
        assert!(agent.has_permission(&AgentPermission::ReadFiles));
        assert!(!agent.has_permission(&AgentPermission::Network));
    }

    #[test]
    fn agent_lifecycle() {
        let mut agent = basic_agent();
        agent.start().unwrap();
        assert_eq!(agent.state, AgentState::Running);
        agent.pause().unwrap();
        assert_eq!(agent.state, AgentState::Paused);
        agent.start().unwrap();
        agent.complete();
        assert_eq!(agent.state, AgentState::Completed);
    }

    #[test]
    fn cannot_start_completed_agent() {
        let mut agent = basic_agent();
        agent.start().unwrap();
        agent.complete();
        assert!(agent.start().is_err());
    }

    #[test]
    fn terminate_sets_reason() {
        let mut agent = basic_agent();
        agent.start().unwrap();
        agent.terminate("policy violation");
        assert!(matches!(agent.state, AgentState::Terminated { .. }));
    }
}
