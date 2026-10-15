use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use let_timer_core::{Command, RequestContext, WorkspaceScope};

use crate::router::AppState;

/// GET /api/v1/health
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let db_ok = db.conn().query_row("SELECT 1", [], |_| Ok(())).is_ok();
    (
        StatusCode::OK,
        Json(json!({
            "status": if db_ok { "ok" } else { "degraded" },
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

/// POST /api/v1/rpc
/// Takes JSON-encoded Command and returns Response JSON.
/// Requires header: `X-Let-Timer-Protocol: 1`
pub async fn rpc_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(cmd): Json<Command>,
) -> Response {
    // 1. Check protocol header
    let proto_version = headers
        .get("x-let-timer-protocol")
        .and_then(|v| v.to_str().ok());

    if proto_version != Some("1") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "code": "invalid_protocol",
                    "message": "missing or unsupported X-Let-Timer-Protocol header (expected '1')"
                }
            })),
        )
            .into_response();
    }

    // 2. Validate remote command policy (SEC-1)
    if let Err(err) = crate::policy::validate_remote(&cmd) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": {
                    "code": "forbidden_remote_command",
                    "message": err.to_string(),
                }
            })),
        )
            .into_response();
    }

    // 3. Extract workspace override header if present: X-Let-Timer-Workspace
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
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let state_clone = Arc::clone(&state);
    let notify_opt = state.timer_notify.clone();

    // 3. Offload command execution to spawn_blocking
    let result = tokio::task::spawn_blocking(move || {
        let db = state_clone.db.lock().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now();
        let ctx = RequestContext::from_settings(db.conn(), now, tz_header.as_deref(), ws_header);
        let notify_ref = notify_opt.as_deref();
        let events_ref = state_clone.events.as_deref();

        let handler = &state_clone.command_handler;
        handler(cmd, db.conn(), &ctx, notify_ref, events_ref)
    })
    .await;

    match result {
        Ok(res) => Json(res).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "task_failed",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

// ─── Auth Handlers ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTokenReq {
    pub name: String,
}

/// POST /api/v1/auth/tokens
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTokenReq>,
) -> Response {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let repo = let_timer_core::TokenRepository::new(db.conn());
    match repo.create(&payload.name) {
        Ok(created) => (StatusCode::CREATED, Json(created)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "create_token_failed",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// GET /api/v1/auth/tokens
pub async fn list_tokens(State(state): State<Arc<AppState>>) -> Response {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let repo = let_timer_core::TokenRepository::new(db.conn());
    match repo.list_all() {
        Ok(tokens) => Json(tokens).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "list_tokens_failed",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct RevokeTokenReq {
    pub name: String,
}

/// POST /api/v1/auth/tokens/revoke
pub async fn revoke_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RevokeTokenReq>,
) -> Response {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let repo = let_timer_core::TokenRepository::new(db.conn());
    match repo.revoke(&payload.name) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "not_found",
                "message": format!("token '{}' not found or already revoked", payload.name)
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "revoke_failed",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// GET /api/v1/auth/me
pub async fn me_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim());

    if let Some(t) = token {
        let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
        let repo = let_timer_core::TokenRepository::new(db.conn());
        if let Ok(Some(tok)) = repo.authenticate(t) {
            return Json(json!({
                "authenticated": true,
                "name": tok.name,
                "created_at": tok.created_at,
                "last_used_at": tok.last_used_at
            }))
            .into_response();
        }
    }

    Json(json!({
        "authenticated": false,
        "mode": if state.is_loopback { "loopback" } else { "unauthenticated" }
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct RotateTokenReq {
    pub name: String,
}

/// POST /api/v1/auth/tokens/rotate
pub async fn rotate_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RotateTokenReq>,
) -> Response {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let repo = let_timer_core::TokenRepository::new(db.conn());
    match repo.rotate(&payload.name) {
        Ok(created) => (
            StatusCode::OK,
            Json(json!({
                "token": created.token,
                "token_info": {
                    "id": created.id,
                    "name": created.name,
                    "created_at": created.created_at,
                    "last_used_at": null,
                    "revoked_at": null
                }
            })),
        )
            .into_response(),
        Err(let_timer_core::DbError::NotFound { .. }) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "not_found",
                "message": format!("token '{}' not found", payload.name)
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "rotate_failed",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// GET /metrics
pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> Response {
    let (tasks_count, active_sessions, tokens_count) = {
        let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
        let tc: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap_or(0);
        let sc: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM task_sessions WHERE ended_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let tkc: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM api_tokens WHERE revoked_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        (tc, sc, tkc)
    };

    let metrics_text = format!(
        "# HELP let_timer_tasks_total Total number of tasks in database\n\
         # TYPE let_timer_tasks_total gauge\n\
         let_timer_tasks_total {}\n\
         # HELP let_timer_active_sessions Active running task sessions\n\
         # TYPE let_timer_active_sessions gauge\n\
         let_timer_active_sessions {}\n\
         # HELP let_timer_active_tokens Active API tokens\n\
         # TYPE let_timer_active_tokens gauge\n\
         let_timer_active_tokens {}\n",
        tasks_count, active_sessions, tokens_count
    );

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics_text,
    )
        .into_response()
}
