use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::db::models::{
    MediaList, NewMediaList, NewTask, NewWorkspace, Priority, Task, TaskStatus, UpdateTask,
    Workspace,
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
    WorkspaceList(Vec<Workspace>),
    Workspace(Workspace),
    CurrentWorkspace(Workspace),
    MediaLists(Vec<MediaList>),
    MediaList(MediaList),
    Error { message: String },
}
