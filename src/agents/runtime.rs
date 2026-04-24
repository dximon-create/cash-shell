// cash — Agent Runtime
//
// Manages the lifecycle of all agents in a cash session.
// Enforces permission walls — agents cannot exceed their declared permissions.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::agent::{Agent, AgentPermission, AgentState};
use super::bus::{MessageBus, Message};
use super::memory::SharedMemory;

pub struct AgentRuntime {
    agents:  Arc<Mutex<HashMap<String, Agent>>>,
    bus:     Arc<MessageBus>,
    memory:  SharedMemory,
}

impl AgentRuntime {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            bus:    Arc::new(MessageBus::new()),
            memory: SharedMemory::new(),
        }
    }

    /// Register a new agent. Returns error if ID already exists.
    pub fn register(&self, agent: Agent) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        if agents.contains_key(&agent.id) {
            return Err(format!("agent '{}' already registered", agent.id));
        }
        agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    /// Start a registered agent.
    pub fn start(&self, agent_id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        agents.get_mut(agent_id)
            .ok_or_else(|| format!("agent '{}' not found", agent_id))?
            .start()
    }

    /// Pause a running agent.
    pub fn pause(&self, agent_id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        agents.get_mut(agent_id)
            .ok_or_else(|| format!("agent '{}' not found", agent_id))?
            .pause()
    }

    /// Terminate an agent.
    pub fn terminate(&self, agent_id: &str, reason: &str) {
        if let Some(agent) = self.agents.lock().unwrap().get_mut(agent_id) {
            agent.terminate(reason);
        }
    }

    /// Send a message from one agent to another.
    /// Enforces that the sender has Messaging permission.
    pub fn send_message(&self, msg: Message) -> Result<(), String> {
        let agents = self.agents.lock().unwrap();
        let sender = agents.get(&msg.from)
            .ok_or_else(|| format!("sender '{}' not found", msg.from))?;

        if !sender.has_permission(&AgentPermission::Messaging) {
            return Err(format!("agent '{}' lacks Messaging permission", msg.from));
        }
        if sender.state != AgentState::Running {
            return Err(format!("agent '{}' is not running", msg.from));
        }
        drop(agents);
        self.bus.send(msg)
    }

    /// Read from shared memory. Requires SharedMemoryRead permission.
    pub fn mem_read(&self, agent_id: &str, key: &str) -> Result<Option<String>, String> {
        let agents = self.agents.lock().unwrap();
        let agent = agents.get(agent_id)
            .ok_or_else(|| format!("agent '{}' not found", agent_id))?;
        if !agent.has_permission(&AgentPermission::SharedMemoryRead) {
            return Err(format!("agent '{}' lacks SharedMemoryRead permission", agent_id));
        }
        Ok(self.memory.get(key))
    }

    /// Write to shared memory. Requires SharedMemoryWrite permission.
    pub fn mem_write(&self, agent_id: &str, key: &str, value: &str) -> Result<(), String> {
        let agents = self.agents.lock().unwrap();
        let agent = agents.get(agent_id)
            .ok_or_else(|| format!("agent '{}' not found", agent_id))?;
        if !agent.has_permission(&AgentPermission::SharedMemoryWrite) {
            return Err(format!("agent '{}' lacks SharedMemoryWrite permission", agent_id));
        }
        drop(agents);
        self.memory.set(key, value);
        Ok(())
    }

    pub fn agent_count(&self) -> usize {
        self.agents.lock().unwrap().len()
    }

    pub fn running_count(&self) -> usize {
        self.agents.lock().unwrap()
            .values()
            .filter(|a| a.state == AgentState::Running)
            .count()
    }

    pub fn bus(&self) -> Arc<MessageBus> {
        self.bus.clone()
    }
}

impl Default for AgentRuntime {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::agent::{Agent, AgentPermission};

    fn make_agent(id: &str, perms: Vec<AgentPermission>) -> Agent {
        Agent::new(id, &format!("Agent {}", id), "0.1.0", perms)
    }

    #[test]
    fn register_and_start() {
        let rt = AgentRuntime::new();
        rt.register(make_agent("a1", vec![])).unwrap();
        rt.start("a1").unwrap();
        assert_eq!(rt.running_count(), 1);
    }

    #[test]
    fn duplicate_registration_fails() {
        let rt = AgentRuntime::new();
        rt.register(make_agent("a1", vec![])).unwrap();
        assert!(rt.register(make_agent("a1", vec![])).is_err());
    }

    #[test]
    fn messaging_requires_permission() {
        let rt = AgentRuntime::new();
        let mut a1 = make_agent("a1", vec![]);
        a1.start().unwrap();
        rt.register(a1).unwrap();
        let msg = Message::new("a1", "a2", "test", "data");
        assert!(rt.send_message(msg).is_err());
    }

    #[test]
    fn shared_memory_permission_wall() {
        let rt = AgentRuntime::new();
        rt.register(make_agent("a1", vec![])).unwrap();
        assert!(rt.mem_read("a1", "key").is_err());
        assert!(rt.mem_write("a1", "key", "val").is_err());
    }

    #[test]
    fn shared_memory_with_permission() {
        let rt = AgentRuntime::new();
        rt.register(make_agent("a1", vec![
            AgentPermission::SharedMemoryRead,
            AgentPermission::SharedMemoryWrite,
        ])).unwrap();
        rt.mem_write("a1", "x", "42").unwrap();
        assert_eq!(rt.mem_read("a1", "x").unwrap(), Some("42".to_string()));
    }

    #[test]
    fn terminate_agent() {
        let rt = AgentRuntime::new();
        rt.register(make_agent("a1", vec![])).unwrap();
        rt.start("a1").unwrap();
        rt.terminate("a1", "test");
        assert_eq!(rt.running_count(), 0);
    }
}
