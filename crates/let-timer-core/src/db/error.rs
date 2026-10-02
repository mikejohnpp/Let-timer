use std::fmt;

/// Errors that can occur during database operations.
#[derive(Debug)]
pub enum DbError {
    /// A rusqlite error occurred.
    Sqlite(rusqlite::Error),
    /// A row with the given id does not exist in the given entity table.
    NotFound { entity: &'static str, id: i64 },
    /// An invalid enum string was stored in / passed to the database.
    InvalidStatus(String),
    /// A row that must be unique already exists (e.g. duplicate membership).
    AlreadyExists { entity: &'static str },
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::Sqlite(e) => write!(f, "database error: {e}"),
            DbError::NotFound { entity, id } => write!(f, "{entity} not found: id={id}"),
            DbError::InvalidStatus(s) => write!(f, "invalid status: \"{s}\""),
            DbError::AlreadyExists { entity } => write!(f, "{entity} already exists"),
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
