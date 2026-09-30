use crate::db::models::{
    MediaList, NewMediaList, NewTask, NewWorkspace, Task, TaskStatus, UpdateTask, Workspace,
};
use serde::{Deserialize, Serialize};

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
    },
    List {
        sort_priority: Option<SortOrder>,
        filter_status: Option<TaskStatus>,
    },
    Current,
    Start {
        id: Option<i64>,
    },
    Stop,
    Done,
    ListWorkspaces,
    CreateWorkspace(NewWorkspace),
    ListMediaLists,
    CreateMediaList(NewMediaList),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok(TaskPayload),
    OkList(Vec<Task>),
    OkEmpty,
    WorkspaceList(Vec<Workspace>),
    Workspace(Workspace),
    MediaListList(Vec<MediaList>),
    MediaList(MediaList),
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPayload {
    Some(Task),
    None,
}
