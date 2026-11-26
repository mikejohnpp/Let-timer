use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub current: Option<String>,
    #[serde(default)]
    pub servers: HashMap<String, ServerProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub url: String,
    pub token: Option<String>,
    pub token_file: Option<String>,
    pub token_env: Option<String>,
    pub workspace: Option<String>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(dir).join("let-timer").join("config.toml")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("let-timer")
                .join("config.toml")
        } else {
            PathBuf::from(".config")
                .join("let-timer")
                .join("config.toml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;

        // Write file atomically and set permissions 0600 on Unix
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(&temp_path, perms);
        }

        fs::rename(temp_path, path)?;
        Ok(())
    }

    pub fn resolve_token(profile: &ServerProfile) -> Option<String> {
        // Token resolution order: LET_TIMER_TOKEN -> token_env -> token_file -> inline token
        if let Ok(tok) = std::env::var("LET_TIMER_TOKEN")
            && !tok.trim().is_empty()
        {
            return Some(tok.trim().to_string());
        }

        if let Some(ref env_var) = profile.token_env
            && let Ok(tok) = std::env::var(env_var)
            && !tok.trim().is_empty()
        {
            return Some(tok.trim().to_string());
        }

        if let Some(ref file_path) = profile.token_file {
            let expanded = expand_tilde(file_path);
            if let Ok(tok) = fs::read_to_string(expanded)
                && !tok.trim().is_empty()
            {
                return Some(tok.trim().to_string());
            }
        }

        profile.token.clone()
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_toml_and_serialize() {
        let toml_str = r#"
current = "home"

[servers.home]
url = "https://timer.example.com"
token = "lt_secret"
workspace = "Study"

[servers.work]
url = "https://corp.example.com"
token_env = "MY_TOKEN_VAR"
"#;

        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.current.as_deref(), Some("home"));
        assert_eq!(cfg.servers.len(), 2);

        let home = cfg.servers.get("home").unwrap();
        assert_eq!(home.url, "https://timer.example.com");
        assert_eq!(home.workspace.as_deref(), Some("Study"));

        let token = Config::resolve_token(home);
        assert_eq!(token.as_deref(), Some("lt_secret"));
    }

    #[test]
    fn resolve_token_precedence() {
        let profile = ServerProfile {
            url: "https://example.com".to_string(),
            token: Some("inline_token".to_string()),
            token_file: None,
            token_env: Some("TEST_NON_EXISTENT_VAR_123".to_string()),
            workspace: None,
        };

        // Falls back to inline token when env is unset
        assert_eq!(
            Config::resolve_token(&profile).as_deref(),
            Some("inline_token")
        );
    }
}
