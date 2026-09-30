use rusqlite::{Connection, params};

use super::error::DbError;
use super::models::{Media, MediaType, NewMedia, UpdateMedia};

/// Provides CRUD operations on the `medias` table.
pub struct MediaRepository<'a> {
    conn: &'a Connection,
}

impl<'a> MediaRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub(crate) fn row_to_media(row: &rusqlite::Row) -> rusqlite::Result<Media> {
        let type_raw: String = row.get("type")?;
        Ok(Media {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            url: row.get("url")?,
            media_type: type_raw.parse().unwrap_or(MediaType::Image),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn query_by_id(&self, id: i64) -> Result<Media, DbError> {
        self.conn
            .query_row("SELECT * FROM medias WHERE id = ?1", params![id], Self::row_to_media)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound { entity: "media", id },
                other => DbError::Sqlite(other),
            })
    }

    fn touch_updated_at(&self, id: i64) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE medias SET updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    // ─── CRUD ────────────────────────────────────────────────────────

    pub fn create(&self, new_media: &NewMedia) -> Result<Media, DbError> {
        self.conn.execute(
            "INSERT INTO medias (name, description, url, type) VALUES (?1, ?2, ?3, ?4)",
            params![
                new_media.name,
                new_media.description,
                new_media.url,
                new_media.media_type.as_str(),
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.query_by_id(id)
    }

    pub fn get_by_id(&self, id: i64) -> Result<Media, DbError> {
        self.query_by_id(id)
    }

    pub fn update(&self, id: i64, update: &UpdateMedia) -> Result<Media, DbError> {
        self.query_by_id(id)?;

        if let Some(ref name) = update.name {
            self.conn.execute(
                "UPDATE medias SET name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
        }

        if let Some(ref desc_opt) = update.description {
            self.conn.execute(
                "UPDATE medias SET description = ?1 WHERE id = ?2",
                params![desc_opt.as_deref(), id],
            )?;
        }

        if let Some(ref url) = update.url {
            self.conn.execute("UPDATE medias SET url = ?1 WHERE id = ?2", params![url, id])?;
        }

        if let Some(media_type) = update.media_type {
            self.conn.execute(
                "UPDATE medias SET type = ?1 WHERE id = ?2",
                params![media_type.as_str(), id],
            )?;
        }

        self.touch_updated_at(id)?;
        self.query_by_id(id)
    }

    pub fn delete(&self, id: i64) -> Result<(), DbError> {
        let rows = self
            .conn
            .execute("DELETE FROM medias WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(DbError::NotFound { entity: "media", id });
        }
        Ok(())
    }

    // ─── Query ───────────────────────────────────────────────────────

    pub fn list_all(&self) -> Result<Vec<Media>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM medias ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], Self::row_to_media)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_by_type(&self, media_type: MediaType) -> Result<Vec<Media>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM medias WHERE type = ?1 ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map(params![media_type.as_str()], Self::row_to_media)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_by_name(&self, query: &str) -> Result<Vec<Media>, DbError> {
        let pattern = format!("%{query}%");
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM medias WHERE name LIKE ?1 ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map(params![pattern], Self::row_to_media)?
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

    fn sample() -> NewMedia {
        NewMedia {
            name: "Focus Rain".to_string(),
            description: Some("Ambient track".to_string()),
            url: "file:///sounds/focus-rain.mp3".to_string(),
            media_type: MediaType::Music,
        }
    }

    #[test]
    fn create_and_get_by_id() {
        let db = setup();
        let repo = MediaRepository::new(db.conn());

        let media = repo.create(&sample()).unwrap();
        assert_eq!(media.media_type, MediaType::Music);

        let fetched = repo.get_by_id(media.id).unwrap();
        assert_eq!(fetched.url, sample().url);
    }

    #[test]
    fn get_by_id_not_found() {
        let db = setup();
        let repo = MediaRepository::new(db.conn());
        assert!(matches!(
            repo.get_by_id(999),
            Err(DbError::NotFound { entity: "media", id: 999 })
        ));
    }

    #[test]
    fn update_fields() {
        let db = setup();
        let repo = MediaRepository::new(db.conn());
        let media = repo.create(&sample()).unwrap();

        let updated = repo
            .update(
                media.id,
                &UpdateMedia {
                    name: Some("Deep Focus".to_string()),
                    media_type: Some(MediaType::Poem),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(updated.name, "Deep Focus");
        assert_eq!(updated.media_type, MediaType::Poem);
    }

    #[test]
    fn list_by_type_and_find() {
        let db = setup();
        let repo = MediaRepository::new(db.conn());
        repo.create(&sample()).unwrap();
        repo.create(&NewMedia {
            name: "Mountain Lake".to_string(),
            description: None,
            url: "file:///img/mountain.jpg".to_string(),
            media_type: MediaType::Image,
        })
        .unwrap();

        let music = repo.list_by_type(MediaType::Music).unwrap();
        assert_eq!(music.len(), 1);
        assert_eq!(music[0].name, "Focus Rain");

        let hits = repo.find_by_name("mountain").unwrap();
        assert_eq!(hits.len(), 1);
    }
}