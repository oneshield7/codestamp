//! Docs descriptions: Markdown kept with a collection, a section or a request.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Long enough for a full page of documentation per item.
pub const MAX_DESCRIPTION_CHARS: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Described {
    Collection,
    Section,
    Request,
}

impl Described {
    /// The table holding the item; fixed names, never built from input.
    const fn table(self) -> &'static str {
        match self {
            Self::Collection => "collections",
            Self::Section => "sections",
            Self::Request => "requests",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Collection => "Collection",
            Self::Section => "Section",
            Self::Request => "Request",
        }
    }
}

/// Trailing blank lines are dropped; line breaks, tabs and Markdown stay as written.
pub fn clean_description(raw: &str) -> Result<String, AppError> {
    let text = raw.trim_end();
    if text.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(AppError::invalid(
            "A description can be up to 20,000 characters",
        ));
    }
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(AppError::invalid(
            "The description has characters that are not allowed",
        ));
    }
    Ok(text.to_owned())
}

pub fn get(conn: &Connection, item: Described, id: &str) -> Result<String, AppError> {
    let sql = format!("SELECT description FROM {} WHERE id = ?1", item.table());
    conn.query_row(&sql, params![id], |row| row.get(0))
        .optional()?
        .ok_or(AppError::NotFound(item.label()))
}

pub fn set(conn: &Connection, item: Described, id: &str, text: &str) -> Result<(), AppError> {
    let sql = format!("UPDATE {} SET description = ?2 WHERE id = ?1", item.table());
    if conn.execute(&sql, params![id, text])? == 0 {
        return Err(AppError::NotFound(item.label()));
    }
    Ok(())
}

/// Gives a copy (a duplicate, say) the description of its original.
pub fn copy(conn: &Connection, item: Described, from: &str, to: &str) -> Result<(), AppError> {
    let text = get(conn, item, from)?;
    if text.is_empty() {
        return Ok(());
    }
    set(conn, item, to, &text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use crate::storage::repo;

    #[test]
    fn keeps_markdown_and_refuses_what_does_not_belong() {
        assert_eq!(
            clean_description("# Users\n\n| a | b |\n\tcode\n\n").expect("valid"),
            "# Users\n\n| a | b |\n\tcode"
        );
        assert!(clean_description("bell\u{7}").is_err());
        assert!(clean_description(&"x".repeat(MAX_DESCRIPTION_CHARS + 1)).is_err());
    }

    #[test]
    fn sets_gets_and_copies() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let first = repo::insert_collection(conn, "First")?;
            let second = repo::insert_collection(conn, "Second")?;
            set(conn, Described::Collection, &first.id, "Users and accounts")?;
            copy(conn, Described::Collection, &first.id, &second.id)?;
            assert_eq!(
                get(conn, Described::Collection, &second.id)?,
                "Users and accounts"
            );
            assert!(matches!(
                set(conn, Described::Section, &first.id, "x"),
                Err(AppError::NotFound("Section"))
            ));
            Ok(())
        })
        .expect("descriptions");
    }
}
