use std::fmt;
use std::path::Path;

use crate::ipc::client::IpcClient;
use crate::protocol::{Command, Response};

#[derive(Debug)]
pub enum TransportError {
    Unreachable(String),
    Unauthorized(String),
    ProtocolMismatch(String),
    Timeout(String),
    Other(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Unreachable(msg) => write!(f, "unreachable: {msg}"),
            TransportError::Unauthorized(msg) => write!(f, "unauthorized: {msg}"),
            TransportError::ProtocolMismatch(msg) => write!(f, "protocol mismatch: {msg}"),
            TransportError::Timeout(msg) => write!(f, "request timed out: {msg}"),
            TransportError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for TransportError {}

pub enum Client {
    Unix(IpcClient),
}

impl Client {
    pub async fn connect_unix(path: &Path) -> Result<Self, TransportError> {
        let client = IpcClient::connect(path)
            .await
            .map_err(|e| TransportError::Unreachable(e.to_string()))?;
        Ok(Client::Unix(client))
    }

    pub async fn request(&mut self, command: &Command) -> Result<Response, TransportError> {
        match self {
            Client::Unix(ipc) => ipc
                .request(command)
                .await
                .map_err(|e| TransportError::Other(e.to_string())),
        }
    }
}
