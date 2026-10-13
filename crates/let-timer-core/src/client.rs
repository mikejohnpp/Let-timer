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

#[derive(Debug, Clone)]
pub struct HttpClient {
    pub url: String,
    pub token: Option<String>,
    pub workspace_pin: Option<String>,
    pub client: reqwest::Client,
}

impl HttpClient {
    pub fn new(
        url: String,
        token: Option<String>,
        workspace_pin: Option<String>,
    ) -> Result<Self, TransportError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| TransportError::Other(e.to_string()))?;

        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            token,
            workspace_pin,
            client,
        })
    }

    pub async fn request(&self, command: &Command) -> Result<Response, TransportError> {
        let rpc_url = format!("{}/api/v1/rpc", self.url);
        let mut req = self
            .client
            .post(&rpc_url)
            .header("X-Let-Timer-Protocol", "1");

        if let Some(ref t) = self.token {
            req = req.header("Authorization", format!("Bearer {t}"));
        }

        if let Some(ref ws) = self.workspace_pin {
            req = req.header("X-Let-Timer-Workspace", ws.clone());
        }

        let res = req.json(command).send().await.map_err(|e| {
            if e.is_timeout() {
                TransportError::Timeout(e.to_string())
            } else if e.is_connect() {
                TransportError::Unreachable(e.to_string())
            } else {
                TransportError::Other(e.to_string())
            }
        })?;

        let status = res.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(TransportError::Unauthorized(
                "token rejected or missing".to_string(),
            ));
        }

        let body_bytes = res
            .bytes()
            .await
            .map_err(|e| TransportError::Other(format!("failed to read response bytes: {e}")))?;

        if status == reqwest::StatusCode::UPGRADE_REQUIRED
            || status == reqwest::StatusCode::BAD_REQUEST
        {
            let body_text = String::from_utf8_lossy(&body_bytes);
            if body_text.contains("invalid_protocol") || body_text.contains("protocol") {
                return Err(TransportError::ProtocolMismatch(
                    "protocol mismatch; upgrade the CLI or server".to_string(),
                ));
            }
        }

        serde_json::from_slice::<Response>(&body_bytes)
            .map_err(|e| TransportError::Other(format!("failed to parse server response: {e}")))
    }
}

pub enum Client {
    Unix(IpcClient),
    Http(HttpClient),
}

impl Client {
    pub async fn connect_unix(path: &Path) -> Result<Self, TransportError> {
        let client = IpcClient::connect(path)
            .await
            .map_err(|e| TransportError::Unreachable(e.to_string()))?;
        Ok(Client::Unix(client))
    }

    pub fn http(
        url: String,
        token: Option<String>,
        workspace_pin: Option<String>,
    ) -> Result<Self, TransportError> {
        let client = HttpClient::new(url, token, workspace_pin)?;
        Ok(Client::Http(client))
    }

    pub async fn request(&mut self, command: &Command) -> Result<Response, TransportError> {
        match self {
            Client::Unix(ipc) => ipc
                .request(command)
                .await
                .map_err(|e| TransportError::Other(e.to_string())),
            Client::Http(http) => http.request(command).await,
        }
    }
}
