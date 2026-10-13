use axum::Router;
use axum::routing::{get, post};
use let_timer_core::{Command, Database, EventBus, RequestContext, Response};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

use crate::auth::AuthLayer;
use crate::handlers::{create_token, health, list_tokens, me_handler, revoke_token, rpc_handler};

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

    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/rpc", post(rpc_handler))
        .route("/api/v1/auth/tokens", post(create_token).get(list_tokens))
        .route("/api/v1/auth/tokens/revoke", post(revoke_token))
        .route("/api/v1/auth/me", get(me_handler))
        .layer(AuthLayer::new(Arc::clone(&state)))
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
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
}
