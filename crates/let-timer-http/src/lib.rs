//! `let-timer-http` provides the Axum HTTP router, RPC, REST, SSE, and auth layer.

pub mod auth;
pub mod dto;
pub mod handlers;
pub mod openapi;
pub mod policy;
pub mod rest;
pub mod router;
pub mod sse;

pub use openapi::ApiDoc;
pub use router::{AppState, build_router};
