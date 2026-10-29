//! Names from before the app was renamed from Postcard to Codestamp. Data saved by those versions
//! still carries them, so each one is read once and moved to the new name.

use std::fs;
use std::path::Path;

use crate::error::AppError;

/// The old app identifier: the name of its data folders and of its key in the credential store.
pub const IDENTIFIER: &str = "dev.postcard.desktop";
pub const BACKUP_EXTENSION: &str = "postcard";
pub const TEAM_MANIFEST: &str = "postcard.json";
const DATABASE_FILE: &str = "postcard.db";
/// SQLite keeps unsaved pages next to the database, named after it. They move first, so a move
/// cut short is finished on the next start.
const DATABASE_SUFFIXES: [&str; 3] = ["-wal", "-shm", ""];

/// Renames the old data folder next to `dir` to `dir`. A folder that already exists wins, so this
/// runs once.
pub fn adopt_folder(dir: &Path) -> Result<(), AppError> {
    let Some(old) = dir.parent().map(|parent| parent.join(IDENTIFIER)) else {
        return Ok(());
    };
    if dir.exists() || !old.is_dir() {
        return Ok(());
    }
    fs::rename(old, dir).map_err(|_| AppError::FileAccess)
}

/// Gives the old database in `dir` the name `name`, unless a database with that name exists.
pub fn adopt_database(dir: &Path, name: &str) -> Result<(), AppError> {
    if dir.join(name).exists() || !dir.join(DATABASE_FILE).exists() {
        return Ok(());
    }
    for suffix in DATABASE_SUFFIXES {
        let old = dir.join(format!("{DATABASE_FILE}{suffix}"));
        if old.exists() {
            fs::rename(old, dir.join(format!("{name}{suffix}")))
                .map_err(|_| AppError::FileAccess)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::new_id;
    use std::path::PathBuf;

    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn scratch() -> Scratch {
        let dir = std::env::temp_dir().join(format!("codestamp-legacy-{}", new_id()));
        fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(dir)
    }

    #[test]
    fn moves_the_old_folder_and_its_database() {
        let root = scratch();
        let old = root.0.join(IDENTIFIER);
        fs::create_dir_all(old.join("repos")).expect("old folder");
        fs::write(old.join(DATABASE_FILE), "db").expect("db");
        fs::write(old.join(format!("{DATABASE_FILE}-wal")), "wal").expect("wal");
        let new = root.0.join("dev.codestamp.desktop");

        adopt_folder(&new).expect("folder");
        adopt_database(&new, "codestamp.db").expect("database");

        assert!(!old.exists());
        assert!(new.join("repos").is_dir());
        assert_eq!(
            fs::read_to_string(new.join("codestamp.db")).ok(),
            Some("db".into())
        );
        assert_eq!(
            fs::read_to_string(new.join("codestamp.db-wal")).ok(),
            Some("wal".into())
        );
        assert!(!new.join(DATABASE_FILE).exists());
    }

    #[test]
    fn leaves_existing_data_alone() {
        let root = scratch();
        let old = root.0.join(IDENTIFIER);
        let new = root.0.join("dev.codestamp.desktop");
        fs::create_dir_all(&old).expect("old folder");
        fs::create_dir_all(&new).expect("new folder");
        fs::write(new.join("codestamp.db"), "new").expect("new db");
        fs::write(new.join(DATABASE_FILE), "old").expect("old db");

        adopt_folder(&new).expect("folder");
        adopt_database(&new, "codestamp.db").expect("database");

        assert!(old.is_dir());
        assert_eq!(
            fs::read_to_string(new.join("codestamp.db")).ok(),
            Some("new".into())
        );
        assert!(new.join(DATABASE_FILE).exists());
    }

    #[test]
    fn does_nothing_on_a_fresh_install() {
        let root = scratch();
        let new = root.0.join("dev.codestamp.desktop");
        adopt_folder(&new).expect("folder");
        assert!(!new.exists());
    }
}
