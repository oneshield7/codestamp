use std::fs;
use std::path::Path;

use rusqlite::{Connection, MAIN_DB, OpenFlags, params};

use crate::error::AppError;
use crate::legacy;
use crate::storage::db::{Database, SCHEMA_VERSION, migrate, new_id};

pub const BACKUP_EXTENSION: &str = "codestamp";
/// Backups made before the rename keep their old extension and restore like any other.
pub const RESTORE_EXTENSIONS: [&str; 2] = [BACKUP_EXTENSION, legacy::BACKUP_EXTENSION];
const REQUIRED_TABLES: [&str; 4] = ["collections", "requests", "environments", "settings"];

/// Writes a consistent copy of every collection, environment, setting and history entry.
/// Secret values stay encrypted with this device's key.
pub fn backup(db: &Database, data_dir: &Path, target: &Path) -> Result<(), AppError> {
    // VACUUM INTO refuses an existing file, and the user may have chosen to replace one.
    let staging = data_dir.join(format!("backup-{}.tmp", new_id()));
    db.with(|conn| {
        conn.execute(
            "VACUUM INTO ?1",
            params![staging.to_string_lossy().as_ref()],
        )?;
        Ok(())
    })?;
    let copied = fs::copy(&staging, target).map_err(|_| AppError::FileAccess);
    let removed = fs::remove_file(&staging).map_err(|_| AppError::FileAccess);
    copied.and(removed)
}

/// Replaces everything with the backup at `source`, after checking it is a Codestamp backup this
/// version can read. Older backups are upgraded like an older database.
pub fn restore(db: &Database, source: &Path) -> Result<(), AppError> {
    check_backup(source)?;
    db.with_mut(|conn| {
        conn.restore(MAIN_DB, source, None::<fn(rusqlite::backup::Progress)>)?;
        migrate(conn)
    })
}

fn check_backup(source: &Path) -> Result<(), AppError> {
    let not_a_backup = || AppError::invalid("This file is not a Codestamp backup");
    let conn = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| not_a_backup())?;
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| not_a_backup())?;
    if version > SCHEMA_VERSION {
        return Err(AppError::invalid(
            "This backup was made by a newer Codestamp. Update Codestamp first",
        ));
    }
    let tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (?1, ?2, ?3, ?4)",
            params![
                REQUIRED_TABLES[0],
                REQUIRED_TABLES[1],
                REQUIRED_TABLES[2],
                REQUIRED_TABLES[3]
            ],
            |row| row.get(0),
        )
        .map_err(|_| not_a_backup())?;
    if version < 1 || tables != 4 {
        return Err(not_a_backup());
    }
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|_| not_a_backup())?;
    if check != "ok" {
        return Err(AppError::invalid("This backup is damaged"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("codestamp-{}-{name}", new_id()))
    }

    #[test]
    fn backs_up_and_restores_everything() {
        let dir = scratch("dir");
        fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("data.codestamp");
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            conn.execute(
                "INSERT INTO collections (id, name, sort_order, created_at, updated_at)
                 VALUES ('c', 'Users API', 1, 0, 0)",
                [],
            )?;
            Ok(())
        })
        .expect("collection");
        backup(&db, &dir, &file).expect("backup");
        backup(&db, &dir, &file).expect("a second backup replaces the file");

        let other = Database::in_memory().expect("database");
        restore(&other, &file).expect("restore");
        let names: Vec<String> = other
            .with(|conn| {
                let mut statement = conn.prepare("SELECT name FROM collections")?;
                Ok(statement
                    .query_map([], |row| row.get(0))?
                    .collect::<Result<_, _>>()?)
            })
            .expect("names");
        assert_eq!(names, ["Users API"]);
        assert_eq!(
            fs::read_dir(&dir).expect("dir").count(),
            1,
            "no staging file is left"
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn refuses_files_that_are_not_backups() {
        let file = scratch("not-a-backup.codestamp");
        fs::write(&file, "hello").expect("write");
        let db = Database::in_memory().expect("database");
        assert!(matches!(
            restore(&db, &file),
            Err(AppError::InvalidInput(message)) if message.contains("not a Codestamp backup")
        ));
        fs::remove_file(&file).expect("cleanup");

        let empty = scratch("empty.codestamp");
        Connection::open(&empty)
            .and_then(|conn| conn.execute_batch("CREATE TABLE notes (text TEXT)"))
            .expect("other database");
        assert!(restore(&db, &empty).is_err());
        fs::remove_file(&empty).expect("cleanup");
    }
}
