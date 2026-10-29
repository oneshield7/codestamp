use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::models::{AppearanceState, DEFAULT_FONT, ImportedFont};
use super::repo;
use super::validate::font_format;
use crate::error::AppError;
use crate::files::{display_name, read_limited};
use crate::storage::db::{Database, new_id};
use crate::storage::validate::check_id;

pub const FONT_EXTENSIONS: [&str; 3] = ["ttf", "otf", "woff2"];
const MAX_FONTS: i64 = 20;
const MAX_FONT_BYTES: u64 = 10 * 1024 * 1024;
const FALLBACK_FONT_NAME: &str = "Imported font";

/// Imported font files, kept in the app data folder as `<id>.<format>` and indexed in SQLite.
pub struct FontStore {
    dir: PathBuf,
}

impl FontStore {
    pub const fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn import(&self, db: &Database, source: &Path) -> Result<ImportedFont, AppError> {
        if db.with(repo::count_fonts)? >= MAX_FONTS {
            return Err(AppError::invalid("Up to 20 fonts can be imported"));
        }
        let bytes = read_limited(source, MAX_FONT_BYTES, "Font files can be up to 10 MB")?;
        let format = font_format(&bytes)
            .ok_or_else(|| AppError::invalid("Choose a .ttf, .otf or .woff2 font file"))?;
        let font = ImportedFont {
            id: new_id(),
            name: display_name(source, FALLBACK_FONT_NAME),
            format: format.to_owned(),
            size: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
        };
        fs::create_dir_all(&self.dir).map_err(|_| AppError::FileAccess)?;
        let path = self.path(&font.id, format);
        fs::write(&path, &bytes).map_err(|_| AppError::FileAccess)?;
        if let Err(error) = db.with(|conn| repo::insert_font(conn, &font)) {
            // The database error is what the user needs to see; a leftover file is harmless.
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(font)
    }

    pub fn read(&self, db: &Database, id: &str) -> Result<Vec<u8>, AppError> {
        let format = stored_format(db, id)?;
        fs::read(self.path(id, &format)).map_err(|_| AppError::FileAccess)
    }

    /// Deletes the font; settings that used it go back to the bundled font.
    pub fn remove(&self, db: &Database, id: &str) -> Result<AppearanceState, AppError> {
        let format = stored_format(db, id)?;
        if let Err(error) = fs::remove_file(self.path(id, &format))
            && error.kind() != ErrorKind::NotFound
        {
            return Err(AppError::FileAccess);
        }
        db.with(|conn| {
            let tx = conn.unchecked_transaction()?;
            repo::delete_font(&tx, id)?;
            let mut appearance = repo::load_appearance(&tx)?.unwrap_or_default();
            if appearance.ui_font == id {
                DEFAULT_FONT.clone_into(&mut appearance.ui_font);
            }
            if appearance.code_font == id {
                DEFAULT_FONT.clone_into(&mut appearance.code_font);
            }
            repo::save_appearance(&tx, &appearance)?;
            let fonts = repo::list_fonts(&tx)?;
            tx.commit()?;
            Ok(AppearanceState { appearance, fonts })
        })
    }

    // Both parts come from validated data (a UUID and a sniffed format), so this stays inside `dir`.
    fn path(&self, id: &str, format: &str) -> PathBuf {
        self.dir.join(format!("{id}.{format}"))
    }
}

fn stored_format(db: &Database, id: &str) -> Result<String, AppError> {
    check_id(id)?;
    db.with(|conn| repo::font_format(conn, id))?
        .ok_or(AppError::NotFound("The font"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::models::Appearance;

    const TTF_HEADER: [u8; 8] = [0, 1, 0, 0, 0, 10, 0, 128];

    struct Scratch {
        dir: PathBuf,
    }

    impl Scratch {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("codestamp-fonts-{}", new_id()));
            fs::create_dir_all(&dir).expect("scratch folder");
            Self { dir }
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.dir.join(name);
            fs::write(&path, bytes).expect("scratch file");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn imports_reads_and_removes_a_font() {
        let scratch = Scratch::new();
        let store = FontStore::new(scratch.dir.join("fonts"));
        let db = Database::in_memory().expect("database");
        let source = scratch.file("Fira Code.ttf", &TTF_HEADER);

        let font = store.import(&db, &source).expect("import");
        assert_eq!(
            (font.name.as_str(), font.format.as_str()),
            ("Fira Code", "ttf")
        );
        assert_eq!(store.read(&db, &font.id).ok(), Some(TTF_HEADER.to_vec()));

        let using = Appearance {
            code_font: font.id.clone(),
            ..Appearance::default()
        };
        db.with(|conn| repo::save_appearance(conn, &using))
            .expect("save");
        let state = store.remove(&db, &font.id).expect("remove");
        assert_eq!(state.appearance, Appearance::default());
        assert!(state.fonts.is_empty());
        assert!(!store.path(&font.id, "ttf").exists());
    }

    #[test]
    fn rejects_files_that_are_not_fonts_or_too_big() {
        let scratch = Scratch::new();
        let store = FontStore::new(scratch.dir.join("fonts"));
        let db = Database::in_memory().expect("database");

        let script = scratch.file("evil.ttf", b"<script>alert(1)</script>");
        assert!(store.import(&db, &script).is_err());

        let mut huge = TTF_HEADER.to_vec();
        huge.resize(usize::try_from(MAX_FONT_BYTES).expect("size") + 1, 0);
        let big = scratch.file("big.ttf", &huge);
        assert!(store.import(&db, &big).is_err());
        assert!(
            db.with(repo::list_fonts)
                .is_ok_and(|fonts| fonts.is_empty())
        );
    }

    #[test]
    fn refuses_ids_that_are_not_uuids() {
        let scratch = Scratch::new();
        let store = FontStore::new(scratch.dir.join("fonts"));
        let db = Database::in_memory().expect("database");
        assert!(store.read(&db, "../codestamp.db").is_err());
    }
}
