pub mod snapshot;
pub mod transition;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

pub use snapshot::{TimerSnapshot, TimerState};
pub use transition::{Action, TransitionError, next_status};

use crate::db::{DbError, Task, TaskRepository, TaskSession, UpdateTask};

/// Errors produced by `TaskService`.
#[derive(Debug)]
pub enum ServiceError {
    Transition(TransitionError),
    Db(DbError),
    AnotherTaskRunning { id: i64, name: String },
    NoTaskRunning,
    NoPausedTask,
    StatusEditNotAllowed,
    NotFound { entity: &'static str, id: i64 },
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::Transition(e) => write!(f, "{e}"),
            ServiceError::Db(e) => write!(f, "{e}"),
            ServiceError::AnotherTaskRunning { id, name } => {
                write!(
                    f,
                    "task #{id} \"{name}\" is already running: use `pause` or `--switch`"
                )
            }
            ServiceError::NoTaskRunning => write!(f, "no task is currently running"),
            ServiceError::NoPausedTask => write!(f, "no paused task to resume"),
            ServiceError::StatusEditNotAllowed => write!(
                f,
                "cannot change status with `edit`: use lifecycle commands (start, pause, done, etc.)"
            ),
            ServiceError::NotFound { entity, id } => write!(f, "{entity} not found: id={id}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<DbError> for ServiceError {
    fn from(err: DbError) -> Self {
        ServiceError::Db(err)
    }
}

impl From<rusqlite::Error> for ServiceError {
    fn from(err: rusqlite::Error) -> Self {
        ServiceError::Db(DbError::Sqlite(err))
    }
}

impl From<TransitionError> for ServiceError {
    fn from(err: TransitionError) -> Self {
        ServiceError::Transition(err)
    }
}

/// Domain engine coordinating task lifecycle and timer sessions.
///
/// Every state-mutating operation executes inside a single database transaction.
pub struct TaskService<'a> {
    conn: &'a Connection,
}

impl<'a> TaskService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ─── Internal helpers ────────────────────────────────────────────

    fn fetch_task(&self, conn: &Connection, id: i64) -> Result<Task, ServiceError> {
        let repo = TaskRepository::new(conn);
        repo.get_by_id(id).map_err(|e| match e {
            DbError::NotFound { entity, id } => ServiceError::NotFound { entity, id },
            other => ServiceError::Db(other),
        })
    }

    fn fetch_workspace_name(&self, conn: &Connection, ws_id: i64) -> Result<String, ServiceError> {
        let name: String = conn
            .query_row(
                "SELECT name FROM workspaces WHERE id = ?1",
                params![ws_id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => ServiceError::NotFound {
                    entity: "workspace",
                    id: ws_id,
                },
                other => ServiceError::Db(DbError::Sqlite(other)),
            })?;
        Ok(name)
    }

    fn get_running_task(&self, conn: &Connection) -> Result<Option<Task>, ServiceError> {
        let task_id: Option<i64> = conn
            .query_row(
                "SELECT task_id FROM task_sessions WHERE ended_at IS NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        match task_id {
            Some(id) => Ok(Some(self.fetch_task(conn, id)?)),
            None => Ok(None),
        }
    }

    fn get_most_recent_paused_task(&self, conn: &Connection) -> Result<Option<Task>, ServiceError> {
        let task_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM tasks WHERE status = 'paused' ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        match task_id {
            Some(id) => Ok(Some(self.fetch_task(conn, id)?)),
            None => Ok(None),
        }
    }

    fn fetch_sessions_for_task(
        &self,
        conn: &Connection,
        task_id: i64,
    ) -> Result<Vec<TaskSession>, ServiceError> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, run_no, started_at, ended_at, end_reason
             FROM task_sessions
             WHERE task_id = ?1
             ORDER BY started_at ASC",
        )?;
        let sessions = stmt
            .query_map(params![task_id], |row| {
                let started_at_raw: String = row.get("started_at")?;
                let started_at = DateTime::parse_from_rfc3339(&started_at_raw)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let ended_at_raw: Option<String> = row.get("ended_at")?;
                let ended_at = match ended_at_raw {
                    Some(s) => Some(
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    3,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })?,
                    ),
                    None => None,
                };

                let end_reason_raw: Option<String> = row.get("end_reason")?;
                let end_reason = match end_reason_raw {
                    Some(r) => Some(r.parse().map_err(|e: DbError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?),
                    None => None,
                };

                Ok(TaskSession {
                    id: row.get("id")?,
                    task_id: row.get("task_id")?,
                    run_no: row.get("run_no")?,
                    started_at,
                    ended_at,
                    end_reason,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    fn build_snapshot(
        &self,
        conn: &Connection,
        task: Task,
        now: DateTime<Utc>,
    ) -> Result<TimerSnapshot, ServiceError> {
        let sessions = self.fetch_sessions_for_task(conn, task.id)?;
        let ws_name = self.fetch_workspace_name(conn, task.workspace_id)?;
        Ok(TimerSnapshot::compute(task, &sessions, ws_name, now))
    }

    // ─── Lifecycle operations ────────────────────────────────────────

    /// Start a task by ID.
    ///
    /// If another task is running and `switch` is true, the running task is paused first.
    pub fn start(
        &self,
        id: i64,
        switch: bool,
        now: DateTime<Utc>,
    ) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;

        if let Some(running) = self.get_running_task(&tx)? {
            if running.id == id {
                return Err(next_status(running.status, Action::Start, false)
                    .unwrap_err()
                    .into());
            } else if switch {
                // Pause currently running task in the same transaction
                let _ = next_status(running.status, Action::Pause, false)?;
                tx.execute(
                    "UPDATE tasks SET status = 'paused', updated_at = ?1 WHERE id = ?2",
                    params![now.to_rfc3339(), running.id],
                )?;
                tx.execute(
                    "UPDATE task_sessions SET ended_at = ?1, end_reason = 'paused'
                     WHERE task_id = ?2 AND ended_at IS NULL",
                    params![now.to_rfc3339(), running.id],
                )?;
            } else {
                return Err(ServiceError::AnotherTaskRunning {
                    id: running.id,
                    name: running.name,
                });
            }
        }

        let task = self.fetch_task(&tx, id)?;
        let new_status = next_status(task.status, Action::Start, false)?;

        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now.to_rfc3339(), id],
        )?;

        tx.execute(
            "INSERT INTO task_sessions (task_id, run_no, started_at, ended_at, end_reason)
             VALUES (?1, ?2, ?3, NULL, NULL)",
            params![id, task.run_no, now.to_rfc3339()],
        )?;

        let updated_task = self.fetch_task(&tx, id)?;
        let snapshot = self.build_snapshot(&tx, updated_task, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Pause a task (defaults to the currently running task).
    pub fn pause(
        &self,
        id: Option<i64>,
        now: DateTime<Utc>,
    ) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;

        let target = match id {
            Some(id) => self.fetch_task(&tx, id)?,
            None => self
                .get_running_task(&tx)?
                .ok_or(ServiceError::NoTaskRunning)?,
        };

        let new_status = next_status(target.status, Action::Pause, false)?;

        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now.to_rfc3339(), target.id],
        )?;

        tx.execute(
            "UPDATE task_sessions SET ended_at = ?1, end_reason = 'paused'
             WHERE task_id = ?2 AND ended_at IS NULL",
            params![now.to_rfc3339(), target.id],
        )?;

        let updated = self.fetch_task(&tx, target.id)?;
        let snapshot = self.build_snapshot(&tx, updated, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Resume a task (defaults to the most recently paused task).
    pub fn resume(
        &self,
        id: Option<i64>,
        now: DateTime<Utc>,
    ) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;

        let target = match id {
            Some(id) => self.fetch_task(&tx, id)?,
            None => self
                .get_most_recent_paused_task(&tx)?
                .ok_or(ServiceError::NoPausedTask)?,
        };

        if let Some(running) = self.get_running_task(&tx)?
            && running.id != target.id
        {
            return Err(ServiceError::AnotherTaskRunning {
                id: running.id,
                name: running.name,
            });
        }

        let new_status = next_status(target.status, Action::Resume, false)?;

        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now.to_rfc3339(), target.id],
        )?;

        tx.execute(
            "INSERT INTO task_sessions (task_id, run_no, started_at, ended_at, end_reason)
             VALUES (?1, ?2, ?3, NULL, NULL)",
            params![target.id, target.run_no, now.to_rfc3339()],
        )?;

        let updated = self.fetch_task(&tx, target.id)?;
        let snapshot = self.build_snapshot(&tx, updated, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Stop a task (defaults to the currently running task). Preserves elapsed time.
    pub fn stop(&self, id: Option<i64>, now: DateTime<Utc>) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;

        let target = match id {
            Some(id) => self.fetch_task(&tx, id)?,
            None => self
                .get_running_task(&tx)?
                .ok_or(ServiceError::NoTaskRunning)?,
        };

        let new_status = next_status(target.status, Action::Stop, false)?;

        tx.execute(
            "UPDATE task_sessions SET ended_at = ?1, end_reason = 'stopped'
             WHERE task_id = ?2 AND ended_at IS NULL",
            params![now.to_rfc3339(), target.id],
        )?;

        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now.to_rfc3339(), target.id],
        )?;

        let updated = self.fetch_task(&tx, target.id)?;
        let snapshot = self.build_snapshot(&tx, updated, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Restart a task (defaults to the currently running task). Resets elapsed time by bumping run_no.
    pub fn restart(
        &self,
        id: Option<i64>,
        now: DateTime<Utc>,
    ) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;

        let target = match id {
            Some(id) => self.fetch_task(&tx, id)?,
            None => self
                .get_running_task(&tx)?
                .ok_or(ServiceError::NoTaskRunning)?,
        };

        if let Some(running) = self.get_running_task(&tx)?
            && running.id != target.id
        {
            return Err(ServiceError::AnotherTaskRunning {
                id: running.id,
                name: running.name,
            });
        }

        let new_status = next_status(target.status, Action::Restart, false)?;

        tx.execute(
            "UPDATE task_sessions SET ended_at = ?1, end_reason = 'restarted'
             WHERE task_id = ?2 AND ended_at IS NULL",
            params![now.to_rfc3339(), target.id],
        )?;

        let new_run_no = target.run_no + 1;

        tx.execute(
            "UPDATE tasks SET status = ?1, run_no = ?2, notified_at = NULL, updated_at = ?3 WHERE id = ?4",
            params![new_status.as_str(), new_run_no, now.to_rfc3339(), target.id],
        )?;

        tx.execute(
            "INSERT INTO task_sessions (task_id, run_no, started_at, ended_at, end_reason)
             VALUES (?1, ?2, ?3, NULL, NULL)",
            params![target.id, new_run_no, now.to_rfc3339()],
        )?;

        let updated = self.fetch_task(&tx, target.id)?;
        let snapshot = self.build_snapshot(&tx, updated, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Mark a task as done (defaults to the currently running task). Only explicit done completes a task.
    pub fn done(
        &self,
        id: Option<i64>,
        force: bool,
        now: DateTime<Utc>,
    ) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;

        let target = match id {
            Some(id) => self.fetch_task(&tx, id)?,
            None => self
                .get_running_task(&tx)?
                .ok_or(ServiceError::NoTaskRunning)?,
        };

        let new_status = next_status(target.status, Action::Done, force)?;

        tx.execute(
            "UPDATE task_sessions SET ended_at = ?1, end_reason = 'completed'
             WHERE task_id = ?2 AND ended_at IS NULL",
            params![now.to_rfc3339(), target.id],
        )?;

        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now.to_rfc3339(), target.id],
        )?;

        let updated = self.fetch_task(&tx, target.id)?;
        let snapshot = self.build_snapshot(&tx, updated, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Cancel a task (defaults to the currently running task).
    pub fn cancel(
        &self,
        id: Option<i64>,
        now: DateTime<Utc>,
    ) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;

        let target = match id {
            Some(id) => self.fetch_task(&tx, id)?,
            None => self
                .get_running_task(&tx)?
                .ok_or(ServiceError::NoTaskRunning)?,
        };

        let new_status = next_status(target.status, Action::Cancel, false)?;

        tx.execute(
            "UPDATE task_sessions SET ended_at = ?1, end_reason = 'cancelled'
             WHERE task_id = ?2 AND ended_at IS NULL",
            params![now.to_rfc3339(), target.id],
        )?;

        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now.to_rfc3339(), target.id],
        )?;

        let updated = self.fetch_task(&tx, target.id)?;
        let snapshot = self.build_snapshot(&tx, updated, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Reopen a completed or cancelled task back to pending.
    pub fn reopen(&self, id: i64, now: DateTime<Utc>) -> Result<TimerSnapshot, ServiceError> {
        let tx = self.conn.unchecked_transaction()?;
        let target = self.fetch_task(&tx, id)?;
        let new_status = next_status(target.status, Action::Reopen, false)?;

        tx.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now.to_rfc3339(), id],
        )?;

        let updated = self.fetch_task(&tx, id)?;
        let snapshot = self.build_snapshot(&tx, updated, now)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Retrieve the current timer snapshot (running task first, otherwise most recent paused task).
    pub fn current(&self, now: DateTime<Utc>) -> Result<Option<TimerSnapshot>, ServiceError> {
        if let Some(running) = self.get_running_task(self.conn)? {
            return Ok(Some(self.build_snapshot(self.conn, running, now)?));
        }

        if let Some(paused) = self.get_most_recent_paused_task(self.conn)? {
            return Ok(Some(self.build_snapshot(self.conn, paused, now)?));
        }

        Ok(None)
    }

    /// Edit task metadata. Rejects status updates (Spec §4 Rule 2).
    /// If estimated_mins changes on a running task and becomes positive, clears notified_at.
    pub fn edit(
        &self,
        id: i64,
        update: UpdateTask,
        now: DateTime<Utc>,
    ) -> Result<Task, ServiceError> {
        if update.status.is_some() {
            return Err(ServiceError::StatusEditNotAllowed);
        }

        let repo = TaskRepository::new(self.conn);
        repo.update(id, &update)?;

        // If the task is running and estimated_mins was updated:
        if let Some(Some(_)) = update.estimated_mins {
            let running = self.get_running_task(self.conn)?;
            if let Some(run_task) = running
                && run_task.id == id
            {
                let snap = self.build_snapshot(self.conn, run_task, now)?;
                if let Some(rem) = snap.remaining_secs
                    && rem > 0
                {
                    self.conn.execute(
                        "UPDATE tasks SET notified_at = NULL WHERE id = ?1",
                        params![id],
                    )?;
                }
            }
        }

        self.fetch_task(self.conn, id)
    }

    /// Delete a task. Cascades open session cleanly.
    pub fn delete(&self, id: i64) -> Result<(), ServiceError> {
        let repo = TaskRepository::new(self.conn);
        repo.delete(id).map_err(|e| match e {
            DbError::NotFound { entity, id } => ServiceError::NotFound { entity, id },
            other => ServiceError::Db(other),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::db::models::{NewTask, Priority, SessionEndReason, TaskStatus};

    fn setup() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn create_test_task(db: &Database, name: &str, estimate: Option<i64>) -> Task {
        let repo = TaskRepository::new(db.conn());
        repo.create(&NewTask {
            workspace_id: 1,
            media_list_id: None,
            name: name.to_string(),
            description: None,
            priority: Priority::NotYet,
            estimated_mins: estimate,
            scheduled_on: None,
        })
        .unwrap()
    }

    fn parse_time(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn single_running_task_enforced_and_switch_works() {
        let db = setup();
        let service = TaskService::new(db.conn());

        let t1 = create_test_task(&db, "Task 1", Some(20));
        let t2 = create_test_task(&db, "Task 2", Some(30));

        let time0 = parse_time("2026-10-12T09:00:00Z");
        let snap1 = service.start(t1.id, false, time0).unwrap();
        assert_eq!(snap1.task.id, t1.id);
        assert_eq!(snap1.state, TimerState::Running);

        // Attempting to start t2 without --switch must fail and name t1
        let time1 = parse_time("2026-10-12T09:05:00Z");
        let err = service.start(t2.id, false, time1).unwrap_err();
        match err {
            ServiceError::AnotherTaskRunning { id, name } => {
                assert_eq!(id, t1.id);
                assert_eq!(name, "Task 1");
            }
            other => panic!("expected AnotherTaskRunning, got {:?}", other),
        }

        // Starting t2 WITH switch pauses t1 and starts t2
        let snap2 = service.start(t2.id, true, time1).unwrap();
        assert_eq!(snap2.task.id, t2.id);
        assert_eq!(snap2.state, TimerState::Running);

        // Verify t1 is now paused
        let t1_fetched = service.fetch_task(db.conn(), t1.id).unwrap();
        assert_eq!(t1_fetched.status, TaskStatus::Paused);
    }

    #[test]
    fn pause_resume_pause_elapsed_accumulates() {
        let db = setup();
        let service = TaskService::new(db.conn());
        let task = create_test_task(&db, "Study", Some(30)); // 1800s

        // 1. Start at 10:00
        let t0 = parse_time("2026-10-12T10:00:00Z");
        service.start(task.id, false, t0).unwrap();

        // 2. Pause at 10:10 (10 mins = 600s)
        let t1 = parse_time("2026-10-12T10:10:00Z");
        let snap_pause = service.pause(None, t1).unwrap();
        assert_eq!(snap_pause.elapsed_secs, 600);
        assert_eq!(snap_pause.state, TimerState::Paused);

        // 3. Resume at 10:20
        let t2 = parse_time("2026-10-12T10:20:00Z");
        let snap_resume = service.resume(None, t2).unwrap();
        assert_eq!(snap_resume.elapsed_secs, 600);
        assert_eq!(snap_resume.state, TimerState::Running);

        // 4. Pause again at 10:25 (5 mins into session 2 -> total 15 mins = 900s)
        let t3 = parse_time("2026-10-12T10:25:00Z");
        let snap_pause2 = service.pause(Some(task.id), t3).unwrap();
        assert_eq!(snap_pause2.elapsed_secs, 900);
        assert_eq!(snap_pause2.remaining_secs, Some(900));
        assert_eq!(snap_pause2.state, TimerState::Paused);
    }

    #[test]
    fn restart_resets_elapsed_to_zero_but_keeps_history() {
        let db = setup();
        let service = TaskService::new(db.conn());
        let task = create_test_task(&db, "Code", Some(25));

        let t0 = parse_time("2026-10-12T10:00:00Z");
        service.start(task.id, false, t0).unwrap();

        let t1 = parse_time("2026-10-12T10:15:00Z"); // 15 mins
        let snap_restart = service.restart(None, t1).unwrap();
        assert_eq!(snap_restart.task.run_no, 1);
        assert_eq!(snap_restart.elapsed_secs, 0);
        assert_eq!(snap_restart.remaining_secs, Some(1500)); // 25 mins

        // Check history: run 0 session is still in DB with reason restarted
        let sessions = service.fetch_sessions_for_task(db.conn(), task.id).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].run_no, 0);
        assert_eq!(sessions[0].end_reason, Some(SessionEndReason::Restarted));
        assert_eq!(sessions[1].run_no, 1);
        assert!(sessions[1].ended_at.is_none());
    }

    #[test]
    fn stop_keeps_elapsed_and_run_no() {
        let db = setup();
        let service = TaskService::new(db.conn());
        let task = create_test_task(&db, "Write", Some(40));

        let t0 = parse_time("2026-10-12T10:00:00Z");
        service.start(task.id, false, t0).unwrap();

        let t1 = parse_time("2026-10-12T10:20:00Z"); // 20 mins
        let snap_stop = service.stop(None, t1).unwrap();
        assert_eq!(snap_stop.task.status, TaskStatus::Pending);
        assert_eq!(snap_stop.task.run_no, 0);
        assert_eq!(snap_stop.elapsed_secs, 1200); // 20 mins preserved

        // Starting again continues run 0
        let t2 = parse_time("2026-10-12T11:00:00Z");
        let snap_start2 = service.start(task.id, false, t2).unwrap();
        assert_eq!(snap_start2.task.run_no, 0);
        assert_eq!(snap_start2.elapsed_secs, 1200);
    }

    #[test]
    fn two_paused_tasks_keep_separate_elapsed() {
        let db = setup();
        let service = TaskService::new(db.conn());
        let t1 = create_test_task(&db, "Task 1", Some(30));
        let t2 = create_test_task(&db, "Task 2", Some(30));

        // Start t1 for 10 mins, then pause
        service
            .start(t1.id, false, parse_time("2026-10-12T10:00:00Z"))
            .unwrap();
        service
            .pause(None, parse_time("2026-10-12T10:10:00Z"))
            .unwrap();

        // Start t2 for 25 mins, then pause
        service
            .start(t2.id, false, parse_time("2026-10-12T10:15:00Z"))
            .unwrap();
        service
            .pause(None, parse_time("2026-10-12T10:40:00Z"))
            .unwrap();

        let snap1 = service
            .build_snapshot(
                db.conn(),
                service.fetch_task(db.conn(), t1.id).unwrap(),
                parse_time("2026-10-12T11:00:00Z"),
            )
            .unwrap();
        let snap2 = service
            .build_snapshot(
                db.conn(),
                service.fetch_task(db.conn(), t2.id).unwrap(),
                parse_time("2026-10-12T11:00:00Z"),
            )
            .unwrap();

        assert_eq!(snap1.elapsed_secs, 600); // 10 mins
        assert_eq!(snap2.elapsed_secs, 1500); // 25 mins
    }

    #[test]
    fn status_edit_rejected() {
        let db = setup();
        let service = TaskService::new(db.conn());
        let task = create_test_task(&db, "Test", None);

        let update = UpdateTask {
            status: Some(TaskStatus::Completed),
            ..Default::default()
        };

        let res = service.edit(task.id, update, Utc::now());
        assert!(matches!(res, Err(ServiceError::StatusEditNotAllowed)));
    }

    #[test]
    fn delete_running_task_cascades_session() {
        let db = setup();
        let service = TaskService::new(db.conn());
        let task = create_test_task(&db, "Delete me", None);

        service.start(task.id, false, Utc::now()).unwrap();
        assert!(service.get_running_task(db.conn()).unwrap().is_some());

        service.delete(task.id).unwrap();
        assert!(service.get_running_task(db.conn()).unwrap().is_none());

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM task_sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "deleting task must cascade all sessions");
    }

    #[test]
    fn done_requires_force_from_pending() {
        let db = setup();
        let service = TaskService::new(db.conn());
        let task = create_test_task(&db, "Pending task", None);

        let err = service.done(Some(task.id), false, Utc::now());
        assert!(err.is_err());

        let ok = service.done(Some(task.id), true, Utc::now());
        assert!(ok.is_ok());
        assert_eq!(ok.unwrap().task.status, TaskStatus::Completed);
    }
}
