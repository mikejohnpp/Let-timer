use std::sync::Arc;

use crate::protocol::{Command, Response};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

pub struct IpcServer {
    pub listener: UnixListener,
    pub path: String,
    handler: Arc<dyn Fn(Command) -> Response + Sync + Send>,
}

impl IpcServer {
    pub async fn new(
        path: &str,
        handler: impl Fn(Command) -> Response + Sync + Send + 'static,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let socket_path = std::path::Path::new(path);
        if socket_path.exists() {
            match UnixStream::connect(path).await {
                Ok(_) => {
                    return Err(format!("daemon already running on {path}").into());
                }
                Err(_) => {
                    // Stale socket from previous crash or ungraceful shutdown.
                    let _ = std::fs::remove_file(path);
                }
            }
        }

        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(Self {
            listener,
            path: path.to_string(),
            handler: Arc::new(handler),
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tracing::info!("listening on {}", self.path);
        loop {
            tokio::select! {
                accepted = self.listener.accept() => {
                    let (stream, _) = accepted?;
                    let handler = Arc::clone(&self.handler);
                    tokio::spawn(handle_connect(stream, handler));
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("received Ctrl+C — closing accept loop");
                    break;
                }
                _ = sigterm.recv() => {
                    tracing::info!("received SIGTERM — closing accept loop");
                    break;
                }
            }
        }
        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        tracing::info!("socket removed: {}", self.path);
    }
}

async fn handle_connect(
    stream: UnixStream,
    handler: Arc<dyn Fn(Command) -> Response + Sync + Send>,
) {
    let (read_half, mut write_half) = stream.into_split();

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                break;
            }

            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let command = match serde_json::from_str::<Command>(trimmed) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        let res = Response::Error {
                            message: format!("invalid command JSON: {e}"),
                        };
                        if let Ok(out) = serde_json::to_string(&res) {
                            let _ = write_half.write_all(out.as_bytes()).await;
                            let _ = write_half.write_all(b"\n").await;
                        }
                        continue;
                    }
                };
                let res = (handler)(command);

                if let Ok(out) = serde_json::to_string(&res) {
                    let _ = write_half.write_all(out.as_bytes()).await;
                    let _ = write_half.write_all(b"\n").await;
                }
            }

            Err(_) => {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn server_replies_error_on_bad_json_and_keeps_connection() {
        let temp_dir = std::env::temp_dir().join(format!("test-ipc-bad-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let sock_path = temp_dir.join("bad-json.sock");
        let sock_str = sock_path.to_string_lossy().to_string();

        let server = IpcServer::new(&sock_str, |_cmd| Response::OkEmpty)
            .await
            .unwrap();

        let accept_handle = tokio::spawn(async move {
            let (stream, _) = server.listener.accept().await.unwrap();
            handle_connect(stream, Arc::clone(&server.handler)).await;
        });

        let client = UnixStream::connect(&sock_str).await.unwrap();
        let (read_half, mut write_half) = client.into_split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        // 1. Send invalid JSON
        write_half.write_all(b"not valid json\n").await.unwrap();
        reader.read_line(&mut line).await.unwrap();
        let res: Response = serde_json::from_str(line.trim()).unwrap();
        match res {
            Response::Error { message } => assert!(message.contains("invalid command JSON")),
            other => panic!("expected Response::Error, got {other:?}"),
        }

        // 2. Connection is still alive! Send a valid command now
        line.clear();
        let valid = serde_json::to_string(&Command::Stop { id: None }).unwrap();
        write_half
            .write_all(format!("{valid}\n").as_bytes())
            .await
            .unwrap();
        reader.read_line(&mut line).await.unwrap();
        let res2: Response = serde_json::from_str(line.trim()).unwrap();
        assert!(matches!(res2, Response::OkEmpty));

        accept_handle.abort();
        let _ = std::fs::remove_file(sock_path);
    }

    #[tokio::test]
    async fn stale_socket_is_cleaned_up_on_startup() {
        let temp_dir = std::env::temp_dir().join(format!("test-ipc-stale-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let sock_path = temp_dir.join("stale.sock");
        let sock_str = sock_path.to_string_lossy().to_string();

        // Create dummy stale socket file
        std::fs::write(&sock_path, b"stale data").unwrap();

        // Should clean up stale socket and succeed
        let server = IpcServer::new(&sock_str, |_| Response::OkEmpty).await;
        assert!(server.is_ok(), "should recover from stale socket");

        // Second server on same path should detect running daemon
        let second = IpcServer::new(&sock_str, |_| Response::OkEmpty).await;
        assert!(second.is_err(), "should detect running daemon");
        let msg = second.err().unwrap().to_string();
        assert!(msg.contains("already running"), "{msg}");

        drop(server);
        let _ = std::fs::remove_file(sock_path);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn socket_has_permissions_0600() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = std::env::temp_dir().join(format!("test-ipc-perm-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let sock_path = temp_dir.join("perm.sock");
        let sock_str = sock_path.to_string_lossy().to_string();

        let server = IpcServer::new(&sock_str, |_| Response::OkEmpty).await;
        assert!(server.is_ok());

        let meta = std::fs::metadata(&sock_path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "socket permissions should be 0600, got {mode:o}");

        drop(server);
        let _ = std::fs::remove_file(sock_path);
    }
}
