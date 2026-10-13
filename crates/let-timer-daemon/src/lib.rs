use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use let_timer_core::{
    ListQuery, MediaListRepository, MediaRepository, MediaType, NewMedia, NewMediaList, NewTask,
    NewWorkspace, Response, SettingsRepository, Task, TaskRepository, TaskService, UpdateTask,
    WorkspaceRepository, WorkspaceScope,
};

fn resolve_workspace_id(
    conn: &Connection,
    scope: &WorkspaceScope,
    override_scope: Option<&WorkspaceScope>,
) -> Result<Option<i64>, String> {
    let effective_scope = match scope {
        WorkspaceScope::Active => override_scope.unwrap_or(&WorkspaceScope::Active),
        other => other,
    };

    match effective_scope {
        WorkspaceScope::Active => {
            let settings = SettingsRepository::new(conn);
            settings
                .active_workspace_id()
                .map(Some)
                .map_err(|e| e.to_string())
        }
        WorkspaceScope::Id(id) => Ok(Some(*id)),
        WorkspaceScope::All => Ok(None),
        WorkspaceScope::Name(name) => {
            let repo = WorkspaceRepository::new(conn);
            if let Ok(Some(ws)) = repo.find_exact_name(name) {
                Ok(Some(ws.id))
            } else {
                match repo.find_by_name(name) {
                    Ok(mut matches) if matches.len() == 1 => Ok(Some(matches.pop().unwrap().id)),
                    Ok(matches) if matches.is_empty() => {
                        Err(format!("workspace '{name}' not found"))
                    }
                    Ok(matches) => Err(format!(
                        "ambiguous workspace name '{name}': matches {} workspaces",
                        matches.len()
                    )),
                    Err(e) => Err(e.to_string()),
                }
            }
        }
    }
}

pub fn create_task(
    conn: &Connection,
    mut task: NewTask,
    override_scope: Option<&WorkspaceScope>,
) -> Response {
    if task.workspace_id == 0 {
        match resolve_workspace_id(conn, &WorkspaceScope::Active, override_scope) {
            Ok(Some(id)) => task.workspace_id = id,
            Ok(None) => match SettingsRepository::new(conn).active_workspace_id() {
                Ok(id) => task.workspace_id = id,
                Err(e) => {
                    return Response::Error {
                        message: e.to_string(),
                    };
                }
            },
            Err(e) => {
                return Response::Error { message: e };
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

pub fn find_tasks(
    conn: &Connection,
    query: &str,
    scope: &WorkspaceScope,
    override_scope: Option<&WorkspaceScope>,
) -> Response {
    let ws_id = match resolve_workspace_id(conn, scope, override_scope) {
        Ok(id) => id,
        Err(e) => return Response::Error { message: e },
    };

    let repo = TaskRepository::new(conn);
    match repo.query(ws_id, None, None, None, Some(query), None, None) {
        Ok(tasks) => Response::OkList(tasks),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn list_tasks(
    conn: &Connection,
    query: &ListQuery,
    override_scope: Option<&WorkspaceScope>,
) -> Response {
    let ws_id = match resolve_workspace_id(conn, &query.workspace, override_scope) {
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
        query.overdue,
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

pub fn get_start_candidates(
    conn: &Connection,
    scope: &WorkspaceScope,
    ctx: &let_timer_core::RequestContext,
) -> Result<Vec<Task>, String> {
    let ws_id = resolve_workspace_id(conn, scope, ctx.workspace.as_ref())?;
    let today = ctx.today;
    let repo = TaskRepository::new(conn);
    let candidates = repo
        .query(ws_id, None, None, None, None, None, None)
        .map_err(|e| e.to_string())?;

    let mut eligible: Vec<Task> = candidates
        .into_iter()
        .filter(|t| {
            t.status == let_timer_core::TaskStatus::Pending
                || t.status == let_timer_core::TaskStatus::Paused
        })
        .collect();

    eligible.sort_by(|a, b| {
        let a_today = a.scheduled_on == Some(today);
        let b_today = b.scheduled_on == Some(today);
        b_today
            .cmp(&a_today)
            .then_with(|| b.priority.to_i64().cmp(&a.priority.to_i64()))
            .then_with(|| a.created_at.cmp(&b.created_at))
    });

    Ok(eligible)
}

pub fn start_candidates(
    conn: &Connection,
    scope: &WorkspaceScope,
    ctx: &let_timer_core::RequestContext,
) -> Response {
    match get_start_candidates(conn, scope, ctx) {
        Ok(tasks) => Response::OkList(tasks),
        Err(e) => Response::Error { message: e },
    }
}

pub fn start_task(
    conn: &Connection,
    id: Option<i64>,
    next: bool,
    switch: bool,
    ctx: &let_timer_core::RequestContext,
) -> Response {
    let target_id = match id {
        Some(id) => id,
        None => {
            if !next {
                return Response::Error {
                    message: "specify a task ID or use --next".to_string(),
                };
            }
            let candidates = match get_start_candidates(conn, &WorkspaceScope::Active, ctx) {
                Ok(c) => c,
                Err(e) => return Response::Error { message: e },
            };
            match candidates.first() {
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
    match service.start(target_id, switch, ctx.now) {
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

pub fn get_media_list(conn: &Connection, id: i64) -> Response {
    let repo = MediaListRepository::new(conn);
    let list = match repo.get_by_id(id) {
        Ok(l) => l,
        Err(e) => {
            return Response::Error {
                message: e.to_string(),
            };
        }
    };
    match repo.list_medias(id) {
        Ok(items) => Response::MediaListDetail { list, items },
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn delete_media_list(conn: &Connection, id: i64) -> Response {
    let repo = MediaListRepository::new(conn);
    match repo.delete(id) {
        Ok(()) => Response::OkEmpty,
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn add_media_to_list(conn: &Connection, list_id: i64, media_id: i64) -> Response {
    let repo = MediaListRepository::new(conn);
    match repo.add_media(list_id, media_id) {
        Ok(()) => Response::OkEmpty,
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn remove_media_from_list(conn: &Connection, list_id: i64, media_id: i64) -> Response {
    let repo = MediaListRepository::new(conn);
    match repo.remove_media(list_id, media_id) {
        Ok(()) => Response::OkEmpty,
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn list_medias(conn: &Connection, media_type: Option<MediaType>) -> Response {
    let repo = MediaRepository::new(conn);
    let res = match media_type {
        Some(t) => repo.list_by_type(t),
        None => repo.list_all(),
    };
    match res {
        Ok(medias) => Response::Medias(medias),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn create_media(conn: &Connection, new_media: &NewMedia) -> Response {
    let repo = MediaRepository::new(conn);
    match repo.create(new_media) {
        Ok(m) => Response::Media(m),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn delete_media(conn: &Connection, id: i64) -> Response {
    let repo = MediaRepository::new(conn);
    match repo.delete(id) {
        Ok(()) => Response::OkEmpty,
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn daemon_status(conn: &Connection) -> Response {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let socket_path = let_timer_core::ipc::socket_path().display().to_string();
    let db_path = let_timer_core::default_db_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let task_repo = TaskRepository::new(conn);
    let running_task = match task_repo.get_current() {
        Ok(Some(t)) => Some(format!("#{} {}", t.id, t.name)),
        _ => None,
    };
    Response::DaemonStatus {
        version,
        socket_path,
        db_path,
        running_task,
    }
}

pub fn current_workspace(conn: &Connection, override_scope: Option<&WorkspaceScope>) -> Response {
    let repo = WorkspaceRepository::new(conn);
    let ws_id = match resolve_workspace_id(conn, &WorkspaceScope::Active, override_scope) {
        Ok(Some(id)) => id,
        Ok(None) => match SettingsRepository::new(conn).active_workspace_id() {
            Ok(id) => id,
            Err(e) => {
                return Response::Error {
                    message: e.to_string(),
                };
            }
        },
        Err(e) => return Response::Error { message: e },
    };
    match repo.get_by_id(ws_id) {
        Ok(ws) => Response::CurrentWorkspace(ws),
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub fn switch_workspace(conn: &Connection, target: let_timer_core::WorkspaceTarget) -> Response {
    use let_timer_core::WorkspaceTarget;
    let repo = WorkspaceRepository::new(conn);
    let target_ws = match target {
        WorkspaceTarget::Id(id) => match repo.get_by_id(id) {
            Ok(ws) => ws,
            Err(e) => {
                return Response::Error {
                    message: e.to_string(),
                };
            }
        },
        WorkspaceTarget::Name(name) => {
            if let Ok(Some(ws)) = repo.find_exact_name(&name) {
                ws
            } else {
                match repo.find_by_name(&name) {
                    Ok(mut matches) if matches.len() == 1 => matches.pop().unwrap(),
                    Ok(matches) if matches.is_empty() => {
                        return Response::Error {
                            message: format!("workspace '{name}' not found"),
                        };
                    }
                    Ok(matches) => {
                        return Response::Error {
                            message: format!(
                                "ambiguous workspace name '{name}': matches {} workspaces",
                                matches.len()
                            ),
                        };
                    }
                    Err(e) => {
                        return Response::Error {
                            message: e.to_string(),
                        };
                    }
                }
            }
        }
    };

    let settings = SettingsRepository::new(conn);
    if let Err(e) = settings.set_active_workspace_id(target_ws.id) {
        return Response::Error {
            message: e.to_string(),
        };
    }

    Response::Workspace(target_ws)
}

pub fn delete_workspace(conn: &Connection, id: i64) -> Response {
    let repo = WorkspaceRepository::new(conn);
    let count = match repo.count() {
        Ok(c) => c,
        Err(e) => {
            return Response::Error {
                message: e.to_string(),
            };
        }
    };

    if count <= 1 {
        return Response::Error {
            message: "cannot delete the last workspace".to_string(),
        };
    }

    if let Err(e) = repo.get_by_id(id) {
        return Response::Error {
            message: e.to_string(),
        };
    }

    let settings = SettingsRepository::new(conn);
    let active_id = settings.active_workspace_id().ok();

    if active_id == Some(id) {
        let next_ws_id: Result<i64, _> = conn.query_row(
            "SELECT id FROM workspaces WHERE id != ?1 ORDER BY id ASC LIMIT 1",
            params![id],
            |row| row.get(0),
        );
        match next_ws_id {
            Ok(new_active) => {
                let _ = settings.set_active_workspace_id(new_active);
            }
            Err(e) => {
                return Response::Error {
                    message: e.to_string(),
                };
            }
        }
    }

    match repo.delete(id) {
        Ok(()) => Response::OkEmpty,
        Err(e) => Response::Error {
            message: e.to_string(),
        },
    }
}

pub mod scheduler;
pub use scheduler::Scheduler;

pub fn handle_command(
    cmd: let_timer_core::Command,
    conn: &Connection,
    ctx: &let_timer_core::RequestContext,
    notify: Option<&tokio::sync::Notify>,
    events: Option<&let_timer_core::EventBus>,
) -> Response {
    let affects_timer = matches!(
        cmd,
        let_timer_core::Command::Create(_)
            | let_timer_core::Command::Delete { .. }
            | let_timer_core::Command::Edit { .. }
            | let_timer_core::Command::Start { .. }
            | let_timer_core::Command::Pause { .. }
            | let_timer_core::Command::Resume { .. }
            | let_timer_core::Command::Stop { .. }
            | let_timer_core::Command::Restart { .. }
            | let_timer_core::Command::Done { .. }
            | let_timer_core::Command::Cancel { .. }
            | let_timer_core::Command::Reopen { .. }
            | let_timer_core::Command::DeleteWorkspace { .. }
    );
    let is_switch = matches!(cmd, let_timer_core::Command::SwitchWorkspace { .. });

    use let_timer_core::Command;
    let res = match cmd {
        Command::Create(new_task) => create_task(conn, new_task, ctx.workspace.as_ref()),
        Command::Delete { id } => delete_task(conn, id),
        Command::Edit { id, update } => edit_task(conn, id, update, ctx.now),
        Command::Find { query, workspace } => {
            find_tasks(conn, &query, &workspace, ctx.workspace.as_ref())
        }
        Command::List(query) => list_tasks(conn, &query, ctx.workspace.as_ref()),
        Command::Current => current_task(conn, ctx.now),
        Command::Start { id, next, switch } => start_task(conn, id, next, switch, ctx),
        Command::StartCandidates { workspace } => start_candidates(conn, &workspace, ctx),
        Command::Pause { id } => pause_task(conn, id, ctx.now),
        Command::Resume { id } => resume_task(conn, id, ctx.now),
        Command::Stop { id } => stop_task(conn, id, ctx.now),
        Command::Restart { id } => restart_task(conn, id, ctx.now),
        Command::Done { id, force } => done_task(conn, id, force, ctx.now),
        Command::Cancel { id } => cancel_task(conn, id, ctx.now),
        Command::Reopen { id } => reopen_task(conn, id, ctx.now),
        Command::Ping => Response::Pong,
        Command::ListWorkspaces => list_workspaces(conn),
        Command::CreateWorkspace(new_workspace) => create_workspace(conn, &new_workspace),
        Command::SwitchWorkspace { target } => switch_workspace(conn, target),
        Command::CurrentWorkspace => current_workspace(conn, ctx.workspace.as_ref()),
        Command::DeleteWorkspace { id } => delete_workspace(conn, id),
        Command::ListMediaLists => list_media_lists(conn),
        Command::CreateMediaList(new_media_list) => create_media_list(conn, &new_media_list),
        Command::GetMediaList { id } => get_media_list(conn, id),
        Command::DeleteMediaList { id } => delete_media_list(conn, id),
        Command::AddMediaToList {
            media_list_id,
            media_id,
        } => add_media_to_list(conn, media_list_id, media_id),
        Command::RemoveMediaFromList {
            media_list_id,
            media_id,
        } => remove_media_from_list(conn, media_list_id, media_id),
        Command::ListMedias { media_type } => list_medias(conn, media_type),
        Command::CreateMedia(new_media) => create_media(conn, &new_media),
        Command::DeleteMedia { id } => delete_media(conn, id),
        Command::DaemonStatus => daemon_status(conn),
        Command::GetTimezone => {
            let settings = SettingsRepository::new(conn);
            Response::Timezone(
                settings
                    .get_timezone()
                    .unwrap_or_else(|_| "UTC".to_string()),
            )
        }
        Command::SetTimezone(ref tz) => {
            let settings = SettingsRepository::new(conn);
            match settings.set_timezone(tz) {
                Ok(()) => Response::OkEmpty,
                Err(e) => Response::Error {
                    message: e.to_string(),
                },
            }
        }
    };

    if affects_timer && let Some(n) = notify {
        n.notify_one();
    }

    if let Some(bus) = events {
        if affects_timer {
            match &res {
                Response::Timer(snap) => {
                    bus.publish(let_timer_core::Event::TimerChanged(snap.clone()));
                }
                Response::Error { .. } => {}
                _ => {
                    if let Ok(Some(snap)) = TaskService::new(conn).current(ctx.now) {
                        bus.publish(let_timer_core::Event::TimerChanged(snap));
                    }
                }
            }
        } else if is_switch && let Response::Workspace(ref ws) = res {
            bus.publish(let_timer_core::Event::WorkspaceChanged(ws.clone()));
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use let_timer_core::{
        Command, Database, Event, EventBus, IpcClient, IpcServer, ListQuery, MediaType, NewMedia,
        NewMediaList, NewTask, Priority, RequestContext, Response, SortOrder, TaskStatus,
        TimerState, UpdateTask, WorkspaceScope, WorkspaceTarget,
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
            let ctx = let_timer_core::RequestContext::utc(now);
            handle_command(cmd, db_guard.conn(), &ctx, None, None)
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
                overdue: None,
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

    #[tokio::test]
    async fn workspace_switching_and_scoping_over_ipc() {
        use let_timer_core::WorkspaceTarget;

        let dir = std::env::temp_dir().join(format!("test-ws-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sock_path = dir.join("let-timer-ws.sock");
        let sock_str = sock_path.to_string_lossy().into_owned();

        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let db_clone = Arc::clone(&db);
        let server = IpcServer::new(&sock_str, move |cmd| {
            let now = Utc::now();
            let db_guard = db_clone.lock().unwrap_or_else(|e| e.into_inner());
            let ctx = let_timer_core::RequestContext::utc(now);
            handle_command(cmd, db_guard.conn(), &ctx, None, None)
        })
        .await
        .unwrap();

        let server_handle = tokio::spawn(async move {
            server.run().await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mut client = IpcClient::connect(&sock_path).await.unwrap();

        // 1. Initial active workspace is Default
        let res = client.request(&Command::CurrentWorkspace).await.unwrap();
        let default_ws = match res {
            Response::CurrentWorkspace(ws) => {
                assert_eq!(ws.name, "Default");
                ws
            }
            other => panic!("expected CurrentWorkspace, got {other:?}"),
        };

        // 2. Create task without workspace_id lands in Default
        let res = client
            .request(&Command::Create(NewTask {
                name: "Task in Default".into(),
                workspace_id: 0,
                media_list_id: None,
                description: None,
                priority: Priority::NotYet,
                estimated_mins: None,
                scheduled_on: None,
            }))
            .await
            .unwrap();
        let _task_default = match res {
            Response::Ok(Some(t)) => {
                assert_eq!(t.workspace_id, default_ws.id);
                t
            }
            other => panic!("expected Ok(Some), got {other:?}"),
        };

        // 3. Create a new workspace "Study"
        let res = client
            .request(&Command::CreateWorkspace(NewWorkspace {
                name: "Study".into(),
                description: Some("Study workspace".into()),
            }))
            .await
            .unwrap();
        let study_ws = match res {
            Response::Workspace(ws) => {
                assert_eq!(ws.name, "Study");
                ws
            }
            other => panic!("expected Workspace, got {other:?}"),
        };

        // 4. Switch to "Study" by name
        let res = client
            .request(&Command::SwitchWorkspace {
                target: WorkspaceTarget::Name("Study".into()),
            })
            .await
            .unwrap();
        match res {
            Response::Workspace(ws) => assert_eq!(ws.id, study_ws.id),
            other => panic!("expected Workspace, got {other:?}"),
        }

        // 5. Current workspace is now Study
        let res = client.request(&Command::CurrentWorkspace).await.unwrap();
        match res {
            Response::CurrentWorkspace(ws) => assert_eq!(ws.id, study_ws.id),
            other => panic!("expected CurrentWorkspace, got {other:?}"),
        }

        // 6. Create task in Study (without explicit workspace_id)
        let res = client
            .request(&Command::Create(NewTask {
                name: "Study Rust".into(),
                workspace_id: 0,
                media_list_id: None,
                description: None,
                priority: Priority::Immediate,
                estimated_mins: Some(30),
                scheduled_on: None,
            }))
            .await
            .unwrap();
        let study_task = match res {
            Response::Ok(Some(t)) => {
                assert_eq!(t.workspace_id, study_ws.id);
                t
            }
            other => panic!("expected Ok(Some), got {other:?}"),
        };

        // 7. List with WorkspaceScope::Active only returns tasks in Study
        let res = client
            .request(&Command::List(ListQuery {
                workspace: WorkspaceScope::Active,
                ..Default::default()
            }))
            .await
            .unwrap();
        match res {
            Response::OkList(tasks) => {
                assert_eq!(tasks.len(), 1);
                assert_eq!(tasks[0].id, study_task.id);
            }
            other => panic!("expected OkList, got {other:?}"),
        }

        // 8. List with WorkspaceScope::All returns tasks in both workspaces
        let res = client
            .request(&Command::List(ListQuery {
                workspace: WorkspaceScope::All,
                ..Default::default()
            }))
            .await
            .unwrap();
        match res {
            Response::OkList(tasks) => {
                assert_eq!(tasks.len(), 2);
            }
            other => panic!("expected OkList, got {other:?}"),
        }

        // 9. Start study_task, then switch to Default -> running task is not affected!
        let res = client
            .request(&Command::Start {
                id: Some(study_task.id),
                next: false,
                switch: false,
            })
            .await
            .unwrap();
        match res {
            Response::Timer(snap) => assert_eq!(snap.task.id, study_task.id),
            other => panic!("expected Timer, got {other:?}"),
        }

        let _ = client
            .request(&Command::SwitchWorkspace {
                target: WorkspaceTarget::Id(default_ws.id),
            })
            .await
            .unwrap();

        let res = client.request(&Command::Current).await.unwrap();
        match res {
            Response::Timer(snap) => {
                assert_eq!(snap.task.id, study_task.id);
                assert_eq!(snap.task.status, TaskStatus::InProgress);
            }
            other => panic!("expected Timer, got {other:?}"),
        }

        // Stop the task so we can test deletion
        let _ = client.request(&Command::Stop { id: None }).await.unwrap();

        // 10. Deleting the active workspace (Default) switches active to Study
        let res = client
            .request(&Command::DeleteWorkspace { id: default_ws.id })
            .await
            .unwrap();
        assert!(matches!(res, Response::OkEmpty));

        let res = client.request(&Command::CurrentWorkspace).await.unwrap();
        match res {
            Response::CurrentWorkspace(ws) => assert_eq!(ws.id, study_ws.id),
            other => panic!("expected CurrentWorkspace, got {other:?}"),
        }

        // 11. Deleting the last remaining workspace (Study) is refused
        let res = client
            .request(&Command::DeleteWorkspace { id: study_ws.id })
            .await
            .unwrap();
        match res {
            Response::Error { message } => {
                assert!(message.contains("cannot delete the last workspace"));
            }
            other => panic!("expected Error, got {other:?}"),
        }

        server_handle.abort();
    }

    #[tokio::test]
    async fn start_candidates_ordering_over_ipc() {
        let dir = std::env::temp_dir().join(format!("test-candidates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sock_path = dir.join("let-timer-candidates.sock");
        let sock_str = sock_path.to_string_lossy().into_owned();

        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let db_clone = Arc::clone(&db);
        let now = DateTime::parse_from_rfc3339("2026-10-12T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let server = IpcServer::new(&sock_str, move |cmd| {
            let db_guard = db_clone.lock().unwrap_or_else(|e| e.into_inner());
            let ctx = let_timer_core::RequestContext::utc(now);
            handle_command(cmd, db_guard.conn(), &ctx, None, None)
        })
        .await
        .unwrap();

        let server_handle = tokio::spawn(async move {
            server.run().await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mut client = IpcClient::connect(&sock_path).await.unwrap();

        // Task 1: unscheduled, not-yet (low)
        let _ = client
            .request(&Command::Create(NewTask {
                name: "T1 unscheduled low".into(),
                workspace_id: 0,
                media_list_id: None,
                description: None,
                priority: Priority::NotYet,
                estimated_mins: None,
                scheduled_on: None,
            }))
            .await
            .unwrap();

        // Task 2: unscheduled, urgent (high)
        let _ = client
            .request(&Command::Create(NewTask {
                name: "T2 unscheduled urgent".into(),
                workspace_id: 0,
                media_list_id: None,
                description: None,
                priority: Priority::Urgent,
                estimated_mins: None,
                scheduled_on: None,
            }))
            .await
            .unwrap();

        // Task 3: scheduled today, not-yet (low)
        let _ = client
            .request(&Command::Create(NewTask {
                name: "T3 today low".into(),
                workspace_id: 0,
                media_list_id: None,
                description: None,
                priority: Priority::NotYet,
                estimated_mins: None,
                scheduled_on: Some(now.date_naive()),
            }))
            .await
            .unwrap();

        // Task 4: scheduled today, urgent (high)
        let _ = client
            .request(&Command::Create(NewTask {
                name: "T4 today urgent".into(),
                workspace_id: 0,
                media_list_id: None,
                description: None,
                priority: Priority::Urgent,
                estimated_mins: None,
                scheduled_on: Some(now.date_naive()),
            }))
            .await
            .unwrap();

        // Query candidates
        let res = client
            .request(&Command::StartCandidates {
                workspace: WorkspaceScope::Active,
            })
            .await
            .unwrap();

        match res {
            Response::OkList(tasks) => {
                assert_eq!(tasks.len(), 4);
                // Ordered: scheduled today first (urgent > not-yet), then unscheduled (urgent > not-yet)
                assert_eq!(tasks[0].name, "T4 today urgent");
                assert_eq!(tasks[1].name, "T3 today low");
                assert_eq!(tasks[2].name, "T2 unscheduled urgent");
                assert_eq!(tasks[3].name, "T1 unscheduled low");
            }
            other => panic!("expected OkList, got {other:?}"),
        }

        server_handle.abort();
    }

    #[tokio::test]
    async fn media_and_media_list_crud_over_ipc() {
        let dir = std::env::temp_dir().join(format!("test-ipc-media-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let socket_path = dir.join("test.sock");
        let _ = std::fs::remove_file(&socket_path);
        let socket_str = socket_path.to_str().unwrap().to_string();

        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let db_clone = Arc::clone(&db);

        let server = IpcServer::new(&socket_str, move |cmd| {
            let now = Utc::now();
            let db_guard = db_clone.lock().unwrap();
            let ctx = let_timer_core::RequestContext::utc(now);
            handle_command(cmd, db_guard.conn(), &ctx, None, None)
        })
        .await
        .unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = server.run().await;
        });

        let mut client = IpcClient::connect(&socket_path).await.unwrap();

        // 1. Create media items
        let res = client
            .request(&Command::CreateMedia(NewMedia {
                name: "Nature Wallpaper".into(),
                description: Some("Forest in morning light".into()),
                url: "https://example.com/forest.jpg".into(),
                media_type: MediaType::Image,
                content: None,
            }))
            .await
            .unwrap();
        let media_image = match res {
            Response::Media(m) => m,
            other => panic!("expected Media, got {other:?}"),
        };
        assert_eq!(media_image.media_type, MediaType::Image);

        let res = client
            .request(&Command::CreateMedia(NewMedia {
                name: "Lo-Fi Beats".into(),
                description: None,
                url: "https://example.com/beats.mp3".into(),
                media_type: MediaType::Music,
                content: None,
            }))
            .await
            .unwrap();
        let media_music = match res {
            Response::Media(m) => m,
            other => panic!("expected Media, got {other:?}"),
        };

        let res = client
            .request(&Command::CreateMedia(NewMedia {
                name: "The Road Not Taken".into(),
                description: Some("Robert Frost poem".into()),
                url: "https://example.com/poem.txt".into(),
                media_type: MediaType::Poem,
                content: Some("Two roads diverged in a yellow wood...".into()),
            }))
            .await
            .unwrap();
        let media_poem = match res {
            Response::Media(m) => m,
            other => panic!("expected Media, got {other:?}"),
        };

        // 2. List medias (all and filtered by type)
        let res = client
            .request(&Command::ListMedias { media_type: None })
            .await
            .unwrap();
        match res {
            Response::Medias(list) => assert_eq!(list.len(), 3),
            other => panic!("expected Medias, got {other:?}"),
        }

        let res = client
            .request(&Command::ListMedias {
                media_type: Some(MediaType::Music),
            })
            .await
            .unwrap();
        match res {
            Response::Medias(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0].id, media_music.id);
            }
            other => panic!("expected Medias, got {other:?}"),
        }

        // 3. Create a media list
        let res = client
            .request(&Command::CreateMediaList(NewMediaList {
                name: "Work Focus".into(),
                description: Some("Music and images for study".into()),
            }))
            .await
            .unwrap();
        let list_id = match res {
            Response::MediaList(ml) => ml.id,
            other => panic!("expected MediaList, got {other:?}"),
        };

        // 4. Add items to media list
        let res = client
            .request(&Command::AddMediaToList {
                media_list_id: list_id,
                media_id: media_image.id,
            })
            .await
            .unwrap();
        assert!(matches!(res, Response::OkEmpty));

        let res = client
            .request(&Command::AddMediaToList {
                media_list_id: list_id,
                media_id: media_music.id,
            })
            .await
            .unwrap();
        assert!(matches!(res, Response::OkEmpty));

        // Adding duplicate fails (idempotency check via UNIQUE constraint)
        let res = client
            .request(&Command::AddMediaToList {
                media_list_id: list_id,
                media_id: media_music.id,
            })
            .await
            .unwrap();
        assert!(matches!(res, Response::Error { .. }));

        // 5. Get media list detail (show)
        let res = client
            .request(&Command::GetMediaList { id: list_id })
            .await
            .unwrap();
        match res {
            Response::MediaListDetail { list, items } => {
                assert_eq!(list.id, list_id);
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected MediaListDetail, got {other:?}"),
        }

        // 6. Create a task assigned to this media list
        let res = client
            .request(&Command::Create(NewTask {
                workspace_id: 1,
                media_list_id: Some(list_id),
                name: "Study with media".into(),
                description: None,
                priority: Priority::Immediate,
                estimated_mins: Some(30),
                scheduled_on: None,
            }))
            .await
            .unwrap();
        let task_id = match res {
            Response::Ok(Some(t)) => {
                assert_eq!(t.media_list_id, Some(list_id));
                t.id
            }
            other => panic!("expected Ok(Some(task)), got {other:?}"),
        };

        // 7. Remove an item from the list
        let res = client
            .request(&Command::RemoveMediaFromList {
                media_list_id: list_id,
                media_id: media_music.id,
            })
            .await
            .unwrap();
        assert!(matches!(res, Response::OkEmpty));

        let res = client
            .request(&Command::GetMediaList { id: list_id })
            .await
            .unwrap();
        match res {
            Response::MediaListDetail { items, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].id, media_image.id);
            }
            other => panic!("expected MediaListDetail, got {other:?}"),
        }

        // 8. Deleting media cascades to join table
        let res = client
            .request(&Command::DeleteMedia { id: media_image.id })
            .await
            .unwrap();
        assert!(matches!(res, Response::OkEmpty));

        let res = client
            .request(&Command::GetMediaList { id: list_id })
            .await
            .unwrap();
        match res {
            Response::MediaListDetail { items, .. } => {
                assert_eq!(items.len(), 0);
            }
            other => panic!("expected MediaListDetail, got {other:?}"),
        }

        // Delete poem too
        let _ = client
            .request(&Command::DeleteMedia { id: media_poem.id })
            .await
            .unwrap();

        // 9. Deleting media list sets tasks.media_list_id to NULL
        let res = client
            .request(&Command::DeleteMediaList { id: list_id })
            .await
            .unwrap();
        assert!(matches!(res, Response::OkEmpty));

        // Verify task now has media_list_id == None
        let res = client
            .request(&Command::List(ListQuery::default()))
            .await
            .unwrap();
        match res {
            Response::OkList(tasks) => {
                let task = tasks.iter().find(|t| t.id == task_id).unwrap();
                assert_eq!(task.media_list_id, None);
            }
            other => panic!("expected OkList, got {other:?}"),
        }

        server_handle.abort();
    }

    #[test]
    fn timezone_order_scheduled_today_uses_local_date() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();

        let repo = TaskRepository::new(conn);
        // UTC time: 2026-10-12 20:00:00 UTC
        // Asia/Ho_Chi_Minh: UTC+7 => 2026-10-13 03:00:00 local time
        let now = DateTime::parse_from_rfc3339("2026-10-12T20:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let date_12 = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        let date_13 = chrono::NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();

        let t1 = repo
            .create(&NewTask {
                name: "Task scheduled on Oct 12".into(),
                workspace_id: 1,
                priority: Priority::Immediate,
                scheduled_on: Some(date_12),
                description: None,
                estimated_mins: None,
                media_list_id: None,
            })
            .unwrap();

        let t2 = repo
            .create(&NewTask {
                name: "Task scheduled on Oct 13".into(),
                workspace_id: 1,
                priority: Priority::Immediate,
                scheduled_on: Some(date_13),
                description: None,
                estimated_mins: None,
                media_list_id: None,
            })
            .unwrap();

        // 1. With UTC context: today is Oct 12 => t1 is first
        let ctx_utc = RequestContext::new(now, date_12, None);
        let candidates_utc = get_start_candidates(conn, &WorkspaceScope::Active, &ctx_utc).unwrap();
        assert_eq!(candidates_utc[0].id, t1.id);

        // 2. With Asia/Ho_Chi_Minh context: today is Oct 13 => t2 is first
        let ctx_vn = RequestContext::from_settings(conn, now, Some("Asia/Ho_Chi_Minh"), None);
        assert_eq!(ctx_vn.today, date_13);
        let candidates_vn = get_start_candidates(conn, &WorkspaceScope::Active, &ctx_vn).unwrap();
        assert_eq!(candidates_vn[0].id, t2.id);
    }

    #[test]
    fn workspace_override_beats_db_setting_without_modifying_it() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();

        let ws_repo = WorkspaceRepository::new(conn);
        let ws2 = ws_repo
            .create(&NewWorkspace {
                name: "Work".into(),
                description: None,
            })
            .unwrap();

        let task_repo = TaskRepository::new(conn);
        task_repo
            .create(&NewTask {
                name: "Task in Default".into(),
                workspace_id: 1,
                priority: Priority::Immediate,
                scheduled_on: None,
                description: None,
                estimated_mins: None,
                media_list_id: None,
            })
            .unwrap();

        let t_work = task_repo
            .create(&NewTask {
                name: "Task in Work".into(),
                workspace_id: ws2.id,
                priority: Priority::Immediate,
                scheduled_on: None,
                description: None,
                estimated_mins: None,
                media_list_id: None,
            })
            .unwrap();

        let now = Utc::now();
        // Request with workspace override = Work (#2)
        let ctx_override =
            RequestContext::new(now, now.date_naive(), Some(WorkspaceScope::Id(ws2.id)));
        let res = handle_command(
            Command::List(ListQuery::default()),
            conn,
            &ctx_override,
            None,
            None,
        );

        match res {
            Response::OkList(tasks) => {
                assert_eq!(tasks.len(), 1);
                assert_eq!(tasks[0].id, t_work.id);
            }
            other => panic!("expected OkList, got {other:?}"),
        }

        // DB active workspace is still 1!
        let settings = SettingsRepository::new(conn);
        assert_eq!(settings.active_workspace_id().unwrap(), 1);
    }

    #[tokio::test]
    async fn event_bus_publishes_on_mutation_and_switch() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        let event_bus = EventBus::new(16);
        let mut sub = event_bus.subscribe();

        let now = Utc::now();
        let ctx = RequestContext::utc(now);

        // 1. Create a workspace
        let res = handle_command(
            Command::CreateWorkspace(NewWorkspace {
                name: "Study".into(),
                description: None,
            }),
            conn,
            &ctx,
            None,
            Some(&event_bus),
        );
        assert!(matches!(res, Response::Workspace(_)));

        // 2. Switch workspace -> should emit WorkspaceChanged
        let res = handle_command(
            Command::SwitchWorkspace {
                target: WorkspaceTarget::Name("Study".into()),
            },
            conn,
            &ctx,
            None,
            Some(&event_bus),
        );
        assert!(matches!(res, Response::Workspace(_)));

        let event = sub.recv().await.unwrap();
        match event {
            Event::WorkspaceChanged(ws) => assert_eq!(ws.name, "Study"),
            other => panic!("expected WorkspaceChanged, got {other:?}"),
        }

        // 3. Create and start a task -> should emit TimerChanged
        let task_res = handle_command(
            Command::Create(NewTask {
                name: "Focus".into(),
                workspace_id: 1,
                priority: Priority::Urgent,
                scheduled_on: None,
                description: None,
                estimated_mins: Some(25),
                media_list_id: None,
            }),
            conn,
            &ctx,
            None,
            Some(&event_bus),
        );
        let task_id = match task_res {
            Response::Ok(Some(t)) => t.id,
            other => panic!("expected Ok(Some), got {other:?}"),
        };

        let start_res = handle_command(
            Command::Start {
                id: Some(task_id),
                next: false,
                switch: false,
            },
            conn,
            &ctx,
            None,
            Some(&event_bus),
        );
        assert!(matches!(start_res, Response::Timer(_)));

        let event2 = sub.recv().await.unwrap();
        match event2 {
            Event::TimerChanged(snap) => {
                assert_eq!(snap.task.id, task_id);
                assert_eq!(snap.state, TimerState::Running);
            }
            other => panic!("expected TimerChanged, got {other:?}"),
        }
    }
}
