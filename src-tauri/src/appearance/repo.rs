use rusqlite::{Connection, OptionalExtension, params};

use super::models::{Appearance, ImportedFont};
use crate::error::AppError;
use crate::storage::db::now_ms;

const APPEARANCE_KEY: &str = "appearance";

/// `None` when nothing was saved yet or the stored value cannot be read anymore.
pub fn load_appearance(conn: &Connection) -> Result<Option<Appearance>, AppError> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![APPEARANCE_KEY],
            |row| row.get(0),
        )
        .optional()?;
    Ok(stored.and_then(|json| serde_json::from_str(&json).ok()))
}

pub fn save_appearance(conn: &Connection, appearance: &Appearance) -> Result<(), AppError> {
    let json = serde_json::to_string(appearance).map_err(|_| AppError::Storage)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![APPEARANCE_KEY, json],
    )?;
    Ok(())
}

pub fn list_fonts(conn: &Connection) -> Result<Vec<ImportedFont>, AppError> {
    let mut statement =
        conn.prepare("SELECT id, name, format, size FROM fonts ORDER BY created_at, name")?;
    let fonts = statement
        .query_map([], |row| {
            Ok(ImportedFont {
                id: row.get(0)?,
                name: row.get(1)?,
                format: row.get(2)?,
                size: row.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(fonts)
}

pub fn count_fonts(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row("SELECT COUNT(*) FROM fonts", [], |row| row.get(0))?)
}

pub fn insert_font(conn: &Connection, font: &ImportedFont) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO fonts (id, name, format, size, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![font.id, font.name, font.format, font.size, now_ms()],
    )?;
    Ok(())
}

pub fn font_format(conn: &Connection, id: &str) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row(
            "SELECT format FROM fonts WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn delete_font(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM fonts WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    fn font(id: &str) -> ImportedFont {
        ImportedFont {
            id: id.to_owned(),
            name: "Fira Code".to_owned(),
            format: "woff2".to_owned(),
            size: 1024,
        }
    }

    #[test]
    fn saves_and_loads_appearance() {
        let db = Database::in_memory().expect("database");
        let saved = Appearance {
            code_font_size: 15,
            ligatures: false,
            ..Appearance::default()
        };
        let loaded = db.with(|conn| {
            assert_eq!(load_appearance(conn)?, None);
            save_appearance(conn, &saved)?;
            load_appearance(conn)
        });
        assert_eq!(loaded.ok().flatten(), Some(saved));
    }

    #[test]
    fn unreadable_appearance_counts_as_unsaved() {
        let db = Database::in_memory().expect("database");
        let loaded = db.with(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, '{\"uiFont\": 3}')",
                params![APPEARANCE_KEY],
            )?;
            load_appearance(conn)
        });
        assert!(loaded.is_ok_and(|appearance| appearance.is_none()));
    }

    #[test]
    fn keeps_an_index_of_imported_fonts() {
        let db = Database::in_memory().expect("database");
        let id = "8f2c1a7e-0000-4000-8000-000000000001";
        let result = db.with(|conn| {
            insert_font(conn, &font(id))?;
            let listed = list_fonts(conn)?;
            let format = font_format(conn, id)?;
            delete_font(conn, id)?;
            Ok((listed, format, count_fonts(conn)?))
        });
        assert!(
            result.is_ok_and(|(listed, format, count)| listed == vec![font(id)]
                && format.as_deref() == Some("woff2")
                && count == 0)
        );
    }
}
