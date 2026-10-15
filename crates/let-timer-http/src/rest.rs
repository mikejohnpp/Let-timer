use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{NaiveDate, Utc};
use serde_json::json;
use sha2::Digest;
use std::sync::Arc;

use let_timer_core::{
    Command, ListQuery, MediaType, NewMedia, NewMediaList, NewTask, NewWorkspace, Priority,
    RequestContext, Response as CoreResponse, SortOrder, TaskStatus, UpdateTask, WorkspaceScope,
    WorkspaceTarget,
};

use crate::dto::*;
use crate::router::AppState;

pub fn error_response(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> Response {
    (
        status,
        Json(json!({
            "error": {
                "code": code,
                "message": message.into()
            }
        })),
    )
        .into_response()
}

pub fn map_core_error(err_msg: &str) -> (StatusCode, &'static str) {
    if err_msg.contains("not found") {
        (StatusCode::NOT_FOUND, "not_found")
    } else if err_msg.contains("already running")
        || err_msg.contains("another task is running")
        || err_msg.contains("conflict")
    {
        (StatusCode::CONFLICT, "conflict")
    } else if err_msg.contains("cannot transition")
        || err_msg.contains("illegal")
        || err_msg.contains("invalid")
    {
        (StatusCode::BAD_REQUEST, "invalid_transition")
    } else {
        (StatusCode::BAD_REQUEST, "bad_request")
    }
}

pub fn extract_context(
    headers: &HeaderMap,
    db: &rusqlite::Connection,
    now: chrono::DateTime<Utc>,
) -> RequestContext {
    let ws_header = headers
        .get("x-let-timer-workspace")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            if let Ok(id) = s.parse::<i64>() {
                WorkspaceScope::Id(id)
            } else {
                WorkspaceScope::Name(s.to_string())
            }
        });

    let tz_header = headers
        .get("x-let-timer-timezone")
        .and_then(|v| v.to_str().ok());

    RequestContext::from_settings(db, now, tz_header, ws_header)
}

pub async fn execute_command(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    cmd: Command,
) -> Result<CoreResponse, (StatusCode, &'static str, String)> {
    if let Err(e) = crate::policy::validate_remote(&cmd) {
        return Err((
            StatusCode::FORBIDDEN,
            "forbidden_remote_command",
            e.to_string(),
        ));
    }

    let state_clone = Arc::clone(state);
    let headers_clone = headers.clone();

    let result = tokio::task::spawn_blocking(move || {
        let db = state_clone.db.lock().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now();
        let ctx = extract_context(&headers_clone, db.conn(), now);
        let notify_ref = state_clone.timer_notify.as_deref();
        let events_ref = state_clone.events.as_deref();

        (state_clone.command_handler)(cmd, db.conn(), &ctx, notify_ref, events_ref)
    })
    .await;

    match result {
        Ok(CoreResponse::Error { message }) => {
            let (status, code) = map_core_error(&message);
            Err((status, code, message))
        }
        Ok(res) => Ok(res),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "task_failed",
            e.to_string(),
        )),
    }
}

// ─── Task Handlers ───────────────────────────────────────────────────

/// GET /api/v1/tasks
#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    params(TasksQuery),
    responses(
        (status = 200, description = "List of tasks", body = Vec<TaskDto>),
        (status = 400, description = "Bad Request", body = ErrorEnvelopeDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Tasks"
)]
pub async fn list_tasks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TasksQuery>,
) -> Response {
    if let Some(ref q) = query.q {
        let ws = match query.workspace.as_deref() {
            Some("all") => WorkspaceScope::All,
            Some(w) => {
                if let Ok(id) = w.parse::<i64>() {
                    WorkspaceScope::Id(id)
                } else {
                    WorkspaceScope::Name(w.to_string())
                }
            }
            None => WorkspaceScope::Active,
        };
        match execute_command(
            &state,
            &headers,
            Command::Find {
                query: q.clone(),
                workspace: ws,
            },
        )
        .await
        {
            Ok(CoreResponse::OkList(tasks)) => {
                let dtos: Vec<TaskDto> = tasks.into_iter().map(TaskDto::from).collect();
                Json(dtos).into_response()
            }
            Ok(_) => Json(Vec::<TaskDto>::new()).into_response(),
            Err((status, code, msg)) => error_response(status, code, msg),
        }
    } else {
        let ws = match query.workspace.as_deref() {
            Some("all") => WorkspaceScope::All,
            Some(w) => {
                if let Ok(id) = w.parse::<i64>() {
                    WorkspaceScope::Id(id)
                } else {
                    WorkspaceScope::Name(w.to_string())
                }
            }
            None => WorkspaceScope::Active,
        };

        let status = match query.status.as_deref() {
            Some(s) => match s.parse::<TaskStatus>() {
                Ok(st) => Some(st),
                Err(_) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "invalid_parameter",
                        format!("invalid status: {s}"),
                    );
                }
            },
            None => None,
        };

        let priority = match query.priority.as_deref() {
            Some("urgent") | Some("2") => Some(Priority::Urgent),
            Some("immediate") | Some("1") => Some(Priority::Immediate),
            Some("not-yet") | Some("0") => Some(Priority::NotYet),
            Some(p) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_parameter",
                    format!("invalid priority: {p}"),
                );
            }
            None => None,
        };

        let sort_priority = match query.sort.as_deref() {
            Some("priority_asc") | Some("asc") => Some(SortOrder::Ascending),
            Some("priority_desc") | Some("desc") => Some(SortOrder::Descending),
            Some(s) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_parameter",
                    format!("invalid sort: {s}"),
                );
            }
            None => None,
        };

        let date = match query.date.as_deref() {
            Some("today") => {
                let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
                let ctx = extract_context(&headers, db.conn(), Utc::now());
                Some(ctx.today)
            }
            Some("tomorrow") => {
                let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
                let ctx = extract_context(&headers, db.conn(), Utc::now());
                Some(ctx.today + chrono::Days::new(1))
            }
            Some(d) => match d.parse::<NaiveDate>() {
                Ok(nd) => Some(nd),
                Err(_) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "invalid_parameter",
                        format!("invalid date format: {d}"),
                    );
                }
            },
            None => None,
        };

        let list_query = ListQuery {
            workspace: ws,
            status,
            sort_priority,
            priority,
            date,
            overdue: query.overdue,
        };

        match execute_command(&state, &headers, Command::List(list_query)).await {
            Ok(CoreResponse::OkList(tasks)) => {
                let dtos: Vec<TaskDto> = tasks.into_iter().map(TaskDto::from).collect();
                Json(dtos).into_response()
            }
            Ok(_) => Json(Vec::<TaskDto>::new()).into_response(),
            Err((status, code, msg)) => error_response(status, code, msg),
        }
    }
}

/// POST /api/v1/tasks
#[utoipa::path(
    post,
    path = "/api/v1/tasks",
    request_body = CreateTaskDto,
    responses(
        (status = 201, description = "Task created", body = TaskDto),
        (status = 400, description = "Bad Request", body = ErrorEnvelopeDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Tasks"
)]
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(dto): Json<CreateTaskDto>,
) -> Response {
    let priority = match dto.priority.unwrap_or(0) {
        2 => Priority::Urgent,
        1 => Priority::Immediate,
        _ => Priority::NotYet,
    };

    let new_task = NewTask {
        workspace_id: dto.workspace_id.unwrap_or(0),
        media_list_id: dto.media_list_id,
        name: dto.name,
        description: dto.description,
        priority,
        estimated_mins: dto.estimated_mins,
        scheduled_on: dto.scheduled_on,
    };

    match execute_command(&state, &headers, Command::Create(new_task)).await {
        Ok(CoreResponse::Ok(Some(task))) => {
            (StatusCode::CREATED, Json(TaskDto::from(task))).into_response()
        }
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// GET /api/v1/tasks/current
#[utoipa::path(
    get,
    path = "/api/v1/tasks/current",
    responses(
        (status = 200, description = "Current timer snapshot", body = Option<TimerSnapshotDto>),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Tasks"
)]
pub async fn current_task(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::Current).await {
        Ok(CoreResponse::Timer(snapshot)) => {
            Json(Some(TimerSnapshotDto::from(snapshot))).into_response()
        }
        Ok(CoreResponse::OkEmpty) => Json(None::<TimerSnapshotDto>).into_response(),
        Ok(_) => Json(None::<TimerSnapshotDto>).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// GET /api/v1/tasks/start-candidates
#[utoipa::path(
    get,
    path = "/api/v1/tasks/start-candidates",
    responses(
        (status = 200, description = "Start candidates", body = Vec<TaskDto>),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Tasks"
)]
pub async fn start_candidates(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(
        &state,
        &headers,
        Command::StartCandidates {
            workspace: WorkspaceScope::Active,
        },
    )
    .await
    {
        Ok(CoreResponse::OkList(tasks)) => {
            let dtos: Vec<TaskDto> = tasks.into_iter().map(TaskDto::from).collect();
            Json(dtos).into_response()
        }
        Ok(_) => Json(Vec::<TaskDto>::new()).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// GET /api/v1/tasks/{id}
#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task details", body = TaskDto),
        (status = 404, description = "Task not found", body = ErrorEnvelopeDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Tasks"
)]
pub async fn get_task(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let repo = let_timer_core::TaskRepository::new(db.conn());
    match repo.get_by_id(id) {
        Ok(task) => Json(TaskDto::from(task)).into_response(),
        Err(let_timer_core::DbError::NotFound { .. }) => error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            format!("task {id} not found"),
        ),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()),
    }
}

/// PATCH /api/v1/tasks/{id}
#[utoipa::path(
    patch,
    path = "/api/v1/tasks/{id}",
    params(("id" = i64, Path, description = "Task ID")),
    request_body = UpdateTaskDto,
    responses(
        (status = 200, description = "Task updated", body = TaskDto),
        (status = 400, description = "Bad Request", body = ErrorEnvelopeDto),
        (status = 404, description = "Task not found", body = ErrorEnvelopeDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Tasks"
)]
pub async fn update_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(dto): Json<UpdateTaskDto>,
) -> Response {
    let priority = dto.priority.map(|p| match p {
        2 => Priority::Urgent,
        1 => Priority::Immediate,
        _ => Priority::NotYet,
    });

    let update = UpdateTask {
        name: dto.name,
        description: dto.description.map(Some),
        priority,
        status: None,
        estimated_mins: dto.estimated_mins.map(Some),
        scheduled_on: dto.scheduled_on.map(Some),
        media_list_id: dto.media_list_id.map(Some),
    };

    match execute_command(&state, &headers, Command::Edit { id, update }).await {
        Ok(CoreResponse::Ok(Some(task))) => Json(TaskDto::from(task)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// DELETE /api/v1/tasks/{id}
#[utoipa::path(
    delete,
    path = "/api/v1/tasks/{id}",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 204, description = "Task deleted"),
        (status = 404, description = "Task not found", body = ErrorEnvelopeDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Tasks"
)]
pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::Delete { id }).await {
        Ok(CoreResponse::OkEmpty) => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

// ─── Lifecycle Endpoints ─────────────────────────────────────────────

/// POST /api/v1/tasks/{id}/start
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/start",
    params(
        ("id" = i64, Path, description = "Task ID"),
        StartTaskQuery
    ),
    responses(
        (status = 200, description = "Task started", body = TimerSnapshotDto),
        (status = 409, description = "Another task is running", body = ErrorEnvelopeDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn start_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(q): Query<StartTaskQuery>,
) -> Response {
    let switch = q.switch.unwrap_or(false);
    match execute_command(
        &state,
        &headers,
        Command::Start {
            id: Some(id),
            next: false,
            switch,
        },
    )
    .await
    {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/tasks/{id}/pause
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/pause",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task paused", body = TimerSnapshotDto),
        (status = 400, description = "Cannot pause", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn pause_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::Pause { id: Some(id) }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/tasks/{id}/resume
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/resume",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task resumed", body = TimerSnapshotDto),
        (status = 400, description = "Cannot resume", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn resume_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::Resume { id: Some(id) }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/tasks/{id}/stop
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/stop",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task stopped", body = TimerSnapshotDto),
        (status = 400, description = "Cannot stop", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn stop_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::Stop { id: Some(id) }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/tasks/{id}/restart
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/restart",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task restarted", body = TimerSnapshotDto),
        (status = 400, description = "Cannot restart", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn restart_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::Restart { id: Some(id) }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/tasks/{id}/done
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/done",
    params(
        ("id" = i64, Path, description = "Task ID"),
        DoneTaskQuery
    ),
    responses(
        (status = 200, description = "Task marked done", body = TimerSnapshotDto),
        (status = 400, description = "Cannot mark done without force", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn done_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(q): Query<DoneTaskQuery>,
) -> Response {
    let force = q.force.unwrap_or(false);
    match execute_command(
        &state,
        &headers,
        Command::Done {
            id: Some(id),
            force,
        },
    )
    .await
    {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/tasks/{id}/cancel
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/cancel",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task cancelled", body = TimerSnapshotDto),
        (status = 400, description = "Cannot cancel", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn cancel_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::Cancel { id: Some(id) }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/tasks/{id}/reopen
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/reopen",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task reopened", body = TimerSnapshotDto),
        (status = 400, description = "Cannot reopen", body = ErrorEnvelopeDto),
    ),
    tag = "Lifecycle"
)]
pub async fn reopen_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::Reopen { id }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

// ─── Global Timer Lifecycle Endpoints ─────────────────────────────────

/// GET /api/v1/timer
#[utoipa::path(
    get,
    path = "/api/v1/timer",
    responses(
        (status = 200, description = "Current timer snapshot", body = Option<TimerSnapshotDto>),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Timer"
)]
pub async fn get_timer(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    current_task(State(state), headers).await
}

/// POST /api/v1/timer/pause
#[utoipa::path(
    post,
    path = "/api/v1/timer/pause",
    responses(
        (status = 200, description = "Running task paused", body = TimerSnapshotDto),
        (status = 400, description = "No running task to pause", body = ErrorEnvelopeDto),
    ),
    tag = "Timer"
)]
pub async fn timer_pause(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::Pause { id: None }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/timer/resume
#[utoipa::path(
    post,
    path = "/api/v1/timer/resume",
    responses(
        (status = 200, description = "Paused task resumed", body = TimerSnapshotDto),
        (status = 400, description = "No paused task to resume", body = ErrorEnvelopeDto),
    ),
    tag = "Timer"
)]
pub async fn timer_resume(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::Resume { id: None }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/timer/stop
#[utoipa::path(
    post,
    path = "/api/v1/timer/stop",
    responses(
        (status = 200, description = "Running task stopped", body = TimerSnapshotDto),
        (status = 400, description = "No running task to stop", body = ErrorEnvelopeDto),
    ),
    tag = "Timer"
)]
pub async fn timer_stop(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::Stop { id: None }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/timer/done
#[utoipa::path(
    post,
    path = "/api/v1/timer/done",
    params(DoneTaskQuery),
    responses(
        (status = 200, description = "Running task marked done", body = TimerSnapshotDto),
        (status = 400, description = "No running task to mark done", body = ErrorEnvelopeDto),
    ),
    tag = "Timer"
)]
pub async fn timer_done(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<DoneTaskQuery>,
) -> Response {
    let force = q.force.unwrap_or(false);
    match execute_command(&state, &headers, Command::Done { id: None, force }).await {
        Ok(CoreResponse::Timer(snapshot)) => Json(TimerSnapshotDto::from(snapshot)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

// ─── Workspace Handlers ──────────────────────────────────────────────

/// GET /api/v1/workspaces
#[utoipa::path(
    get,
    path = "/api/v1/workspaces",
    responses(
        (status = 200, description = "List of workspaces", body = Vec<WorkspaceDto>),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Workspaces"
)]
pub async fn list_workspaces(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::ListWorkspaces).await {
        Ok(CoreResponse::WorkspaceList(ws)) => {
            let dtos: Vec<WorkspaceDto> = ws.into_iter().map(WorkspaceDto::from).collect();
            Json(dtos).into_response()
        }
        Ok(_) => Json(Vec::<WorkspaceDto>::new()).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/workspaces
#[utoipa::path(
    post,
    path = "/api/v1/workspaces",
    request_body = CreateWorkspaceDto,
    responses(
        (status = 201, description = "Workspace created", body = WorkspaceDto),
        (status = 400, description = "Bad Request", body = ErrorEnvelopeDto),
    ),
    tag = "Workspaces"
)]
pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(dto): Json<CreateWorkspaceDto>,
) -> Response {
    let new_ws = NewWorkspace {
        name: dto.name,
        description: dto.description,
    };
    match execute_command(&state, &headers, Command::CreateWorkspace(new_ws)).await {
        Ok(CoreResponse::Workspace(ws)) => {
            (StatusCode::CREATED, Json(WorkspaceDto::from(ws))).into_response()
        }
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// GET /api/v1/workspaces/current
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/current",
    responses(
        (status = 200, description = "Current workspace", body = WorkspaceDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Workspaces"
)]
pub async fn current_workspace(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::CurrentWorkspace).await {
        Ok(CoreResponse::CurrentWorkspace(ws)) => Json(WorkspaceDto::from(ws)).into_response(),
        Ok(CoreResponse::Workspace(ws)) => Json(WorkspaceDto::from(ws)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/workspaces/switch
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/switch",
    request_body = SwitchWorkspaceDto,
    responses(
        (status = 200, description = "Switched active workspace", body = WorkspaceDto),
        (status = 404, description = "Workspace not found", body = ErrorEnvelopeDto),
    ),
    tag = "Workspaces"
)]
pub async fn switch_workspace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(dto): Json<SwitchWorkspaceDto>,
) -> Response {
    let target = if let Some(id) = dto.id {
        WorkspaceTarget::Id(id)
    } else if let Some(name) = dto.name {
        WorkspaceTarget::Name(name)
    } else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_parameter",
            "id or name is required",
        );
    };

    match execute_command(&state, &headers, Command::SwitchWorkspace { target }).await {
        Ok(CoreResponse::Workspace(ws)) => Json(WorkspaceDto::from(ws)).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// DELETE /api/v1/workspaces/{id}
#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{id}",
    params(("id" = i64, Path, description = "Workspace ID")),
    responses(
        (status = 204, description = "Workspace deleted"),
        (status = 400, description = "Cannot delete last workspace", body = ErrorEnvelopeDto),
        (status = 404, description = "Workspace not found", body = ErrorEnvelopeDto),
    ),
    tag = "Workspaces"
)]
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::DeleteWorkspace { id }).await {
        Ok(CoreResponse::OkEmpty) => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

// ─── Media & Media Lists ─────────────────────────────────────────────

fn get_media_signing_secret(state: &AppState) -> String {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let repo = let_timer_core::SettingsRepository::new(db.conn());
    repo.get_or_create_media_signing_secret()
        .unwrap_or_default()
}

/// GET /api/v1/medias
#[utoipa::path(
    get,
    path = "/api/v1/medias",
    params(MediasQuery),
    responses(
        (status = 200, description = "List of medias", body = Vec<MediaDto>),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Medias"
)]
pub async fn list_medias(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<MediasQuery>,
) -> Response {
    let media_type = match q.media_type.as_deref() {
        Some(t) => match t.parse::<MediaType>() {
            Ok(mt) => Some(mt),
            Err(_) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_parameter",
                    format!("invalid media type: {t}"),
                );
            }
        },
        None => None,
    };

    match execute_command(&state, &headers, Command::ListMedias { media_type }).await {
        Ok(CoreResponse::Medias(m)) => {
            let secret = get_media_signing_secret(&state);
            let dtos: Vec<MediaDto> = m
                .into_iter()
                .map(|item| MediaDto::from(item).with_signed_url(&secret))
                .collect();
            Json(dtos).into_response()
        }
        Ok(_) => Json(Vec::<MediaDto>::new()).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/medias
#[utoipa::path(
    post,
    path = "/api/v1/medias",
    request_body = CreateMediaDto,
    responses(
        (status = 201, description = "Media created", body = MediaDto),
        (status = 400, description = "Bad Request", body = ErrorEnvelopeDto),
    ),
    tag = "Medias"
)]
pub async fn create_media(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(dto): Json<CreateMediaDto>,
) -> Response {
    let media_type = match dto.media_type.parse::<MediaType>() {
        Ok(mt) => mt,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_parameter",
                format!("invalid media type: {}", dto.media_type),
            );
        }
    };

    let new_media = NewMedia {
        name: dto.name,
        description: dto.description,
        url: dto.url,
        media_type,
        content: dto.content,
    };

    match execute_command(
        &state,
        &headers,
        Command::CreateMedia {
            media: new_media,
            import_file: None,
        },
    )
    .await
    {
        Ok(CoreResponse::Media(m)) => {
            let secret = get_media_signing_secret(&state);
            (
                StatusCode::CREATED,
                Json(MediaDto::from(m).with_signed_url(&secret)),
            )
                .into_response()
        }
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// DELETE /api/v1/medias/{id}
#[utoipa::path(
    delete,
    path = "/api/v1/medias/{id}",
    params(("id" = i64, Path, description = "Media ID")),
    responses(
        (status = 204, description = "Media deleted"),
        (status = 404, description = "Media not found", body = ErrorEnvelopeDto),
    ),
    tag = "Medias"
)]
pub async fn delete_media(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::DeleteMedia { id }).await {
        Ok(CoreResponse::OkEmpty) => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

struct TempFileGuard {
    path: std::path::PathBuf,
    committed: bool,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// POST /api/v1/media/upload
#[utoipa::path(
    post,
    path = "/api/v1/media/upload",
    responses(
        (status = 201, description = "Media uploaded", body = MediaDto),
        (status = 400, description = "Bad Request", body = ErrorEnvelopeDto),
    ),
    tag = "Medias"
)]
pub async fn upload_media(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: axum::extract::Multipart,
) -> Response {
    let Ok(media_root) = let_timer_core::default_media_dir() else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage_error",
            "failed to locate media storage directory",
        );
    };

    if let Err(e) = std::fs::create_dir_all(&media_root) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage_error",
            format!("failed to initialize media storage directory: {e}"),
        );
    }

    let mut temp_guard: Option<TempFileGuard> = None;
    let mut file_name: Option<String> = None;
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut media_type_str: Option<String> = None;
    let mut header_buf = Vec::with_capacity(1024);
    let mut total_bytes = 0u64;
    let mut hasher = sha2::Sha256::new();

    let mut multipart_error: Option<axum::extract::multipart::MultipartError> = None;

    loop {
        match multipart.next_field().await {
            Ok(Some(mut field)) => {
                let field_name = field.name().unwrap_or("").to_string();
                match field_name.as_str() {
                    "file" => {
                        file_name = field.file_name().map(|s| s.to_string());
                        static UPLOAD_COUNTER: std::sync::atomic::AtomicU64 =
                            std::sync::atomic::AtomicU64::new(1);
                        let count =
                            UPLOAD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
                        let temp_filename =
                            format!(".tmp_upload_{}_{}_{}", std::process::id(), nanos, count);
                        let temp_path = media_root.join(temp_filename);
                        let file = match std::fs::File::create(&temp_path) {
                            Ok(f) => f,
                            Err(e) => {
                                return error_response(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "storage_error",
                                    format!("failed to create temporary upload file: {e}"),
                                );
                            }
                        };
                        let mut writer = std::io::BufWriter::new(file);
                        temp_guard = Some(TempFileGuard {
                            path: temp_path,
                            committed: false,
                        });

                        let global_max_bytes = {
                            let max_mb = std::env::var("LET_TIMER_MAX_UPLOAD_MB")
                                .ok()
                                .and_then(|v| v.parse::<u64>().ok())
                                .unwrap_or(200);
                            max_mb * 1024 * 1024
                        };

                        loop {
                            match field.chunk().await {
                                Ok(Some(chunk)) => {
                                    total_bytes += chunk.len() as u64;
                                    if total_bytes > global_max_bytes {
                                        return error_response(
                                            StatusCode::PAYLOAD_TOO_LARGE,
                                            "payload_too_large",
                                            format!(
                                                "file exceeds maximum upload limit ({global_max_bytes} bytes)"
                                            ),
                                        );
                                    }
                                    sha2::Digest::update(&mut hasher, &chunk);
                                    if header_buf.len() < 1024 {
                                        let to_copy = (1024 - header_buf.len()).min(chunk.len());
                                        header_buf.extend_from_slice(&chunk[..to_copy]);
                                    }
                                    use std::io::Write;
                                    if let Err(e) = writer.write_all(&chunk) {
                                        return error_response(
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            "io_error",
                                            format!("failed to write upload chunk to disk: {e}"),
                                        );
                                    }
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    let status = e.status();
                                    return error_response(
                                        status,
                                        if status == StatusCode::PAYLOAD_TOO_LARGE {
                                            "payload_too_large"
                                        } else {
                                            "upload_failed"
                                        },
                                        format!("error reading multipart chunk: {e}"),
                                    );
                                }
                            }
                        }
                        use std::io::Write;
                        if let Err(e) = writer.flush() {
                            return error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "io_error",
                                format!("failed to flush upload file: {e}"),
                            );
                        }
                    }
                    "name" => {
                        if let Ok(text) = field.text().await {
                            name = Some(text);
                        }
                    }
                    "description" => {
                        if let Ok(text) = field.text().await {
                            description = Some(text);
                        }
                    }
                    "media_type" => {
                        if let Ok(text) = field.text().await {
                            media_type_str = Some(text);
                        }
                    }
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(e) => {
                multipart_error = Some(e);
                break;
            }
        }
    }

    if let Some(e) = multipart_error {
        let status = e.status();
        return error_response(
            status,
            if status == StatusCode::PAYLOAD_TOO_LARGE {
                "payload_too_large"
            } else {
                "upload_failed"
            },
            format!("multipart error: {e}"),
        );
    }

    let mut guard = match temp_guard {
        Some(g) => g,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "missing_file",
                "multipart request must include a 'file' field",
            );
        }
    };

    let filename = file_name.unwrap_or_else(|| "media.bin".to_string());
    let media_type = match media_type_str
        .as_deref()
        .unwrap_or("image")
        .parse::<MediaType>()
    {
        Ok(mt) => mt,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_media_type",
                "media_type must be image, music, or poem",
            );
        }
    };

    // Validate size and magic bytes against allow-list
    match let_timer_core::validate_media(media_type, &filename, &header_buf, total_bytes) {
        Ok(_) => {}
        Err(let_timer_core::MediaValidationError::Oversized { size, max }) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                format!(
                    "file size ({size} bytes) exceeds maximum limit ({max} bytes) for {media_type:?}"
                ),
            );
        }
        Err(err) => {
            return error_response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                format!("{err}"),
            );
        }
    }

    let sub_dir = match media_type {
        MediaType::Image => "images",
        MediaType::Music => "audio",
        MediaType::Poem => "poems",
    };

    let hash = sha2::Digest::finalize(hasher);
    let hash_hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    let short_hash = &hash_hex[..12];

    let sanitized: String = filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let final_filename = format!("{short_hash}_{sanitized}");
    let target_dir = media_root.join(sub_dir);
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage_error",
            format!("failed to create target media directory: {e}"),
        );
    }

    let dest_path = target_dir.join(&final_filename);
    if let Err(e) = std::fs::rename(&guard.path, &dest_path) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage_error",
            format!("failed to move uploaded file to permanent storage: {e}"),
        );
    }
    guard.committed = true;

    let stored_url = format!("/api/v1/media/files/{sub_dir}/{final_filename}");
    let final_name = name.unwrap_or_else(|| {
        filename
            .rsplit_once('.')
            .map(|(base, _)| base.to_string())
            .unwrap_or(filename)
    });

    let new_media = NewMedia {
        name: final_name,
        description,
        url: stored_url,
        media_type,
        content: None,
    };

    match execute_command(
        &state,
        &headers,
        Command::CreateMedia {
            media: new_media,
            import_file: None,
        },
    )
    .await
    {
        Ok(CoreResponse::Media(m)) => {
            let secret = get_media_signing_secret(&state);
            (
                StatusCode::CREATED,
                Json(MediaDto::from(m).with_signed_url(&secret)),
            )
                .into_response()
        }
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err((status, code, msg)) => {
            let _ = std::fs::remove_file(&dest_path);
            error_response(status, code, msg)
        }
    }
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct MediaUrlQuery {
    pub ttl: Option<u64>,
}

/// POST /api/v1/media/{id}/url
#[utoipa::path(
    post,
    path = "/api/v1/media/{id}/url",
    params(
        ("id" = i64, Path, description = "Media ID"),
        MediaUrlQuery,
    ),
    responses(
        (status = 200, description = "Signed media URL", body = SignedMediaUrlDto),
        (status = 404, description = "Media not found", body = ErrorEnvelopeDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Medias"
)]
pub async fn get_signed_media_url(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Query(query): Query<MediaUrlQuery>,
) -> Response {
    let ttl = query.ttl.unwrap_or(3600);
    let (url, secret) = {
        let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
        let media_repo = let_timer_core::MediaRepository::new(db.conn());
        let media = match media_repo.get_by_id(id) {
            Ok(m) => m,
            Err(let_timer_core::DbError::NotFound { .. }) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    "not_found",
                    format!("media {id} not found"),
                );
            }
            Err(e) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    e.to_string(),
                );
            }
        };
        let settings_repo = let_timer_core::SettingsRepository::new(db.conn());
        let secret = settings_repo
            .get_or_create_media_signing_secret()
            .unwrap_or_default();
        (media.url, secret)
    };

    let signed_url = crate::media_auth::sign_media_url(&secret, &url, ttl);
    let expires_at = (chrono::Utc::now().timestamp() as u64) + ttl;

    Json(SignedMediaUrlDto {
        url: signed_url,
        expires_at,
    })
    .into_response()
}

/// GET /api/v1/media-lists
#[utoipa::path(
    get,
    path = "/api/v1/media-lists",
    responses(
        (status = 200, description = "List of media lists", body = Vec<MediaListDto>),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "MediaLists"
)]
pub async fn list_media_lists(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::ListMediaLists).await {
        Ok(CoreResponse::MediaLists(lists)) => {
            let dtos: Vec<MediaListDto> = lists.into_iter().map(MediaListDto::from).collect();
            Json(dtos).into_response()
        }
        Ok(_) => Json(Vec::<MediaListDto>::new()).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/media-lists
#[utoipa::path(
    post,
    path = "/api/v1/media-lists",
    request_body = CreateMediaListDto,
    responses(
        (status = 201, description = "Media list created", body = MediaListDto),
        (status = 400, description = "Bad Request", body = ErrorEnvelopeDto),
    ),
    tag = "MediaLists"
)]
pub async fn create_media_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(dto): Json<CreateMediaListDto>,
) -> Response {
    let new_list = NewMediaList {
        name: dto.name,
        description: dto.description,
    };
    match execute_command(&state, &headers, Command::CreateMediaList(new_list)).await {
        Ok(CoreResponse::MediaList(list)) => {
            (StatusCode::CREATED, Json(MediaListDto::from(list))).into_response()
        }
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// GET /api/v1/media-lists/{id}
#[utoipa::path(
    get,
    path = "/api/v1/media-lists/{id}",
    params(("id" = i64, Path, description = "Media List ID")),
    responses(
        (status = 200, description = "Media list detail with items", body = MediaListDetailDto),
        (status = 404, description = "Media list not found", body = ErrorEnvelopeDto),
    ),
    tag = "MediaLists"
)]
pub async fn get_media_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::GetMediaList { id }).await {
        Ok(CoreResponse::MediaListDetail { list, items }) => {
            let secret = get_media_signing_secret(&state);
            let detail = MediaListDetailDto {
                list: MediaListDto::from(list),
                items: items
                    .into_iter()
                    .map(|item| MediaDto::from(item).with_signed_url(&secret))
                    .collect(),
            };
            Json(detail).into_response()
        }
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// DELETE /api/v1/media-lists/{id}
#[utoipa::path(
    delete,
    path = "/api/v1/media-lists/{id}",
    params(("id" = i64, Path, description = "Media List ID")),
    responses(
        (status = 204, description = "Media list deleted"),
        (status = 404, description = "Media list not found", body = ErrorEnvelopeDto),
    ),
    tag = "MediaLists"
)]
pub async fn delete_media_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    match execute_command(&state, &headers, Command::DeleteMediaList { id }).await {
        Ok(CoreResponse::OkEmpty) => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// POST /api/v1/media-lists/{id}/medias
#[utoipa::path(
    post,
    path = "/api/v1/media-lists/{id}/medias",
    params(("id" = i64, Path, description = "Media List ID")),
    request_body = AddMediaItemDto,
    responses(
        (status = 204, description = "Media item added to list"),
        (status = 404, description = "List or Media not found", body = ErrorEnvelopeDto),
    ),
    tag = "MediaLists"
)]
pub async fn add_media_to_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(dto): Json<AddMediaItemDto>,
) -> Response {
    match execute_command(
        &state,
        &headers,
        Command::AddMediaToList {
            media_list_id: id,
            media_id: dto.media_id,
        },
    )
    .await
    {
        Ok(CoreResponse::OkEmpty) => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// DELETE /api/v1/media-lists/{id}/medias/{media_id}
#[utoipa::path(
    delete,
    path = "/api/v1/media-lists/{id}/medias/{media_id}",
    params(
        ("id" = i64, Path, description = "Media List ID"),
        ("media_id" = i64, Path, description = "Media ID")
    ),
    responses(
        (status = 204, description = "Media item removed from list"),
        (status = 404, description = "Not found", body = ErrorEnvelopeDto),
    ),
    tag = "MediaLists"
)]
pub async fn remove_media_from_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, media_id)): Path<(i64, i64)>,
) -> Response {
    match execute_command(
        &state,
        &headers,
        Command::RemoveMediaFromList {
            media_list_id: id,
            media_id,
        },
    )
    .await
    {
        Ok(CoreResponse::OkEmpty) => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

// ─── Settings Handlers ───────────────────────────────────────────────

/// GET /api/v1/settings/timezone
#[utoipa::path(
    get,
    path = "/api/v1/settings/timezone",
    responses(
        (status = 200, description = "Current configured timezone", body = TimezoneDto),
        (status = 500, description = "Internal Server Error", body = ErrorEnvelopeDto),
    ),
    tag = "Settings"
)]
pub async fn get_timezone(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match execute_command(&state, &headers, Command::GetTimezone).await {
        Ok(CoreResponse::Timezone(tz)) => Json(TimezoneDto { timezone: tz }).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}

/// PUT /api/v1/settings/timezone
#[utoipa::path(
    put,
    path = "/api/v1/settings/timezone",
    request_body = SetTimezoneDto,
    responses(
        (status = 200, description = "Timezone updated", body = TimezoneDto),
        (status = 400, description = "Invalid timezone", body = ErrorEnvelopeDto),
    ),
    tag = "Settings"
)]
pub async fn set_timezone(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(dto): Json<SetTimezoneDto>,
) -> Response {
    match execute_command(&state, &headers, Command::SetTimezone(dto.timezone)).await {
        Ok(CoreResponse::Timezone(tz)) => Json(TimezoneDto { timezone: tz }).into_response(),
        Ok(res) => Json(res).into_response(),
        Err((status, code, msg)) => error_response(status, code, msg),
    }
}
