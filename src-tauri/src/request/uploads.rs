use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::rules::{MAX_PATH_LENGTH, MAX_UPLOAD_BYTES};
use super::types::RequestBody;
use crate::error::AppError;
use crate::storage::db::now_ms;

/// Files picked longest ago are forgotten first; a request that still uses one asks again.
const MAX_GRANTS: i64 = 500;

/// A file the user picked in the system dialog. Only such paths may be read when a request is
/// sent, so an imported or edited request cannot make the app upload any other file.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedFile {
    pub path: String,
    pub name: String,
    pub size: u64,
}

pub fn grant(conn: &Connection, path: &Path) -> Result<PickedFile, AppError> {
    let text = path
        .to_str()
        .ok_or_else(|| AppError::invalid("This file name cannot be used"))?;
    if text.len() > MAX_PATH_LENGTH {
        return Err(AppError::invalid("The file path is too long"));
    }
    let metadata = path.metadata().map_err(|_| AppError::FileAccess)?;
    if !metadata.is_file() {
        return Err(AppError::invalid("Choose a file, not a folder"));
    }
    if metadata.len() > MAX_UPLOAD_BYTES {
        return Err(AppError::invalid("Uploads can be up to 100 MB in total"));
    }
    conn.execute(
        "INSERT INTO file_grants (path, granted_at) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET granted_at = excluded.granted_at",
        params![text, now_ms()],
    )?;
    conn.execute(
        "DELETE FROM file_grants WHERE path NOT IN
           (SELECT path FROM file_grants ORDER BY granted_at DESC LIMIT ?1)",
        params![MAX_GRANTS],
    )?;
    Ok(PickedFile {
        path: text.to_owned(),
        name: file_label(text),
        size: metadata.len(),
    })
}

/// Refuses a body that would read a file the user did not pick on this device.
pub fn check_granted(conn: &Connection, body: &RequestBody) -> Result<(), AppError> {
    // An empty path is left to validation, which asks the user to choose a file.
    for path in body
        .file_paths()
        .into_iter()
        .filter(|path| !path.is_empty())
    {
        if !is_granted(conn, path)? {
            return Err(AppError::InvalidInput(format!(
                "Choose “{}” again: only files picked on this device can be uploaded",
                file_label(path)
            )));
        }
    }
    Ok(())
}

/// Whether the user picked this file on this device.
pub fn is_granted(conn: &Connection, path: &str) -> Result<bool, AppError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM file_grants WHERE path = ?1",
            params![path],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// The file name alone, for labels and messages, so full paths stay out of the UI text.
pub fn file_label(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::types::FormPart;
    use crate::storage::db::Database;

    #[test]
    fn only_picked_files_may_be_uploaded() {
        let db = Database::in_memory().expect("database");
        let path =
            std::env::temp_dir().join(format!("codestamp-{}.txt", crate::storage::db::new_id()));
        std::fs::write(&path, "hello").expect("write file");
        let text = path.display().to_string();
        let body = |value: &str| RequestBody::FormData {
            parts: vec![FormPart {
                name: "file".to_owned(),
                value: value.to_owned(),
                file: true,
            }],
        };

        db.with(|conn| {
            assert!(check_granted(conn, &body(&text)).is_err());
            let picked = grant(conn, &path)?;
            assert_eq!(picked.size, 5);
            assert!(check_granted(conn, &body(&text)).is_ok());
            assert!(check_granted(conn, &body("C:\\Users\\someone\\.ssh\\id_rsa")).is_err());
            Ok(())
        })
        .expect("grants");
        std::fs::remove_file(&path).expect("remove file");
    }

    #[test]
    fn labels_show_only_the_file_name() {
        assert_eq!(file_label("C:\\Users\\rina\\photo.png"), "photo.png");
        assert_eq!(file_label("/home/rina/report.pdf"), "report.pdf");
        assert_eq!(file_label("plain"), "plain");
    }
}
