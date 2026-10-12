use std::sync::{Arc, Mutex};

/// Trait for firing desktop or system notifications when task timer expires.
pub trait Notifier: Send + Sync {
    fn notify(&self, title: &str, body: &str);
}

/// Spawns `notify-send` on Linux desktops. Failures are logged and never panic.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotifySend;

impl Notifier for NotifySend {
    fn notify(&self, title: &str, body: &str) {
        if let Err(e) = std::process::Command::new("notify-send")
            .arg(title)
            .arg(body)
            .spawn()
        {
            eprintln!("Failed to spawn notify-send: {e}");
        }
    }
}

/// Simple logger notifier for terminal or testing.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn notify(&self, title: &str, body: &str) {
        println!("[NOTIFICATION] {title}: {body}");
    }
}

/// Recording notifier for unit and integration tests.
#[derive(Debug, Default, Clone)]
pub struct RecordingNotifier {
    records: Arc<Mutex<Vec<(String, String)>>>,
}

impl RecordingNotifier {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn notifications(&self) -> Vec<(String, String)> {
        self.records
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn count(&self) -> usize {
        self.records.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn clear(&self) {
        self.records
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}

impl Notifier for RecordingNotifier {
    fn notify(&self, title: &str, body: &str) {
        self.records
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((title.to_string(), body.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_notifier_records_events() {
        let notifier = RecordingNotifier::new();
        assert_eq!(notifier.count(), 0);

        notifier.notify("Time's up", "Task 1 (25 mins)");
        assert_eq!(notifier.count(), 1);
        assert_eq!(
            notifier.notifications(),
            vec![("Time's up".to_string(), "Task 1 (25 mins)".to_string())]
        );

        notifier.clear();
        assert_eq!(notifier.count(), 0);
    }
}
