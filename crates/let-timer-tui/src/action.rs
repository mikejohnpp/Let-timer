//! Every change to the application state, in one place.
//!
//! This is the Flux "action" concept: user input, IPC replies and the timer
//! tick all become an [`Action`], and nothing else is allowed to mutate a
//! store. Stores pattern-match on the actions they care about and ignore the
//! rest, so adding an action never requires touching stores that ignore it.

use crossterm::event::KeyEvent;
use let_timer_core::{MediaList, Response, Task, TaskPayload, Workspace};

/// Which kind of record the panel is showing.
///
/// The same `OpenCreate` action means a different form depending on this, and
/// it is also what the CLI's first positional argument maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    Task,
    Workspace,
    MediaList,
}

/// Something that happened.
#[derive(Debug, Clone)]
pub enum Action {
    // ── Inputs ────────────────────────────────────────────────────────
    /// A key press. Already mapped through the keymap by the event loop.
    Key(KeyEvent),
    /// The periodic timer fired.
    Tick,

    // ── IPC replies ───────────────────────────────────────────────────
    /// A list of tasks arrived.
    TaskListLoaded(Vec<Task>),
    /// A list of workspaces arrived.
    WorkspaceListLoaded(Vec<Workspace>),
    /// A list of media lists arrived.
    MediaListLoaded(Vec<MediaList>),
    /// A single task was created or updated.
    TaskSaved(Task),
    /// A single workspace was created.
    WorkspaceSaved(Workspace),
    /// A single media list was created.
    MediaListSaved(MediaList),
    /// A command failed, or the daemon connection dropped.
    IpcFailed(String),

    // ── Selection and scrolling ───────────────────────────────────────
    /// Move the selection to an absolute index.
    Select(usize),
    /// Move the selection by a relative offset, clamped to the list.
    MoveSelection(i32),

    // ── Screens ───────────────────────────────────────────────────────
    /// Open a blank create form for the given component.
    OpenCreate(Component),
    /// Open an edit form seeded from this task.
    ///
    /// The task is passed by value rather than by row index because stores do
    /// not read each other: the event loop reads the selected row out of the
    /// snapshot and hands it over here.
    OpenEdit(Box<Task>),
    /// Open the delete confirmation for the task at this index.
    ConfirmDelete(usize),
    /// Open the calendar popup on the focused form field.
    OpenCalendar,
    /// Toggle the help overlay.
    Help,
    /// Close whatever overlay is on top: calendar, then help, then the panel.
    Cancel,
    /// Submit the current form or confirm the current action.
    Submit,
    /// Leave the TUI.
    Quit,

    // ── Form editing ──────────────────────────────────────────────────
    /// Move focus to the next field.
    FormNextField,
    /// Move focus to the previous field.
    FormPrevField,
    /// Type one character into the focused field.
    FormInput(char),
    /// Delete the character before the cursor.
    FormBackspace,
    /// Pick the calendar's highlighted day.
    PickDate,
    /// Clear the focused date field.
    ClearDate,

    // ── Notifications ─────────────────────────────────────────────────
    /// Dismiss the toast message.
    DismissToast,
}

impl Action {
    /// Turn a daemon reply into the action that applies it.
    ///
    /// Returns `None` when the reply carries nothing to show: `OkEmpty` and
    /// `Ok(TaskPayload::None)` mean the daemon did the job and had nothing to
    /// report, which is not the same as a failure and should not be dressed up
    /// as one.
    pub fn from_response(response: Response) -> Option<Action> {
        match response {
            Response::OkList(tasks) => Some(Action::TaskListLoaded(tasks)),
            Response::Ok(TaskPayload::Some(task)) => Some(Action::TaskSaved(task)),
            Response::Ok(TaskPayload::None) | Response::OkEmpty => None,

            Response::WorkspaceList(workspaces) => Some(Action::WorkspaceListLoaded(workspaces)),
            // A single workspace is a freshly created one, not a whole list.
            Response::Workspace(workspace) => Some(Action::WorkspaceSaved(workspace)),

            Response::MediaListList(media_lists) => Some(Action::MediaListLoaded(media_lists)),
            Response::MediaList(media_list) => Some(Action::MediaListSaved(media_list)),

            Response::Error { message } => Some(Action::IpcFailed(message)),
        }
    }
}

#[cfg(test)]
mod tests {
    use let_timer_core::{Priority, TaskStatus};

    use super::*;

    fn task_with_id(id: i64) -> Task {
        Task {
            id,
            workspace_id: 1,
            media_list_id: None,
            name: format!("Task {id}"),
            description: None,
            priority: Priority::NotYet,
            status: TaskStatus::Pending,
            estimated_mins: None,
            scheduled_on: None,
            created_at: "2026-10-01 00:00:00".to_string(),
            updated_at: "2026-10-01 00:00:00".to_string(),
        }
    }

    fn workspace_with_id(id: i64) -> Workspace {
        Workspace {
            id,
            name: format!("Workspace {id}"),
            description: None,
            created_at: "2026-10-01 00:00:00".to_string(),
            updated_at: "2026-10-01 00:00:00".to_string(),
        }
    }

    fn media_list_with_id(id: i64) -> MediaList {
        MediaList {
            id,
            name: format!("Media list {id}"),
            description: None,
            created_at: "2026-10-01 00:00:00".to_string(),
            updated_at: "2026-10-01 00:00:00".to_string(),
        }
    }

    fn from(response: Response) -> Option<Action> {
        Action::from_response(response)
    }

    #[test]
    fn a_task_list_becomes_a_load_action() {
        let action = from(Response::OkList(vec![task_with_id(1), task_with_id(2)]));
        match action {
            Some(Action::TaskListLoaded(tasks)) => assert_eq!(tasks.len(), 2),
            other => panic!("expected a task list, got {other:?}"),
        }
    }

    #[test]
    fn one_task_becomes_a_saved_task() {
        match from(Response::Ok(TaskPayload::Some(task_with_id(7)))) {
            Some(Action::TaskSaved(task)) => assert_eq!(task.id, 7),
            other => panic!("expected a saved task, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_task_has_nothing_to_report() {
        assert!(from(Response::Ok(TaskPayload::None)).is_none());
    }

    #[test]
    fn an_empty_reply_has_nothing_to_report() {
        assert!(from(Response::OkEmpty).is_none());
    }

    #[test]
    fn a_workspace_list_becomes_a_load_action() {
        match from(Response::WorkspaceList(vec![workspace_with_id(1)])) {
            Some(Action::WorkspaceListLoaded(workspaces)) => assert_eq!(workspaces.len(), 1),
            other => panic!("expected a workspace list, got {other:?}"),
        }
    }

    #[test]
    fn one_workspace_becomes_a_saved_workspace() {
        match from(Response::Workspace(workspace_with_id(5))) {
            Some(Action::WorkspaceSaved(workspace)) => assert_eq!(workspace.id, 5),
            other => panic!("expected a saved workspace, got {other:?}"),
        }
    }

    #[test]
    fn a_media_list_list_becomes_a_load_action() {
        match from(Response::MediaListList(vec![media_list_with_id(1)])) {
            Some(Action::MediaListLoaded(lists)) => assert_eq!(lists.len(), 1),
            other => panic!("expected a media list, got {other:?}"),
        }
    }

    #[test]
    fn one_media_list_becomes_a_saved_media_list() {
        match from(Response::MediaList(media_list_with_id(9))) {
            Some(Action::MediaListSaved(list)) => assert_eq!(list.id, 9),
            other => panic!("expected a saved media list, got {other:?}"),
        }
    }

    #[test]
    fn an_error_becomes_a_failure_carrying_its_message() {
        match from(Response::Error {
            message: "boom".to_string(),
        }) {
            Some(Action::IpcFailed(message)) => assert_eq!(message, "boom"),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_error_message_still_becomes_a_failure() {
        // The daemon can send an empty message; deciding what that means is
        // the ui store's job, not the translation's.
        match from(Response::Error {
            message: String::new(),
        }) {
            Some(Action::IpcFailed(message)) => assert!(message.is_empty()),
            other => panic!("expected a failure, got {other:?}"),
        }
    }
}
