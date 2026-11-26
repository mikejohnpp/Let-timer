//! Policy checks for commands arriving via remote transports (HTTP).
//!
//! Remote transports (HTTP RPC & REST) are untrusted by default. Any command
//! containing server-side paths or intended for local-only execution must be
//! rejected before dispatch.

use let_timer_core::Command;

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    ForbiddenRemoteCommand(&'static str),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::ForbiddenRemoteCommand(msg) => {
                write!(f, "forbidden remote command: {msg}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// Validate that a command is safe to execute over remote transports (HTTP).
///
/// MUST be implemented as an exhaustive match without wildcard `_`
/// so that new `Command` variants fail to compile until explicitly classified.
pub fn validate_remote(cmd: &Command) -> Result<(), PolicyError> {
    match cmd {
        // Local-only commands or commands carrying server-side filesystem paths:
        Command::CreateMedia { import_file, .. } => {
            if import_file.is_some() {
                return Err(PolicyError::ForbiddenRemoteCommand(
                    "import_file is only allowed over local Unix socket; use /api/v1/media/upload for HTTP",
                ));
            }
            Ok(())
        }

        // Allowed remote commands (exhaustive list, NO wildcard `_`):
        Command::Create(_)
        | Command::Delete { .. }
        | Command::Edit { .. }
        | Command::Find { .. }
        | Command::List(_)
        | Command::Current
        | Command::Start { .. }
        | Command::StartCandidates { .. }
        | Command::Pause { .. }
        | Command::Resume { .. }
        | Command::Stop { .. }
        | Command::Restart { .. }
        | Command::Done { .. }
        | Command::Cancel { .. }
        | Command::Reopen { .. }
        | Command::Ping
        | Command::ListWorkspaces
        | Command::CreateWorkspace(_)
        | Command::SwitchWorkspace { .. }
        | Command::CurrentWorkspace
        | Command::DeleteWorkspace { .. }
        | Command::ListMediaLists
        | Command::CreateMediaList(_)
        | Command::GetMediaList { .. }
        | Command::DeleteMediaList { .. }
        | Command::AddMediaToList { .. }
        | Command::RemoveMediaFromList { .. }
        | Command::ListMedias { .. }
        | Command::DeleteMedia { .. }
        | Command::DaemonStatus
        | Command::GetTimezone
        | Command::SetTimezone(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use let_timer_core::{MediaType, NewMedia};

    #[test]
    fn test_rejects_create_media_with_import_file() {
        let cmd = Command::CreateMedia {
            media: NewMedia {
                name: "test".into(),
                description: None,
                url: String::new(),
                media_type: MediaType::Image,
                content: None,
            },
            import_file: Some("/etc/passwd".into()),
        };
        assert!(matches!(
            validate_remote(&cmd),
            Err(PolicyError::ForbiddenRemoteCommand(_))
        ));
    }

    #[test]
    fn test_allows_create_media_without_import_file() {
        let cmd = Command::CreateMedia {
            media: NewMedia {
                name: "test".into(),
                description: None,
                url: "http://example.com/pic.png".into(),
                media_type: MediaType::Image,
                content: None,
            },
            import_file: None,
        };
        assert!(validate_remote(&cmd).is_ok());
    }

    #[test]
    fn test_allows_standard_commands() {
        assert!(validate_remote(&Command::Ping).is_ok());
        assert!(validate_remote(&Command::Current).is_ok());
        assert!(validate_remote(&Command::ListWorkspaces).is_ok());
    }
}
