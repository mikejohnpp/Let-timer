use std::sync::{Arc, Mutex};

use let_timer_core::{Command, Database, IpcServer, Response, TaskRepository};

fn invoke_command(cmd: Command, db: &Arc<Mutex<Database>>) -> Response {
    let _db = db.lock().unwrap();
    let _repo = TaskRepository::new(_db.conn());
    match cmd {
        Command::Start => Response {
            data: "Start command received".to_string(),
        },
        Command::Stop => Response {
            data: "Stop command received".to_string(),
        },
        Command::End => Response {
            data: "End command received".to_string(),
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "/tmp/let-timer.sock";
    let database = Arc::new(Mutex::new(
        Database::open_default().expect("Failed to open database"),
    ));
    let server = IpcServer::new(path, move |cmd| invoke_command(cmd, &database)).await?;
    server.run().await?;
    println!("daemon exited cleanly");
    Ok(())
}
