use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use super::error::DbError;
use super::models::{SessionEndReason, TaskSession};

/// Provides persistence operations for the `task_sessions` table.
pub struct SessionRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SessionRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Map a row to a `TaskSession`.
    fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<TaskSession> {
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
    }

    /// Open a new session for a task run.
    pub fn open(
        &self,
        task_id: i64,
        run_no: i64,
        now: DateTime<Utc>,
    ) -> Result<TaskSession, DbError> {
        let now_str = now.to_rfc3339();
        self.conn.execute(
            "INSERT INTO task_sessions (task_id, run_no, started_at, ended_at, end_reason)
             VALUES (?1, ?2, ?3, NULL, NULL)",
            params![task_id, run_no, now_str],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Retrieve a single session by ID.
    pub fn get_by_id(&self, id: i64) -> Result<TaskSession, DbError> {
        self.conn
            .query_row(
                "SELECT id, task_id, run_no, started_at, ended_at, end_reason
                 FROM task_sessions WHERE id = ?1",
                params![id],
                Self::row_to_session,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "task_session",
                    id,
                },
                other => DbError::Sqlite(other),
            })
    }

    /// Close the currently open session (if any) with a given timestamp and end reason.
    pub fn close_open(
        &self,
        now: DateTime<Utc>,
        reason: SessionEndReason,
    ) -> Result<Option<TaskSession>, DbError> {
        let open_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM task_sessions WHERE ended_at IS NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        match open_id {
            Some(id) => {
                let now_str = now.to_rfc3339();
                self.conn.execute(
                    "UPDATE task_sessions SET ended_at = ?1, end_reason = ?2 WHERE id = ?3",
                    params![now_str, reason.as_str(), id],
                )?;
                Ok(Some(self.get_by_id(id)?))
            }
            None => Ok(None),
        }
    }

    /// Retrieve the currently open session (at most 1 in the whole table).
    pub fn open_session(&self) -> Result<Option<TaskSession>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, run_no, started_at, ended_at, end_reason
             FROM task_sessions WHERE ended_at IS NULL LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], Self::row_to_session)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// List all sessions for a specific run of a task, ordered by start time.
    pub fn sessions_for_run(&self, task_id: i64, run_no: i64) -> Result<Vec<TaskSession>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, run_no, started_at, ended_at, end_reason
             FROM task_sessions
             WHERE task_id = ?1 AND run_no = ?2
             ORDER BY started_at ASC",
        )?;
        let sessions = stmt
            .query_map(params![task_id, run_no], Self::row_to_session)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    /// List all sessions for a task across all runs, ordered by run_no, started_at.
    pub fn all_sessions_for_task(&self, task_id: i64) -> Result<Vec<TaskSession>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, run_no, started_at, ended_at, end_reason
             FROM task_sessions
             WHERE task_id = ?1
             ORDER BY run_no ASC, started_at ASC",
        )?;
        let sessions = stmt
            .query_map(params![task_id], Self::row_to_session)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;

    fn setup() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn sample_task_id(db: &Database) -> i64 {
        db.conn()
            .execute(
                "INSERT INTO tasks (workspace_id, name) VALUES (1, 'Test')",
                [],
            )
            .unwrap();
        db.conn().last_insert_rowid()
    }

    #[test]
    fn open_and_close_session() {
        let db = setup();
        let repo = SessionRepository::new(db.conn());
        let task_id = sample_task_id(&db);

        let t0 = DateTime::parse_from_rfc3339("2026-10-12T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let session = repo.open(task_id, 0, t0).unwrap();
        assert_eq!(session.task_id, task_id);
        assert_eq!(session.run_no, 0);
        assert_eq!(session.started_at, t0);
        assert!(session.ended_at.is_none());
        assert!(session.end_reason.is_none());

        let open = repo.open_session().unwrap();
        assert!(open.is_some());
        assert_eq!(open.unwrap().id, session.id);

        let t1 = DateTime::parse_from_rfc3339("2026-10-12T10:25:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let closed = repo
            .close_open(t1, SessionEndReason::Paused)
            .unwrap()
            .unwrap();
        assert_eq!(closed.id, session.id);
        assert_eq!(closed.ended_at, Some(t1));
        assert_eq!(closed.end_reason, Some(SessionEndReason::Paused));

        assert!(repo.open_session().unwrap().is_none());
    }

    #[test]
    fn sessions_for_run_ordering() {
        let db = setup();
        let repo = SessionRepository::new(db.conn());
        let task_id = sample_task_id(&db);

        let t0 = DateTime::parse_from_rfc3339("2026-10-12T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t1 = DateTime::parse_from_rfc3339("2026-10-12T09:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        repo.open(task_id, 0, t0).unwrap();
        repo.close_open(t1, SessionEndReason::Paused).unwrap();

        let t2 = DateTime::parse_from_rfc3339("2026-10-12T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t3 = DateTime::parse_from_rfc3339("2026-10-12T10:15:00Z")
            .unwrap()
            .with_timezone(&Utc);
        repo.open(task_id, 0, t2).unwrap();
        repo.close_open(t3, SessionEndReason::Completed).unwrap();

        let sessions = repo.sessions_for_run(task_id, 0).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].started_at, t0);
        assert_eq!(sessions[1].started_at, t2);
    }
}
