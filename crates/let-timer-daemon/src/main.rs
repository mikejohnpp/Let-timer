use std::sync::{Arc, Mutex};

use chrono::Utc;
use let_timer_core::{Command, Database, IpcServer, NotifySend, Response};
use let_timer_daemon::{Scheduler, handle_command};
use tokio::sync::Notify;

fn invoke_command(cmd: Command, db: &Arc<Mutex<Database>>, notify: &Arc<Notify>) -> Response {
    let now = Utc::now();
    let db_guard = db.lock().unwrap_or_else(|e| e.into_inner());
    handle_command(cmd, db_guard.conn(), now, Some(notify))
}

fn init_logging() {
    let filter = std::env::var("LET_TIMER_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "info".to_string());
    let filter_layer = tracing_subscriber::EnvFilter::new(filter);

    let format_json = std::env::var("LET_TIMER_LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    if format_json {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter_layer)
            .json()
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter_layer)
            .try_init();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let path = let_timer_core::ipc::socket_path();
    let path = path.to_string_lossy().into_owned();
    let database = Arc::new(Mutex::new(Database::open_default()?));
    let notify_signal = Arc::new(Notify::new());
    let notifier = Arc::new(NotifySend);

    let scheduler = Scheduler::new(Arc::clone(&database), notifier, Arc::clone(&notify_signal));
    let scheduler_handle = tokio::spawn(async move {
        scheduler.run().await;
    });

    let db_clone = Arc::clone(&database);
    let notify_clone = Arc::clone(&notify_signal);
    let server = IpcServer::new(&path, move |cmd| {
        invoke_command(cmd, &db_clone, &notify_clone)
    })
    .await?;

    server.run().await?;
    scheduler_handle.abort();
    tracing::info!("daemon exited cleanly");
    Ok(())
}
