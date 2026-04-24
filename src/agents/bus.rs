// cash — Message Bus
//
// Agent-to-agent messaging via an in-process channel bus.
// Unix domain socket support is the next step (Module 12).
// For now: in-process channels so agents in the same session can message.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};

#[derive(Debug, Clone)]
pub struct Message {
    pub from:    String,
    pub to:      String,
    pub topic:   String,
    pub payload: String,
}

impl Message {
    pub fn new(from: &str, to: &str, topic: &str, payload: &str) -> Self {
        Self {
            from:    from.to_string(),
            to:      to.to_string(),
            topic:   topic.to_string(),
            payload: payload.to_string(),
        }
    }
}

pub struct MessageBus {
    senders: Arc<Mutex<HashMap<String, Sender<Message>>>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribe an agent to the bus. Returns a Receiver for incoming messages.
    pub fn subscribe(&self, agent_id: &str) -> Receiver<Message> {
        let (tx, rx) = mpsc::channel();
        self.senders.lock().unwrap().insert(agent_id.to_string(), tx);
        rx
    }

    /// Send a message to a specific agent.
    pub fn send(&self, msg: Message) -> Result<(), String> {
        let senders = self.senders.lock().unwrap();
        match senders.get(&msg.to) {
            Some(tx) => tx.send(msg).map_err(|e| e.to_string()),
            None => Err(format!("agent '{}' not subscribed", msg.to)),
        }
    }

    /// Unsubscribe an agent.
    pub fn unsubscribe(&self, agent_id: &str) {
        self.senders.lock().unwrap().remove(agent_id);
    }

    pub fn subscriber_count(&self) -> usize {
        self.senders.lock().unwrap().len()
    }
}

impl Default for MessageBus {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn send_and_receive() {
        let bus = MessageBus::new();
        let rx = bus.subscribe("agent-b");
        bus.send(Message::new("agent-a", "agent-b", "hello", "world")).unwrap();
        let msg = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(msg.from, "agent-a");
        assert_eq!(msg.payload, "world");
    }

    #[test]
    fn send_to_unsubscribed_fails() {
        let bus = MessageBus::new();
        let result = bus.send(Message::new("a", "nobody", "test", "data"));
        assert!(result.is_err());
    }

    #[test]
    fn unsubscribe_removes_agent() {
        let bus = MessageBus::new();
        bus.subscribe("agent-x");
        assert_eq!(bus.subscriber_count(), 1);
        bus.unsubscribe("agent-x");
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn multiple_agents_receive_own_messages() {
        let bus = MessageBus::new();
        let rx_a = bus.subscribe("agent-a");
        let rx_b = bus.subscribe("agent-b");
        bus.send(Message::new("sys", "agent-a", "ping", "for-a")).unwrap();
        bus.send(Message::new("sys", "agent-b", "ping", "for-b")).unwrap();
        assert_eq!(rx_a.recv_timeout(Duration::from_millis(100)).unwrap().payload, "for-a");
        assert_eq!(rx_b.recv_timeout(Duration::from_millis(100)).unwrap().payload, "for-b");
    }
}
