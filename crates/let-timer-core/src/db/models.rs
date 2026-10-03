use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;
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

/// A priority value that is not a known priority level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePriorityError {
    /// The value the user supplied.
    pub value: String,
}

impl fmt::Display for ParsePriorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid priority value: {}", self.value)
    }
}

impl std::error::Error for ParsePriorityError {}

impl Priority {
    /// Database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::NotYet => "not-yet",
            Priority::Immediate => "immediate",
            Priority::Urgent => "urgent",
        }
    }

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

impl FromStr for Priority {
    type Err = ParsePriorityError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "urgent" => Ok(Priority::Urgent),
            "immediate" => Ok(Priority::Immediate),
            "not-yet" => Ok(Priority::NotYet),
            other => Err(ParsePriorityError {
                value: other.to_string(),
            }),
        }
    }
}

// ─── TaskStatus ──────────────────────────────────────────────────────

/// Current status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl TaskStatus {
    /// Database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in-progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Cancelled => "cancelled",
        }
    }
}

impl FromStr for TaskStatus {
    type Err = DbError;
    fn from_str(s: &str) -> Result<Self, DbError> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in-progress" => Ok(TaskStatus::InProgress),
            "completed" => Ok(TaskStatus::Completed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            _ => Err(DbError::InvalidStatus(s.to_string())),
        }
    }
}

// ─── MediaType ───────────────────────────────────────────────────────

/// Kind of media stored in the `medias` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Image,
    Music,
    Poem,
}

impl MediaType {
    /// Database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "image",
            MediaType::Music => "music",
            MediaType::Poem => "poem",
        }
    }
}

impl FromStr for MediaType {
    type Err = DbError;
    fn from_str(s: &str) -> Result<Self, DbError> {
        match s {
            "image" => Ok(MediaType::Image),
            "music" => Ok(MediaType::Music),
            "poem" => Ok(MediaType::Poem),
            _ => Err(DbError::InvalidStatus(format!("unknown media type: {s}"))),
        }
    }
}

// ─── Workspace ───────────────────────────────────────────────────────

/// A workspace groups tasks together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a new workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkspace {
    pub name: String,
    pub description: Option<String>,
}

/// Fields that can be updated on an existing workspace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateWorkspace {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

// ─── Media ───────────────────────────────────────────────────────────

/// A single piece of media (image, music, or poem).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub media_type: MediaType,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a new media entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMedia {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub media_type: MediaType,
}

/// Fields that can be updated on an existing media entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateMedia {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub url: Option<String>,
    pub media_type: Option<MediaType>,
}

// ─── MediaList ───────────────────────────────────────────────────────

/// A curated list of media (0..n `Media` via `media_media_list`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaList {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a new media list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMediaList {
    pub name: String,
    pub description: Option<String>,
}

/// Fields that can be updated on an existing media list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateMediaList {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

// ─── Task ────────────────────────────────────────────────────────────

/// A task entry stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub workspace_id: i64,
    pub media_list_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub status: TaskStatus,
    pub estimated_mins: Option<i64>,
    /// Day the task is planned for, if any (`YYYY-MM-DD`).
    pub scheduled_on: Option<NaiveDate>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a new task (no id or timestamps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub workspace_id: i64,
    pub media_list_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub estimated_mins: Option<i64>,
    /// Day the task is planned for, if any (`YYYY-MM-DD`).
    pub scheduled_on: Option<NaiveDate>,
}

/// Fields that can be updated on an existing task.
/// `None` means "don't change". For nullable columns like `description`,
/// `Some(None)` means "set to NULL" and `Some(Some(v))` means "set to v".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTask {
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub media_list_id: Option<Option<i64>>,
    pub priority: Option<Priority>,
    pub status: Option<TaskStatus>,
    pub estimated_mins: Option<i64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub scheduled_on: Option<Option<NaiveDate>>,
}

/// Serde helper for `Option<Option<T>>` fields.
///
/// `None` (leave alone) and `Some(None)` (set NULL) both serialize to `null`,
/// so a "clear this column" update would be indistinguishable from "leave it
/// alone" after a round trip through the IPC layer. Skipping `None` keeps the
/// three states apart: field absent / `null` / a value.
mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, T>(value: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        value.serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_task_round_trip_keeps_clear_distinct_from_no_change() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();

        let clear = UpdateTask {
            scheduled_on: Some(None),
            ..Default::default()
        };
        let set = UpdateTask {
            scheduled_on: Some(Some(day)),
            ..Default::default()
        };
        let untouched = UpdateTask {
            name: Some("renamed".to_string()),
            ..Default::default()
        };

        for original in [&clear, &set, &untouched] {
            let json = serde_json::to_string(original).unwrap();
            let decoded: UpdateTask = serde_json::from_str(&json).unwrap();
            match &decoded.scheduled_on {
                Some(inner) => assert_eq!(
                    *inner,
                    original.scheduled_on.unwrap(),
                    "round trip changed the meaning of scheduled_on ({json})"
                ),
                None => assert!(
                    original.scheduled_on.is_none(),
                    "a value got lost in the round trip ({json})"
                ),
            }
        }

        // "Leave alone" omits the key entirely.
        let untouched_json = serde_json::to_string(&untouched).unwrap();
        assert!(!untouched_json.contains("scheduled_on"));
        // "Clear" sends an explicit null.
        assert!(
            serde_json::to_string(&clear)
                .unwrap()
                .contains("\"scheduled_on\":null")
        );
    }

    #[test]
    fn priority_parses_the_cli_spellings() {
        assert_eq!("urgent".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!(
            "immediate".parse::<Priority>().unwrap(),
            Priority::Immediate
        );
        assert_eq!("not-yet".parse::<Priority>().unwrap(), Priority::NotYet);
    }

    #[test]
    fn priority_rejects_unknown_value() {
        let err = "high".parse::<Priority>().unwrap_err();
        assert_eq!(err.value, "high");
        assert_eq!(err.to_string(), "invalid priority value: high");
    }

    #[test]
    fn priority_round_trips_through_its_database_integer() {
        for priority in [Priority::NotYet, Priority::Immediate, Priority::Urgent] {
            let as_int = match priority {
                Priority::NotYet => 0,
                Priority::Immediate => 1,
                Priority::Urgent => 2,
            };
            assert_eq!(Priority::from_i64(as_int).unwrap(), priority);
        }
    }
}
