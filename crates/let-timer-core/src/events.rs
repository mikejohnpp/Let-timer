use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::db::models::{Task, Workspace};
use crate::service::TimerSnapshot;

/// System events broadcast across daemon scheduler, HTTP SSE, and internal subscribers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    TimerChanged(TimerSnapshot),
    TimerExpired { task: Task, estimate_mins: i64 },
    WorkspaceChanged(Workspace),
}

/// Broadcast event bus for distributing real-time domain events.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn event_bus_publishes_and_subscribes() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe();

        let ws = Workspace {
            id: 1,
            name: "Work".to_string(),
            description: None,
            created_at: "2026-10-12T08:00:00Z".to_string(),
            updated_at: "2026-10-12T08:00:00Z".to_string(),
        };

        bus.publish(Event::WorkspaceChanged(ws.clone()));
        let received = sub.recv().await.unwrap();
        assert_eq!(received, Event::WorkspaceChanged(ws));
    }
}
