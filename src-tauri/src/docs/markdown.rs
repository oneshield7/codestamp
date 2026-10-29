//! Docs as one Markdown file. Credentials typed into a request are masked, secrets are never
//! read, and local file paths are cut down to file names.

use super::models::{CollectionDocs, RequestDocs};
use crate::auth::models::{ApiKeyLocation, Auth, OAuthGrant};
use crate::sensitive::{is_sensitive_name, is_written_out};
use crate::storage::models::{FormFieldKind, KeyValue, RequestKind};

const MASK: &str = "••••••••";

pub fn render(docs: &CollectionDocs) -> String {
    let mut out = format!("# {}\n\n", docs.name.trim());
    paragraph(&mut out, &docs.description);
    if !docs.variables.is_empty() {
        out.push_str("## Variables\n\n| Name | Value |\n| --- | --- |\n");
        for variable in &docs.variables {
            // A token kept in a plain variable is masked like one typed into a header.
            let value = if variable.secret {
                "Secret, set on each device".to_owned()
            } else {
                code(&shown_value(&variable.key, &variable.value))
            };
            out.push_str(&format!("| {} | {} |\n", code(&variable.key), cell(&value)));
        }
        out.push('\n');
    }
    if docs.auth != Auth::None {
        out.push_str(&format!("## Auth\n\n{}\n\n", auth_summary(&docs.auth)));
    }
    let loose: Vec<&RequestDocs> = docs
        .requests
        .iter()
        .filter(|request| request.request.section_id.is_none())
        .collect();
    if !loose.is_empty() {
        out.push_str("## Requests\n\n");
        for request in loose {
            render_request(&mut out, request);
        }
    }
    for section in &docs.sections {
        out.push_str(&format!("## {}\n\n", section.name.trim()));
        paragraph(&mut out, &section.description);
        for request in docs
            .requests
            .iter()
            .filter(|request| request.request.section_id.as_deref() == Some(section.id.as_str()))
        {
            render_request(&mut out, request);
        }
    }
    let trimmed = out.trim_end().len();
    out.truncate(trimmed);
    out.push('\n');
    out
}

fn render_request(out: &mut String, docs: &RequestDocs) {
    let content = &docs.request.content;
    let stamp = content.kind.stamp(&content.method);
    out.push_str(&format!("### {stamp} {}\n\n", content.name.trim()));
    if !content.url.trim().is_empty() {
        out.push_str(&format!(
            "{}\n\n",
            code(&format!("{stamp} {}", content.url.trim()))
        ));
    }
    paragraph(out, &docs.description);
    if content.kind != RequestKind::Mcp {
        out.push_str(&format!("**Auth:** {}\n\n", auth_summary(&content.auth)));
    }
    let path_variables: Vec<(String, String)> = content
        .path_variables
        .iter()
        .map(|variable| (variable.key.clone(), variable.value.clone()))
        .collect();
    table(out, "Path variables", &path_variables);
    table(out, "Query parameters", &enabled(&content.params));
    let headers_title = if content.kind == RequestKind::Grpc {
        "Metadata"
    } else {
        "Headers"
    };
    table(out, headers_title, &enabled(&content.headers));
    body(out, docs);
    for example in &docs.examples {
        out.push_str(&format!(
            "#### Example: {} ({})\n\n",
            example.name.trim(),
            example.status
        ));
        if !example.body.is_empty() {
            let language = example
                .content_type
                .as_deref()
                .map_or("", language_of_content_type);
            fenced(out, language, &example.body);
            if example.cut {
                out.push_str("_The example is longer; the rest is left out here._\n\n");
            }
        }
    }
}

fn body(out: &mut String, docs: &RequestDocs) {
    let content = &docs.request.content;
    match content.body_mode.as_str() {
        "urlencoded" | "formdata" => {
            let rows: Vec<(String, String)> = content
                .form
                .iter()
                .filter(|field| field.enabled)
                .map(|field| {
                    let value = match field.kind {
                        FormFieldKind::File => format!("File: {}", file_name(&field.value)),
                        FormFieldKind::Text => shown_value(&field.key, &field.value),
                    };
                    (field.key.clone(), value)
                })
                .collect();
            table(out, "Body", &rows);
        }
        "binary" => {
            out.push_str(&format!(
                "**Body:** file {}\n\n",
                code(&file_name(&content.body_file))
            ));
        }
        mode if !content.body.trim().is_empty() && mode != "none" => {
            let language = match content.kind {
                RequestKind::Graphql => "graphql",
                RequestKind::Soap => "xml",
                RequestKind::Grpc | RequestKind::Mcp => "json",
                RequestKind::Http | RequestKind::Websocket => match mode {
                    "json" | "xml" | "html" => mode,
                    _ => "",
                },
            };
            out.push_str("**Body**\n\n");
            fenced(out, language, &content.body);
            if content.kind == RequestKind::Graphql && !content.graphql_variables.trim().is_empty()
            {
                out.push_str("**Variables**\n\n");
                fenced(out, "json", &content.graphql_variables);
            }
        }
        _ => {}
    }
}

fn enabled(rows: &[KeyValue]) -> Vec<(String, String)> {
    rows.iter()
        .filter(|row| row.enabled && !row.key.trim().is_empty())
        .map(|row| (row.key.clone(), shown_value(&row.key, &row.value)))
        .collect()
}

/// A credential typed in as it is stays out of the file; a `{{variable}}` is shown as written.
fn shown_value(name: &str, value: &str) -> String {
    if is_sensitive_name(name) && is_written_out(value) {
        MASK.to_owned()
    } else {
        value.to_owned()
    }
}

fn table(out: &mut String, title: &str, rows: &[(String, String)]) {
    if rows.is_empty() {
        return;
    }
    out.push_str(&format!("**{title}**\n\n| Name | Value |\n| --- | --- |\n"));
    for (name, value) in rows {
        out.push_str(&format!("| {} | {} |\n", code(name), cell(&code(value))));
    }
    out.push('\n');
}

fn paragraph(out: &mut String, text: &str) {
    let text = text.trim();
    if !text.is_empty() {
        out.push_str(&format!("{text}\n\n"));
    }
}

/// Inline code that survives backticks inside the value.
fn code(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let ticks = "`".repeat(longest_run(text, '`') + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{ticks}{pad}{text}{pad}{ticks}")
}

/// A table cell stays on one line, and its pipes do not end the cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace(['\r', '\n'], " ")
}

/// A code block fenced with more backticks than the text holds in a row.
fn fenced(out: &mut String, language: &str, text: &str) {
    let fence = "`".repeat((longest_run(text, '`') + 1).max(3));
    out.push_str(&format!(
        "{fence}{language}\n{}\n{fence}\n\n",
        text.trim_end()
    ));
}

fn longest_run(text: &str, wanted: char) -> usize {
    let (mut longest, mut current) = (0, 0);
    for c in text.chars() {
        if c == wanted {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

fn language_of_content_type(content_type: &str) -> &'static str {
    let lower = content_type.to_ascii_lowercase();
    if lower.contains("json") {
        "json"
    } else if lower.contains("xml") {
        "xml"
    } else if lower.contains("html") {
        "html"
    } else {
        ""
    }
}

/// Only the file's name: where it sits on this device is nobody else's business.
fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// What the auth is, never its values.
fn auth_summary(auth: &Auth) -> String {
    match auth {
        Auth::Inherit => "Uses the collection's auth".to_owned(),
        Auth::None => "None".to_owned(),
        Auth::Bearer { .. } => "Bearer token".to_owned(),
        Auth::Basic { .. } => "Basic auth".to_owned(),
        Auth::Digest { .. } => "Digest auth".to_owned(),
        Auth::ApiKey { key, location, .. } => {
            let place = match location {
                ApiKeyLocation::Header => "header",
                ApiKeyLocation::Query => "query parameter",
            };
            format!("API key in the {place} {}", code(key))
        }
        Auth::AwsSigV4 {
            region, service, ..
        } => format!("AWS Signature for {} in {}", code(service), code(region)),
        Auth::OAuth2(settings) => {
            let grant = match settings.grant {
                OAuthGrant::AuthorizationCode => "authorization code",
                OAuthGrant::ClientCredentials => "client credentials",
                OAuthGrant::Password => "password",
            };
            format!("OAuth 2.0, {grant} grant")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::models::{DocExample, DocVariable, SectionDocs};
    use crate::storage::models::{RequestContent, SavedRequest};

    fn request(id: &str, section: Option<&str>, name: &str) -> RequestDocs {
        RequestDocs {
            request: SavedRequest {
                id: id.to_owned(),
                collection_id: "c".to_owned(),
                section_id: section.map(str::to_owned),
                content: RequestContent {
                    kind: RequestKind::Http,
                    name: name.to_owned(),
                    method: "POST".to_owned(),
                    url: "{{baseUrl}}/users".to_owned(),
                    params: vec![KeyValue {
                        enabled: true,
                        key: "api_key".to_owned(),
                        value: "plain-123".to_owned(),
                    }],
                    headers: vec![KeyValue {
                        enabled: true,
                        key: "Authorization".to_owned(),
                        value: "Bearer {{token}}".to_owned(),
                    }],
                    body_mode: "json".to_owned(),
                    body: "{\"code\": \"```\"}".to_owned(),
                    form: Vec::new(),
                    body_file: String::new(),
                    pre_request_script: String::new(),
                    test_script: String::new(),
                    auth: Auth::Inherit,
                    mcp_operation: None,
                    path_variables: Vec::new(),
                    graphql_variables: String::new(),
                    grpc: None,
                    soap: None,
                },
            },
            description: "Creates a user.".to_owned(),
            examples: vec![DocExample {
                name: "Created".to_owned(),
                status: 201,
                content_type: Some("application/json".to_owned()),
                body: "{\"id\": 1}".to_owned(),
                cut: false,
            }],
        }
    }

    #[test]
    fn writes_sections_requests_and_examples_without_secrets() {
        let docs = CollectionDocs {
            id: "c".to_owned(),
            name: "Users API".to_owned(),
            icon: "👤".to_owned(),
            description: "Everything about users.".to_owned(),
            auth: Auth::Bearer {
                token: "never-shown".to_owned(),
                prefix: "Bearer".to_owned(),
            },
            variables: vec![
                DocVariable {
                    key: "baseUrl".to_owned(),
                    value: "https://api.test".to_owned(),
                    secret: false,
                },
                DocVariable {
                    key: "token".to_owned(),
                    value: String::new(),
                    secret: true,
                },
                DocVariable {
                    key: "botToken".to_owned(),
                    value: "12345:plain-bot-token".to_owned(),
                    secret: false,
                },
            ],
            sections: vec![SectionDocs {
                id: "s".to_owned(),
                name: "Admin".to_owned(),
                description: "Needs an admin token.".to_owned(),
            }],
            requests: vec![
                request("a", None, "Create user"),
                request("b", Some("s"), "Ban user"),
            ],
        };
        let markdown = render(&docs);
        assert!(markdown.starts_with("# Users API\n\nEverything about users.\n"));
        assert!(markdown.contains("| `baseUrl` | `https://api.test` |"));
        assert!(markdown.contains("| `token` | Secret, set on each device |"));
        assert!(markdown.contains("| `botToken` | `••••••••` |"));
        assert!(!markdown.contains("plain-bot-token"));
        assert!(markdown.contains("## Auth\n\nBearer token\n"));
        assert!(!markdown.contains("never-shown"));
        assert!(
            markdown.contains("## Requests\n\n### POST Create user\n\n`POST {{baseUrl}}/users`")
        );
        assert!(markdown.contains("| `api_key` | `••••••••` |"));
        assert!(markdown.contains("| `Authorization` | `Bearer {{token}}` |"));
        assert!(markdown.contains("````json\n{\"code\": \"```\"}\n````"));
        assert!(markdown.contains("#### Example: Created (201)\n\n```json\n{\"id\": 1}\n```"));
        assert!(markdown.contains("## Admin\n\nNeeds an admin token.\n\n### POST Ban user"));
        assert!(markdown.ends_with("```\n"));
    }

    #[test]
    fn inline_code_and_cells_stay_intact() {
        assert_eq!(code("a`b"), "``a`b``");
        assert_eq!(code("`x"), "`` `x ``");
        assert_eq!(cell("a|b\nc"), "a\\|b c");
        assert_eq!(file_name("C:\\Users\\me\\photo.png"), "photo.png");
    }
}
