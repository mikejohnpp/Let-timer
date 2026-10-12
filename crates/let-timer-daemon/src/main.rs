use std::sync::{Arc, Mutex};

use chrono::Utc;
use let_timer_core::{Command, Database, IpcServer, Response};
use let_timer_daemon::handle_command;

fn invoke_command(cmd: Command, db: &Arc<Mutex<Database>>) -> Response {
    let now = Utc::now();
    let db_guard = db.lock().unwrap_or_else(|e| e.into_inner());
    handle_command(cmd, db_guard.conn(), now)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = let_timer_core::ipc::socket_path();
    let path = path.to_string_lossy().into_owned();
    let database = Arc::new(Mutex::new(Database::open_default()?));
    let server = IpcServer::new(&path, move |cmd| invoke_command(cmd, &database)).await?;
    server.run().await?;
    println!("daemon exited cleanly");
    Ok(())
}
