//! Host and Origin validation middleware for DNS rebinding and CSRF protection (SEC-3b).

use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;

use crate::router::{AppState, get_allowed_origins};

/// Check if a given Host header is permitted.
pub fn is_host_allowed(host_str: &str) -> bool {
    let host_trimmed = host_str.trim();
    if host_trimmed.is_empty() {
        return false;
    }

    // Extract hostname without port
    let host_no_port = if host_trimmed.starts_with('[') {
        if let Some(end_bracket) = host_trimmed.find(']') {
            &host_trimmed[..=end_bracket]
        } else {
            host_trimmed
        }
    } else if let Some(colon) = host_trimmed.find(':') {
        &host_trimmed[..colon]
    } else {
        host_trimmed
    };

    // 1. Loopback addresses are allowed by default
    if host_no_port.eq_ignore_ascii_case("localhost")
        || host_no_port == "127.0.0.1"
        || host_no_port == "[::1]"
        || host_no_port == "::1"
    {
        return true;
    }

    // 2. Check LET_TIMER_ALLOWED_HOSTS (comma-separated list of allowed hostnames/host:ports)
    if let Ok(allowed_env) = std::env::var("LET_TIMER_ALLOWED_HOSTS") {
        for allowed in allowed_env.split(',') {
            let trimmed = allowed.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.eq_ignore_ascii_case(host_trimmed)
                || trimmed.eq_ignore_ascii_case(host_no_port)
            {
                return true;
            }
        }
    }

    false
}

/// Check if a given Origin header is permitted for the request.
pub fn is_origin_allowed(origin_str: &str, host_str: &str) -> bool {
    let origin_trimmed = origin_str.trim();

    // 1. Same-origin match against current Host
    let http_prefix = format!("http://{host_str}");
    let https_prefix = format!("https://{host_str}");
    if origin_trimmed.eq_ignore_ascii_case(&http_prefix)
        || origin_trimmed.eq_ignore_ascii_case(&https_prefix)
    {
        return true;
    }

    // 2. CORS allow-list
    let allowed_origins = get_allowed_origins();
    for allowed in allowed_origins {
        if origin_trimmed.eq_ignore_ascii_case(&allowed) {
            return true;
        }
    }

    false
}

/// Middleware guarding against DNS rebinding, CSRF, and untrusted cross-origin requests.
pub async fn host_origin_guard(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();

    // Health endpoint is public and exempt from host/origin guards
    if path == "/api/v1/health" {
        return next.run(req).await;
    }

    // 1. Host header validation (DNS rebinding prevention)
    let host_header = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok());

    let host_str = match host_header {
        Some(h) if h.trim().is_empty() => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({
                    "error": "bad_request",
                    "message": "Host header cannot be empty"
                })),
            )
                .into_response();
        }
        Some(h) => h,
        None => "localhost",
    };

    if !is_host_allowed(host_str) {
        return (
            StatusCode::MISDIRECTED_REQUEST,
            axum::Json(json!({
                "error": "forbidden_host",
                "message": format!("Host '{host_str}' is not in the allowed hosts set")
            })),
        )
            .into_response();
    }

    let is_mutating = matches!(
        req.method(),
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    );

    if is_mutating {
        // 2. Origin check for mutating requests if Origin header is present
        if let Some(origin_val) = req.headers().get(axum::http::header::ORIGIN)
            && let Ok(origin_str) = origin_val.to_str()
            && !is_origin_allowed(origin_str, host_str)
        {
            return (
                StatusCode::FORBIDDEN,
                axum::Json(json!({
                    "error": "forbidden_origin",
                    "message": format!("Origin '{origin_str}' is not allowed")
                })),
            )
                .into_response();
        }

        // 3. Simple-request CSRF protection: require application/json or multipart/form-data
        if matches!(req.method(), &Method::POST | &Method::PUT | &Method::PATCH)
            && let Some(ct_val) = req.headers().get(axum::http::header::CONTENT_TYPE)
            && let Ok(ct_str) = ct_val.to_str()
        {
            let ct_lower = ct_str.to_lowercase();
            if !ct_lower.starts_with("application/json")
                && !ct_lower.starts_with("multipart/form-data")
            {
                return (
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    axum::Json(json!({
                        "error": "unsupported_media_type",
                        "message": "Content-Type must be application/json or multipart/form-data"
                    })),
                )
                    .into_response();
            }
        }

        // 4. In mode A (local loopback with no tokens), require X-Let-Timer-Client: web|cli
        let is_mode_a = {
            if state.is_loopback {
                let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
                let repo = let_timer_core::TokenRepository::new(db.conn());
                !repo.has_active_tokens().unwrap_or(false)
            } else {
                false
            }
        };

        if is_mode_a {
            let client_header = req
                .headers()
                .get("x-let-timer-client")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_lowercase());

            match client_header.as_deref() {
                Some("web") | Some("cli") => {}
                _ => {
                    return (
                        StatusCode::FORBIDDEN,
                        axum::Json(json!({
                            "error": "missing_client_header",
                            "message": "X-Let-Timer-Client header ('web' or 'cli') is required for mutating requests in unauthenticated mode"
                        })),
                    )
                        .into_response();
                }
            }
        }
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_host_allowed_loopback() {
        assert!(is_host_allowed("localhost"));
        assert!(is_host_allowed("localhost:8080"));
        assert!(is_host_allowed("127.0.0.1"));
        assert!(is_host_allowed("127.0.0.1:3000"));
        assert!(is_host_allowed("[::1]"));
        assert!(is_host_allowed("[::1]:8080"));
    }

    #[test]
    fn test_is_host_allowed_rebinding_rejected() {
        assert!(!is_host_allowed("attacker.example"));
        assert!(!is_host_allowed("attacker.example:8080"));
        assert!(!is_host_allowed("evil.com"));
    }
}
