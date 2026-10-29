use std::collections::HashMap;

use super::types::NameValue;
use crate::error::AppError;

/// Path variables per request, matching what the UI shows.
pub const MAX_PATH_VARIABLES: usize = 50;
const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// A whole path segment `:name` is a path variable, as in `/users/:id`.
pub fn variable_name(segment: &str) -> Option<&str> {
    let name = segment.strip_prefix(':')?;
    let mut chars = name.chars();
    let first = chars.next()?;
    let valid = (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    valid.then_some(name)
}

/// Replaces `:name` segments with their values, after `{{variables}}` are filled in. Segments
/// with no path variable of that name stay as they are; a path variable left empty is refused,
/// so the request never goes out with `:id` in it by mistake.
pub fn fill(url: &str, values: &[NameValue]) -> Result<String, AppError> {
    if values.is_empty() {
        return Ok(url.to_owned());
    }
    let (start, end) = path_bounds(url);
    let lookup: HashMap<&str, &str> = values
        .iter()
        .map(|pair| (pair.name.as_str(), pair.value.as_str()))
        .collect();
    let mut filled = String::with_capacity(url.len());
    filled.push_str(&url[..start]);
    for (index, segment) in url[start..end].split('/').enumerate() {
        if index > 0 {
            filled.push('/');
        }
        match variable_name(segment).and_then(|name| Some((name, *lookup.get(name)?))) {
            Some((name, "")) => {
                return Err(AppError::InvalidInput(format!(
                    "Fill in the path variable :{name}"
                )));
            }
            Some((_, value)) => encode_segment(value, &mut filled),
            None => filled.push_str(segment),
        }
    }
    filled.push_str(&url[end..]);
    Ok(filled)
}

/// The path part of a request URL (no query or fragment), as written: `/users/:id`.
pub fn path_template(url: &str) -> &str {
    let (start, end) = path_bounds(url);
    &url[start..end]
}

/// The path variable names of a URL, in order and without repeats.
pub fn names(url: &str) -> Vec<&str> {
    let (start, end) = path_bounds(url);
    let mut found = Vec::new();
    for name in url[start..end].split('/').filter_map(variable_name) {
        if !found.contains(&name) {
            found.push(name);
        }
    }
    found
}

/// The path begins after `scheme://host[:port]`, or at the first `/` of a URL without a scheme
/// (such as one starting with `{{baseUrl}}`), and ends at the query or fragment.
fn path_bounds(url: &str) -> (usize, usize) {
    let (host_start, rest) = match url.find("://") {
        Some(at) => (at + 3, &url[at + 3..]),
        None => (0, url),
    };
    let start = rest
        .find(['/', '?', '#'])
        .map_or(url.len(), |at| host_start + at);
    let end = url[start..]
        .find(['?', '#'])
        .map_or(url.len(), |at| start + at);
    (start, end)
}

/// A value stays inside its segment: `/`, `?`, `#`, spaces and other characters a path cannot
/// hold are percent-encoded. `%` is kept, so values that are already encoded work unchanged.
fn encode_segment(value: &str, out: &mut String) {
    for byte in value.bytes() {
        let keep = byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@%".contains(&byte);
        if keep {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0F)]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(name: &str, value: &str) -> NameValue {
        NameValue {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn recognises_whole_segments_only() {
        assert_eq!(variable_name(":id"), Some("id"));
        assert_eq!(variable_name(":order_id-2"), Some("order_id-2"));
        assert_eq!(variable_name(":"), None);
        assert_eq!(variable_name(":1st"), None);
        assert_eq!(variable_name("v1:batchGet"), None);
        assert_eq!(variable_name(":id.json"), None);
    }

    #[test]
    fn fills_path_segments_and_leaves_the_rest() {
        let values = [pair("id", "42"), pair("orderId", "a/b c")];
        assert_eq!(
            fill(
                "http://localhost:3000/users/:id/orders/:orderId?expand=:id#:id",
                &values
            )
            .ok()
            .as_deref(),
            Some("http://localhost:3000/users/42/orders/a%2Fb%20c?expand=:id#:id")
        );
        assert_eq!(
            fill("localhost:3000/users/:id", &values).ok().as_deref(),
            Some("localhost:3000/users/42")
        );
        assert_eq!(
            fill("https://api.test/users/:unknown/x%20y", &values)
                .ok()
                .as_deref(),
            Some("https://api.test/users/:unknown/x%20y")
        );
        assert_eq!(
            fill("https://api.test/u/:id", &[pair("id", "ü%20")])
                .ok()
                .as_deref(),
            Some("https://api.test/u/%C3%BC%20")
        );
    }

    #[test]
    fn lists_names_in_order_once() {
        assert_eq!(
            names("{{baseUrl}}/users/:id/orders/:orderId/:id?x=:q"),
            ["id", "orderId"]
        );
        assert_eq!(names("http://localhost:3000/users"), Vec::<&str>::new());
        assert_eq!(names("localhost:3000/a/:b"), ["b"]);
    }

    #[test]
    fn refuses_an_empty_path_variable() {
        let error = fill("https://api.test/users/:id", &[pair("id", "")]);
        assert!(error.is_err_and(|e| e.to_string() == "Fill in the path variable :id"));
    }
}
