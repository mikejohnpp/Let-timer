use rusqlite::{Connection, params};

use super::error::DbError;
use super::models::{NewWorkspace, UpdateWorkspace, Workspace};

/// Provides CRUD operations on the `workspaces` table.
pub struct WorkspaceRepository<'a> {
    conn: &'a Connection,
}

impl<'a> WorkspaceRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn row_to_workspace(row: &rusqlite::Row) -> rusqlite::Result<Workspace> {
        Ok(Workspace {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn query_by_id(&self, id: i64) -> Result<Workspace, DbError> {
        self.conn
            .query_row(
                "SELECT * FROM workspaces WHERE id = ?1",
                params![id],
                Self::row_to_workspace,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound { entity: "workspace", id },
                other => DbError::Sqlite(other),
            })
    }

    fn touch_updated_at(&self, id: i64) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE workspaces SET updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    // ─── CRUD ────────────────────────────────────────────────────────

    pub fn create(&self, new_workspace: &NewWorkspace) -> Result<Workspace, DbError> {
        self.conn.execute(
            "INSERT INTO workspaces (name, description) VALUES (?1, ?2)",
            params![new_workspace.name, new_workspace.description],
        )?;
        let id = self.conn.last_insert_rowid();
        self.query_by_id(id)
    }

    pub fn get_by_id(&self, id: i64) -> Result<Workspace, DbError> {
        self.query_by_id(id)
    }

    pub fn update(&self, id: i64, update: &UpdateWorkspace) -> Result<Workspace, DbError> {
        self.query_by_id(id)?;

        if let Some(ref name) = update.name {
            self.conn.execute(
                "UPDATE workspaces SET name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
        }

        if let Some(ref desc_opt) = update.description {
            self.conn.execute(
                "UPDATE workspaces SET description = ?1 WHERE id = ?2",
                params![desc_opt.as_deref(), id],
            )?;
        }

        self.touch_updated_at(id)?;
        self.query_by_id(id)
    }

    pub fn delete(&self, id: i64) -> Result<(), DbError> {
        let rows = self
            .conn
            .execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(DbError::NotFound { entity: "workspace", id });
        }
        Ok(())
    }

    // ─── Query ───────────────────────────────────────────────────────

    pub fn list_all(&self) -> Result<Vec<Workspace>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM workspaces ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], Self::row_to_workspace)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_by_name(&self, query: &str) -> Result<Vec<Workspace>, DbError> {
        let pattern = format!("%{query}%");
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM workspaces WHERE name LIKE ?1 ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map(params![pattern], Self::row_to_workspace)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;

    fn setup() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn sample() -> NewWorkspace {
        NewWorkspace {
            name: "Work".to_string(),
            description: Some("Main workspace".to_string()),
        }
    }

    #[test]
    fn create_and_get_by_id() {
        let db = setup();
        let repo = WorkspaceRepository::new(db.conn());

        let ws = repo.create(&sample()).unwrap();
        assert_eq!(ws.name, "Work");
        assert_eq!(ws.description.as_deref(), Some("Main workspace"));

        let fetched = repo.get_by_id(ws.id).unwrap();
        assert_eq!(fetched.id, ws.id);
    }

    #[test]
    fn get_by_id_not_found() {
        let db = setup();
        let repo = WorkspaceRepository::new(db.conn());
        assert!(matches!(
            repo.get_by_id(999),
            Err(DbError::NotFound { entity: "workspace", id: 999 })
        ));
    }

    #[test]
    fn update_partial_fields() {
        let db = setup();
        let repo = WorkspaceRepository::new(db.conn());
        let ws = repo.create(&sample()).unwrap();

        let updated = repo
            .update(
                ws.id,
                &UpdateWorkspace {
                    name: Some("Renamed".to_string()),
                    description: Some(None),
                },
            )
            .unwrap();

        assert_eq!(updated.name, "Renamed");
        assert!(updated.description.is_none());
    }

    #[test]
    fn delete_existing_and_not_found() {
        let db = setup();
        let repo = WorkspaceRepository::new(db.conn());
        let ws = repo.create(&sample()).unwrap();

        repo.delete(ws.id).unwrap();
        assert!(matches!(
            repo.get_by_id(ws.id),
            Err(DbError::NotFound { .. })
        ));
        assert!(matches!(
            repo.delete(ws.id),
            Err(DbError::NotFound { entity: "workspace", id: _ })
        ));
    }

    #[test]
    fn list_and_find() {
        let db = setup();
        let repo = WorkspaceRepository::new(db.conn());
        repo.create(&sample()).unwrap();
        repo.create(&NewWorkspace {
            name: "Study".to_string(),
            description: None,
        })
        .unwrap();

        assert_eq!(repo.list_all().unwrap().len(), 2);
        let hits = repo.find_by_name("udy").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Study");
    }
}