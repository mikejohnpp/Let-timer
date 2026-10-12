use std::fmt;

use crate::db::TaskStatus;

/// Actions that can cause a task lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Pause,
    Resume,
    Stop,
    Restart,
    Done,
    Cancel,
    Reopen,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Start => "start",
            Action::Pause => "pause",
            Action::Resume => "resume",
            Action::Stop => "stop",
            Action::Restart => "restart",
            Action::Done => "done",
            Action::Cancel => "cancel",
            Action::Reopen => "reopen",
        }
    }
}

/// An error indicating that a requested transition is not permitted from the current status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionError {
    pub current: TaskStatus,
    pub action: Action,
    pub message: String,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TransitionError {}

/// Pure state machine function implementing the lifecycle transition table verbatim (Spec §4).
///
/// Returns the next `TaskStatus` on success, or a `TransitionError` with a clear explanation of what
/// the user should do instead.
pub fn next_status(
    current: TaskStatus,
    action: Action,
    force: bool,
) -> Result<TaskStatus, TransitionError> {
    match (current, action) {
        // From `pending`
        (TaskStatus::Pending, Action::Start) => Ok(TaskStatus::InProgress),
        (TaskStatus::Pending, Action::Pause) => Err(TransitionError {
            current,
            action,
            message: "cannot pause a pending task: start it first".to_string(),
        }),
        (TaskStatus::Pending, Action::Resume) => Err(TransitionError {
            current,
            action,
            message: "cannot resume a pending task: use `start`".to_string(),
        }),
        (TaskStatus::Pending, Action::Stop) => Err(TransitionError {
            current,
            action,
            message: "cannot stop a pending task: it is not running".to_string(),
        }),
        (TaskStatus::Pending, Action::Restart) => Ok(TaskStatus::InProgress),
        (TaskStatus::Pending, Action::Done) => {
            if force {
                Ok(TaskStatus::Completed)
            } else {
                Err(TransitionError {
                    current,
                    action,
                    message:
                        "cannot mark pending task as done: start it first, or use `done --force`"
                            .to_string(),
                })
            }
        }
        (TaskStatus::Pending, Action::Cancel) => Ok(TaskStatus::Cancelled),
        (TaskStatus::Pending, Action::Reopen) => Err(TransitionError {
            current,
            action,
            message: "cannot reopen a pending task: it is already open".to_string(),
        }),

        // From `in-progress`
        (TaskStatus::InProgress, Action::Start) => Err(TransitionError {
            current,
            action,
            message: "task is already running".to_string(),
        }),
        (TaskStatus::InProgress, Action::Pause) => Ok(TaskStatus::Paused),
        (TaskStatus::InProgress, Action::Resume) => Err(TransitionError {
            current,
            action,
            message: "task is already running".to_string(),
        }),
        (TaskStatus::InProgress, Action::Stop) => Ok(TaskStatus::Pending),
        (TaskStatus::InProgress, Action::Restart) => Ok(TaskStatus::InProgress),
        (TaskStatus::InProgress, Action::Done) => Ok(TaskStatus::Completed),
        (TaskStatus::InProgress, Action::Cancel) => Ok(TaskStatus::Cancelled),
        (TaskStatus::InProgress, Action::Reopen) => Err(TransitionError {
            current,
            action,
            message: "cannot reopen an in-progress task: it is already running".to_string(),
        }),

        // From `paused`
        (TaskStatus::Paused, Action::Start) => Ok(TaskStatus::InProgress),
        (TaskStatus::Paused, Action::Pause) => Err(TransitionError {
            current,
            action,
            message: "task is already paused".to_string(),
        }),
        (TaskStatus::Paused, Action::Resume) => Ok(TaskStatus::InProgress),
        (TaskStatus::Paused, Action::Stop) => Ok(TaskStatus::Pending),
        (TaskStatus::Paused, Action::Restart) => Ok(TaskStatus::InProgress),
        (TaskStatus::Paused, Action::Done) => Ok(TaskStatus::Completed),
        (TaskStatus::Paused, Action::Cancel) => Ok(TaskStatus::Cancelled),
        (TaskStatus::Paused, Action::Reopen) => Err(TransitionError {
            current,
            action,
            message: "cannot reopen a paused task: resume or restart it instead".to_string(),
        }),

        // From `completed`
        (TaskStatus::Completed, Action::Reopen) => Ok(TaskStatus::Pending),
        (TaskStatus::Completed, Action::Done) => Err(TransitionError {
            current,
            action,
            message: "task is already completed".to_string(),
        }),
        (TaskStatus::Completed, any_other) => Err(TransitionError {
            current,
            action: any_other,
            message: format!(
                "task is completed: use `reopen` before `{}`",
                any_other.as_str()
            ),
        }),

        // From `cancelled`
        (TaskStatus::Cancelled, Action::Reopen) => Ok(TaskStatus::Pending),
        (TaskStatus::Cancelled, Action::Cancel) => Err(TransitionError {
            current,
            action,
            message: "task is already cancelled".to_string(),
        }),
        (TaskStatus::Cancelled, any_other) => Err(TransitionError {
            current,
            action: any_other,
            message: format!(
                "task is cancelled: use `reopen` before `{}`",
                any_other.as_str()
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_driven_transition_matrix_coverage() {
        // Complete 5 statuses x 8 actions = 40 transitions matrix coverage
        let cases = [
            // (Current, Action, force, Expected Result)
            // Pending
            (
                TaskStatus::Pending,
                Action::Start,
                false,
                Ok(TaskStatus::InProgress),
            ),
            (
                TaskStatus::Pending,
                Action::Pause,
                false,
                Err("cannot pause a pending task"),
            ),
            (
                TaskStatus::Pending,
                Action::Resume,
                false,
                Err("cannot resume a pending task"),
            ),
            (
                TaskStatus::Pending,
                Action::Stop,
                false,
                Err("cannot stop a pending task"),
            ),
            (
                TaskStatus::Pending,
                Action::Restart,
                false,
                Ok(TaskStatus::InProgress),
            ),
            (
                TaskStatus::Pending,
                Action::Done,
                false,
                Err("cannot mark pending task as done"),
            ),
            (
                TaskStatus::Pending,
                Action::Done,
                true,
                Ok(TaskStatus::Completed),
            ),
            (
                TaskStatus::Pending,
                Action::Cancel,
                false,
                Ok(TaskStatus::Cancelled),
            ),
            (
                TaskStatus::Pending,
                Action::Reopen,
                false,
                Err("cannot reopen a pending task"),
            ),
            // InProgress
            (
                TaskStatus::InProgress,
                Action::Start,
                false,
                Err("already running"),
            ),
            (
                TaskStatus::InProgress,
                Action::Pause,
                false,
                Ok(TaskStatus::Paused),
            ),
            (
                TaskStatus::InProgress,
                Action::Resume,
                false,
                Err("already running"),
            ),
            (
                TaskStatus::InProgress,
                Action::Stop,
                false,
                Ok(TaskStatus::Pending),
            ),
            (
                TaskStatus::InProgress,
                Action::Restart,
                false,
                Ok(TaskStatus::InProgress),
            ),
            (
                TaskStatus::InProgress,
                Action::Done,
                false,
                Ok(TaskStatus::Completed),
            ),
            (
                TaskStatus::InProgress,
                Action::Cancel,
                false,
                Ok(TaskStatus::Cancelled),
            ),
            (
                TaskStatus::InProgress,
                Action::Reopen,
                false,
                Err("cannot reopen an in-progress task"),
            ),
            // Paused
            (
                TaskStatus::Paused,
                Action::Start,
                false,
                Ok(TaskStatus::InProgress),
            ),
            (
                TaskStatus::Paused,
                Action::Pause,
                false,
                Err("already paused"),
            ),
            (
                TaskStatus::Paused,
                Action::Resume,
                false,
                Ok(TaskStatus::InProgress),
            ),
            (
                TaskStatus::Paused,
                Action::Stop,
                false,
                Ok(TaskStatus::Pending),
            ),
            (
                TaskStatus::Paused,
                Action::Restart,
                false,
                Ok(TaskStatus::InProgress),
            ),
            (
                TaskStatus::Paused,
                Action::Done,
                false,
                Ok(TaskStatus::Completed),
            ),
            (
                TaskStatus::Paused,
                Action::Cancel,
                false,
                Ok(TaskStatus::Cancelled),
            ),
            (
                TaskStatus::Paused,
                Action::Reopen,
                false,
                Err("cannot reopen a paused task"),
            ),
            // Completed
            (
                TaskStatus::Completed,
                Action::Start,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Completed,
                Action::Pause,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Completed,
                Action::Resume,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Completed,
                Action::Stop,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Completed,
                Action::Restart,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Completed,
                Action::Done,
                false,
                Err("already completed"),
            ),
            (
                TaskStatus::Completed,
                Action::Cancel,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Completed,
                Action::Reopen,
                false,
                Ok(TaskStatus::Pending),
            ),
            // Cancelled
            (
                TaskStatus::Cancelled,
                Action::Start,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Cancelled,
                Action::Pause,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Cancelled,
                Action::Resume,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Cancelled,
                Action::Stop,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Cancelled,
                Action::Restart,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Cancelled,
                Action::Done,
                false,
                Err("use `reopen`"),
            ),
            (
                TaskStatus::Cancelled,
                Action::Cancel,
                false,
                Err("already cancelled"),
            ),
            (
                TaskStatus::Cancelled,
                Action::Reopen,
                false,
                Ok(TaskStatus::Pending),
            ),
        ];

        for (cur, act, force, expected) in cases {
            let res = next_status(cur, act, force);
            match expected {
                Ok(expected_status) => {
                    assert_eq!(
                        res,
                        Ok(expected_status),
                        "Expected {:?} on {:?} + {:?} (force={})",
                        expected_status,
                        cur,
                        act,
                        force
                    );
                }
                Err(expected_substring) => {
                    assert!(
                        res.is_err(),
                        "Expected error on {:?} + {:?} (force={}), got {:?}",
                        cur,
                        act,
                        force,
                        res
                    );
                    let err_msg = res.unwrap_err().message;
                    assert!(
                        err_msg.contains(expected_substring),
                        "Expected error containing {:?}, got {:?}",
                        expected_substring,
                        err_msg
                    );
                }
            }
        }
    }
}
