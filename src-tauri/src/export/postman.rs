use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::auth::models::{ApiKeyLocation, Auth, ClientAuth, OAuth2Settings, OAuthGrant};
use crate::environment::dynamic::iso_timestamp;
use crate::environment::models::{EnvironmentSummary, VariableView};
use crate::error::AppError;
use crate::import::url_parts::split_url;
use crate::request::saved::soap_defaults;
use crate::request::uploads::file_label;
use crate::storage::db::{new_id, now_ms};
use crate::storage::examples::Example;
use crate::storage::models::{
    CollectionDetail, FormField, FormFieldKind, KeyValue, RequestContent, RequestKind, SavedRequest,
};

const COLLECTION_SCHEMA: &str =
    "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";
const EXPORTED_USING: &str = concat!("Codestamp/", env!("CARGO_PKG_VERSION"));

/// The file content plus what the user should know about it (things left out or emptied).
pub struct Exported {
    pub json: String,
    pub notes: Vec<String>,
}

#[derive(Default)]
struct Counts {
    websockets: usize,
    mcp: usize,
    grpc: usize,
    secrets: usize,
    files: usize,
}

impl Counts {
    fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if self.websockets > 0 {
            notes.push(format!(
                "{} WebSocket request(s) were left out: Collection v2.1 files cannot hold them.",
                self.websockets
            ));
        }
        if self.mcp > 0 {
            notes.push(format!(
                "{} MCP request(s) were left out: Collection v2.1 files cannot hold them.",
                self.mcp
            ));
        }
        if self.grpc > 0 {
            notes.push(format!(
                "{} gRPC request(s) were left out: Collection v2.1 files cannot hold them.",
                self.grpc
            ));
        }
        if self.secrets > 0 {
            notes.push(format!(
                "{} secret value(s) were left empty. Fill them in after importing.",
                self.secrets
            ));
        }
        if self.files > 0 {
            notes.push(
                "Upload files are saved by name only; choose them again after importing."
                    .to_owned(),
            );
        }
        notes
    }
}

/// Docs descriptions to write where Postman keeps them: the collection's, and by section or
/// request id.
#[derive(Debug, Default)]
pub struct Docs {
    pub collection: String,
    pub by_id: HashMap<String, String>,
}

impl Docs {
    fn of(&self, id: &str) -> &str {
        self.by_id.get(id).map_or("", String::as_str)
    }
}

/// A Postman Collection v2.1 file. Requests outside sections come first, then one folder per
/// section, the same order as the notebook.
pub fn collection(
    detail: &CollectionDetail,
    requests: &[SavedRequest],
    examples: &HashMap<String, Vec<Example>>,
    variables: &[VariableView],
    docs: &Docs,
) -> Result<Exported, AppError> {
    let mut counts = Counts::default();
    let mut item_of = |request: &SavedRequest| {
        let kept = examples.get(&request.id).map_or(&[][..], Vec::as_slice);
        item(request, kept, docs.of(&request.id), &mut counts)
    };
    let mut items: Vec<Value> = requests
        .iter()
        .filter(|request| request.section_id.is_none())
        .filter_map(&mut item_of)
        .collect();
    for section in &detail.sections {
        let children: Vec<Value> = requests
            .iter()
            .filter(|request| request.section_id.as_deref() == Some(section.id.as_str()))
            .filter_map(&mut item_of)
            .collect();
        let mut folder = json!({ "name": section.name, "item": children });
        if !docs.of(&section.id).is_empty() {
            folder["description"] = json!(docs.of(&section.id));
        }
        items.push(folder);
    }

    let mut root = Map::new();
    root.insert(
        "info".to_owned(),
        json!({
            "_postman_id": new_id(),
            "name": detail.name,
            "description": if docs.collection.is_empty() {
                format!("Exported from {EXPORTED_USING}")
            } else {
                docs.collection.clone()
            },
            "schema": COLLECTION_SCHEMA,
        }),
    );
    root.insert("item".to_owned(), Value::Array(items));
    if let Some(auth) = auth_json(&detail.auth).filter(|_| detail.auth != Auth::None) {
        root.insert("auth".to_owned(), auth);
    }
    if !variables.is_empty() {
        let entries: Vec<Value> = variables
            .iter()
            .map(|variable| {
                counts.secrets += usize::from(variable.is_secret);
                let mut entry = json!({
                    "key": variable.key,
                    "value": variable.value,
                    "type": if variable.is_secret { "secret" } else { "string" },
                });
                if !variable.enabled {
                    entry["disabled"] = json!(true);
                }
                entry
            })
            .collect();
        root.insert("variable".to_owned(), Value::Array(entries));
    }
    Ok(Exported {
        json: serde_json::to_string_pretty(&Value::Object(root)).map_err(|_| AppError::Storage)?,
        notes: counts.notes(),
    })
}

/// A Postman environment file, or a globals file for Globals.
pub fn environment(
    summary: &EnvironmentSummary,
    variables: &[VariableView],
) -> Result<Exported, AppError> {
    let mut counts = Counts::default();
    let values: Vec<Value> = variables
        .iter()
        .map(|variable| {
            counts.secrets += usize::from(variable.is_secret);
            json!({
                "key": variable.key,
                "value": variable.value,
                "type": if variable.is_secret { "secret" } else { "default" },
                "enabled": variable.enabled,
            })
        })
        .collect();
    let file = json!({
        "id": new_id(),
        "name": summary.name,
        "values": values,
        "_postman_variable_scope": if summary.is_global { "globals" } else { "environment" },
        "_postman_exported_at": iso_timestamp(now_ms()),
        "_postman_exported_using": EXPORTED_USING,
    });
    Ok(Exported {
        json: serde_json::to_string_pretty(&file).map_err(|_| AppError::Storage)?,
        notes: counts.notes(),
    })
}

fn item(
    request: &SavedRequest,
    examples: &[Example],
    description: &str,
    counts: &mut Counts,
) -> Option<Value> {
    let content = &request.content;
    match content.kind {
        RequestKind::Websocket => {
            counts.websockets += 1;
            return None;
        }
        RequestKind::Mcp => {
            counts.mcp += 1;
            return None;
        }
        RequestKind::Grpc => {
            counts.grpc += 1;
            return None;
        }
        RequestKind::Http | RequestKind::Graphql | RequestKind::Soap => {}
    }
    let mut fields = Map::new();
    // Collection v2.1 has no SOAP requests: one is written as the POST it sends, headers included.
    let mut headers = content.headers.clone();
    if content.kind == RequestKind::Soap {
        fields.insert("method".to_owned(), json!("POST"));
        headers.extend(
            soap_defaults(content.soap.as_ref(), &content.headers)
                .into_iter()
                .map(|header| KeyValue {
                    enabled: true,
                    key: header.name,
                    value: header.value,
                }),
        );
    } else {
        fields.insert("method".to_owned(), json!(content.method));
    }
    fields.insert("header".to_owned(), rows(&headers));
    let mut url = url_object(&content.url, &content.params);
    if !content.path_variables.is_empty() {
        url["variable"] = Value::Array(
            content
                .path_variables
                .iter()
                .map(|variable| json!({ "key": variable.key, "value": variable.value }))
                .collect(),
        );
    }
    fields.insert("url".to_owned(), url);
    if let Some(body) = body_json(content, counts) {
        fields.insert("body".to_owned(), body);
    }
    if let Some(auth) = auth_json(&content.auth) {
        fields.insert("auth".to_owned(), auth);
    }
    if !description.is_empty() {
        fields.insert("description".to_owned(), json!(description));
    }
    let mut item = json!({ "name": content.name, "request": Value::Object(fields) });
    let events = events(&content.pre_request_script, &content.test_script);
    if !events.is_empty() {
        item["event"] = Value::Array(events);
    }
    if !examples.is_empty() {
        item["response"] = Value::Array(examples.iter().map(example_json).collect());
    }
    Some(item)
}

/// A saved example as a Postman response, which Postman also uses for its mock servers.
fn example_json(example: &Example) -> Value {
    let json = example.headers.iter().any(|header| {
        header.name.eq_ignore_ascii_case("content-type") && header.value.contains("json")
    });
    json!({
        "name": example.name,
        "code": example.status,
        "status": ::http::StatusCode::from_u16(example.status)
            .ok()
            .and_then(|status| status.canonical_reason())
            .unwrap_or_default(),
        "_postman_previewlanguage": if json { "json" } else { "text" },
        "header": example
            .headers
            .iter()
            .map(|header| json!({ "key": header.name, "value": header.value }))
            .collect::<Vec<_>>(),
        "body": example.body,
    })
}

fn rows(rows: &[KeyValue]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                let mut entry = json!({ "key": row.key, "value": row.value });
                if !row.enabled {
                    entry["disabled"] = json!(true);
                }
                entry
            })
            .collect(),
    )
}

/// Postman reads both `raw` and the parts, so they are given consistently. The query holds the
/// typed query (if any) and every parameter row, disabled ones included.
fn url_object(url: &str, params: &[KeyValue]) -> Value {
    let (base, mut query) = split_url(url);
    query.extend(params.iter().cloned());
    let enabled: Vec<String> = query
        .iter()
        .filter(|param| param.enabled)
        .map(|param| format!("{}={}", param.key, param.value))
        .collect();
    let raw = if enabled.is_empty() {
        base.clone()
    } else {
        format!("{base}?{}", enabled.join("&"))
    };
    let mut object = Map::new();
    object.insert("raw".to_owned(), json!(raw));
    let (protocol, rest) = match base.split_once("://") {
        Some((protocol, rest)) => (Some(protocol), rest),
        None => (None, base.as_str()),
    };
    if let Some(protocol) = protocol {
        object.insert("protocol".to_owned(), json!(protocol));
    }
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port) = split_port(authority);
    if !host.is_empty() {
        let parts: Vec<&str> = if host.contains("{{") {
            vec![host]
        } else {
            host.split('.').collect()
        };
        object.insert("host".to_owned(), json!(parts));
    }
    if let Some(port) = port {
        object.insert("port".to_owned(), json!(port));
    }
    if !path.is_empty() {
        object.insert(
            "path".to_owned(),
            json!(path.split('/').collect::<Vec<_>>()),
        );
    }
    if !query.is_empty() {
        object.insert("query".to_owned(), rows(&query));
    }
    Value::Object(object)
}

fn split_port(authority: &str) -> (&str, Option<&str>) {
    match authority.rsplit_once(':') {
        Some((host, port))
            if !port.is_empty()
                && (port.chars().all(|c| c.is_ascii_digit()) || port.starts_with("{{")) =>
        {
            (host, Some(port))
        }
        _ => (authority, None),
    }
}

/// File paths are cut down to the file name: a full path would reveal the user's folders and
/// would not exist on another device anyway.
fn body_json(content: &RequestContent, counts: &mut Counts) -> Option<Value> {
    if content.kind == RequestKind::Graphql {
        return Some(json!({
            "mode": "graphql",
            "graphql": { "query": content.body, "variables": content.graphql_variables },
        }));
    }
    match content.body_mode.as_str() {
        "json" | "text" | "xml" | "html" => Some(json!({
            "mode": "raw",
            "raw": content.body,
            "options": { "raw": { "language": content.body_mode } },
        })),
        "urlencoded" => {
            let fields: Vec<Value> = content
                .form
                .iter()
                .filter(|field| field.kind == FormFieldKind::Text)
                .map(form_text)
                .collect();
            Some(json!({ "mode": "urlencoded", "urlencoded": fields }))
        }
        "formdata" => {
            let fields: Vec<Value> = content
                .form
                .iter()
                .map(|field| match field.kind {
                    FormFieldKind::Text => form_text(field),
                    FormFieldKind::File => {
                        let mut entry = json!({ "key": field.key, "type": "file" });
                        if !field.value.is_empty() {
                            counts.files += 1;
                            entry["src"] = json!(file_label(&field.value));
                        }
                        if !field.enabled {
                            entry["disabled"] = json!(true);
                        }
                        entry
                    }
                })
                .collect();
            Some(json!({ "mode": "formdata", "formdata": fields }))
        }
        "binary" => {
            let mut file = Map::new();
            if !content.body_file.is_empty() {
                counts.files += 1;
                file.insert("src".to_owned(), json!(file_label(&content.body_file)));
            }
            Some(json!({ "mode": "file", "file": Value::Object(file) }))
        }
        _ => None,
    }
}

fn form_text(field: &FormField) -> Value {
    let mut entry = json!({ "key": field.key, "value": field.value, "type": "text" });
    if !field.enabled {
        entry["disabled"] = json!(true);
    }
    entry
}

/// `Inherit` is Postman's default (no `auth` key). Postman's bearer type always writes
/// "Bearer", so a different prefix is exported as an API key header with the full value.
fn auth_json(auth: &Auth) -> Option<Value> {
    match auth {
        Auth::Inherit => None,
        Auth::None => Some(json!({ "type": "noauth" })),
        Auth::Bearer { token, prefix } if prefix.trim().eq_ignore_ascii_case("bearer") => {
            Some(json!({ "type": "bearer", "bearer": [param("token", token)] }))
        }
        Auth::Bearer { token, prefix } => {
            let value = match prefix.trim() {
                "" => token.clone(),
                prefix => format!("{prefix} {token}"),
            };
            Some(api_key("Authorization", &value, "header"))
        }
        Auth::Basic { username, password } => Some(json!({
            "type": "basic",
            "basic": [param("username", username), param("password", password)],
        })),
        Auth::ApiKey {
            key,
            value,
            location,
        } => {
            let place = match location {
                ApiKeyLocation::Header => "header",
                ApiKeyLocation::Query => "query",
            };
            Some(api_key(key, value, place))
        }
        Auth::Digest { username, password } => Some(json!({
            "type": "digest",
            "digest": [param("username", username), param("password", password)],
        })),
        Auth::AwsSigV4 {
            access_key,
            secret_key,
            session_token,
            region,
            service,
        } => Some(json!({
            "type": "awsv4",
            "awsv4": [
                param("accessKey", access_key),
                param("secretKey", secret_key),
                param("sessionToken", session_token),
                param("region", region),
                param("service", service),
            ],
        })),
        Auth::OAuth2(settings) => Some(oauth2_json(settings)),
    }
}

/// Postman's names for the grant types; Authorization Code with PKCE is a grant of its own.
fn oauth2_json(settings: &OAuth2Settings) -> Value {
    let grant = match (settings.grant, settings.use_pkce) {
        (OAuthGrant::AuthorizationCode, true) => "authorization_code_with_pkce",
        (OAuthGrant::AuthorizationCode, false) => "authorization_code",
        (OAuthGrant::ClientCredentials, _) => "client_credentials",
        (OAuthGrant::Password, _) => "password_credentials",
    };
    let client_authentication = match settings.client_auth {
        ClientAuth::Header => "header",
        ClientAuth::Body => "body",
    };
    json!({
        "type": "oauth2",
        "oauth2": [
            param("grant_type", grant),
            param("authUrl", &settings.auth_url),
            param("accessTokenUrl", &settings.token_url),
            param("clientId", &settings.client_id),
            param("clientSecret", &settings.client_secret),
            param("scope", &settings.scope),
            param("redirect_uri", &settings.redirect_uri),
            param("username", &settings.username),
            param("password", &settings.password),
            param("client_authentication", client_authentication),
            param("headerPrefix", &settings.header_prefix),
            param("addTokenTo", "header"),
        ],
    })
}

fn api_key(key: &str, value: &str, location: &str) -> Value {
    json!({
        "type": "apikey",
        "apikey": [param("key", key), param("value", value), param("in", location)],
    })
}

fn param(key: &str, value: &str) -> Value {
    json!({ "key": key, "value": value, "type": "string" })
}

fn events(pre_request: &str, test: &str) -> Vec<Value> {
    [("prerequest", pre_request), ("test", test)]
        .into_iter()
        .filter(|(_, script)| !script.trim().is_empty())
        .map(|(listen, script)| {
            json!({
                "listen": listen,
                "script": { "type": "text/javascript", "exec": script.lines().collect::<Vec<_>>() },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::file::plan;
    use crate::storage::models::SectionInfo;

    fn row(key: &str, value: &str, enabled: bool) -> KeyValue {
        KeyValue {
            enabled,
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    fn saved(name: &str, section: Option<&str>, content: RequestContent) -> SavedRequest {
        SavedRequest {
            id: new_id(),
            collection_id: "c".to_owned(),
            section_id: section.map(str::to_owned),
            content: RequestContent {
                name: name.to_owned(),
                ..content
            },
        }
    }

    fn base_content() -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: String::new(),
            method: "GET".to_owned(),
            url: "{{baseUrl}}/users".to_owned(),
            params: vec![row("page", "1", true), row("debug", "1", false)],
            headers: vec![row("Accept", "application/json", true)],
            body_mode: "none".to_owned(),
            body: String::new(),
            form: Vec::new(),
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: "pm.test('ok', () => {});\npm.expect(1).to.equal(1);".to_owned(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    #[test]
    fn exports_a_collection_that_imports_back() {
        let detail = CollectionDetail {
            id: "c".to_owned(),
            name: "Users API".to_owned(),
            auth: Auth::Bearer {
                token: "{{token}}".to_owned(),
                prefix: "Bearer".to_owned(),
            },
            icon: "👤".to_owned(),
            icon_custom: false,
            sections: vec![SectionInfo {
                id: "s".to_owned(),
                name: "Uploads".to_owned(),
            }],
            requests: Vec::new(),
        };
        let upload = RequestContent {
            method: "POST".to_owned(),
            body_mode: "formdata".to_owned(),
            form: vec![
                FormField {
                    enabled: true,
                    key: "title".to_owned(),
                    value: "Q3".to_owned(),
                    kind: FormFieldKind::Text,
                },
                FormField {
                    enabled: true,
                    key: "doc".to_owned(),
                    value: "C:\\Users\\rina\\q3.pdf".to_owned(),
                    kind: FormFieldKind::File,
                },
            ],
            auth: Auth::None,
            ..base_content()
        };
        let socket = RequestContent {
            kind: RequestKind::Websocket,
            ..base_content()
        };
        let rpc = RequestContent {
            kind: RequestKind::Grpc,
            ..base_content()
        };
        let requests = [
            saved("List users", None, base_content()),
            saved("Upload", Some("s"), upload),
            saved("Live", None, socket),
            saved("Orders", None, rpc),
        ];
        let variables = [VariableView {
            id: "v".to_owned(),
            key: "apiKey".to_owned(),
            value: String::new(),
            is_secret: true,
            enabled: true,
            restricted: None,
            locked: false,
        }];

        let docs = Docs {
            collection: "# Users API\n\nAll about users.".to_owned(),
            by_id: HashMap::from([
                ("s".to_owned(), "Files people upload".to_owned()),
                (requests[0].id.clone(), "Lists every user.".to_owned()),
            ]),
        };
        let exported =
            collection(&detail, &requests, &HashMap::new(), &variables, &docs).expect("export");
        assert!(
            !exported.json.contains("C:\\\\Users"),
            "no local folders in the file"
        );
        assert_eq!(exported.notes.len(), 4);

        let value: Value = serde_json::from_str(&exported.json).expect("json");
        assert_eq!(value["info"]["schema"], COLLECTION_SCHEMA);
        assert_eq!(value["auth"]["type"], "bearer");
        let first = &value["item"][0]["request"]["url"];
        assert_eq!(first["raw"], "{{baseUrl}}/users?page=1");
        assert_eq!(first["host"][0], "{{baseUrl}}");
        assert_eq!(first["query"][1]["disabled"], true);

        let imported = plan(&exported.json).expect("re-import");
        assert_eq!(imported.section_names(), ["Uploads"]);
        assert_eq!(imported.requests.len(), 2);
        let list = imported
            .requests
            .iter()
            .find(|r| r.content.name == "List users")
            .expect("list");
        assert_eq!(list.content.url, "{{baseUrl}}/users");
        assert_eq!(list.content.params.len(), 2);
        assert!(list.content.test_script.contains("pm.expect(1)"));
        let upload = imported
            .requests
            .iter()
            .find(|r| r.content.name == "Upload")
            .expect("upload");
        assert_eq!(upload.content.body_mode, "formdata");
        assert_eq!(upload.section, Some(0));
        assert_eq!(imported.variables.first().map(|v| v.is_secret), Some(true));
        assert_eq!(imported.description, "# Users API\n\nAll about users.");
        assert_eq!(imported.sections[0].description, "Files people upload");
        assert_eq!(list.description, "Lists every user.");
        assert_eq!(upload.description, "");
    }

    #[test]
    fn maps_auth_types_to_postman() {
        let custom = auth_json(&Auth::Bearer {
            token: "abc".to_owned(),
            prefix: "Token".to_owned(),
        })
        .expect("auth");
        assert_eq!(custom["type"], "apikey");
        assert_eq!(custom["apikey"][1]["value"], "Token abc");
        let query = auth_json(&Auth::ApiKey {
            key: "api_key".to_owned(),
            value: "{{key}}".to_owned(),
            location: ApiKeyLocation::Query,
        })
        .expect("auth");
        assert_eq!(query["apikey"][2]["value"], "query");
        assert!(auth_json(&Auth::Inherit).is_none());
        assert_eq!(
            auth_json(&Auth::None).map(|a| a["type"].clone()),
            Some(json!("noauth"))
        );
    }

    #[test]
    fn splits_urls_like_postman() {
        let url = url_object("https://api.example.com:8443/v1/users", &[]);
        assert_eq!(url["protocol"], "https");
        assert_eq!(url["host"], json!(["api", "example", "com"]));
        assert_eq!(url["port"], "8443");
        assert_eq!(url["path"], json!(["v1", "users"]));
    }

    #[test]
    fn exports_environments_without_secret_values() {
        let summary = EnvironmentSummary {
            id: "e".to_owned(),
            name: "Staging".to_owned(),
            is_global: false,
            collection_id: None,
            variable_count: 2,
        };
        let variables = [
            VariableView {
                id: "1".to_owned(),
                key: "baseUrl".to_owned(),
                value: "https://staging.test".to_owned(),
                is_secret: false,
                enabled: true,
                restricted: None,
                locked: false,
            },
            VariableView {
                id: "2".to_owned(),
                key: "token".to_owned(),
                value: String::new(),
                is_secret: true,
                enabled: false,
                restricted: None,
                locked: false,
            },
        ];
        let exported = environment(&summary, &variables).expect("export");
        let value: Value = serde_json::from_str(&exported.json).expect("json");
        assert_eq!(value["_postman_variable_scope"], "environment");
        assert_eq!(value["values"][1]["type"], "secret");
        assert_eq!(value["values"][1]["value"], "");
        assert_eq!(exported.notes.len(), 1);
        let imported = plan(&exported.json).expect("re-import");
        assert_eq!(imported.variables.len(), 2);
    }
}
