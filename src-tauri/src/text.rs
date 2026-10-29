/// Cuts `text` to at most `max` bytes without splitting a character; true when it was cut.
pub fn clip(text: &str, max: usize) -> (&str, bool) {
    if text.len() <= max {
        return (text, false);
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], true)
}

/// Text for XML attributes; characters XML 1.0 cannot hold are dropped.
pub fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\n' => escaped.push_str("&#10;"),
            '\t' | '\r' => escaped.push(' '),
            control if control.is_control() => {}
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_without_splitting_characters() {
        assert_eq!(clip("hello", 10), ("hello", false));
        assert_eq!(clip("h\u{e9}llo", 2), ("h", true));
        assert_eq!(clip("h\u{e9}llo", 3), ("h\u{e9}", true));
    }
}
