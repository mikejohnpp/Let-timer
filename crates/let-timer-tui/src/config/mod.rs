//! Where the interface gets its settings from.
//!
//! Two files sit side by side in the config directory: `settings.toml` for a
//! handful of numbers, and `keymap.toml` for the keys. Keeping them apart means
//! the file people edit to change a key stays short enough to read.

pub mod keymap;
pub mod keymap_file;
pub mod settings;

use std::path::{Path, PathBuf};

use keymap::ParseKeyError;

/// The directory holding let-timer's config files.
///
/// `XDG_CONFIG_HOME` if it is set, otherwise `~/.config`, in both cases with
/// `let-timer` on the end. `None` when there is nowhere to look, which leaves
/// every file at its defaults.
pub fn config_home() -> Option<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME")
        && !config_home.is_empty()
    {
        return Some(PathBuf::from(config_home).join("let-timer"));
    }
    let home = std::env::var_os("HOME").filter(|home| !home.is_empty())?;
    Some(PathBuf::from(home).join(".config").join("let-timer"))
}

/// Why a config file could not be used.
///
/// The user only ever sees the message, so each case says what is wrong and
/// where, rather than which enum variant it was.
#[derive(Debug)]
pub enum ConfigError {
    /// The file could not be read.
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The file is not valid TOML, or has the wrong shape.
    Parse { path: PathBuf, message: String },
    /// A number that is not a usable value.
    OutOfRange {
        path: PathBuf,
        field: &'static str,
        value: String,
        allowed: String,
    },
    /// An action name that is not one of ours.
    UnknownAction {
        path: PathBuf,
        section: &'static str,
        name: String,
    },
    /// Keys that make no sense.
    BadKey {
        path: PathBuf,
        section: &'static str,
        action: &'static str,
        source: ParseKeyError,
    },
    /// A key bound to two actions, so one of them could never happen.
    Conflict {
        path: PathBuf,
        conflict: keymap::KeyConflict,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            ConfigError::Parse { path, message } => {
                write!(f, "{} is not usable: {message}", path.display())
            }
            ConfigError::OutOfRange {
                path,
                field,
                value,
                allowed,
            } => write!(f, "{}: {field} = {value} but {allowed}", path.display()),
            ConfigError::UnknownAction {
                path,
                section,
                name,
            } => write!(
                f,
                "{}: [{section}] has no action called {name:?}",
                path.display()
            ),
            ConfigError::Conflict { path, conflict } => {
                write!(f, "{}: {conflict}", path.display())
            }
            ConfigError::BadKey {
                path,
                section,
                action,
                source,
            } => write!(f, "{}: [{section}] {action} = {source}", path.display()),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::BadKey { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Read a config file, treating a missing file as no changes at all.
///
/// Both loaders need this, and both would rather start from the defaults than
/// complain about a file the user never wrote.
pub(crate) fn read_optional(path: &Path) -> Result<Option<String>, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ConfigError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}
