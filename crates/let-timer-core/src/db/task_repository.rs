use rusqlite::{Connection, params};

use crate::SortOrder;

use super::error::DbError;
use super::models::{NewTask, Priority, Task, TaskStatus, UpdateTask};

/// Provides CRUD and status-transition operations on the `tasks` table.
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
            workspace_id: row.get::<_, i64>("workspace_id")?,
            media_list_id: row.get::<_, Option<i64>>("media_list_id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            priority: Priority::from_i64(priority_raw).unwrap_or(Priority::NotYet),
            status: status_raw.parse().unwrap_or(TaskStatus::Pending),
            estimated_mins: row.get("estimated_mins")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn query_by_id(&self, id: i64) -> Result<Task, DbError> {
        self.conn
            .query_row(
                "SELECT * FROM tasks WHERE id = ?1",
                params![id],
                Self::row_to_task,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound { entity: "task", id },
                other => DbError::Sqlite(other),
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

    /// Set `status` on a task. Returns `DbError::NotFound` if missing.
    fn set_status(&self, id: i64, status: TaskStatus) -> Result<Task, DbError> {
        self.query_by_id(id)?;
        self.conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        self.query_by_id(id)
    }

    // ─── CRUD ────────────────────────────────────────────────────────

    /// Insert a new task and return the created `Task`.
    pub fn create(&self, new_task: &NewTask) -> Result<Task, DbError> {
        let ws_exists: i64 = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
            params![new_task.workspace_id],
            |row| row.get(0),
        )?;
        if ws_exists == 0 {
            return Err(DbError::NotFound {
                entity: "workspace",
                id: new_task.workspace_id,
            });
        }

        if let Some(media_list_id) = new_task.media_list_id {
            let ml_exists: i64 = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM media_list WHERE id = ?1)",
                params![media_list_id],
                |row| row.get(0),
            )?;
            if ml_exists == 0 {
                return Err(DbError::NotFound { entity: "media_list", id: media_list_id });
            }
        }

        self.conn.execute(
            "INSERT INTO tasks (workspace_id, media_list_id, name, description, priority, estimated_mins)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                new_task.workspace_id,
                new_task.media_list_id,
                new_task.name,
                new_task.description,
                new_task.priority as i64,
                new_task.estimated_mins,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.query_by_id(id)
    }

    /// Retrieve a single task by its ID.
    pub fn get_by_id(&self, id: i64) -> Result<Task, DbError> {
        self.query_by_id(id)
    }

    /// Update specific fields of a task. Only `Some` fields are changed.
    pub fn update(&self, id: i64, update: &UpdateTask) -> Result<Task, DbError> {
        self.query_by_id(id)?;

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

        if let Some(ref ml_opt) = update.media_list_id {
            if let Some(media_list_id) = *ml_opt {
                let ml_exists: i64 = self.conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM media_list WHERE id = ?1)",
                    params![media_list_id],
                    |row| row.get(0),
                )?;
                if ml_exists == 0 {
                    return Err(DbError::NotFound { entity: "media_list", id: media_list_id });
                }
            }
            self.conn.execute(
                "UPDATE tasks SET media_list_id = ?1 WHERE id = ?2",
                params![*ml_opt, id],
            )?;
        }

        if let Some(priority) = update.priority {
            self.conn.execute(
                "UPDATE tasks SET priority = ?1 WHERE id = ?2",
                params![priority as i64, id],
            )?;
        }

        if let Some(status) = update.status {
            self.conn.execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                params![status.as_str(), id],
            )?;
        }

        if let Some(estimated_mins) = update.estimated_mins {
            self.conn.execute(
                "UPDATE tasks SET estimated_mins = ?1 WHERE id = ?2",
                params![estimated_mins, id],
            )?;
        }

        self.touch_updated_at(id)?;
        self.query_by_id(id)
    }

    /// Delete a task by ID. Returns `DbError::NotFound` if it doesn't exist.
    pub fn delete(&self, id: i64) -> Result<(), DbError> {
        let rows = self
            .conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(DbError::NotFound { entity: "task", id });
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

    /// List tasks belonging to a workspace.
    pub fn list_by_workspace(&self, workspace_id: i64) -> Result<Vec<Task>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM tasks WHERE workspace_id = ?1 ORDER BY created_at DESC",
        )?;
        let tasks = stmt
            .query_map(params![workspace_id], Self::row_to_task)?
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
    pub fn list_sorted_by_priority(&self, sort_by: SortOrder) -> Result<Vec<Task>, DbError> {
        let sql = match sort_by {
            SortOrder::Ascending => "SELECT * FROM tasks ORDER BY priority ASC, created_at DESC",
            SortOrder::Descending => "SELECT * FROM tasks ORDER BY priority DESC, created_at DESC",
        };
        let mut stmt = self.conn.prepare(sql)?;
        let tasks = stmt
            .query_map([], Self::row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    // ─── Status Operations ───────────────────────────────────────────

    /// Get the currently running task (status = `InProgress`), if any.
    pub fn get_current(&self) -> Result<Option<Task>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks WHERE status = 'in-progress' LIMIT 1")?;
        let mut rows = stmt
            .query_map([], Self::row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.pop())
    }

    /// Mark a task as in progress.
    pub fn start_task(&self, id: i64) -> Result<Task, DbError> {
        self.set_status(id, TaskStatus::InProgress)
    }

    /// Mark a task as completed.
    pub fn complete_task(&self, id: i64) -> Result<Task, DbError> {
        self.set_status(id, TaskStatus::Completed)
    }

    /// Cancel a task.
    pub fn cancel_task(&self, id: i64) -> Result<Task, DbError> {
        self.set_status(id, TaskStatus::Cancelled)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use crate::db::workspace_repository::WorkspaceRepository;

    fn setup() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn sample_task(workspace_id: i64) -> NewTask {
        NewTask {
            workspace_id,
            media_list_id: None,
            name: "Write report".to_string(),
            description: Some("Quarterly report".to_string()),
            priority: Priority::Immediate,
            estimated_mins: Some(30),
        }
    }

    fn sample_workspace(repo: &WorkspaceRepository) -> i64 {
        repo.create(&crate::db::models::NewWorkspace {
            name: "Work".to_string(),
            description: None,
        })
        .unwrap()
        .id
    }

    #[test]
    fn create_and_get_by_id() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);

        let task = repo.create(&sample_task(ws_id)).unwrap();
        assert_eq!(task.name, "Write report");
        assert_eq!(task.workspace_id, ws_id);
        assert_eq!(task.priority, Priority::Immediate);
        assert_eq!(task.status, TaskStatus::Pending);

        let fetched = repo.get_by_id(task.id).unwrap();
        assert_eq!(fetched.id, task.id);
    }

    #[test]
    fn create_requires_existing_workspace() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        assert!(matches!(
            repo.create(&sample_task(999)),
            Err(DbError::NotFound { entity: "workspace", id: 999 })
        ));
    }

    #[test]
    fn get_by_id_not_found() {
        let db = setup();
        let repo = TaskRepository::new(db.conn());
        assert!(matches!(
            repo.get_by_id(999),
            Err(DbError::NotFound { entity: "task", id: 999 })
        ));
    }

    #[test]
    fn update_partial_fields() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);
        let task = repo.create(&sample_task(ws_id)).unwrap();

        let updated = repo
            .update(
                task.id,
                &UpdateTask {
                    name: Some("Updated name".to_string()),
                    status: Some(TaskStatus::InProgress),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(updated.name, "Updated name");
        assert_eq!(updated.status, TaskStatus::InProgress);
        assert_eq!(updated.description, Some("Quarterly report".to_string()));
    }

    #[test]
    fn update_clear_media_list_id() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);
        let task = repo.create(&sample_task(ws_id)).unwrap();

        let updated = repo
            .update(
                task.id,
                &UpdateTask {
                    media_list_id: Some(Some(42)),
                    ..Default::default()
                },
            )
            .unwrap_err();
        assert!(matches!(
            updated,
            DbError::NotFound { entity: "media_list", id: 42 }
        ));
    }

    #[test]
    fn delete_existing() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);
        let task = repo.create(&sample_task(ws_id)).unwrap();

        repo.delete(task.id).unwrap();
        assert!(matches!(
            repo.get_by_id(task.id),
            Err(DbError::NotFound { .. })
        ));
    }

    #[test]
    fn list_and_filter() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);

        let t1 = repo.create(&sample_task(ws_id)).unwrap();
        let t2 = repo
            .create(&NewTask {
                workspace_id: ws_id,
                media_list_id: None,
                name: "Buy groceries".to_string(),
                description: None,
                priority: Priority::NotYet,
                estimated_mins: None,
            })
            .unwrap();
        let _ = (t1, t2);

        assert_eq!(repo.list_by_workspace(ws_id).unwrap().len(), 2);
        assert_eq!(repo.list_all().unwrap().len(), 2);

        let hits = repo.find_by_name("report").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Write report");
    }

    #[test]
    fn list_sorted_by_priority() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);

        repo.create(&NewTask {
            workspace_id: ws_id,
            media_list_id: None,
            name: "Low".to_string(),
            description: None,
            priority: Priority::NotYet,
            estimated_mins: None,
        })
        .unwrap();
        repo.create(&NewTask {
            workspace_id: ws_id,
            media_list_id: None,
            name: "High".to_string(),
            description: None,
            priority: Priority::Urgent,
            estimated_mins: None,
        })
        .unwrap();

        let asc = repo.list_sorted_by_priority(SortOrder::Ascending).unwrap();
        assert_eq!(asc[0].name, "Low");
        assert_eq!(asc[1].name, "High");

        let desc = repo.list_sorted_by_priority(SortOrder::Descending).unwrap();
        assert_eq!(desc[0].name, "High");
        assert_eq!(desc[1].name, "Low");
    }

    #[test]
    fn status_transitions_and_current() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);
        let task = repo.create(&sample_task(ws_id)).unwrap();

        let started = repo.start_task(task.id).unwrap();
        assert_eq!(started.status, TaskStatus::InProgress);

        let current = repo.get_current().unwrap();
        assert!(current.is_some());
        assert_eq!(current.unwrap().id, task.id);

        let done = repo.complete_task(task.id).unwrap();
        assert_eq!(done.status, TaskStatus::Completed);
        assert!(repo.get_current().unwrap().is_none());

        let cancelled = repo.cancel_task(task.id).unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);

        let in_progress = repo.list_by_status(TaskStatus::InProgress).unwrap();
        assert!(in_progress.is_empty());
        let done_list = repo.list_by_status(TaskStatus::Completed).unwrap();
        assert!(done_list.is_empty());
        let cancelled_list = repo.list_by_status(TaskStatus::Cancelled).unwrap();
        assert_eq!(cancelled_list.len(), 1);
    }

    #[test]
    fn delete_workspace_cascades_tasks() {
        let db = setup();
        let ws = WorkspaceRepository::new(db.conn());
        let repo = TaskRepository::new(db.conn());
        let ws_id = sample_workspace(&ws);
        let task = repo.create(&sample_task(ws_id)).unwrap();

        ws.delete(ws_id).unwrap();
        assert!(matches!(
            repo.get_by_id(task.id),
            Err(DbError::NotFound { entity: "task", .. })
        ));
    }
}