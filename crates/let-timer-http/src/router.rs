use axum::Router;
use axum::routing::{delete, get, post};
use let_timer_core::{Command, Database, EventBus, RequestContext, Response};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth::AuthLayer;
use crate::handlers::{
    create_token, health, list_tokens, me_handler, metrics_handler, revoke_token, rotate_token,
    rpc_handler,
};
use crate::openapi::ApiDoc;
use crate::rest::*;

pub type CommandHandler = Arc<
    dyn Fn(
            Command,
            &Connection,
            &RequestContext,
            Option<&tokio::sync::Notify>,
            Option<&EventBus>,
        ) -> Response
        + Send
        + Sync,
>;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub is_loopback: bool,
    pub command_handler: CommandHandler,
    pub timer_notify: Option<Arc<tokio::sync::Notify>>,
    pub events: Option<Arc<EventBus>>,
}

/// Build the Axum router for let-timer-http.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let openapi = ApiDoc::openapi();

    Router::new()
        // Unauthenticated / Doc endpoints
        .route("/api/v1/health", get(health))
        .merge(SwaggerUi::new("/swagger-ui").url("/api/v1/openapi.json", openapi))
        // RPC parity endpoint
        .route("/api/v1/rpc", post(rpc_handler))
        // Real-time Event Stream (SSE)
        .route("/api/v1/events", get(crate::sse::events_handler))
        // Auth management
        .route("/api/v1/auth/tokens", post(create_token).get(list_tokens))
        .route("/api/v1/auth/tokens/revoke", post(revoke_token))
        .route("/api/v1/auth/tokens/rotate", post(rotate_token))
        .route("/api/v1/auth/me", get(me_handler))
        // Metrics endpoint
        .route("/metrics", get(metrics_handler))
        // REST: Tasks
        .route("/api/v1/tasks", get(list_tasks).post(create_task))
        .route("/api/v1/tasks/current", get(current_task))
        .route("/api/v1/tasks/start-candidates", get(start_candidates))
        .route(
            "/api/v1/tasks/{id}",
            get(get_task).patch(update_task).delete(delete_task),
        )
        .route("/api/v1/tasks/{id}/start", post(start_task))
        .route("/api/v1/tasks/{id}/pause", post(pause_task))
        .route("/api/v1/tasks/{id}/resume", post(resume_task))
        .route("/api/v1/tasks/{id}/stop", post(stop_task))
        .route("/api/v1/tasks/{id}/restart", post(restart_task))
        .route("/api/v1/tasks/{id}/done", post(done_task))
        .route("/api/v1/tasks/{id}/cancel", post(cancel_task))
        .route("/api/v1/tasks/{id}/reopen", post(reopen_task))
        // REST: Timer
        .route("/api/v1/timer", get(get_timer))
        .route("/api/v1/timer/pause", post(timer_pause))
        .route("/api/v1/timer/resume", post(timer_resume))
        .route("/api/v1/timer/stop", post(timer_stop))
        .route("/api/v1/timer/done", post(timer_done))
        // REST: Workspaces
        .route(
            "/api/v1/workspaces",
            get(list_workspaces).post(create_workspace),
        )
        .route("/api/v1/workspaces/current", get(current_workspace))
        .route("/api/v1/workspaces/switch", post(switch_workspace))
        .route("/api/v1/workspaces/{id}", delete(delete_workspace))
        // REST: Medias
        .route("/api/v1/medias", get(list_medias).post(create_media))
        .route("/api/v1/media/upload", post(upload_media))
        .route("/api/v1/medias/{id}", delete(delete_media))
        // REST: Media Lists
        .route(
            "/api/v1/media-lists",
            get(list_media_lists).post(create_media_list),
        )
        .route(
            "/api/v1/media-lists/{id}",
            get(get_media_list).delete(delete_media_list),
        )
        .route("/api/v1/media-lists/{id}/medias", post(add_media_to_list))
        .route(
            "/api/v1/media-lists/{id}/medias/{media_id}",
            delete(remove_media_from_list),
        )
        // REST: Settings
        .route(
            "/api/v1/settings/timezone",
            get(get_timezone).put(set_timezone),
        )
        // Middlewares
        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024))
        .layer(AuthLayer::new(Arc::clone(&state)))
        .layer(cors)
        .fallback(static_or_spa_handler)
        .with_state(state)
}

#[cfg(feature = "embed-web")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

async fn static_or_spa_handler(req: axum::extract::Request) -> axum::response::Response {
    use axum::response::IntoResponse;
    let path = req.uri().path().trim_start_matches('/').to_string();

    // 0. Managed Media Assets: /api/v1/media/files/*
    if let Some(media_subpath) = path.strip_prefix("api/v1/media/files/") {
        if let Ok(media_root) = let_timer_core::default_media_dir() {
            let clean_subpath = media_subpath.trim_start_matches('/');
            if !clean_subpath.contains("..") {
                let file_path = media_root.join(clean_subpath);
                if file_path.is_file() {
                    let mut resp = serve_file_from_disk(&file_path, req).await;
                    resp.headers_mut().insert(
                        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                        axum::http::HeaderValue::from_static("*"),
                    );
                    return resp;
                }
            }
        }
        return (axum::http::StatusCode::NOT_FOUND, "Media file not found").into_response();
    }

    // 1. Check disk directory override if LET_TIMER_WEB_DIR is set
    if let Ok(web_dir) = std::env::var("LET_TIMER_WEB_DIR") {
        let dir_path = std::path::Path::new(&web_dir);
        let file_path = if path.is_empty() {
            dir_path.join("index.html")
        } else {
            dir_path.join(&path)
        };

        if file_path.is_file() {
            return serve_file_from_disk(&file_path, req).await;
        }

        // SPA fallback: index.html
        let index_path = dir_path.join("index.html");
        if index_path.is_file() {
            return serve_file_from_disk(&index_path, req).await;
        }
    }

    // 2. Embedded assets
    #[cfg(feature = "embed-web")]
    {
        let asset_path = if path.is_empty() { "index.html" } else { &path };

        if let Some(file) = WebAssets::get(asset_path) {
            return serve_embedded_file(asset_path, file);
        }

        // SPA fallback to index.html
        if let Some(file) = WebAssets::get("index.html") {
            return serve_embedded_file("index.html", file);
        }
    }

    (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response()
}

async fn serve_file_from_disk(
    path: &std::path::Path,
    req: axum::extract::Request,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use tower::ServiceExt;

    match tower_http::services::ServeFile::new(path)
        .oneshot(req)
        .await
    {
        Ok(res) => res.into_response(),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

#[cfg(feature = "embed-web")]
fn serve_embedded_file(path: &str, file: rust_embed::EmbeddedFile) -> axum::response::Response {
    use axum::response::IntoResponse;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache_control = if path == "index.html" || path.ends_with(".html") {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    };

    (
        [
            (axum::http::header::CONTENT_TYPE, mime.as_ref()),
            (axum::http::header::CACHE_CONTROL, cache_control),
        ],
        file.data,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use let_timer_core::ListQuery;
    use serde_json::Value;
    use tower::ServiceExt;

    fn test_handler(
        cmd: Command,
        _conn: &Connection,
        _ctx: &RequestContext,
        _notify: Option<&tokio::sync::Notify>,
        _events: Option<&EventBus>,
    ) -> Response {
        match cmd {
            Command::Ping => Response::Pong,
            _ => Response::OkEmpty,
        }
    }

    #[tokio::test]
    async fn health_check_returns_ok() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let state = Arc::new(AppState {
            db,
            is_loopback: true,
            command_handler: Arc::new(test_handler),
            timer_notify: None,
            events: None,
        });

        let app = build_router(state);
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn rpc_requires_protocol_header() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let state = Arc::new(AppState {
            db,
            is_loopback: true,
            command_handler: Arc::new(test_handler),
            timer_notify: None,
            events: None,
        });

        let app = build_router(state);

        // Missing protocol header -> 400 Bad Request
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/rpc")
            .header("content-type", "application/json")
            .body(Body::from("\"Ping\""))
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rpc_succeeds_with_protocol_header() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let state = Arc::new(AppState {
            db,
            is_loopback: true,
            command_handler: Arc::new(test_handler),
            timer_notify: None,
            events: None,
        });

        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/rpc")
            .header("content-type", "application/json")
            .header("x-let-timer-protocol", "1")
            .body(Body::from("\"Ping\""))
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, "Pong");
    }

    #[tokio::test]
    async fn auth_enforced_when_tokens_exist() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        // Create a token in the DB
        let created = {
            let guard = db.lock().unwrap();
            let repo = let_timer_core::TokenRepository::new(guard.conn());
            repo.create("my-key").unwrap()
        };

        let state = Arc::new(AppState {
            db,
            is_loopback: true,
            command_handler: Arc::new(test_handler),
            timer_notify: None,
            events: None,
        });

        let app = build_router(state);

        // Without token -> 401
        let req1 = Request::builder()
            .method("POST")
            .uri("/api/v1/rpc")
            .header("content-type", "application/json")
            .header("x-let-timer-protocol", "1")
            .body(Body::from("\"Ping\""))
            .unwrap();
        let res1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::UNAUTHORIZED);

        // With valid token in Authorization -> 200 OK
        let req2 = Request::builder()
            .method("POST")
            .uri("/api/v1/rpc")
            .header("content-type", "application/json")
            .header("x-let-timer-protocol", "1")
            .header("authorization", format!("Bearer {}", created.token))
            .body(Body::from("\"Ping\""))
            .unwrap();
        let res2 = app.oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rest_task_lifecycle_parity_test() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let state = Arc::new(AppState {
            db: Arc::clone(&db),
            is_loopback: true,
            command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
                let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
            }),
            timer_notify: None,
            events: None,
        });

        let app = build_router(state);

        // 1. Create a task via REST POST /api/v1/tasks
        let create_body = serde_json::json!({
            "name": "Test REST task",
            "priority": 2,
            "estimated_mins": 30
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/tasks")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&create_body).unwrap()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let task_json: Value = serde_json::from_slice(&body).unwrap();
        let task_id = task_json["id"].as_i64().unwrap();
        assert_eq!(task_json["name"], "Test REST task");
        assert_eq!(task_json["status"], "pending");

        // 2. Start the task via REST POST /api/v1/tasks/{id}/start
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/tasks/{task_id}/start"))
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let start_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(start_json["state"], "running");
        assert_eq!(start_json["task"]["id"], task_id);

        // 3. Pause via REST POST /api/v1/timer/pause
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/timer/pause")
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let pause_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(pause_json["state"], "paused");

        // 4. Resume via REST POST /api/v1/timer/resume
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/timer/resume")
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let resume_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(resume_json["state"], "running");

        // 5. Complete via REST POST /api/v1/tasks/{id}/done
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/tasks/{task_id}/done"))
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let done_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(done_json["task"]["status"], "completed");
        assert_eq!(done_json["state"], "idle");

        // 6. Verify via REST GET /api/v1/tasks
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/tasks")
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let list_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list_json.as_array().unwrap().len(), 1);
        assert_eq!(list_json[0]["id"], task_id);
        assert_eq!(list_json[0]["status"], "completed");

        // 7. Verify parity with /rpc
        let rpc_req = Request::builder()
            .method("POST")
            .uri("/api/v1/rpc")
            .header("content-type", "application/json")
            .header("x-let-timer-protocol", "1")
            .body(Body::from(
                serde_json::to_string(&Command::List(ListQuery::default())).unwrap(),
            ))
            .unwrap();
        let rpc_res = app.oneshot(rpc_req).await.unwrap();
        assert_eq!(rpc_res.status(), StatusCode::OK);
        let rpc_body = rpc_res.into_body().collect().await.unwrap().to_bytes();
        let rpc_json: Value = serde_json::from_slice(&rpc_body).unwrap();
        assert_eq!(rpc_json["OkList"][0]["id"], task_id);
    }

    #[tokio::test]
    async fn rest_error_mapping_test() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let state = Arc::new(AppState {
            db: Arc::clone(&db),
            is_loopback: true,
            command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
                let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
            }),
            timer_notify: None,
            events: None,
        });

        let app = build_router(state);

        // Delete non-existent task -> 404 Not Found
        let req = Request::builder()
            .method("DELETE")
            .uri("/api/v1/tasks/999999")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let err_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(err_json["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn sse_stream_delivers_events() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let events = Arc::new(EventBus::default());
        let state = Arc::new(AppState {
            db: Arc::clone(&db),
            is_loopback: true,
            command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
                let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
            }),
            timer_notify: None,
            events: Some(Arc::clone(&events)),
        });

        let app = build_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/events")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/event-stream"
        );
    }

    #[test]
    fn openapi_spec_snapshot_matches_committed_file() {
        let generated = ApiDoc::openapi();
        let generated_json = serde_json::to_string_pretty(&generated).unwrap();

        let committed_path = std::path::Path::new("../../web/openapi.json");
        let path_to_read = if committed_path.exists() {
            committed_path
        } else {
            std::path::Path::new("web/openapi.json")
        };

        let committed_content = std::fs::read_to_string(path_to_read)
            .expect("web/openapi.json must exist and be readable");
        assert_eq!(
            generated_json.trim(),
            committed_content.trim(),
            "web/openapi.json differs from generated OpenAPI schema! Run `cargo run -p let-timer-http --bin gen-openapi` to update."
        );
    }
}
