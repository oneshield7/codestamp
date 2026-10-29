use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::error::AppError;
use crate::storage::db::new_id;

/// Bodies up to this size are kept whole so they can be saved to a file; the UI only ever
/// receives the 2 MB preview.
pub const MAX_KEPT_BODY: usize = 32 * 1024 * 1024;
/// Recent bodies stay until together they pass this; the oldest leave first.
const KEPT_BUDGET: usize = 64 * 1024 * 1024;

struct KeptBody {
    id: String,
    content_type: Option<String>,
    bytes: Arc<Vec<u8>>,
}

/// The latest response bodies, held in the Rust core for "Save response body".
#[derive(Default)]
pub struct ResponseBodies {
    kept: Mutex<VecDeque<KeptBody>>,
}

impl ResponseBodies {
    /// Keeps a whole body and returns the id the UI uses to save it.
    pub fn keep(&self, bytes: Vec<u8>, content_type: Option<String>) -> String {
        let id = new_id();
        let mut kept = self.lock();
        let mut total = bytes.len() + kept.iter().map(|body| body.bytes.len()).sum::<usize>();
        while total > KEPT_BUDGET {
            let Some(oldest) = kept.pop_front() else {
                break;
            };
            total -= oldest.bytes.len();
        }
        kept.push_back(KeptBody {
            id: id.clone(),
            content_type,
            bytes: Arc::new(bytes),
        });
        id
    }

    pub fn get(&self, id: &str) -> Result<(Arc<Vec<u8>>, Option<String>), AppError> {
        self.lock()
            .iter()
            .find(|body| body.id == id)
            .map(|body| (Arc::clone(&body.bytes), body.content_type.clone()))
            .ok_or_else(|| {
                AppError::invalid(
                    "This response is no longer kept. Send the request again to save it",
                )
            })
    }

    fn lock(&self) -> MutexGuard<'_, VecDeque<KeptBody>> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The file type a body is saved as, picked from the response's Content-Type. The server
/// chooses that header, so only these extensions can ever be suggested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFormat {
    Json,
    Xml,
    Html,
    Csv,
    Text,
    Svg,
    Png,
    Jpeg,
    Gif,
    Webp,
    Pdf,
    Zip,
    Binary,
}

impl BodyFormat {
    pub fn from_content_type(content_type: Option<&str>) -> Self {
        let essence = content_type
            .and_then(|value| value.split(';').next())
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        match essence.as_str() {
            "application/json" => Self::Json,
            "image/svg+xml" => Self::Svg,
            "application/xml" | "text/xml" => Self::Xml,
            "text/html" | "application/xhtml+xml" => Self::Html,
            "text/csv" => Self::Csv,
            "image/png" => Self::Png,
            "image/jpeg" => Self::Jpeg,
            "image/gif" => Self::Gif,
            "image/webp" => Self::Webp,
            "application/pdf" => Self::Pdf,
            "application/zip" => Self::Zip,
            other if other.ends_with("+json") => Self::Json,
            other if other.ends_with("+xml") => Self::Xml,
            other if other.starts_with("text/") => Self::Text,
            _ => Self::Binary,
        }
    }

    /// One extension per format, so the save dialog only offers that one.
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Json => &["json"],
            Self::Xml => &["xml"],
            Self::Html => &["html"],
            Self::Csv => &["csv"],
            Self::Text => &["txt"],
            Self::Svg => &["svg"],
            Self::Png => &["png"],
            Self::Jpeg => &["jpg"],
            Self::Gif => &["gif"],
            Self::Webp => &["webp"],
            Self::Pdf => &["pdf"],
            Self::Zip => &["zip"],
            Self::Binary => &["bin"],
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON file",
            Self::Xml => "XML file",
            Self::Html => "HTML file",
            Self::Csv => "CSV file",
            Self::Text => "Text file",
            Self::Svg => "SVG image",
            Self::Png => "PNG image",
            Self::Jpeg => "JPEG image",
            Self::Gif => "GIF image",
            Self::Webp => "WebP image",
            Self::Pdf => "PDF document",
            Self::Zip => "ZIP archive",
            Self::Binary => "Binary file",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_format_from_the_content_type() {
        let format = |value: &str| BodyFormat::from_content_type(Some(value));
        assert_eq!(format("application/json; charset=utf-8"), BodyFormat::Json);
        assert_eq!(format("application/problem+json"), BodyFormat::Json);
        assert_eq!(format("image/svg+xml"), BodyFormat::Svg);
        assert_eq!(format("application/atom+xml"), BodyFormat::Xml);
        assert_eq!(format("TEXT/HTML"), BodyFormat::Html);
        assert_eq!(format("text/markdown"), BodyFormat::Text);
        assert_eq!(format("application/x-msdownload"), BodyFormat::Binary);
        assert_eq!(BodyFormat::from_content_type(None), BodyFormat::Binary);
    }

    #[test]
    fn drops_the_oldest_bodies_past_the_budget() {
        let bodies = ResponseBodies::default();
        let first = bodies.keep(vec![0; MAX_KEPT_BODY], None);
        let second = bodies.keep(vec![1; MAX_KEPT_BODY], Some("text/plain".to_owned()));
        let third = bodies.keep(vec![2; 10], None);
        assert!(bodies.get(&first).is_err());
        assert_eq!(bodies.get(&second).map(|(bytes, _)| bytes[0]).ok(), Some(1));
        assert_eq!(
            bodies.get(&third).map(|(bytes, _)| bytes.len()).ok(),
            Some(10)
        );
    }
}
