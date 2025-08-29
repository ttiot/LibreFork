use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct RecentDb {
    conn: Connection,
}

impl RecentDb {
    pub fn new() -> Result<Self> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("librefork");
        std::fs::create_dir_all(&path)?;
        path.push("recents.db");
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS recents (\n                path TEXT PRIMARY KEY,\n                last_opened INTEGER NOT NULL\n            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn touch(&self, path: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO recents(path, last_opened) VALUES(?1, ?2)\n             ON CONFLICT(path) DO UPDATE SET last_opened = excluded.last_opened",
            params![path, now],
        )?;
        Ok(())
    }

    pub fn list(&self, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM recents ORDER BY last_opened DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit as i64], |row| {
            let p: String = row.get(0)?;
            Ok(p)
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM recents", [])?;
        Ok(())
    }
}

