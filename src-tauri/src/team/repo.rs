use rusqlite::{Connection, OptionalExtension, Row, params};

use super::models::{Account, Author};
use crate::error::AppError;
use crate::storage::db::now_ms;

const AUTHOR_KEY: &str = "team_author";
const LINK_COLUMNS: &str =
    "collection_id, remote, branch, synced_at, username, author_name, author_email, synced_commit";

#[derive(Debug, Clone)]
pub struct TeamLink {
    pub collection_id: String,
    pub remote: String,
    pub branch: String,
    pub synced_at: Option<i64>,
    pub username: Option<String>,
    /// `None` for links made before collections had their own author; they use the profile.
    pub author: Option<Author>,
    /// The remote commit this device last matched; `None` for links older than schema 24.
    pub synced_commit: Option<String>,
}

impl TeamLink {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let name: Option<String> = row.get(5)?;
        let email: Option<String> = row.get(6)?;
        Ok(Self {
            collection_id: row.get(0)?,
            remote: row.get(1)?,
            branch: row.get(2)?,
            synced_at: row.get(3)?,
            username: row.get(4)?,
            author: name.zip(email).map(|(name, email)| Author { name, email }),
            synced_commit: row.get(7)?,
        })
    }
}

pub fn insert_link(conn: &Connection, link: &TeamLink) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO team_links
             (collection_id, remote, branch, synced_at, username, author_name, author_email,
              synced_commit, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            link.collection_id,
            link.remote,
            link.branch,
            link.synced_at,
            link.username,
            link.author.as_ref().map(|author| &author.name),
            link.author.as_ref().map(|author| &author.email),
            link.synced_commit,
            now_ms()
        ],
    )?;
    Ok(())
}

pub fn link(conn: &Connection, collection_id: &str) -> Result<Option<TeamLink>, AppError> {
    Ok(conn
        .query_row(
            &format!("SELECT {LINK_COLUMNS} FROM team_links WHERE collection_id = ?1"),
            params![collection_id],
            TeamLink::from_row,
        )
        .optional()?)
}

pub fn links(conn: &Connection) -> Result<Vec<TeamLink>, AppError> {
    let mut statement = conn.prepare(&format!(
        "SELECT {LINK_COLUMNS} FROM team_links ORDER BY created_at"
    ))?;
    let rows = statement.query_map([], TeamLink::from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn delete_link(conn: &Connection, collection_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM team_links WHERE collection_id = ?1",
        params![collection_id],
    )?;
    Ok(())
}

pub fn mark_synced(
    conn: &Connection,
    collection_id: &str,
    at: i64,
    commit: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE team_links SET synced_at = ?2, synced_commit = ?3 WHERE collection_id = ?1",
        params![collection_id, at, commit],
    )?;
    Ok(())
}

/// Expects a checked account.
pub fn set_account(
    conn: &Connection,
    collection_id: &str,
    account: &Account,
) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE team_links SET username = ?2, author_name = ?3, author_email = ?4
         WHERE collection_id = ?1",
        params![
            collection_id,
            account.username,
            account.author.name,
            account.author.email
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound("Shared collection"));
    }
    Ok(())
}

pub fn author(conn: &Connection) -> Result<Option<Author>, AppError> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![AUTHOR_KEY],
            |row| row.get(0),
        )
        .optional()?;
    // An unreadable profile is asked for again rather than failing the sync.
    Ok(stored.and_then(|text| serde_json::from_str(&text).ok()))
}

pub fn save_author(conn: &Connection, author: &Author) -> Result<(), AppError> {
    let text = serde_json::to_string(author).map_err(|_| AppError::Storage)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![AUTHOR_KEY, text],
    )?;
    Ok(())
}
