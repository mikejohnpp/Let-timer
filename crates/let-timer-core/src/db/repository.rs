use rusqlite::{Connection, params};

use super::error::DbError;
use super::models::{NewTask, Priority, Task, TaskStatus, UpdateTask};

/// Provides CRUD and timer operations on the `tasks` table.
pub struct TaskRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TaskRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ─── Helpers ─────────────────────────────────────────────────────

    /// Map a `rusqlite::Row` into a `Task`.
    fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        let priority_raw: i64 = row.get("priority")?;
        let status_raw: String = row.get("status")?;

        Ok(Task {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            priority: Priority::from_i64(priority_raw).unwrap_or(Priority::NotYet),
            status: status_raw.parse().unwrap_or(TaskStatus::Pending),
            estimated_mins: row.get("estimated_mins")?,
            elapsed_secs: row.get("elapsed_secs")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Touch `updated_at` to the current time for a given task.
    fn touch_updated_at(&self, id: i64) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE tasks SET updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    // ─── CRUD ────────────────────────────────────────────────────────

    /// Insert a new task and return the created `Task`.
    pub fn create(&self, new_task: &NewTask) -> Result<Task, DbError> {
        self.conn.execute(
            "INSERT INTO tasks (name, description, priority, estimated_mins)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                new_task.name,
                new_task.description,
                new_task.priority as i64,
                new_task.estimated_mins,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Retrieve a single task by its ID.
    pub fn get_by_id(&self, id: i64) -> Result<Task, DbError> {
        self.conn
            .query_row(
                "SELECT * FROM tasks WHERE id = ?1",
                params![id],
                Self::row_to_task,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(id),
                other => DbError::Sqlite(other),
            })
    }

    /// Update specific fields of a task. Only `Some` fields are changed.
    pub fn update(&self, id: i64, update: &UpdateTask) -> Result<Task, DbError> {
        // Verify the task exists first.
        self.get_by_id(id)?;

        if let Some(ref name) = update.name {
            self.conn.execute(
                "UPDATE tasks SET name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
        }

        if let Some(ref desc_opt) = update.description {
            self.conn.execute(
                "UPDATE tasks SET description = ?1 WHERE id = ?2",
                params![desc_opt.as_deref(), id],
            )?;
        }

        if let Some(priority) = update.priority {
            self.conn.execute(
                "UPDATE tasks SET priority = ?1 WHERE id = ?2",
                params![priority as i64, id],
            )?;
        }

        if let Some(ref est_opt) = update.estimated_mins {
            self.conn.execute(
                "UPDATE tasks SET estimated_mins = ?1 WHERE id = ?2",
                params![*est_opt, id],
            )?;
        }

        self.touch_updated_at(id)?;
        self.get_by_id(id)
    }

    /// Delete a task by ID. Returns `DbError::NotFound` if it doesn't exist.
    pub fn delete(&self, id: i64) -> Result<(), DbError> {
        let rows = self
            .conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(DbError::NotFound(id));
        }
        Ok(())
    }

    // ─── Query ───────────────────────────────────────────────────────

    /// List all tasks (ordered by `created_at` descending).
    pub fn list_all(&self) -> Result<Vec<Task>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks ORDER BY created_at DESC")?;
        let tasks = stmt
            .query_map([], Self::row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    /// Search tasks whose name contains `query` (case-insensitive).
    pub fn find_by_name(&self, query: &str) -> Result<Vec<Task>, DbError> {
        let pattern = format!("%{query}%");
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks WHERE name LIKE ?1 ORDER BY created_at DESC")?;
        let tasks = stmt
            .query_map(params![pattern], Self::row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    /// List tasks filtered by status.
    pub fn list_by_status(&self, status: TaskStatus) -> Result<Vec<Task>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks WHERE status = ?1 ORDER BY created_at DESC")?;
        let tasks = stmt
            .query_map(params![status.as_str()], Self::row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    /// List all tasks sorted by priority.
    pub fn list_sorted_by_priority(&self, ascending: bool) -> Result<Vec<Task>, DbError> {
        let sql = if ascending {
            "SELECT * FROM tasks ORDER BY priority ASC, created_at DESC"
        } else {
            "SELECT * FROM tasks ORDER BY priority DESC, created_at DESC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let tasks = stmt
            .query_map([], Self::row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    // ─── Timer Operations ────────────────────────────────────────────

    /// Get the currently running task (status = `InProgress`), if any.
    pub fn get_current(&self) -> Result<Option<Task>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks WHERE status = 'in_progress' LIMIT 1")?;
        let mut rows = stmt
            .query_map([], Self::row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.pop())
    }

    /// Start a task: set its status to `InProgress`.
    ///
    /// Returns `DbError::AlreadyInProgress` if another task is already running.
    pub fn start_task(&self, id: i64) -> Result<Task, DbError> {
        // Check that no other task is already in progress.
        if let Some(current) = self.get_current()? {
            if current.id != id {
                return Err(DbError::AlreadyInProgress);
            }
            // Already running this same task — just return it.
            return Ok(current);
        }

        self.conn.execute(
            "UPDATE tasks SET status = 'in_progress', updated_at = datetime('now')
             WHERE id = ?1",
            params![id],
        )?;
        self.get_by_id(id)
    }

    /// Stop a task: set its status back to `Pending` and accumulate elapsed seconds.
    pub fn stop_task(&self, id: i64, elapsed: i64) -> Result<Task, DbError> {
        let task = self.get_by_id(id)?;
        let new_elapsed = task.elapsed_secs + elapsed;

        self.conn.execute(
            "UPDATE tasks SET status = 'pending', elapsed_secs = ?1, updated_at = datetime('now')
             WHERE id = ?2",
            params![new_elapsed, id],
        )?;
        self.get_by_id(id)
    }

    /// Mark a task as done.
    pub fn done_task(&self, id: i64) -> Result<Task, DbError> {
        self.conn.execute(
            "UPDATE tasks SET status = 'done', updated_at = datetime('now')
             WHERE id = ?1",
            params![id],
        )?;
        self.get_by_id(id)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;

    /// Helper: open an in-memory DB and return the connection.
    fn setup() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn sample_task() -> NewTask {
        NewTask {
            name: "Write report".to_string(),
            description: Some("Quarterly report".to_string()),
            priority: Priority::Immediate,
            estimated_mins: Some(30),
        }
    }

    // ── CRUD ──

    #[test]
    fn create_and_get_by_id() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());

        let task = repo.create(&sample_task()).unwrap();
        assert_eq!(task.name, "Write report");
        assert_eq!(task.priority, Priority::Immediate);
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.elapsed_secs, 0);

        let fetched = repo.get_by_id(task.id).unwrap();
        assert_eq!(fetched.id, task.id);
    }

    #[test]
    fn get_by_id_not_found() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());

        let result = repo.get_by_id(999);
        assert!(matches!(result, Err(DbError::NotFound(999))));
    }

    #[test]
    fn update_partial_fields() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        let task = repo.create(&sample_task()).unwrap();

        let updated = repo
            .update(
                task.id,
                &UpdateTask {
                    name: Some("Updated name".to_string()),
                    priority: Some(Priority::Urgent),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(updated.name, "Updated name");
        assert_eq!(updated.priority, Priority::Urgent);
        // Unchanged fields stay the same.
        assert_eq!(updated.description, Some("Quarterly report".to_string()));
    }

    #[test]
    fn update_set_description_to_null() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        let task = repo.create(&sample_task()).unwrap();
        assert!(task.description.is_some());

        let updated = repo
            .update(
                task.id,
                &UpdateTask {
                    description: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(updated.description.is_none());
    }

    #[test]
    fn delete_existing() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        let task = repo.create(&sample_task()).unwrap();

        repo.delete(task.id).unwrap();
        assert!(matches!(repo.get_by_id(task.id), Err(DbError::NotFound(_))));
    }

    #[test]
    fn delete_not_found() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        assert!(matches!(repo.delete(999), Err(DbError::NotFound(999))));
    }

    // ── Query ──

    #[test]
    fn list_all() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        repo.create(&sample_task()).unwrap();
        repo.create(&NewTask {
            name: "Second task".to_string(),
            description: None,
            priority: Priority::NotYet,
            estimated_mins: None,
        })
        .unwrap();

        let all = repo.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn find_by_name() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        repo.create(&sample_task()).unwrap();
        repo.create(&NewTask {
            name: "Buy groceries".to_string(),
            description: None,
            priority: Priority::NotYet,
            estimated_mins: None,
        })
        .unwrap();

        let results = repo.find_by_name("report").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Write report");
    }

    #[test]
    fn list_by_status() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        let t = repo.create(&sample_task()).unwrap();
        repo.done_task(t.id).unwrap();

        let done = repo.list_by_status(TaskStatus::Done).unwrap();
        assert_eq!(done.len(), 1);

        let pending = repo.list_by_status(TaskStatus::Pending).unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn list_sorted_by_priority() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());

        repo.create(&NewTask {
            name: "Low".to_string(),
            description: None,
            priority: Priority::NotYet,
            estimated_mins: None,
        })
        .unwrap();
        repo.create(&NewTask {
            name: "High".to_string(),
            description: None,
            priority: Priority::Urgent,
            estimated_mins: None,
        })
        .unwrap();

        let asc = repo.list_sorted_by_priority(true).unwrap();
        assert_eq!(asc[0].name, "Low");
        assert_eq!(asc[1].name, "High");

        let desc = repo.list_sorted_by_priority(false).unwrap();
        assert_eq!(desc[0].name, "High");
        assert_eq!(desc[1].name, "Low");
    }

    // ── Timer ──

    #[test]
    fn start_stop_done_flow() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        let task = repo.create(&sample_task()).unwrap();

        // Start
        let started = repo.start_task(task.id).unwrap();
        assert_eq!(started.status, TaskStatus::InProgress);

        // get_current should return this task
        let current = repo.get_current().unwrap();
        assert!(current.is_some());
        assert_eq!(current.unwrap().id, task.id);

        // Stop with 120 seconds elapsed
        let stopped = repo.stop_task(task.id, 120).unwrap();
        assert_eq!(stopped.status, TaskStatus::Pending);
        assert_eq!(stopped.elapsed_secs, 120);

        // get_current should be None now
        assert!(repo.get_current().unwrap().is_none());

        // Start again, stop with 60 more seconds
        repo.start_task(task.id).unwrap();
        let stopped2 = repo.stop_task(task.id, 60).unwrap();
        assert_eq!(stopped2.elapsed_secs, 180); // accumulated

        // Done
        let done = repo.done_task(task.id).unwrap();
        assert_eq!(done.status, TaskStatus::Done);
    }

    #[test]
    fn start_task_already_in_progress() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());

        let t1 = repo.create(&sample_task()).unwrap();
        let t2 = repo
            .create(&NewTask {
                name: "Other".to_string(),
                description: None,
                priority: Priority::NotYet,
                estimated_mins: None,
            })
            .unwrap();

        repo.start_task(t1.id).unwrap();

        // Trying to start a different task should fail.
        let result = repo.start_task(t2.id);
        assert!(matches!(result, Err(DbError::AlreadyInProgress)));
    }

    #[test]
    fn start_same_task_twice_is_idempotent() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        let task = repo.create(&sample_task()).unwrap();

        repo.start_task(task.id).unwrap();
        // Starting the same task again should just return it.
        let again = repo.start_task(task.id).unwrap();
        assert_eq!(again.status, TaskStatus::InProgress);
    }
}
