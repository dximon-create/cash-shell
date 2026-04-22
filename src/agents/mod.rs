// cash — Layer 3: Agent Platform
//
// Owns: native agent runtime, shared memory, Unix domain socket
//       messaging, agent marketplace, token-efficient execution.
//
// Agents use CLI tools to act. AI APIs only to think.

pub struct AgentPlatform;

impl AgentPlatform {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
}
