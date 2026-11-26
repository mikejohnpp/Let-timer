use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::{Task, TaskSession, TaskStatus};

/// Visual state of the timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerState {
    Idle,
    Running,
    Paused,
    Overtime,
}

/// A computed view of the timer for one task at a specific point in time.
///
/// Never stored in the database; always derived dynamically from `task`,
/// `task_sessions`, and the current wall-clock instant `now`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerSnapshot {
    pub task: Task,
    pub state: TimerState,
    pub elapsed_secs: i64,
    pub remaining_secs: Option<i64>,
    pub workspace_name: String,
}

impl TimerSnapshot {
    /// Compute a snapshot given a task, its sessions, the workspace name, and current time `now`.
    pub fn compute(
        task: Task,
        sessions: &[TaskSession],
        workspace_name: String,
        now: DateTime<Utc>,
    ) -> Self {
        // Only sessions for the current run contribute to elapsed time (Spec §5).
        let current_run = task.run_no;
        let mut elapsed_secs: i64 = 0;

        for s in sessions {
            if s.run_no != current_run {
                continue;
            }
            match s.ended_at {
                Some(ended_at) => {
                    let diff = (ended_at - s.started_at).num_seconds();
                    if diff > 0 {
                        elapsed_secs += diff;
                    }
                }
                None => {
                    let diff = (now - s.started_at).num_seconds();
                    if diff > 0 {
                        elapsed_secs += diff;
                    }
                }
            }
        }

        let remaining_secs = task.estimated_mins.map(|mins| mins * 60 - elapsed_secs);

        let state = match task.status {
            TaskStatus::InProgress => match remaining_secs {
                Some(rem) if rem < 0 => TimerState::Overtime,
                _ => TimerState::Running,
            },
            TaskStatus::Paused => TimerState::Paused,
            TaskStatus::Pending | TaskStatus::Completed | TaskStatus::Cancelled => TimerState::Idle,
        };

        Self {
            task,
            state,
            elapsed_secs,
            remaining_secs,
            workspace_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Priority, SessionEndReason};

    fn make_task(id: i64, status: TaskStatus, estimated_mins: Option<i64>, run_no: i64) -> Task {
        Task {
            id,
            workspace_id: 1,
            media_list_id: None,
            name: "Test task".to_string(),
            description: None,
            priority: Priority::NotYet,
            status,
            estimated_mins,
            scheduled_on: None,
            run_no,
            notified_at: None,
            created_at: "2026-10-12T08:00:00Z".to_string(),
            updated_at: "2026-10-12T08:00:00Z".to_string(),
        }
    }

    fn parse_time(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn elapsed_computed_across_sessions() {
        let task = make_task(1, TaskStatus::InProgress, Some(25), 0);
        let sessions = vec![
            TaskSession {
                id: 1,
                task_id: 1,
                run_no: 0,
                started_at: parse_time("2026-10-12T10:00:00Z"),
                ended_at: Some(parse_time("2026-10-12T10:10:00Z")), // 10 mins = 600s
                end_reason: Some(SessionEndReason::Paused),
            },
            TaskSession {
                id: 2,
                task_id: 1,
                run_no: 0,
                started_at: parse_time("2026-10-12T10:20:00Z"),
                ended_at: None, // open session started at 10:20
                end_reason: None,
            },
        ];

        // Now is 10:25:00Z (5 mins = 300s into open session)
        let now = parse_time("2026-10-12T10:25:00Z");
        let snap = TimerSnapshot::compute(task, &sessions, "Default".to_string(), now);

        assert_eq!(snap.elapsed_secs, 900); // 600s + 300s = 900s (15 mins)
        assert_eq!(snap.remaining_secs, Some(600)); // 25*60 - 900 = 600s (10 mins)
        assert_eq!(snap.state, TimerState::Running);
    }

    #[test]
    fn overtime_flips_when_remaining_is_negative() {
        let task = make_task(1, TaskStatus::InProgress, Some(10), 0); // 10 mins = 600s
        let sessions = vec![TaskSession {
            id: 1,
            task_id: 1,
            run_no: 0,
            started_at: parse_time("2026-10-12T10:00:00Z"),
            ended_at: None,
            end_reason: None,
        }];

        // Exactly at 10 mins (600s): remaining = 0 => Running
        let at_limit = parse_time("2026-10-12T10:10:00Z");
        let snap = TimerSnapshot::compute(task.clone(), &sessions, "Default".to_string(), at_limit);
        assert_eq!(snap.remaining_secs, Some(0));
        assert_eq!(snap.state, TimerState::Running);

        // 1 second later: remaining = -1 => Overtime
        let past_limit = parse_time("2026-10-12T10:10:01Z");
        let snap_over = TimerSnapshot::compute(task, &sessions, "Default".to_string(), past_limit);
        assert_eq!(snap_over.remaining_secs, Some(-1));
        assert_eq!(snap_over.state, TimerState::Overtime);
    }

    #[test]
    fn task_without_estimate_has_no_remaining() {
        let task = make_task(1, TaskStatus::InProgress, None, 0);
        let sessions = vec![TaskSession {
            id: 1,
            task_id: 1,
            run_no: 0,
            started_at: parse_time("2026-10-12T10:00:00Z"),
            ended_at: None,
            end_reason: None,
        }];

        let now = parse_time("2026-10-12T10:05:00Z");
        let snap = TimerSnapshot::compute(task, &sessions, "Default".to_string(), now);
        assert_eq!(snap.elapsed_secs, 300);
        assert_eq!(snap.remaining_secs, None);
        assert_eq!(snap.state, TimerState::Running);
    }

    #[test]
    fn restart_run_ignores_previous_runs_elapsed() {
        let task = make_task(1, TaskStatus::InProgress, Some(20), 1); // Run 1
        let sessions = vec![
            TaskSession {
                id: 1,
                task_id: 1,
                run_no: 0, // Old run 0
                started_at: parse_time("2026-10-12T09:00:00Z"),
                ended_at: Some(parse_time("2026-10-12T09:30:00Z")),
                end_reason: Some(SessionEndReason::Restarted),
            },
            TaskSession {
                id: 2,
                task_id: 1,
                run_no: 1, // Current run 1
                started_at: parse_time("2026-10-12T10:00:00Z"),
                ended_at: None,
                end_reason: None,
            },
        ];

        let now = parse_time("2026-10-12T10:05:00Z");
        let snap = TimerSnapshot::compute(task, &sessions, "Default".to_string(), now);
        assert_eq!(snap.elapsed_secs, 300); // Only run 1 elapsed
        assert_eq!(snap.remaining_secs, Some(900)); // 20*60 - 300 = 900
    }
}
