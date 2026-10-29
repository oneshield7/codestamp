//! How fields and values of the repository's files are shown in conflict and history views.

use serde_json::Value;

const FIELD_LABELS: [(&str, &str); 21] = [
    ("name", "Name"),
    ("description", "Docs description"),
    ("kind", "Kind"),
    ("method", "Method"),
    ("url", "URL"),
    ("sectionId", "Section"),
    ("params", "Query parameters"),
    ("pathVariables", "Path variables"),
    ("headers", "Headers"),
    ("bodyMode", "Body type"),
    ("body", "Body"),
    ("graphqlVariables", "GraphQL variables"),
    ("form", "Form fields"),
    ("preRequestScript", "Pre-request script"),
    ("testScript", "Test script"),
    ("auth", "Auth"),
    ("mcpOperation", "MCP tool"),
    ("grpc", "gRPC method"),
    ("soap", "SOAP operation"),
    ("examples", "Examples"),
    ("id", "Id"),
];
const MAX_DISPLAY_CHARS: usize = 600;

/// Request fields in the order they are shown.
pub fn request_fields() -> impl Iterator<Item = (&'static str, &'static str)> {
    FIELD_LABELS.iter().copied().filter(|(key, _)| *key != "id")
}

pub fn label(key: &str) -> &str {
    FIELD_LABELS
        .iter()
        .find(|(name, _)| *name == key)
        .map_or(key, |(_, label)| label)
}

/// A short, readable form of a value: rows as `key: value` lines, text as it is.
pub fn display(value: Option<&Value>) -> String {
    let text = match value {
        None | Some(Value::Null) => "(none)".to_owned(),
        Some(Value::String(text)) if text.is_empty() => "(empty)".to_owned(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(rows)) if rows.is_empty() => "(none)".to_owned(),
        Some(Value::Array(rows)) if rows.iter().all(|row| row.get("key").is_some()) => {
            rows.iter().map(row_text).collect::<Vec<_>>().join("\n")
        }
        Some(row @ Value::Object(fields)) if fields.contains_key("key") => row_text(row),
        Some(other) => other.to_string(),
    };
    if text.chars().count() > MAX_DISPLAY_CHARS {
        let cut: String = text.chars().take(MAX_DISPLAY_CHARS).collect();
        format!("{cut}…")
    } else {
        text
    }
}

/// One `key: value` row. Uploads and restricted values show what they are, never their content.
fn row_text(row: &Value) -> String {
    let off = row.get("enabled").and_then(Value::as_bool) == Some(false);
    let key = row.get("key").and_then(Value::as_str).unwrap_or_default();
    let value = match row.get("kind").and_then(Value::as_str) {
        Some("file") => "(file)",
        _ if row.get("restricted").is_some() => "(restricted)",
        _ => row.get("value").and_then(Value::as_str).unwrap_or_default(),
    };
    format!("{}{key}: {value}", if off { "(off) " } else { "" })
}

/// The members of an access group file, by name and email.
pub fn members_of(group: Option<&Value>) -> String {
    let members: Vec<String> = group
        .and_then(|group| group.get("members"))
        .and_then(Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(|member| {
                    let name = member.get("name")?.as_str()?;
                    let email = member.get("email")?.as_str()?;
                    Some(format!("{name} <{email}>"))
                })
                .collect()
        })
        .unwrap_or_default();
    if members.is_empty() {
        "(none)".to_owned()
    } else {
        members.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn shows_rows_files_and_long_text_readably() {
        let rows = json!([
            { "enabled": true, "key": "page", "value": "1" },
            { "enabled": false, "key": "debug", "value": "x" },
            { "enabled": true, "key": "doc", "value": "", "kind": "file" }
        ]);
        assert_eq!(display(Some(&rows)), "page: 1\n(off) debug: x\ndoc: (file)");
        assert_eq!(display(None), "(none)");
        assert_eq!(display(Some(&json!(""))), "(empty)");
        assert!(display(Some(&json!("x".repeat(700)))).ends_with('…'));
        assert_eq!(label("url"), "URL");
        let restricted = json!({ "key": "token", "value": "", "restricted": "g", "sealed": {} });
        assert_eq!(display(Some(&restricted)), "token: (restricted)");
        let group = json!({ "members": [{ "name": "Ana", "email": "ana@x.test" }] });
        assert_eq!(members_of(Some(&group)), "Ana <ana@x.test>");
    }
}
