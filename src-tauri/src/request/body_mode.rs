//! Telling a raw body's kind apart, for requests that come from history, cURL or other tools.

/// The body mode for raw text: JSON, XML and HTML get their own highlighting and formatting.
/// `media` is the declared Content-Type (or a tool's language, mapped by the caller); without
/// one, the text itself decides.
pub fn raw_body_mode(media: Option<&str>, text: &str) -> &'static str {
    let media = media
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let start = text.trim_start();
    if media.contains("json") || start.starts_with('{') || start.starts_with('[') {
        return "json";
    }
    if media == "text/html" || media == "application/xhtml+xml" {
        return "html";
    }
    if media.ends_with("/xml") || media.ends_with("+xml") {
        return "xml";
    }
    if media.is_empty() && start.starts_with('<') {
        let head = start.get(..14).unwrap_or(start).to_ascii_lowercase();
        return if head.starts_with("<!doctype html") || head.starts_with("<html") {
            "html"
        } else {
            "xml"
        };
    }
    "text"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_declared_type_then_the_text() {
        assert_eq!(
            raw_body_mode(Some("application/json; charset=utf-8"), ""),
            "json"
        );
        assert_eq!(raw_body_mode(Some("application/problem+json"), "x"), "json");
        assert_eq!(
            raw_body_mode(Some("text/xml; charset=utf-8"), "<a/>"),
            "xml"
        );
        assert_eq!(raw_body_mode(Some("application/soap+xml"), "<a/>"), "xml");
        assert_eq!(raw_body_mode(Some("text/html"), "<p>"), "html");
        assert_eq!(raw_body_mode(Some("text/plain"), "<a/>"), "text");
        assert_eq!(raw_body_mode(Some("text/plain"), " {\"a\":1}"), "json");
        assert_eq!(raw_body_mode(None, "<?xml version=\"1.0\"?><a/>"), "xml");
        assert_eq!(raw_body_mode(None, "<!DOCTYPE html><html></html>"), "html");
        assert_eq!(raw_body_mode(None, "hello"), "text");
    }
}
