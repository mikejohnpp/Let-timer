use std::sync::{Arc, Mutex};

use let_timer_core::{
    Command, Database, IpcServer, MediaListRepository, Response, TaskRepository,
    WorkspaceRepository,
};
use let_timer_daemon::{create_commands, delete_task, find_by_name, get_list_task};

fn invoke_command(cmd: Command, db: &Arc<Mutex<Database>>) -> Response {
    let db_guard = db.lock().unwrap_or_else(|e| e.into_inner());
    let repo = TaskRepository::new(db_guard.conn());
    match cmd {
        Command::Create(new_task) => create_commands(&repo, &new_task),
        Command::Delete { id } => delete_task(&repo, id),
        Command::Edit { id, update } => match repo.update(id, &update) {
            Ok(task) => Response::Ok(Some(task)),
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        },
        Command::Find { query } => find_by_name(&repo, &query),
        Command::List {
            sort_priority,
            filter_status,
        } => get_list_task(&repo, filter_status, sort_priority),
        Command::Current => Response::Error {
            message: "not implemented yet".to_string(),
        },
        Command::Start { .. } => Response::Error {
            message: "not implemented yet".to_string(),
        },
        Command::Stop => Response::Error {
            message: "not implemented yet".to_string(),
        },
        Command::Done => Response::Error {
            message: "not implemented yet".to_string(),
        },
        Command::ListWorkspaces => {
            let workspaces = WorkspaceRepository::new(db_guard.conn()).list_all();
            match workspaces {
                Ok(list) => Response::WorkspaceList(list),
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Command::CreateWorkspace(new_workspace) => {
            let created = WorkspaceRepository::new(db_guard.conn()).create(&new_workspace);
            match created {
                Ok(workspace) => Response::Workspace(workspace),
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Command::ListMediaLists => {
            let lists = MediaListRepository::new(db_guard.conn()).list_all();
            match lists {
                Ok(list) => Response::MediaLists(list),
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Command::CreateMediaList(new_media_list) => {
            let created = MediaListRepository::new(db_guard.conn()).create(&new_media_list);
            match created {
                Ok(media_list) => Response::MediaList(media_list),
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
    }
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
