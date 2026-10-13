//! `let-timer-http` provides the Axum HTTP router, RPC, REST, SSE, and auth layer.

pub mod auth;
pub mod handlers;
pub mod router;

pub use router::{AppState, build_router};
