use axum::Router;
use axum::routing::{delete, get, post};
use let_timer_core::{Command, Database, EventBus, RequestContext, Response};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tower_http::cors::{AllowOrigin, CorsLayer};
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

/// Get list of configured allowed CORS origins.
pub fn get_allowed_origins() -> Vec<String> {
    let mut allowed_origins: Vec<String> = Vec::new();

    // 1. From environment variable LET_TIMER_CORS_ORIGINS (comma-separated)
    if let Ok(origins_env) = std::env::var("LET_TIMER_CORS_ORIGINS") {
        for origin in origins_env.split(',') {
            let trimmed = origin.trim();
            if !trimmed.is_empty() && !allowed_origins.iter().any(|o| o == trimmed) {
                allowed_origins.push(trimmed.to_string());
            }
        }
    }

    // 2. In dev mode (LET_TIMER_DEV=1), include Vite origins
    if std::env::var("LET_TIMER_DEV")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        let dev_origins = [
            "http://localhost:5173",
            "http://localhost:5174",
            "http://127.0.0.1:5173",
            "http://127.0.0.1:5174",
        ];
        for origin in dev_origins {
            if !allowed_origins.iter().any(|o| o == origin) {
                allowed_origins.push(origin.to_string());
            }
        }
    }

    allowed_origins
}

/// Build CORS layer with allow-list from LET_TIMER_CORS_ORIGINS or dev defaults.
pub fn build_cors_layer() -> CorsLayer {
    let origin_strings = get_allowed_origins();
    let mut allowed_origins: Vec<axum::http::HeaderValue> = Vec::new();

    for origin in origin_strings {
        if let Ok(val) = axum::http::HeaderValue::from_str(&origin) {
            allowed_origins.push(val);
        }
    }

    let cors = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::RANGE,
            axum::http::HeaderName::from_static("x-let-timer-protocol"),
            axum::http::HeaderName::from_static("x-let-timer-workspace"),
            axum::http::HeaderName::from_static("x-let-timer-timezone"),
            axum::http::HeaderName::from_static("x-let-timer-token"),
            axum::http::HeaderName::from_static("x-let-timer-client"),
        ]);

    if allowed_origins.is_empty() {
        cors.allow_origin(AllowOrigin::list([]))
    } else {
        cors.allow_origin(AllowOrigin::list(allowed_origins))
    }
}

/// Publicly accessible routes exempt from authentication.
pub const PUBLIC_ROUTES: &[&str] = &["/api/v1/health"];

#[derive(Debug, serde::Deserialize)]
pub struct MediaAuthQuery {
    pub exp: Option<u64>,
    pub sig: Option<String>,
}

async fn serve_media(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::extract::Path(subpath): axum::extract::Path<String>,
    axum::extract::Query(auth_query): axum::extract::Query<MediaAuthQuery>,
    req: axum::extract::Request,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::http::header::{
        CACHE_CONTROL, CONTENT_SECURITY_POLICY, HeaderName, X_CONTENT_TYPE_OPTIONS,
    };
    use axum::response::IntoResponse;
    use serde_json::json;

    let now_unix = chrono::Utc::now().timestamp() as u64;

    // 1. Authorization check: either valid signed URL or authenticated token/loopback
    let mut authed_by_signature = false;
    let mut exp_ttl = 3600u64;

    if let (Some(exp), Some(sig)) = (auth_query.exp, auth_query.sig.as_deref()) {
        let secret = {
            let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
            let repo = let_timer_core::SettingsRepository::new(db.conn());
            repo.get_or_create_media_signing_secret()
                .unwrap_or_default()
        };

        if crate::media_auth::verify_media_signature(&secret, &subpath, exp, sig, now_unix) {
            authed_by_signature = true;
            exp_ttl = exp.saturating_sub(now_unix);
        } else {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": "unauthorized",
                    "message": "invalid or expired media signature"
                })),
            )
                .into_response();
        }
    } else {
        // Fall back to token check (Authorization: Bearer or X-Let-Timer-Token)
        let token_opt = crate::auth::extract_token(&req);
        let is_authorized = {
            let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
            let repo = let_timer_core::TokenRepository::new(db.conn());
            if let Some(ref token) = token_opt {
                matches!(repo.authenticate(token), Ok(Some(_)))
            } else {
                let has_tokens = repo.has_active_tokens().unwrap_or(false);
                state.is_loopback && !has_tokens
            }
        };

        if !is_authorized {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": "unauthorized",
                    "message": "missing API token or valid signed media URL"
                })),
            )
                .into_response();
        }
    }

    // 2. Locate file on disk safely
    let Ok(media_root) = let_timer_core::default_media_dir() else {
        return (StatusCode::NOT_FOUND, "Media file not found").into_response();
    };

    let Some(file_path) = let_timer_core::safe_join(&media_root, &subpath) else {
        return (StatusCode::NOT_FOUND, "Media file not found").into_response();
    };

    if !file_path.is_file() {
        return (StatusCode::NOT_FOUND, "Media file not found").into_response();
    }

    // 3. Serve file with Range support and security headers
    let mut resp = serve_file_from_disk(&file_path, req).await;

    // Cache-Control: private, max-age=<ttl> (SEC-2.5)
    let cache_control = if authed_by_signature {
        format!("private, max-age={exp_ttl}")
    } else {
        "private, no-cache".to_string()
    };
    if let Ok(cc_val) = axum::http::HeaderValue::from_str(&cache_control) {
        resp.headers_mut().insert(CACHE_CONTROL, cc_val);
    }

    // Security headers (SEC-5): nosniff, sandbox CSP, CORP same-origin
    resp.headers_mut().insert(
        X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    resp.headers_mut().insert(
        CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static("sandbox; default-src 'none'"),
    );
    resp.headers_mut().insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        axum::http::HeaderValue::from_static("same-origin"),
    );

    // Note: Access-Control-Allow-Origin: * is intentionally omitted (SEC-2.4)

    resp
}

/// Build the Axum router for let-timer-http.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = build_cors_layer();
    let openapi = ApiDoc::openapi();

    let public = Router::new().route("/api/v1/health", get(health));

    let media = Router::new().route("/api/v1/media/files/{*path}", get(serve_media));

    let protected = Router::new()
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
        .route("/api/v1/media/{id}/url", post(get_signed_media_url))
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
        .layer(AuthLayer::new(Arc::clone(&state)));

    let guarded = Router::new()
        .merge(protected)
        .merge(media)
        .fallback(static_or_spa_handler)
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::guard::host_origin_guard,
        ));

    Router::new()
        .merge(public)
        .merge(guarded)
        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024))
        .layer(cors)
        .with_state(state)
}

#[cfg(feature = "embed-web")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

async fn static_or_spa_handler(req: axum::extract::Request) -> axum::response::Response {
    use axum::response::IntoResponse;
    let path = req.uri().path().trim_start_matches('/').to_string();

    // 0. Unknown API requests in fallback must return 404, not the SPA shell
    if path.starts_with("api/") || path == "api" {
        return (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response();
    }

    // 1. Check disk directory override if LET_TIMER_WEB_DIR is set
    if let Ok(web_dir) = std::env::var("LET_TIMER_WEB_DIR") {
        let dir_path = std::path::Path::new(&web_dir);
        let clean = path.trim_start_matches('/');

        // Reject invalid path characters and traversal attempts outright
        if clean.contains("..") || clean.contains('\0') || clean.contains('\\') {
            return (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response();
        }

        // Check if file exists on disk
        if clean.is_empty() {
            if let Some(index_path) = let_timer_core::safe_join(dir_path, "index.html")
                && index_path.is_file()
            {
                return serve_file_from_disk(&index_path, req).await;
            }
        } else if let Some(file_path) = let_timer_core::safe_join(dir_path, clean) {
            if file_path.is_file() {
                return serve_file_from_disk(&file_path, req).await;
            }
        } else if dir_path.join(clean).is_symlink() || clean.contains('%') {
            // Path was a symlink pointing outside root or unhandled % sequence: reject with 404
            return (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response();
        }

        // SPA fallback: index.html for client-side navigation routes
        if let Some(index_path) = let_timer_core::safe_join(dir_path, "index.html")
            && index_path.is_file()
        {
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
            .header("x-let-timer-client", "cli")
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
