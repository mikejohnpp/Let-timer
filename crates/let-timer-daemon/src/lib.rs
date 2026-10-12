use chrono::{DateTime, Utc};
use rusqlite::Connection;

use let_timer_core::{
    ListQuery, MediaListRepository, NewMediaList, NewTask, NewWorkspace, Response,
    SettingsRepository, Task, TaskRepository, TaskService, UpdateTask, WorkspaceRepository,
    WorkspaceScope,
};

fn resolve_workspace_id(conn: &Connection, scope: &WorkspaceScope) -> Result<Option<i64>, String> {
    match scope {
        WorkspaceScope::Active => {
            let settings = SettingsRepository::new(conn);
            settings
                .active_workspace_id()
                .map(Some)
                .map_err(|e| e.to_string())
        }
        WorkspaceScope::Id(id) => Ok(Some(*id)),
        WorkspaceScope::All => Ok(None),
    }
}

pub fn create_task(conn: &Connection, mut task: NewTask) -> Response {
    if task.workspace_id == 0 {
        match SettingsRepository::new(conn).active_workspace_id() {
            Ok(id) => task.workspace_id = id,
            Err(e) => {
                return Response::Error {
                    message: e.to_string(),
                };
            }
        }
    }

    let repo = TaskRepository::new(conn);
    match repo.create(&task) {
        Ok(created) => Response::Ok(Some(created)),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn delete_task(conn: &Connection, id: i64) -> Response {
    let service = TaskService::new(conn);
    match service.delete(id) {
        Ok(()) => Response::OkEmpty,
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn edit_task(conn: &Connection, id: i64, update: UpdateTask, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.edit(id, update, now) {
        Ok(task) => Response::Ok(Some(task)),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn find_tasks(conn: &Connection, query: &str, scope: &WorkspaceScope) -> Response {
    let ws_id = match resolve_workspace_id(conn, scope) {
        Ok(id) => id,
        Err(e) => return Response::Error { message: e },
    };

    let repo = TaskRepository::new(conn);
    match repo.query(ws_id, None, None, None, Some(query), None) {
        Ok(tasks) => Response::OkList(tasks),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn list_tasks(conn: &Connection, query: &ListQuery) -> Response {
    let ws_id = match resolve_workspace_id(conn, &query.workspace) {
        Ok(id) => id,
        Err(e) => return Response::Error { message: e },
    };

    let repo = TaskRepository::new(conn);
    match repo.query(
        ws_id,
        query.status,
        query.priority,
        query.date,
        None,
        query.sort_priority,
    ) {
        Ok(tasks) => Response::OkList(tasks),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn current_task(conn: &Connection, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.current(now) {
        Ok(Some(snap)) => Response::Timer(snap),
        Ok(None) => Response::Ok(None),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn start_task(
    conn: &Connection,
    id: Option<i64>,
    next: bool,
    switch: bool,
    now: DateTime<Utc>,
) -> Response {
    let target_id = match id {
        Some(id) => id,
        None => {
            if !next {
                return Response::Error {
                    message: "specify a task ID or use --next".to_string(),
                };
            }
            // Resolve next candidate task from active workspace
            let ws_id = match SettingsRepository::new(conn).active_workspace_id() {
                Ok(id) => id,
                Err(e) => {
                    return Response::Error {
                        message: e.to_string(),
                    };
                }
            };

            let today = now.date_naive();
            let repo = TaskRepository::new(conn);
            let candidates = match repo.query(Some(ws_id), None, None, None, None, None) {
                Ok(tasks) => tasks,
                Err(e) => {
                    return Response::Error {
                        message: e.to_string(),
                    };
                }
            };

            let mut eligible: Vec<Task> = candidates
                .into_iter()
                .filter(|t| {
                    t.status == let_timer_core::TaskStatus::Pending
                        || t.status == let_timer_core::TaskStatus::Paused
                })
                .collect();

            // Sort: scheduled today first, priority desc, oldest first
            eligible.sort_by(|a, b| {
                let a_today = a.scheduled_on == Some(today);
                let b_today = b.scheduled_on == Some(today);
                b_today
                    .cmp(&a_today)
                    .then_with(|| b.priority.to_i64().cmp(&a.priority.to_i64()))
                    .then_with(|| a.created_at.cmp(&b.created_at))
            });

            match eligible.first() {
                Some(task) => task.id,
                None => {
                    return Response::Error {
                        message: "no pending or paused tasks found in active workspace".to_string(),
                    };
                }
            }
        }
    };

    let service = TaskService::new(conn);
    match service.start(target_id, switch, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn pause_task(conn: &Connection, id: Option<i64>, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.pause(id, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn resume_task(conn: &Connection, id: Option<i64>, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.resume(id, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn stop_task(conn: &Connection, id: Option<i64>, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.stop(id, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn restart_task(conn: &Connection, id: Option<i64>, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.restart(id, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn done_task(conn: &Connection, id: Option<i64>, force: bool, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.done(id, force, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn cancel_task(conn: &Connection, id: Option<i64>, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.cancel(id, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn reopen_task(conn: &Connection, id: i64, now: DateTime<Utc>) -> Response {
    let service = TaskService::new(conn);
    match service.reopen(id, now) {
        Ok(snap) => Response::Timer(snap),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn list_workspaces(conn: &Connection) -> Response {
    let repo = WorkspaceRepository::new(conn);
    match repo.list_all() {
        Ok(list) => Response::WorkspaceList(list),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn create_workspace(conn: &Connection, new_ws: &NewWorkspace) -> Response {
    let repo = WorkspaceRepository::new(conn);
    match repo.create(new_ws) {
        Ok(ws) => Response::Workspace(ws),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn list_media_lists(conn: &Connection) -> Response {
    let repo = MediaListRepository::new(conn);
    match repo.list_all() {
        Ok(lists) => Response::MediaLists(lists),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn create_media_list(conn: &Connection, new_ml: &NewMediaList) -> Response {
    let repo = MediaListRepository::new(conn);
    match repo.create(new_ml) {
        Ok(ml) => Response::MediaList(ml),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn handle_command(
    cmd: let_timer_core::Command,
    conn: &Connection,
    now: DateTime<Utc>,
) -> Response {
    use let_timer_core::Command;
    match cmd {
        Command::Create(new_task) => create_task(conn, new_task),
        Command::Delete { id } => delete_task(conn, id),
        Command::Edit { id, update } => edit_task(conn, id, update, now),
        Command::Find { query, workspace } => find_tasks(conn, &query, &workspace),
        Command::List(query) => list_tasks(conn, &query),
        Command::Current => current_task(conn, now),
        Command::Start { id, next, switch } => start_task(conn, id, next, switch, now),
        Command::Pause { id } => pause_task(conn, id, now),
        Command::Resume { id } => resume_task(conn, id, now),
        Command::Stop { id } => stop_task(conn, id, now),
        Command::Restart { id } => restart_task(conn, id, now),
        Command::Done { id, force } => done_task(conn, id, force, now),
        Command::Cancel { id } => cancel_task(conn, id, now),
        Command::Reopen { id } => reopen_task(conn, id, now),
        Command::Ping => Response::Pong,
        Command::ListWorkspaces => list_workspaces(conn),
        Command::CreateWorkspace(new_workspace) => create_workspace(conn, &new_workspace),
        Command::ListMediaLists => list_media_lists(conn),
        Command::CreateMediaList(new_media_list) => create_media_list(conn, &new_media_list),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use let_timer_core::{
        Command, Database, IpcClient, IpcServer, ListQuery, NewTask, Priority, Response, SortOrder,
        TaskStatus, TimerState, UpdateTask, WorkspaceScope,
    };
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn e2e_lifecycle_over_ipc() {
        let dir = std::env::temp_dir().join(format!("test-ipc-e2e-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let socket_path = dir.join("test.sock");
        let _ = std::fs::remove_file(&socket_path);
        let socket_str = socket_path.to_str().unwrap().to_string();

        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let db_clone = Arc::clone(&db);

        let server = IpcServer::new(&socket_str, move |cmd| {
            let now = Utc::now();
            let db_guard = db_clone.lock().unwrap();
            handle_command(cmd, db_guard.conn(), now)
        })
        .await
        .unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = server.run().await;
        });

        let mut client = IpcClient::connect(&socket_path).await.unwrap();

        // 1. Ping
        let res = client.request(&Command::Ping).await.unwrap();
        assert!(matches!(res, Response::Pong));

        // 2. Create tasks
        let res = client
            .request(&Command::Create(NewTask {
                workspace_id: 1,
                media_list_id: None,
                name: "Task A".to_string(),
                description: None,
                priority: Priority::Urgent,
                estimated_mins: Some(25),
                scheduled_on: None,
            }))
            .await
            .unwrap();

        let task_a = match res {
            Response::Ok(Some(t)) => t,
            other => panic!("expected Ok(Some(task)), got {other:?}"),
        };

        let res = client
            .request(&Command::Create(NewTask {
                workspace_id: 1,
                media_list_id: None,
                name: "Task B".to_string(),
                description: None,
                priority: Priority::Immediate,
                estimated_mins: Some(15),
                scheduled_on: None,
            }))
            .await
            .unwrap();

        let task_b = match res {
            Response::Ok(Some(t)) => t,
            other => panic!("expected Ok(Some(task)), got {other:?}"),
        };

        // 3. Start task A
        let res = client
            .request(&Command::Start {
                id: Some(task_a.id),
                next: false,
                switch: false,
            })
            .await
            .unwrap();
        match res {
            Response::Timer(snap) => {
                assert_eq!(snap.task.id, task_a.id);
                assert_eq!(snap.state, TimerState::Running);
            }
            other => panic!("expected Timer, got {other:?}"),
        }

        // 4. Start task B without switch -> errors
        let res = client
            .request(&Command::Start {
                id: Some(task_b.id),
                next: false,
                switch: false,
            })
            .await
            .unwrap();
        match res {
            Response::Error { message } => assert!(message.contains("is already running")),
            other => panic!("expected Error, got {other:?}"),
        }

        // 5. Start task B with switch -> pauses A, starts B
        let res = client
            .request(&Command::Start {
                id: Some(task_b.id),
                next: false,
                switch: true,
            })
            .await
            .unwrap();
        match res {
            Response::Timer(snap) => {
                assert_eq!(snap.task.id, task_b.id);
                assert_eq!(snap.state, TimerState::Running);
            }
            other => panic!("expected Timer, got {other:?}"),
        }

        // 6. Pause task B
        let res = client.request(&Command::Pause { id: None }).await.unwrap();
        match res {
            Response::Timer(snap) => {
                assert_eq!(snap.task.id, task_b.id);
                assert_eq!(snap.state, TimerState::Paused);
            }
            other => panic!("expected Timer, got {other:?}"),
        }

        // 7. Resume task B
        let res = client.request(&Command::Resume { id: None }).await.unwrap();
        match res {
            Response::Timer(snap) => {
                assert_eq!(snap.task.id, task_b.id);
                assert_eq!(snap.state, TimerState::Running);
            }
            other => panic!("expected Timer, got {other:?}"),
        }

        // 8. Done task B
        let res = client
            .request(&Command::Done {
                id: None,
                force: false,
            })
            .await
            .unwrap();
        match res {
            Response::Timer(snap) => {
                assert_eq!(snap.task.id, task_b.id);
                assert_eq!(snap.task.status, TaskStatus::Completed);
            }
            other => panic!("expected Timer, got {other:?}"),
        }

        // 9. Done on idle (task A is paused, task B completed, no task running)
        let res = client
            .request(&Command::Done {
                id: None,
                force: false,
            })
            .await
            .unwrap();
        match res {
            Response::Error { message } => {
                assert!(message.contains("no task is currently running"))
            }
            other => panic!("expected Error, got {other:?}"),
        }

        // 10. List combined: status + sort
        let res = client
            .request(&Command::List(ListQuery {
                workspace: WorkspaceScope::All,
                status: Some(TaskStatus::Completed),
                sort_priority: Some(SortOrder::Descending),
                priority: None,
                date: None,
            }))
            .await
            .unwrap();
        match res {
            Response::OkList(tasks) => {
                assert_eq!(tasks.len(), 1);
                assert_eq!(tasks[0].id, task_b.id);
            }
            other => panic!("expected OkList, got {other:?}"),
        }

        // 11. Edit status directly is rejected
        let res = client
            .request(&Command::Edit {
                id: task_a.id,
                update: UpdateTask {
                    status: Some(TaskStatus::Completed),
                    ..Default::default()
                },
            })
            .await
            .unwrap();
        match res {
            Response::Error { message } => {
                assert!(message.contains("cannot change status with `edit`"))
            }
            other => panic!("expected Error, got {other:?}"),
        }

        server_handle.abort();
    }
}
