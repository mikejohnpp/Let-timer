use serde::{Deserialize, Serialize};

use super::error::DbError;

// ─── Priority ────────────────────────────────────────────────────────

/// Priority level of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    NotYet = 0,
    Immediate = 1,
    Urgent = 2,
}

impl Priority {
    /// Convert an integer stored in the database to a `Priority`.
    pub fn from_i64(value: i64) -> Result<Self, DbError> {
        match value {
            0 => Ok(Priority::NotYet),
            1 => Ok(Priority::Immediate),
            2 => Ok(Priority::Urgent),
            _ => Err(DbError::InvalidStatus(format!(
                "unknown priority value: {value}"
            ))),
        }
    }
}

// ─── TaskStatus ──────────────────────────────────────────────────────

/// Current status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
}

impl TaskStatus {
    /// Database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = DbError;
    fn from_str(s: &str) -> Result<Self, DbError> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            _ => Err(DbError::InvalidStatus(s.to_string())),
        }
    }
}

// ─── Task ────────────────────────────────────────────────────────────

/// A task entry stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub status: TaskStatus,
    pub estimated_mins: Option<i64>,
    pub elapsed_secs: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ─── NewTask ─────────────────────────────────────────────────────────

/// Input for creating a new task (no id or timestamps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub name: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub estimated_mins: Option<i64>,
}

// ─── UpdateTask ──────────────────────────────────────────────────────

/// Fields that can be updated on an existing task.
/// `None` means "don't change". For nullable columns like `description`,
/// `Some(None)` means "set to NULL" and `Some(Some(v))` means "set to v".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTask {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub priority: Option<Priority>,
    pub estimated_mins: Option<Option<i64>>,
}
