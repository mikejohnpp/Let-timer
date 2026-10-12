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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    println!("daemon exited cleanly");
    Ok(())
}
