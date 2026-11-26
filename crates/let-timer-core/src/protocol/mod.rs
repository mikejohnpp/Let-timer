use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::db::models::{
    Media, MediaList, MediaType, NewMedia, NewMediaList, NewTask, NewWorkspace, Priority, Task,
    TaskStatus, UpdateTask, Workspace,
};
use crate::service::TimerSnapshot;

/// Workspace scope for filtering tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkspaceScope {
    #[default]
    Active,
    All,
    Id(i64),
    Name(String),
}

/// Query parameters for listing tasks with combined filters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListQuery {
    pub workspace: WorkspaceScope,
    pub status: Option<TaskStatus>,
    pub sort_priority: Option<SortOrder>,
    pub priority: Option<Priority>,
    pub date: Option<NaiveDate>,
    pub overdue: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Create(NewTask),
    Delete {
        id: i64,
    },
    Edit {
        id: i64,
        update: UpdateTask,
    },
    Find {
        query: String,
        workspace: WorkspaceScope,
    },
    List(ListQuery),
    Current,
    Start {
        id: Option<i64>,
        next: bool,
        switch: bool,
    },
    StartCandidates {
        workspace: WorkspaceScope,
    },
    Pause {
        id: Option<i64>,
    },
    Resume {
        id: Option<i64>,
    },
    Stop {
        id: Option<i64>,
    },
    Restart {
        id: Option<i64>,
    },
    Done {
        id: Option<i64>,
        force: bool,
    },
    Cancel {
        id: Option<i64>,
    },
    Reopen {
        id: i64,
    },
    Ping,
    ListWorkspaces,
    CreateWorkspace(NewWorkspace),
    SwitchWorkspace {
        target: WorkspaceTarget,
    },
    CurrentWorkspace,
    DeleteWorkspace {
        id: i64,
    },
    ListMediaLists,
    CreateMediaList(NewMediaList),
    GetMediaList {
        id: i64,
    },
    DeleteMediaList {
        id: i64,
    },
    AddMediaToList {
        media_list_id: i64,
        media_id: i64,
    },
    RemoveMediaFromList {
        media_list_id: i64,
        media_id: i64,
    },
    ListMedias {
        media_type: Option<MediaType>,
    },
    /// Create a new media item.
    ///
    /// # Transport Security Policy
    /// - **Remote (HTTP)**: `import_file` MUST be `None`. Remote requests carrying `Some(...)`
    ///   are rejected with `403 forbidden_remote_command`. Remote clients upload files via
    ///   multipart POST to `/api/v1/media/upload`.
    /// - **Local (Unix socket)**: `import_file: Some(path)` is allowed, subject to `copy_local`
    ///   hardening (rejecting symlinks, devices, daemon data directory, and oversized files).
    CreateMedia {
        media: NewMedia,
        import_file: Option<String>,
    },
    DeleteMedia {
        id: i64,
    },
    DaemonStatus,
    GetTimezone,
    SetTimezone(String),
}

impl Command {
    /// Returns the static variant name of the command without any parameters or payloads (SEC-7).
    pub fn name(&self) -> &'static str {
        match self {
            Self::Create(_) => "tasks.create",
            Self::Delete { .. } => "tasks.delete",
            Self::Edit { .. } => "tasks.edit",
            Self::Find { .. } => "tasks.find",
            Self::List(_) => "tasks.list",
            Self::Current => "tasks.current",
            Self::Start { .. } => "tasks.start",
            Self::StartCandidates { .. } => "tasks.start_candidates",
            Self::Pause { .. } => "tasks.pause",
            Self::Resume { .. } => "tasks.resume",
            Self::Stop { .. } => "tasks.stop",
            Self::Restart { .. } => "tasks.restart",
            Self::Done { .. } => "tasks.done",
            Self::Cancel { .. } => "tasks.cancel",
            Self::Reopen { .. } => "tasks.reopen",
            Self::Ping => "ping",
            Self::ListWorkspaces => "workspaces.list",
            Self::CreateWorkspace(_) => "workspaces.create",
            Self::SwitchWorkspace { .. } => "workspaces.switch",
            Self::CurrentWorkspace => "workspaces.current",
            Self::DeleteWorkspace { .. } => "workspaces.delete",
            Self::ListMediaLists => "media_lists.list",
            Self::CreateMediaList(_) => "media_lists.create",
            Self::GetMediaList { .. } => "media_lists.get",
            Self::DeleteMediaList { .. } => "media_lists.delete",
            Self::AddMediaToList { .. } => "media_lists.add_media",
            Self::RemoveMediaFromList { .. } => "media_lists.remove_media",
            Self::ListMedias { .. } => "medias.list",
            Self::CreateMedia { .. } => "medias.create",
            Self::DeleteMedia { .. } => "medias.delete",
            Self::DaemonStatus => "daemon.status",
            Self::GetTimezone => "timezone.get",
            Self::SetTimezone(_) => "timezone.set",
        }
    }
}

/// Target identifier when switching workspaces (by ID or exact/fuzzy name).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceTarget {
    Id(i64),
    Name(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok(Option<Task>),
    OkList(Vec<Task>),
    OkEmpty,
    Timer(TimerSnapshot),
    Pong,
    DaemonStatus {
        version: String,
        socket_path: String,
        db_path: String,
        running_task: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        auth_mode: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bind_address: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cors_origins: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        docs_enabled: Option<bool>,
    },
    WorkspaceList(Vec<Workspace>),
    Workspace(Workspace),
    CurrentWorkspace(Workspace),
    MediaLists(Vec<MediaList>),
    MediaList(MediaList),
    MediaListDetail {
        list: MediaList,
        items: Vec<Media>,
    },
    Medias(Vec<Media>),
    Media(Media),
    Timezone(String),
    Error {
        message: String,
    },
}
