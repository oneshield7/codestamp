use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::AppError;

const MAX_DISPLAY_NAME_CHARS: usize = 60;

/// Reads a file the user picked, refusing it when it is larger than `max_bytes` without
/// reading more than that.
pub fn read_limited(path: &Path, max_bytes: u64, too_big: &str) -> Result<Vec<u8>, AppError> {
    let file = File::open(path).map_err(|_| AppError::FileAccess)?;
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::FileAccess)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_bytes {
        return Err(AppError::invalid(too_big));
    }
    Ok(bytes)
}

/// A display name from a file's name, limited to plain characters.
pub fn display_name(path: &Path, fallback: &str) -> String {
    let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
    let cleaned: String = stem
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        .take(MAX_DISPLAY_NAME_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_keep_plain_characters_only() {
        let name = |text: &str| display_name(Path::new(text), "Fallback");
        assert_eq!(name("C:/fonts/FiraCode-Regular.ttf"), "FiraCode-Regular");
        assert_eq!(name(" <b>Bold<b> .ttf"), "bBoldb");
        assert_eq!(name("C:/certs/.."), "Fallback");
        assert_eq!(name("<>.pem"), "Fallback");
    }

    #[test]
    fn refuses_files_over_the_limit() {
        let path = std::env::temp_dir().join(format!("codestamp-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&path, "0123456789").expect("write");
        assert_eq!(
            read_limited(&path, 10, "too big").ok().map(|b| b.len()),
            Some(10)
        );
        assert!(matches!(
            read_limited(&path, 9, "too big"),
            Err(AppError::InvalidInput(message)) if message == "too big"
        ));
        std::fs::remove_file(&path).expect("remove");
    }
}
