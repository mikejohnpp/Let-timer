use std::fmt;

/// Errors that can occur during database operations.
#[derive(Debug)]
pub enum DbError {
    /// A rusqlite error occurred.
    Sqlite(rusqlite::Error),
    /// The requested task was not found.
    NotFound(i64),
    /// An invalid status string was encountered.
    InvalidStatus(String),
    /// Attempted to start a task while another is already in progress.
    AlreadyInProgress,
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::Sqlite(e) => write!(f, "database error: {e}"),
            DbError::NotFound(id) => write!(f, "task not found: id={id}"),
            DbError::InvalidStatus(s) => write!(f, "invalid task status: \"{s}\""),
            DbError::AlreadyInProgress => {
                write!(f, "another task is already in progress")
            }
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::Sqlite(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        DbError::Sqlite(err)
    }
}
