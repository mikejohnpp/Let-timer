use crate::db::models::{NewTask, Priority, Task, TaskStatus, UpdateTask};
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
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPayload {
    Some(Task),
    None,
}
