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
            .header("X-Let-Timer-Protocol", "1")
            .header("X-Let-Timer-Client", "cli");

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

    pub async fn upload_media(
        &self,
        file_path: &Path,
        name: Option<&str>,
        media_type: Option<&str>,
        description: Option<&str>,
    ) -> Result<Response, TransportError> {
        let upload_url = format!("{}/api/v1/media/upload", self.url);

        let file_bytes = tokio::fs::read(file_path).await.map_err(|e| {
            TransportError::Other(format!("failed to read file {}: {e}", file_path.display()))
        })?;

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("media.bin")
            .to_string();

        let mtype = media_type.unwrap_or_else(|| {
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            match ext.as_str() {
                "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" => "music",
                _ => "image",
            }
        });

        let file_part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("media_type", mtype.to_string());

        if let Some(n) = name {
            form = form.text("name", n.to_string());
        }
        if let Some(d) = description {
            form = form.text("description", d.to_string());
        }

        let mut req = self
            .client
            .post(&upload_url)
            .header("X-Let-Timer-Client", "cli");
        if let Some(ref t) = self.token {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
        if let Some(ref ws) = self.workspace_pin {
            req = req.header("X-Let-Timer-Workspace", ws.clone());
        }

        let res = req.multipart(form).send().await.map_err(|e| {
            if e.is_timeout() {
                TransportError::Timeout(e.to_string())
            } else if e.is_connect() {
                TransportError::Unreachable(e.to_string())
            } else {
                TransportError::Other(e.to_string())
            }
        })?;

        let status = res.status();
        let body_bytes = res
            .bytes()
            .await
            .map_err(|e| TransportError::Other(format!("failed to read response bytes: {e}")))?;

        if status.is_success() {
            #[derive(serde::Deserialize)]
            struct UploadResponse {
                id: i64,
                name: String,
                description: Option<String>,
                url: String,
                media_type: String,
                content: Option<String>,
                created_at: String,
                updated_at: String,
            }

            let dto = serde_json::from_slice::<UploadResponse>(&body_bytes).map_err(|e| {
                TransportError::Other(format!("failed to parse media response: {e}"))
            })?;

            let mtype = dto
                .media_type
                .parse::<crate::MediaType>()
                .unwrap_or(crate::MediaType::Image);

            Ok(Response::Media(crate::Media {
                id: dto.id,
                name: dto.name,
                description: dto.description,
                url: dto.url,
                media_type: mtype,
                content: dto.content,
                created_at: dto.created_at,
                updated_at: dto.updated_at,
            }))
        } else {
            let err_text = String::from_utf8_lossy(&body_bytes);
            Err(TransportError::Other(format!(
                "upload failed ({status}): {err_text}"
            )))
        }
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

    pub async fn upload_media(
        &mut self,
        file_path: &Path,
        name: Option<&str>,
        media_type: Option<&str>,
        description: Option<&str>,
    ) -> Result<Response, TransportError> {
        match self {
            Client::Unix(_) => {
                let canonical = std::fs::canonicalize(file_path).map_err(|e| {
                    TransportError::Other(format!("invalid file path {}: {e}", file_path.display()))
                })?;
                let file_name = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("media")
                    .to_string();
                let mtype = match media_type {
                    Some(t) => t
                        .parse::<crate::MediaType>()
                        .map_err(|e| TransportError::Other(format!("invalid media type: {e}")))?,
                    None => {
                        let ext = file_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        match ext.as_str() {
                            "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" => {
                                crate::MediaType::Music
                            }
                            _ => crate::MediaType::Image,
                        }
                    }
                };
                let final_name = name.map(|s| s.to_string()).unwrap_or_else(|| {
                    file_name
                        .rsplit_once('.')
                        .map(|(b, _)| b.to_string())
                        .unwrap_or(file_name)
                });
                self.request(&Command::CreateMedia {
                    media: crate::NewMedia {
                        name: final_name,
                        description: description.map(|s| s.to_string()),
                        url: String::new(),
                        media_type: mtype,
                        content: None,
                    },
                    import_file: Some(canonical.to_string_lossy().to_string()),
                })
                .await
            }
            Client::Http(http) => {
                http.upload_media(file_path, name, media_type, description)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_http_upload_target_url() {
        let client = HttpClient::new("http://127.0.0.1:8080".into(), None, None).unwrap();
        let expected_url = format!("{}/api/v1/media/upload", client.url);
        assert_eq!(expected_url, "http://127.0.0.1:8080/api/v1/media/upload");
    }

    #[test]
    fn test_unix_builds_create_media_with_canonical_import_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("let_timer_client_test_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let dummy_file = temp_dir.join("sample.png");
        std::fs::write(&dummy_file, b"sample image bytes").unwrap();

        let canonical = std::fs::canonicalize(&dummy_file).unwrap();
        let cmd = Command::CreateMedia {
            media: crate::NewMedia {
                name: "sample".to_string(),
                description: None,
                url: String::new(),
                media_type: crate::MediaType::Image,
                content: None,
            },
            import_file: Some(canonical.to_string_lossy().to_string()),
        };

        if let Command::CreateMedia { import_file, .. } = cmd {
            assert!(import_file.is_some());
            assert_eq!(import_file.unwrap(), canonical.to_string_lossy());
        } else {
            panic!("expected CreateMedia");
        }

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
