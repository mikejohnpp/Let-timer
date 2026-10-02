use rusqlite::{Connection, params};

use super::error::DbError;
use super::media_repository::MediaRepository;
use super::models::{Media, MediaList, NewMediaList, UpdateMediaList};

/// Provides CRUD operations on the `media_list` table plus membership
/// management in the `media_media_list` join table.
pub struct MediaListRepository<'a> {
    conn: &'a Connection,
}

impl<'a> MediaListRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn row_to_media_list(row: &rusqlite::Row) -> rusqlite::Result<MediaList> {
        Ok(MediaList {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn query_by_id(&self, id: i64) -> Result<MediaList, DbError> {
        self.conn
            .query_row(
                "SELECT * FROM media_list WHERE id = ?1",
                params![id],
                Self::row_to_media_list,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entity: "media_list",
                    id,
                },
                other => DbError::Sqlite(other),
            })
    }

    fn touch_updated_at(&self, id: i64) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE media_list SET updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    fn exists(&self, table: &str, id: i64) -> Result<bool, DbError> {
        let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)");
        let found: i64 = self.conn.query_row(&sql, params![id], |row| row.get(0))?;
        Ok(found != 0)
    }

    fn ensure_exists(&self, table: &'static str, id: i64) -> Result<(), DbError> {
        if self.exists(table, id)? {
            Ok(())
        } else {
            Err(DbError::NotFound { entity: table, id })
        }
    }

    // ─── CRUD ────────────────────────────────────────────────────────

    pub fn create(&self, new_media_list: &NewMediaList) -> Result<MediaList, DbError> {
        self.conn.execute(
            "INSERT INTO media_list (name, description) VALUES (?1, ?2)",
            params![new_media_list.name, new_media_list.description],
        )?;
        let id = self.conn.last_insert_rowid();
        self.query_by_id(id)
    }

    pub fn get_by_id(&self, id: i64) -> Result<MediaList, DbError> {
        self.query_by_id(id)
    }

    pub fn update(&self, id: i64, update: &UpdateMediaList) -> Result<MediaList, DbError> {
        self.query_by_id(id)?;

        if let Some(ref name) = update.name {
            self.conn.execute(
                "UPDATE media_list SET name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
        }

        if let Some(ref desc_opt) = update.description {
            self.conn.execute(
                "UPDATE media_list SET description = ?1 WHERE id = ?2",
                params![desc_opt.as_deref(), id],
            )?;
        }

        self.touch_updated_at(id)?;
        self.query_by_id(id)
    }

    pub fn delete(&self, id: i64) -> Result<(), DbError> {
        let rows = self
            .conn
            .execute("DELETE FROM media_list WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(DbError::NotFound {
                entity: "media_list",
                id,
            });
        }
        Ok(())
    }

    // ─── Query ───────────────────────────────────────────────────────

    pub fn list_all(&self) -> Result<Vec<MediaList>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM media_list ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], Self::row_to_media_list)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_by_name(&self, query: &str) -> Result<Vec<MediaList>, DbError> {
        let pattern = format!("%{query}%");
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM media_list WHERE name LIKE ?1 ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map(params![pattern], Self::row_to_media_list)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ─── Membership ──────────────────────────────────────────────────

    /// Attach `media_id` to `media_list_id`.
    pub fn add_media(&self, media_list_id: i64, media_id: i64) -> Result<(), DbError> {
        self.ensure_exists("media_list", media_list_id)?;
        self.ensure_exists("medias", media_id)?;

        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO media_media_list (media_id, media_list_id)
             VALUES (?1, ?2)",
            params![media_id, media_list_id],
        )?;
        if rows == 0 {
            return Err(DbError::AlreadyExists {
                entity: "media in media_list",
            });
        }
        Ok(())
    }

    /// Detach `media_id` from `media_list_id`.
    pub fn remove_media(&self, media_list_id: i64, media_id: i64) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM media_media_list WHERE media_id = ?1 AND media_list_id = ?2",
            params![media_id, media_list_id],
        )?;
        Ok(())
    }

    /// Check whether `media_id` is attached to `media_list_id`.
    pub fn is_member(&self, media_list_id: i64, media_id: i64) -> Result<bool, DbError> {
        let found: i64 = self.conn.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM media_media_list
                 WHERE media_id = ?1 AND media_list_id = ?2
             )",
            params![media_id, media_list_id],
            |row| row.get(0),
        )?;
        Ok(found != 0)
    }

    /// List the medias attached to a media list.
    pub fn list_medias(&self, media_list_id: i64) -> Result<Vec<Media>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.* FROM medias m
             JOIN media_media_list mml ON mml.media_id = m.id
             WHERE mml.media_list_id = ?1
             ORDER BY mml.created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![media_list_id], MediaRepository::row_to_media)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// List the media lists that contain a given media.
    pub fn list_media_lists(&self, media_id: i64) -> Result<Vec<MediaList>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT ml.* FROM media_list ml
             JOIN media_media_list mml ON mml.media_list_id = ml.id
             WHERE mml.media_id = ?1
             ORDER BY mml.created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![media_id], Self::row_to_media_list)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use crate::db::media_repository::MediaRepository;
    use crate::db::models::{Media, MediaType, NewMedia};

    fn setup() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn sample() -> NewMediaList {
        NewMediaList {
            name: "Focus Mix".to_string(),
            description: Some("Deep focus playlist".to_string()),
        }
    }

    fn sample_media(repo: &MediaRepository) -> Media {
        repo.create(&NewMedia {
            name: "Rain".to_string(),
            description: None,
            url: "file:///sound.mp3".to_string(),
            media_type: MediaType::Music,
        })
        .unwrap()
    }

    #[test]
    fn create_and_get_by_id() {
        let db = setup();
        let repo = MediaListRepository::new(db.conn());
        let list = repo.create(&sample()).unwrap();
        assert_eq!(list.name, "Focus Mix");
        assert_eq!(list.description.as_deref(), Some("Deep focus playlist"));
    }

    #[test]
    fn update_and_delete() {
        let db = setup();
        let repo = MediaListRepository::new(db.conn());
        let list = repo.create(&sample()).unwrap();

        let updated = repo
            .update(
                list.id,
                &UpdateMediaList {
                    description: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(updated.description.is_none());

        repo.delete(list.id).unwrap();
        assert!(matches!(
            repo.get_by_id(list.id),
            Err(DbError::NotFound {
                entity: "media_list",
                ..
            })
        ));
    }

    #[test]
    fn membership_add_remove_query() {
        let db = setup();
        let list_repo = MediaListRepository::new(db.conn());
        let media_repo = MediaRepository::new(db.conn());

        let list = list_repo.create(&sample()).unwrap();
        let media = sample_media(&media_repo);

        assert!(!list_repo.is_member(list.id, media.id).unwrap());
        list_repo.add_media(list.id, media.id).unwrap();
        assert!(list_repo.is_member(list.id, media.id).unwrap());

        let medias = list_repo.list_medias(list.id).unwrap();
        assert_eq!(medias.len(), 1);
        assert_eq!(medias[0].id, media.id);

        let lists = list_repo.list_media_lists(media.id).unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].id, list.id);

        list_repo.remove_media(list.id, media.id).unwrap();
        assert!(!list_repo.is_member(list.id, media.id).unwrap());
    }

    #[test]
    fn add_duplicate_member_errors() {
        let db = setup();
        let list_repo = MediaListRepository::new(db.conn());
        let media_repo = MediaRepository::new(db.conn());

        let list = list_repo.create(&sample()).unwrap();
        let media = sample_media(&media_repo);

        list_repo.add_media(list.id, media.id).unwrap();
        assert!(matches!(
            list_repo.add_media(list.id, media.id),
            Err(DbError::AlreadyExists { .. })
        ));
    }

    #[test]
    fn add_missing_media_errors() {
        let db = setup();
        let list_repo = MediaListRepository::new(db.conn());
        let list = list_repo.create(&sample()).unwrap();

        assert!(matches!(
            list_repo.add_media(list.id, 999),
            Err(DbError::NotFound {
                entity: "medias",
                id: 999
            })
        ));
    }
}
