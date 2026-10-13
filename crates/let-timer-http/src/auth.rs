use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tower::{Layer, Service};

use crate::router::AppState;

/// Tower Layer for API authentication.
#[derive(Clone)]
pub struct AuthLayer {
    pub state: Arc<AppState>,
}

impl AuthLayer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            state: Arc::clone(&self.state),
        }
    }
}

/// Tower Service for API authentication.
#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    state: Arc<AppState>,
}

impl<S, ReqBody> Service<Request<ReqBody>> for AuthService<S>
where
    S: Service<Request<ReqBody>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();
        let state = Arc::clone(&self.state);

        Box::pin(async move {
            let path = req.uri().path();

            // 1. Unauthenticated endpoints
            if path == "/api/v1/health"
                || path == "/api/v1/auth/login"
                || path == "/api/v1/openapi.json"
                || path.starts_with("/swagger-ui")
                || !path.starts_with("/api")
            {
                return inner.call(req).await;
            }

            // 2. Extract token from Authorization or X-Let-Timer-Token
            let token_opt = extract_token(&req);

            // Perform DB checks synchronously in a block so no MutexGuard lives across await
            enum AuthCheckResult {
                Authorized,
                Unauthorized(StatusCode, &'static str),
                DatabaseError(String),
            }

            let check_result = {
                let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
                let repo = let_timer_core::TokenRepository::new(db.conn());

                if let Some(ref token) = token_opt {
                    match repo.authenticate(token) {
                        Ok(Some(_)) => AuthCheckResult::Authorized,
                        Ok(None) => AuthCheckResult::Unauthorized(
                            StatusCode::UNAUTHORIZED,
                            "invalid or revoked API token",
                        ),
                        Err(e) => AuthCheckResult::DatabaseError(e.to_string()),
                    }
                } else {
                    let has_tokens = repo.has_active_tokens().unwrap_or(false);
                    if state.is_loopback && !has_tokens {
                        AuthCheckResult::Authorized
                    } else {
                        AuthCheckResult::Unauthorized(
                            StatusCode::UNAUTHORIZED,
                            "missing API token in Authorization header or X-Let-Timer-Token",
                        )
                    }
                }
            };

            match check_result {
                AuthCheckResult::Authorized => inner.call(req).await,
                AuthCheckResult::Unauthorized(status, msg) => Ok((
                    status,
                    Json(json!({
                        "error": "unauthorized",
                        "message": msg
                    })),
                )
                    .into_response()),
                AuthCheckResult::DatabaseError(msg) => Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "database_error",
                        "message": msg
                    })),
                )
                    .into_response()),
            }
        })
    }
}

fn extract_token<B>(req: &Request<B>) -> Option<String> {
    if let Some(auth_val) = req.headers().get("Authorization")
        && let Ok(auth_str) = auth_val.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
    {
        return Some(token.trim().to_string());
    }

    if let Some(custom_val) = req.headers().get("X-Let-Timer-Token")
        && let Ok(custom_str) = custom_val.to_str()
    {
        return Some(custom_str.trim().to_string());
    }

    if let Some(cookie_val) = req.headers().get("Cookie")
        && let Ok(cookie_str) = cookie_val.to_str()
    {
        for part in cookie_str.split(';') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("let_timer_token=") {
                return Some(val.to_string());
            }
        }
    }

    if let Some(query) = req.uri().query() {
        for part in query.split('&') {
            if let Some(val) = part.strip_prefix("token=") {
                return Some(val.to_string());
            }
        }
    }

    None
}
