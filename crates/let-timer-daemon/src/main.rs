use std::sync::{Arc, Mutex};

use let_timer_core::{Command, Database, IpcServer, Response, TaskPayload, TaskRepository};

fn invoke_command(cmd: Command, db: &Arc<Mutex<Database>>) -> Response {
    let _db = db.lock().unwrap();
    let _repo = TaskRepository::new(_db.conn());
    match cmd {
        Command::Create(new_task) => {
            println!("Creating task: {:?}", new_task);
            Response::OkEmpty
        }
        Command::Delete { id } => {
            println!("Deleting task with id: {}", id);
            Response::OkEmpty
        }
        Command::Edit { id, update } => {
            println!("Editing task with id: {}, update: {:?}", id, update);
            Response::OkEmpty
        }
        Command::Find { query } => {
            println!("Finding tasks with query: {}", query);
            Response::OkList(vec![])
        }
        Command::List {
            sort_priority,
            filter_status,
        } => {
            println!(
                "Listing tasks with sort_priority: {:?}, filter_status: {:?}",
                sort_priority, filter_status
            );
            Response::OkList(vec![])
        }
        Command::Current => {
            println!("Getting current task");
            Response::Ok(TaskPayload::None)
        }
        Command::Start { id } => {
            println!("Starting task with id: {:?}", id);
            Response::OkEmpty
        }
        Command::Stop => {
            println!("Stopping current task");
            Response::OkEmpty
        }
        Command::Done => {
            println!("Marking current task as done");
            Response::OkEmpty
        }
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
