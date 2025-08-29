use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum StarredItem {
    Branch(String),
    Commit(String),
}

pub struct StarDb {
    conn: Connection,
}

impl StarDb {
    pub fn new() -> Result<Self> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("librefork");
        std::fs::create_dir_all(&path)?;
        path.push("stars.db");
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS stars (
                repo TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                PRIMARY KEY(repo, kind, name)
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn load(&self, repo: &str) -> Result<HashSet<StarredItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT kind, name FROM stars WHERE repo = ?1")?;
        let iter = stmt.query_map([repo], |row| {
            let kind: String = row.get(0)?;
            let name: String = row.get(1)?;
            let item = if kind == "branch" {
                StarredItem::Branch(name)
            } else {
                StarredItem::Commit(name)
            };
            Ok(item)
        })?;
        let mut set = HashSet::new();
        for r in iter {
            set.insert(r?);
        }
        Ok(set)
    }

    pub fn add(&self, repo: &str, item: &StarredItem) -> Result<()> {
        let (kind, name) = match item {
            StarredItem::Branch(n) => ("branch", n),
            StarredItem::Commit(n) => ("commit", n),
        };
        self.conn.execute(
            "INSERT OR IGNORE INTO stars (repo, kind, name) VALUES (?1, ?2, ?3)",
            params![repo, kind, name],
        )?;
        Ok(())
    }

    pub fn remove(&self, repo: &str, item: &StarredItem) -> Result<()> {
        let (kind, name) = match item {
            StarredItem::Branch(n) => ("branch", n),
            StarredItem::Commit(n) => ("commit", n),
        };
        self.conn.execute(
            "DELETE FROM stars WHERE repo = ?1 AND kind = ?2 AND name = ?3",
            params![repo, kind, name],
        )?;
        Ok(())
    }
}
