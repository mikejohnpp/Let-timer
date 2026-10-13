use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use let_timer_core::{Media, MediaList, Task, TimerSnapshot, TimerState, Workspace};

// ─── Query Parameters ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct TasksQuery {
    pub workspace: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub sort: Option<String>,
    pub date: Option<String>,
    pub q: Option<String>,
    pub overdue: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct StartTaskQuery {
    #[param(default = false)]
    pub switch: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct DoneTaskQuery {
    #[param(default = false)]
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct MediasQuery {
    #[param(rename = "type")]
    pub media_type: Option<String>,
}

// ─── Request Bodies ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaskDto {
    pub name: String,
    pub description: Option<String>,
    #[schema(default = 0, example = 1)]
    pub priority: Option<i64>,
    pub estimated_mins: Option<i64>,
    pub scheduled_on: Option<NaiveDate>,
    pub workspace_id: Option<i64>,
    pub media_list_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTaskDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i64>,
    pub estimated_mins: Option<i64>,
    pub scheduled_on: Option<NaiveDate>,
    pub media_list_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceDto {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwitchWorkspaceDto {
    pub id: Option<i64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMediaListDto {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddMediaItemDto {
    pub media_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMediaDto {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    #[schema(example = "image")]
    pub media_type: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetTimezoneDto {
    pub timezone: String,
}

// ─── Response / DTO Wrappers for OpenAPI ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskDto {
    pub id: i64,
    pub workspace_id: i64,
    pub media_list_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub priority: i64,
    pub status: String,
    pub estimated_mins: Option<i64>,
    pub scheduled_on: Option<NaiveDate>,
    pub run_no: i64,
    pub notified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Task> for TaskDto {
    fn from(t: Task) -> Self {
        Self {
            id: t.id,
            workspace_id: t.workspace_id,
            media_list_id: t.media_list_id,
            name: t.name,
            description: t.description,
            priority: t.priority.to_i64(),
            status: t.status.as_str().to_string(),
            estimated_mins: t.estimated_mins,
            scheduled_on: t.scheduled_on,
            run_no: t.run_no,
            notified_at: t.notified_at,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Workspace> for WorkspaceDto {
    fn from(w: Workspace) -> Self {
        Self {
            id: w.id,
            name: w.name,
            description: w.description,
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub media_type: String,
    pub content: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Media> for MediaDto {
    fn from(m: Media) -> Self {
        Self {
            id: m.id,
            name: m.name,
            description: m.description,
            url: m.url,
            media_type: m.media_type.as_str().to_string(),
            content: m.content,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaListDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MediaList> for MediaListDto {
    fn from(l: MediaList) -> Self {
        Self {
            id: l.id,
            name: l.name,
            description: l.description,
            created_at: l.created_at,
            updated_at: l.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaListDetailDto {
    pub list: MediaListDto,
    pub items: Vec<MediaDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimerSnapshotDto {
    pub task: TaskDto,
    pub state: String,
    pub elapsed_secs: i64,
    pub remaining_secs: Option<i64>,
    pub workspace_name: String,
}

impl From<TimerSnapshot> for TimerSnapshotDto {
    fn from(s: TimerSnapshot) -> Self {
        let state_str = match s.state {
            TimerState::Idle => "idle",
            TimerState::Running => "running",
            TimerState::Paused => "paused",
            TimerState::Overtime => "overtime",
        };
        Self {
            task: TaskDto::from(s.task),
            state: state_str.to_string(),
            elapsed_secs: s.elapsed_secs,
            remaining_secs: s.remaining_secs,
            workspace_name: s.workspace_name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimezoneDto {
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthDto {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetailDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorEnvelopeDto {
    pub error: ErrorDetailDto,
}
