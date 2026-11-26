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

/// Ntfy HTTP notifier: POSTs notification to ntfy topic.
#[derive(Debug, Clone)]
pub struct NtfyNotifier {
    pub url: String,
}

impl NtfyNotifier {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl Notifier for NtfyNotifier {
    fn notify(&self, title: &str, body: &str) {
        let url = self.url.clone();
        let title = title.to_string();
        let body = body.to_string();

        tokio::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to build reqwest client for Ntfy: {e}");
                    return;
                }
            };

            let res = client
                .post(&url)
                .header("Title", title)
                .header("Priority", "urgent")
                .body(body)
                .send()
                .await;

            if let Err(e) = res {
                tracing::warn!("Failed to send Ntfy notification to {url}: {e}");
            }
        });
    }
}

/// Generic Webhook notifier: POSTs JSON payload to target URL.
#[derive(Debug, Clone)]
pub struct WebhookNotifier {
    pub url: String,
}

impl WebhookNotifier {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl Notifier for WebhookNotifier {
    fn notify(&self, title: &str, body: &str) {
        let url = self.url.clone();
        let payload = serde_json::json!({
            "event": "timer_expired",
            "title": title,
            "body": body,
            "at": chrono::Utc::now().to_rfc3339(),
        });

        tokio::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to build reqwest client for Webhook: {e}");
                    return;
                }
            };

            let res = client.post(&url).json(&payload).send().await;
            if let Err(e) = res {
                tracing::warn!("Failed to send Webhook notification to {url}: {e}");
            }
        });
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

    /// Build a composite notifier from comma-separated list of names (e.g. "desktop,ntfy,webhook").
    pub fn from_env_or_default(default_desktop: bool) -> Self {
        let env_val = std::env::var("LET_TIMER_NOTIFY").ok();
        let default_val = if default_desktop { "desktop" } else { "none" };
        let names = env_val.as_deref().unwrap_or(default_val);

        let mut list: Vec<Arc<dyn Notifier>> = Vec::new();
        for name in names.split(',').map(|s| s.trim()) {
            match name {
                "desktop" => list.push(Arc::new(NotifySend)),
                "log" => list.push(Arc::new(LogNotifier)),
                "ntfy" => {
                    if let Ok(url) = std::env::var("LET_TIMER_NTFY_URL") {
                        list.push(Arc::new(NtfyNotifier::new(url)));
                    } else {
                        tracing::warn!(
                            "LET_TIMER_NOTIFY contains 'ntfy' but LET_TIMER_NTFY_URL is unset"
                        );
                    }
                }
                "webhook" => {
                    if let Ok(url) = std::env::var("LET_TIMER_WEBHOOK_URL") {
                        list.push(Arc::new(WebhookNotifier::new(url)));
                    } else {
                        tracing::warn!(
                            "LET_TIMER_NOTIFY contains 'webhook' but LET_TIMER_WEBHOOK_URL is unset"
                        );
                    }
                }
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

    #[tokio::test]
    async fn ntfy_notifier_sends_http_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}/test-topic");

        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
                .await
                .unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
            tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes())
                .await
                .unwrap();
            req_str.to_string()
        });

        let notifier = NtfyNotifier::new(url);
        notifier.notify("Test Alert", "Time has run out");

        let req_content = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .unwrap()
            .unwrap();

        assert!(
            req_content.contains("Title: Test Alert") || req_content.contains("title: Test Alert")
        );
        assert!(
            req_content.contains("Priority: urgent") || req_content.contains("priority: urgent")
        );
        assert!(req_content.contains("Time has run out"));
    }
}
