use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio::sync::broadcast::error::RecvError;

use let_timer_core::{Command, Event, Response as CoreResponse};

use crate::dto::*;
use crate::router::AppState;

/// GET /api/v1/events (Server-Sent Events)
///
/// On connect, immediately transmits the current `snapshot`.
/// Subsequently forwards domain events:
/// - `TimerChanged` -> event: snapshot
/// - `TimerExpired` -> event: timer_expired
/// - `WorkspaceChanged` -> event: workspace_changed
///
/// Keep-alive ping sent every 15 seconds.
/// Lagging subscribers receive an event: `resync` and continue.
pub async fn events_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    // 1. Fetch current snapshot for immediate delivery on connect
    let initial_snapshot = {
        let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
        let now = chrono::Utc::now();
        let ctx = let_timer_core::RequestContext::from_settings(db.conn(), now, None, None);
        let handler = &state.command_handler;
        let res = handler(Command::Current, db.conn(), &ctx, None, None);
        match res {
            CoreResponse::Timer(snap) => Some(TimerSnapshotDto::from(snap)),
            _ => None,
        }
    };

    let rx = state.events.as_ref().map(|bus| bus.subscribe());

    let stream = async_stream::stream! {
        // Send initial snapshot on connect if available or idle
        if let Some(snap) = initial_snapshot
            && let Ok(data) = serde_json::to_string(&snap)
        {
            yield Ok(SseEvent::default().event("snapshot").data(data));
        }

        if let Some(mut rx) = rx {
            loop {
                match rx.recv().await {
                    Ok(event) => match event {
                        Event::TimerChanged(snap) => {
                            let dto = TimerSnapshotDto::from(snap);
                            if let Ok(data) = serde_json::to_string(&dto) {
                                yield Ok(SseEvent::default().event("snapshot").data(data));
                            }
                        }
                        Event::TimerExpired { task, estimate_mins } => {
                            let payload = serde_json::json!({
                                "task": TaskDto::from(task),
                                "estimate_mins": estimate_mins,
                            });
                            if let Ok(data) = serde_json::to_string(&payload) {
                                yield Ok(SseEvent::default().event("timer_expired").data(data));
                            }
                        }
                        Event::WorkspaceChanged(ws) => {
                            let dto = WorkspaceDto::from(ws);
                            if let Ok(data) = serde_json::to_string(&dto) {
                                yield Ok(SseEvent::default().event("workspace_changed").data(data));
                            }
                        }
                    },
                    Err(RecvError::Lagged(_)) => {
                        // Notify lagging client to resync
                        yield Ok(SseEvent::default().event("resync").data("lagged"));
                    }
                    Err(RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
