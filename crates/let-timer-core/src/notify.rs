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
            tracing::warn!("Failed to spawn notify-send: {e}");
        }
    }
}

/// Simple logger notifier for terminal or testing.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn notify(&self, title: &str, body: &str) {
        tracing::info!("[NOTIFICATION] {title}: {body}");
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

/// Composite notifier that dispatches to multiple child notifiers safely.
pub struct CompositeNotifier {
    notifiers: Vec<Arc<dyn Notifier>>,
}

impl CompositeNotifier {
    pub fn new(notifiers: Vec<Arc<dyn Notifier>>) -> Self {
        Self { notifiers }
    }

    /// Build a composite notifier from comma-separated list of names (e.g. "desktop,sse").
    pub fn from_env_or_default(default_desktop: bool) -> Self {
        let env_val = std::env::var("LET_TIMER_NOTIFY").ok();
        let default_val = if default_desktop { "desktop" } else { "none" };
        let names = env_val.as_deref().unwrap_or(default_val);

        let mut list: Vec<Arc<dyn Notifier>> = Vec::new();
        for name in names.split(',').map(|s| s.trim()) {
            match name {
                "desktop" => list.push(Arc::new(NotifySend)),
                "log" => list.push(Arc::new(LogNotifier)),
                // ntfy, webhook, sse will be wired in S4
                _ => {}
            }
        }
        Self::new(list)
    }
}

impl Notifier for CompositeNotifier {
    fn notify(&self, title: &str, body: &str) {
        for notifier in &self.notifiers {
            let n = Arc::clone(notifier);
            let t = title.to_string();
            let b = body.to_string();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                n.notify(&t, &b);
            }));
        }
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

    struct PanickingNotifier;
    impl Notifier for PanickingNotifier {
        fn notify(&self, _title: &str, _body: &str) {
            panic!("intended notifier panic");
        }
    }

    #[test]
    fn composite_calls_all_children_and_ignores_panics() {
        let rec1 = Arc::new(RecordingNotifier::new());
        let panic_notifier = Arc::new(PanickingNotifier);
        let rec2 = Arc::new(RecordingNotifier::new());

        let composite = CompositeNotifier::new(vec![
            Arc::clone(&rec1) as Arc<dyn Notifier>,
            panic_notifier as Arc<dyn Notifier>,
            Arc::clone(&rec2) as Arc<dyn Notifier>,
        ]);

        composite.notify("Title", "Body");

        assert_eq!(rec1.count(), 1);
        assert_eq!(rec2.count(), 1);
    }
}
