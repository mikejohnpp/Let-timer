//! Test helpers: a throwaway daemon on a real Unix socket.
//!
//! The actor is only worth testing against an actual socket, since the whole
//! reason it exists is that the socket is hard to use directly. These helpers
//! speak the same newline-delimited JSON protocol the real daemon does.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use let_timer_core::Response;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

/// How a test daemon should behave once a command arrives.
pub(crate) enum Behaviour {
    /// Answer every command with this reply.
    Answer(Response),
    /// Answer the first command only after this delay, and everything after
    /// that straight away.
    SlowFirst {
        delay: std::time::Duration,
        response: Response,
    },
    /// Accept the connection, read the command, then hang without replying.
    Hang,
    /// Accept the connection and drop it immediately.
    DropConnection,
}

static NEXT_SOCKET: AtomicU64 = AtomicU64::new(0);

/// A socket path no other test is using.
pub(crate) fn socket_path() -> PathBuf {
    let n = NEXT_SOCKET.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("let-timer-tui-{}-{n}.sock", std::process::id()))
}

/// Start a daemon on `path` that behaves as `behaviour`.
///
/// The socket file is removed when the returned guard is dropped, so a failed
/// test does not leave debris behind.
pub(crate) async fn spawn_daemon(path: PathBuf, behaviour: Behaviour) -> Daemon {
    let listener = UnixListener::bind(&path).expect("bind test socket");
    let guard = Daemon { path: path.clone() };

    tokio::spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        serve(stream, behaviour).await;
    });

    guard
}

/// Removes the socket file when dropped.
pub(crate) struct Daemon {
    path: PathBuf,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Read one command from `stream` and act on it.
async fn serve(stream: UnixStream, behaviour: Behaviour) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    let mut seen_first = false;

    while let Ok(n) = reader.read_line(&mut line).await {
        if n == 0 || line.trim().is_empty() {
            break;
        }

        match &behaviour {
            Behaviour::DropConnection => return,
            Behaviour::Hang => {
                // Never answer: the client must give up on its own.
                std::future::pending::<()>().await;
            }
            Behaviour::SlowFirst { delay, response } if !seen_first => {
                seen_first = true;
                tokio::time::sleep(*delay).await;
                let out = serde_json::to_string(response).expect("serialize reply");
                if write_half.write_all(out.as_bytes()).await.is_err() {
                    break;
                }
                let _ = write_half.write_all(b"\n").await;
            }
            Behaviour::SlowFirst { response, .. } | Behaviour::Answer(response) => {
                let out = serde_json::to_string(response).expect("serialize reply");
                if write_half.write_all(out.as_bytes()).await.is_err() {
                    break;
                }
                let _ = write_half.write_all(b"\n").await;
            }
        }

        line.clear();
    }
}
