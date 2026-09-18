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
        let listener = UnixListener::bind(path)?;
        Ok(Self {
            listener,
            path: path.to_string(),
            handler: Arc::new(handler),
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        println!("listening on {}", self.path);
        loop {
            tokio::select! {
                accepted = self.listener.accept() => {
                    let (stream, _) = accepted?;
                    let handler = Arc::clone(&self.handler);
                    tokio::spawn(handle_connect(stream, handler));
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("received Ctrl+C — closing accept loop");
                    break;
                }
                _ = sigterm.recv() => {
                    println!("received SIGTERM — closing accept loop");
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
        println!("socket removed: {}", self.path);
    }
}

async fn handle_connect(
    stream: UnixStream,
    _handler: Arc<dyn Fn(Command) -> Response + Sync + Send>,
) {
    let (read_half, mut write_half) = stream.into_split();

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("Client disconected");
                break;
            }

            Ok(_) => {
                println!("Client invoke command: {}", line);
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let command = match serde_json::from_str::<Command>(trimmed) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        eprintln!("Bad json: {e}");
                        break;
                    }
                };
                let res = (_handler)(command);
                println!("Res of handler: {:?}", res);

                let out = serde_json::to_string(&res).expect("serialize");
                write_half.write_all(out.as_bytes()).await.ok();
                write_half.write_all(b"\n").await.ok();
                break;
            }

            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        }
    }
}
