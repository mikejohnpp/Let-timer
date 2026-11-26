use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tower::{Layer, Service};

use crate::router::AppState;

/// In-memory tracker for failed authentications keyed by peer IP (SEC-8).
#[derive(Debug, Clone)]
pub struct AuthRateLimiter {
    state: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_attempts: usize,
    window: Duration,
    max_tracked_ips: usize,
}

impl Default for AuthRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthRateLimiter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_attempts: 10,
            window: Duration::from_secs(60),
            max_tracked_ips: 10_000,
        }
    }

    /// Check if the client IP has exceeded the allowed failed authentication rate.
    /// Returns Ok(()) if allowed, or Err(retry_after_secs) if rate limited.
    pub fn check(&self, ip: &str, now: Instant) -> Result<(), u64> {
        let mut map = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(timestamps) = map.get_mut(ip) {
            let window = self.window;
            timestamps.retain(|&t| now.duration_since(t) < window);
            if timestamps.len() >= self.max_attempts {
                let oldest = timestamps[0];
                let elapsed = now.duration_since(oldest);
                let retry_after = if elapsed < window {
                    (window - elapsed).as_secs().max(1)
                } else {
                    1
                };
                return Err(retry_after);
            }
        }
        Ok(())
    }

    /// Forget all recorded failures for an IP (called after a successful
    /// authentication so a legitimate login is never locked out).
    pub fn clear(&self, ip: &str) {
        let mut map = self.state.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(ip);
    }

    /// Record a failed authentication attempt for a given IP.
    pub fn record_failure(&self, ip: &str, now: Instant) {
        let mut map = self.state.lock().unwrap_or_else(|e| e.into_inner());

        // Bounded memory protection: if at capacity, sweep expired entries
        if map.len() >= self.max_tracked_ips && !map.contains_key(ip) {
            let window = self.window;
            map.retain(|_, timestamps| {
                timestamps.retain(|&t| now.duration_since(t) < window);
                !timestamps.is_empty()
            });
            // If still full after sweep, evict an arbitrary entry
            if map.len() >= self.max_tracked_ips
                && let Some(first_key) = map.keys().next().cloned()
            {
                map.remove(&first_key);
            }
        }

        let timestamps = map.entry(ip.to_string()).or_default();
        let window = self.window;
        timestamps.retain(|&t| now.duration_since(t) < window);
        timestamps.push(now);
    }
}

/// Tower Layer for API authentication and rate limiting.
#[derive(Clone)]
pub struct AuthLayer {
    pub state: Arc<AppState>,
    pub limiter: Arc<AuthRateLimiter>,
}

impl AuthLayer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            limiter: Arc::new(AuthRateLimiter::new()),
        }
    }

    pub fn with_limiter(state: Arc<AppState>, limiter: Arc<AuthRateLimiter>) -> Self {
        Self { state, limiter }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            state: Arc::clone(&self.state),
            limiter: Arc::clone(&self.limiter),
        }
    }
}

/// Tower Service for API authentication and rate limiting.
#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    state: Arc<AppState>,
    limiter: Arc<AuthRateLimiter>,
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
        let limiter = Arc::clone(&self.limiter);

        Box::pin(async move {
            let path = req.uri().path().to_string();

            // 1. Unauthenticated endpoints (only explicit public allow-list)
            if crate::router::PUBLIC_ROUTES.contains(&path.as_str()) {
                return inner.call(req).await;
            }

            let client_ip = req
                .extensions()
                .get::<crate::router::ClientIp>()
                .map(|c| c.0.clone())
                .unwrap_or_else(|| crate::router::extract_client_ip(&req));

            // 2. Extract token and validate FIRST so a legitimate login is
            // never locked out by the rate limiter below (a valid token
            // always passes and clears the IP's failure counter).
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
                AuthCheckResult::Authorized => {
                    limiter.clear(&client_ip);
                    inner.call(req).await
                }
                AuthCheckResult::Unauthorized(status, msg) => {
                    // 3. Only failed attempts consult the rate limiter (SEC-8).
                    let now = Instant::now();
                    if let Err(retry_after) = limiter.check(&client_ip, now) {
                        tracing::warn!(
                            target: "audit",
                            ip = %client_ip,
                            path = %path,
                            retry_after = %retry_after,
                            "Authentication rate limited"
                        );
                        return Ok((
                            StatusCode::TOO_MANY_REQUESTS,
                            [(
                                axum::http::HeaderName::from_static("retry-after"),
                                axum::http::HeaderValue::from(retry_after),
                            )],
                            Json(json!({
                                "error": {
                                    "code": "rate_limited",
                                    "message": "Too many failed authentication attempts. Please retry after 60 seconds."
                                }
                            })),
                        )
                            .into_response());
                    }
                    limiter.record_failure(&client_ip, Instant::now());

                    tracing::warn!(
                        target: "audit",
                        path = %path,
                        ip = %client_ip,
                        reason = %msg,
                        "Authentication failed"
                    );
                    Ok((
                        status,
                        Json(json!({
                            "error": {
                                "code": "unauthorized",
                                "message": msg
                            }
                        })),
                    )
                        .into_response())
                }
                AuthCheckResult::DatabaseError(msg) => {
                    tracing::error!(
                        target: "audit",
                        path = %path,
                        ip = %client_ip,
                        error = %msg,
                        "Authentication database error"
                    );
                    Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
                }
            }
        })
    }
}

pub fn extract_token<B>(req: &Request<B>) -> Option<String> {
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

    None
}
