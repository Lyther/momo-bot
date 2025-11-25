use anyhow::Result;
use chrono::Utc;
use tokio_rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct DbEntry {
    pub chat_id: i64,
    pub message_id: i32,
    pub user_id: i64,
    pub username: String,
    pub text: String,
    pub media_type: Option<String>,
    pub media_ref: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path).await?;
        let db = Self { conn };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<()> {
        self.conn
            .call(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS messages (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        chat_id INTEGER NOT NULL,
                        message_id INTEGER NOT NULL,
                        user_id INTEGER NOT NULL,
                        username TEXT NOT NULL,
                        text TEXT,
                        media_type TEXT,
                        media_ref TEXT,
                        summary TEXT NOT NULL,
                        created_at INTEGER NOT NULL
                    );
                    CREATE INDEX IF NOT EXISTS idx_messages_chat_created ON messages(chat_id, created_at);
                    CREATE INDEX IF NOT EXISTS idx_messages_media ON messages(chat_id, media_type);
                    "#,
                )?;

                // Check if image_cache table exists and has correct schema
                let table_exists: bool = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='image_cache'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .map(|c| c > 0)
                    .unwrap_or(false);

                if table_exists {
                    // Check if it has the file_id column
                    let has_file_id: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM pragma_table_info('image_cache') WHERE name='file_id'",
                            [],
                            |row| row.get::<_, i64>(0),
                        )
                        .map(|c| c > 0)
                        .unwrap_or(false);

                    if !has_file_id {
                        // Old schema - drop and recreate
                        log::info!("*db migration* Recreating image_cache table with new schema");
                        conn.execute("DROP TABLE image_cache", [])?;
                    }
                }

                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS image_cache (
                        file_id TEXT PRIMARY KEY,
                        mime_type TEXT NOT NULL,
                        data_url TEXT NOT NULL,
                        description TEXT,
                        created_at INTEGER NOT NULL
                    );
                    CREATE INDEX IF NOT EXISTS idx_image_cache_created ON image_cache(created_at);
                    "#,
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn log_message(&self, entry: DbEntry) -> Result<()> {
        let ts = Utc::now().timestamp();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO messages (chat_id, message_id, user_id, username, text, media_type, media_ref, summary, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    (
                        entry.chat_id,
                        entry.message_id,
                        entry.user_id,
                        entry.username,
                        entry.text,
                        entry.media_type,
                        entry.media_ref,
                        entry.summary,
                        ts,
                    ),
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn recent_summaries(&self, chat_id: i64, limit: usize) -> Result<Vec<String>> {
        let limit = limit as i64;
        let rows = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT summary FROM messages WHERE chat_id = ?1 ORDER BY created_at DESC LIMIT ?2",
                )?;
                let iter = stmt.query_map((chat_id, limit), |row| row.get::<_, String>(0))?;
                let mut out = Vec::new();
                for s in iter {
                    out.push(s?);
                }
                Ok(out)
            })
            .await?;
        Ok(rows)
    }

    pub async fn random_sticker(&self, chat_id: i64) -> Result<Option<String>> {
        let row: Option<String> = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT media_ref FROM messages WHERE chat_id = ?1 AND media_type = 'sticker' AND media_ref IS NOT NULL ORDER BY RANDOM() LIMIT 1",
                )?;
                let mut iter = stmt.query([chat_id])?;
                if let Some(row) = iter.next()? {
                    let val: Option<String> = row.get(0)?;
                    Ok(val)
                } else {
                    Ok(None)
                }
            })
            .await?;
        Ok(row)
    }

    /// Cache a processed image (data URL)
    pub async fn cache_image(&self, file_id: &str, mime_type: &str, data_url: &str) -> Result<()> {
        let ts = Utc::now().timestamp();
        let file_id = file_id.to_string();
        let mime_type = mime_type.to_string();
        let data_url = data_url.to_string();

        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO image_cache (file_id, mime_type, data_url, description, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    (file_id, mime_type, data_url, None::<String>, ts),
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Get cached image data URL and optional description
    pub async fn get_cached_image(&self, file_id: &str) -> Result<Option<CachedImage>> {
        let file_id = file_id.to_string();
        let row: Option<(String, String, Option<String>)> = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT mime_type, data_url, description FROM image_cache WHERE file_id = ?1",
                )?;
                let mut iter = stmt.query_map([file_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })?;
                if let Some(row) = iter.next() {
                    Ok(Some(row?))
                } else {
                    Ok(None)
                }
            })
            .await?;

        Ok(row.map(|(mime_type, data_url, description)| CachedImage {
            mime_type,
            data_url,
            description,
        }))
    }

    /// Set image description (for future AI-generated descriptions)
    pub async fn set_image_description(&self, file_id: &str, description: &str) -> Result<()> {
        let file_id = file_id.to_string();
        let description = description.to_string();

        self.conn
            .call(move |conn| {
                conn.execute(
                    "UPDATE image_cache SET description = ?1 WHERE file_id = ?2",
                    (description, file_id),
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Cleanup orphaned cache files (stub - implement if needed)
    pub async fn cleanup_orphaned_cache(&self, _cache_dir: &std::path::Path) -> Result<()> {
        // TODO: Implement cache cleanup logic if needed
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CachedImage {
    pub mime_type: String,
    pub data_url: String,
    pub description: Option<String>,
}
