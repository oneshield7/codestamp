use std::ffi::OsStr;
use std::path::PathBuf;

use crate::error::AppError;
use crate::request::bodies::BodyFormat;

const MAX_STEM_CHARS: usize = 80;

/// Kinds of files the app writes to a place the user picks.
#[derive(Debug, Clone, Copy)]
pub enum ExportKind {
    Png,
    Json,
    Collection,
    /// A collection for another Codestamp.
    CodestampCollection,
    Environment,
    Globals,
    ResponseBody(BodyFormat),
    Docs,
}

impl ExportKind {
    const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Json
            | Self::Collection
            | Self::CodestampCollection
            | Self::Environment
            | Self::Globals => "json",
            Self::ResponseBody(format) => format.extensions()[0],
            Self::Docs => "md",
        }
    }

    /// Names the kind inside the file name, e.g. `users-api.collection.json`.
    const fn suffix(self) -> &'static str {
        match self {
            Self::Png | Self::Json | Self::Globals | Self::ResponseBody(_) => "",
            Self::Collection => ".collection",
            Self::CodestampCollection => ".codestamp",
            Self::Environment => ".environment",
            Self::Docs => ".docs",
        }
    }

    pub const fn fallback_stem(self) -> &'static str {
        match self {
            Self::Png => "codestamp-capture",
            Self::Json => "codestamp-report",
            Self::Collection | Self::CodestampCollection => "collection",
            Self::Environment => "environment",
            Self::Globals => "globals",
            Self::ResponseBody(_) => "response",
            Self::Docs => "collection",
        }
    }

    pub const fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Png => ("PNG image", &["png"]),
            Self::Json => ("JSON report", &["json"]),
            Self::Collection => ("Collection", &["json"]),
            Self::CodestampCollection => ("Codestamp collection", &["json"]),
            Self::Environment | Self::Globals => ("Environment", &["json"]),
            Self::ResponseBody(format) => (format.label(), format.extensions()),
            Self::Docs => ("Markdown", &["md"]),
        }
    }
}

/// The suggested file name, limited to plain ASCII characters.
pub fn file_name(stem: Option<&str>, kind: ExportKind) -> String {
    let cleaned: String = stem
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(MAX_STEM_CHARS)
        .collect();
    let stem = if cleaned.is_empty() {
        kind.fallback_stem()
    } else {
        cleaned.as_str()
    };
    format!("{stem}{}.{}", kind.suffix(), kind.extension())
}

/// A name turned into a lowercase file stem: "Users API (v2)" becomes `users-api-v2`.
pub fn slug(name: &str) -> String {
    let mut stem = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            stem.push(c.to_ascii_lowercase());
        } else if !stem.is_empty() && !stem.ends_with('-') {
            stem.push('-');
        }
    }
    stem.trim_end_matches('-').to_owned()
}

/// Adds the extension when the name has none; any other extension is refused so that saving
/// never silently replaces a different kind of file.
pub fn checked_path(path: PathBuf, kind: ExportKind) -> Result<PathBuf, AppError> {
    let wanted = kind.extension();
    match path.extension().and_then(OsStr::to_str) {
        Some(extension) if extension.eq_ignore_ascii_case(wanted) => Ok(path),
        Some(_) => Err(AppError::InvalidInput(format!(
            "Save the file with a .{wanted} extension"
        ))),
        None => {
            let with_extension = path.with_extension(wanted);
            if with_extension.exists() {
                return Err(AppError::invalid("A file with that name already exists"));
            }
            Ok(with_extension)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_plain_file_names() {
        assert_eq!(
            file_name(Some("codestamp-list-users"), ExportKind::Png),
            "codestamp-list-users.png"
        );
        assert_eq!(
            file_name(Some("..\\..\\Windows\\evil"), ExportKind::Json),
            "Windowsevil.json"
        );
        assert_eq!(
            file_name(Some("ÄÖ"), ExportKind::Png),
            "codestamp-capture.png"
        );
        assert_eq!(file_name(None, ExportKind::Json), "codestamp-report.json");
        assert_eq!(
            file_name(Some(&slug("Users API (v2)")), ExportKind::Collection),
            "users-api-v2.collection.json"
        );
        assert_eq!(
            file_name(Some(&slug("Ärger")), ExportKind::Environment),
            "rger.environment.json"
        );
        assert_eq!(file_name(Some(""), ExportKind::Globals), "globals.json");
    }

    #[test]
    fn keeps_or_adds_the_extension() {
        let folder = std::env::temp_dir();
        assert_eq!(
            checked_path(folder.join("shot.PNG"), ExportKind::Png).ok(),
            Some(folder.join("shot.PNG"))
        );
        assert!(checked_path(folder.join("shot.exe"), ExportKind::Png).is_err());
        assert!(checked_path(folder.join("report.png"), ExportKind::Json).is_err());
        let fresh = folder.join(format!("codestamp-{}", crate::storage::db::new_id()));
        assert_eq!(
            checked_path(fresh.clone(), ExportKind::Json).ok(),
            Some(fresh.with_extension("json"))
        );
    }
}
