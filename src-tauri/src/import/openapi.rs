//! OpenAPI 3.x and Swagger 2.0 specs turned into a collection: one request per operation,
//! sections by tag, `{{baseUrl}}` from the servers, `{id}` as the path variable `:id`, example
//! bodies from the schemas, and auth from the security schemes.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::models::{ImportKind, ImportPlan, PlannedRequest, PlannedSection};
use super::postman::{MAX_REQUESTS, MAX_WARNINGS, clean_name, imported_description};
use crate::auth::models::{ApiKeyLocation, Auth, ClientAuth, OAuth2Settings, OAuthGrant};
use crate::environment::models::VariableInput;
use crate::error::AppError;
use crate::request::body_mode::raw_body_mode;
use crate::request::path_vars::variable_name;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::request::types::NameValue;
use crate::storage::examples::{ExampleInput, check_example};
use crate::storage::models::{
    FormField, FormFieldKind, KeyValue, PathVariable, RequestContent, RequestKind,
};
use crate::storage::validate::check_request;

const METHODS: [&str; 7] = ["get", "post", "put", "patch", "delete", "head", "options"];
const MAX_REF_HOPS: usize = 32;
const MAX_EXAMPLE_DEPTH: usize = 8;
/// Schema nodes an example body may expand, so a huge or self-referencing schema stays small.
const EXAMPLE_BUDGET: usize = 500;
/// Headers OpenAPI describes elsewhere (content types and security), so parameters skip them.
const DESCRIBED_ELSEWHERE: [&str; 3] = ["accept", "content-type", "authorization"];
/// Examples kept per operation, one per documented status code.
const MAX_EXAMPLES_FROM_SPEC: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Version {
    Swagger2,
    OpenApi3,
}

pub fn is_spec(value: &Value) -> bool {
    value.get("openapi").is_some() || value.get("swagger").is_some()
}

pub fn plan(spec: &Value) -> Result<ImportPlan, AppError> {
    let version = match (spec.get("swagger"), spec.get("openapi")) {
        (Some(Value::String(v)), _) if v.starts_with('2') => Version::Swagger2,
        (_, Some(Value::String(v))) if v.starts_with('3') => Version::OpenApi3,
        _ => {
            return Err(AppError::invalid(
                "Only OpenAPI 3.x and Swagger 2.0 specs can be imported",
            ));
        }
    };
    let mut reader = Reader {
        root: spec,
        version,
        warnings: Vec::new(),
        variables: Vec::new(),
        external_refs: false,
    };
    reader.read()
}

struct Reader<'a> {
    root: &'a Value,
    version: Version,
    warnings: Vec<String>,
    variables: Vec<VariableInput>,
    external_refs: bool,
}

/// The body of one operation, in the shape a stored request keeps it.
#[derive(Default)]
struct Body {
    mode: &'static str,
    text: String,
    form: Vec<FormField>,
    content_type: Option<String>,
}

impl<'a> Reader<'a> {
    fn read(&mut self) -> Result<ImportPlan, AppError> {
        let source = match self.version {
            Version::Swagger2 => "Swagger 2.0".to_owned(),
            Version::OpenApi3 => format!("OpenAPI {}", text_of(self.root.get("openapi"))),
        };
        let title = text_of(self.root.pointer("/info/title"));
        let name = clean_name(&title, "Imported API");
        let base_url = self.base_url();
        self.add_variable("baseUrl", base_url, false);

        let global_auth = self
            .root
            .get("security")
            .and_then(|requirements| self.auth_for(requirements))
            .unwrap_or(Auth::None);
        let mut sections = self.declared_tags();
        let mut requests = Vec::new();
        let paths = self.root.get("paths").and_then(Value::as_object);
        for (path, item) in paths.into_iter().flatten() {
            let item = self.resolve(item);
            for method in METHODS {
                let Some(operation) = item.get(method) else {
                    continue;
                };
                if requests.len() >= MAX_REQUESTS {
                    self.warn(format!(
                        "Only the first {MAX_REQUESTS} operations were imported"
                    ));
                    break;
                }
                let section = operation
                    .pointer("/tags/0")
                    .and_then(Value::as_str)
                    .map(|tag| section_index(&mut sections, tag));
                if let Some(content) = self.request(path, method, item, operation) {
                    let examples = self.examples(operation);
                    requests.push(PlannedRequest {
                        section,
                        content,
                        examples,
                        // The summary already names the request.
                        description: imported_description(&text_of(operation.get("description"))),
                    });
                }
            }
        }
        if self.external_refs {
            self.warn(
                "References to other files ($ref to another document) were left out".to_owned(),
            );
        }
        // Tags without operations would only be empty sections.
        let used: BTreeSet<usize> = requests
            .iter()
            .filter_map(|request| request.section)
            .collect();
        let (sections, requests) = drop_unused_sections(sections, requests, &used);
        Ok(ImportPlan {
            kind: ImportKind::Collection,
            source,
            name,
            description: imported_description(&text_of(self.root.pointer("/info/description"))),
            auth: global_auth,
            sections,
            requests,
            variables: std::mem::take(&mut self.variables),
            environments: Vec::new(),
            icon: None,
            script_count: 0,
            warnings: std::mem::take(&mut self.warnings),
        })
    }

    fn warn(&mut self, message: String) {
        if self.warnings.len() < MAX_WARNINGS && !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    fn add_variable(&mut self, key: &str, value: String, is_secret: bool) {
        if self.variables.iter().any(|variable| variable.key == key) {
            return;
        }
        self.variables.push(VariableInput {
            id: None,
            key: key.to_owned(),
            value: Some(value),
            is_secret,
            enabled: true,
            restricted: None,
        });
    }

    /// Follows local `$ref`s (`#/components/...`); references to other files are not read.
    fn resolve(&mut self, mut value: &'a Value) -> &'a Value {
        for _ in 0..MAX_REF_HOPS {
            let Some(reference) = value.get("$ref").and_then(Value::as_str) else {
                return value;
            };
            let Some(pointer) = reference.strip_prefix('#') else {
                self.external_refs = true;
                return value;
            };
            match self.root.pointer(pointer) {
                Some(target) => value = target,
                None => return value,
            }
        }
        value
    }

    fn base_url(&mut self) -> String {
        let url = match self.version {
            Version::OpenApi3 => {
                let server = self.root.pointer("/servers/0");
                let mut url = text_of(server.and_then(|server| server.get("url")));
                let variables = server
                    .and_then(|server| server.get("variables"))
                    .and_then(Value::as_object);
                for (name, variable) in variables.into_iter().flatten() {
                    url = url.replace(&format!("{{{name}}}"), &text_of(variable.get("default")));
                }
                url
            }
            Version::Swagger2 => {
                let host = text_of(self.root.get("host"));
                if host.is_empty() {
                    String::new()
                } else {
                    let scheme = self
                        .root
                        .pointer("/schemes/0")
                        .and_then(Value::as_str)
                        .unwrap_or("https");
                    format!("{scheme}://{host}{}", text_of(self.root.get("basePath")))
                }
            }
        };
        if url.is_empty() || url.starts_with('/') {
            self.warn(
                "The spec names no full server URL; set {{baseUrl}} in the collection variables"
                    .to_owned(),
            );
        }
        url.trim_end_matches('/').to_owned()
    }

    fn declared_tags(&self) -> Vec<PlannedSection> {
        self.root
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|tag| {
                let name = tag.get("name").and_then(Value::as_str)?;
                Some(PlannedSection {
                    name: clean_name(name, "Untitled"),
                    description: imported_description(&text_of(tag.get("description"))),
                })
            })
            .collect()
    }

    fn request(
        &mut self,
        path: &str,
        method: &str,
        item: &'a Value,
        operation: &'a Value,
    ) -> Option<RequestContent> {
        let parameters = self.parameters(item, operation);
        let mut params = Vec::new();
        let mut headers = Vec::new();
        let mut path_values = Vec::new();
        for parameter in parameters.iter().copied() {
            let name = text_of(parameter.get("name"));
            let required = parameter
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let value = self.parameter_example(parameter);
            match parameter.get("in").and_then(Value::as_str) {
                Some("query") => params.push(row(&name, &value, required)),
                Some("header")
                    if !DESCRIBED_ELSEWHERE.contains(&name.to_ascii_lowercase().as_str()) =>
                {
                    headers.push(row(&name, &value, required));
                }
                Some("path") => path_values.push((name, value)),
                Some("cookie") => self.warn(
                    "Cookie parameters were left out; add them as a Cookie header if needed"
                        .to_owned(),
                ),
                _ => {}
            }
        }
        let (url, path_variables) = convert_path(path, &path_values);
        let body = match self.version {
            Version::OpenApi3 => self.body_v3(operation),
            Version::Swagger2 => self.body_v2(operation, &parameters),
        };
        if let Some(content_type) = &body.content_type {
            headers.push(row("Content-Type", content_type, true));
        }
        let auth = match operation.get("security") {
            Some(requirements) => self.auth_for(requirements).unwrap_or(Auth::Inherit),
            None => Auth::Inherit,
        };
        let fallback = format!("{} {path}", method.to_ascii_uppercase());
        let title = operation
            .get("summary")
            .or_else(|| operation.get("operationId"))
            .and_then(Value::as_str)
            .unwrap_or(&fallback);
        let name = clean_name(title, &fallback);
        let content = RequestContent {
            kind: RequestKind::Http,
            name: name.clone(),
            method: method.to_ascii_uppercase(),
            url: format!("{{{{baseUrl}}}}{url}"),
            params,
            headers,
            body_mode: if body.mode.is_empty() {
                "none"
            } else {
                body.mode
            }
            .to_owned(),
            body: body.text,
            form: body.form,
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: String::new(),
            auth,
            mcp_operation: None,
            path_variables,
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        };
        match check_request(content) {
            Ok(content) => Some(content),
            Err(error) => {
                self.warn(format!("Skipped “{name}”: {error}"));
                None
            }
        }
    }

    /// Path-level parameters, overridden by the operation's own with the same name and place.
    fn parameters(&mut self, item: &'a Value, operation: &'a Value) -> Vec<&'a Value> {
        let mut merged: Vec<&'a Value> = Vec::new();
        for list in [item.get("parameters"), operation.get("parameters")] {
            for parameter in list.and_then(Value::as_array).into_iter().flatten() {
                let parameter = self.resolve(parameter);
                let key = (parameter.get("name"), parameter.get("in"));
                merged.retain(|existing| (existing.get("name"), existing.get("in")) != key);
                merged.push(parameter);
            }
        }
        merged
    }

    fn parameter_example(&mut self, parameter: &'a Value) -> String {
        if let Some(example) = parameter.get("example") {
            return example_text(example);
        }
        let schema = match self.version {
            Version::OpenApi3 => parameter.get("schema").unwrap_or(&Value::Null),
            Version::Swagger2 => parameter,
        };
        let schema = self.resolve(schema);
        ["example", "default"]
            .iter()
            .find_map(|key| schema.get(key))
            .or_else(|| schema.pointer("/enum/0"))
            .map(example_text)
            .unwrap_or_default()
    }

    fn body_v3(&mut self, operation: &'a Value) -> Body {
        let Some(request_body) = operation.get("requestBody") else {
            return Body::default();
        };
        let request_body = self.resolve(request_body);
        let Some(content) = request_body.get("content").and_then(Value::as_object) else {
            return Body::default();
        };
        let preferred = [
            "application/json",
            "application/x-www-form-urlencoded",
            "multipart/form-data",
            "text/plain",
        ];
        let chosen = preferred
            .iter()
            .find_map(|wanted| content.get_key_value(*wanted))
            .or_else(|| content.iter().find(|(media, _)| media.ends_with("+json")))
            .or_else(|| content.iter().next());
        let Some((media, entry)) = chosen else {
            return Body::default();
        };
        let schema = entry.get("schema").map(|schema| self.resolve(schema));
        let example = entry
            .get("example")
            .cloned()
            .or_else(|| {
                let first = entry
                    .get("examples")
                    .and_then(Value::as_object)?
                    .values()
                    .next()?;
                self.resolve(first).get("value").cloned()
            })
            .or_else(|| schema.map(|schema| self.sample(schema)));
        self.body_for(media, schema, example)
    }

    fn body_v2(&mut self, operation: &'a Value, parameters: &[&'a Value]) -> Body {
        if let Some(body) = parameters
            .iter()
            .find(|p| p.get("in").and_then(Value::as_str) == Some("body"))
        {
            let schema = body.get("schema").map(|schema| self.resolve(schema));
            let example = schema.map(|schema| self.sample(schema));
            return self.body_for("application/json", schema, example);
        }
        let fields: Vec<&'a Value> = parameters
            .iter()
            .copied()
            .filter(|p| p.get("in").and_then(Value::as_str) == Some("formData"))
            .collect();
        if fields.is_empty() {
            return Body::default();
        }
        let multipart = fields
            .iter()
            .any(|field| field.get("type").and_then(Value::as_str) == Some("file"))
            || consumes(operation, self.root)
                .iter()
                .any(|media| media == "multipart/form-data");
        let form = fields
            .into_iter()
            .map(|field| FormField {
                enabled: true,
                key: text_of(field.get("name")),
                value: if field.get("type").and_then(Value::as_str) == Some("file") {
                    String::new()
                } else {
                    self.parameter_example(field)
                },
                kind: if field.get("type").and_then(Value::as_str) == Some("file") {
                    FormFieldKind::File
                } else {
                    FormFieldKind::Text
                },
            })
            .collect();
        Body {
            mode: if multipart { "formdata" } else { "urlencoded" },
            form,
            ..Body::default()
        }
    }

    fn body_for(&mut self, media: &str, schema: Option<&'a Value>, example: Option<Value>) -> Body {
        let media = media.to_ascii_lowercase();
        if media == "application/x-www-form-urlencoded" || media == "multipart/form-data" {
            let multipart = media == "multipart/form-data";
            let properties = schema
                .and_then(|schema| schema.get("properties"))
                .and_then(Value::as_object);
            let form = properties
                .into_iter()
                .flatten()
                .map(|(key, property)| {
                    let property = self.resolve(property);
                    let binary = matches!(
                        property.get("format").and_then(Value::as_str),
                        Some("binary" | "base64")
                    );
                    let file = multipart && binary;
                    let value = if file {
                        String::new()
                    } else {
                        example
                            .as_ref()
                            .and_then(|example| example.get(key))
                            .map(example_text)
                            .unwrap_or_else(|| example_text(&self.sample(property)))
                    };
                    FormField {
                        enabled: true,
                        key: key.clone(),
                        value,
                        kind: if file {
                            FormFieldKind::File
                        } else {
                            FormFieldKind::Text
                        },
                    }
                })
                .collect();
            return Body {
                mode: if multipart { "formdata" } else { "urlencoded" },
                form,
                ..Body::default()
            };
        }
        let json = media == "application/json" || media.ends_with("+json");
        let text = match &example {
            Some(Value::String(text)) if !json => text.clone(),
            Some(Value::Null) | None => String::new(),
            Some(value) => serde_json::to_string_pretty(value).unwrap_or_default(),
        };
        let text = if text.len() > MAX_KEY_VALUE_LENGTH * 64 {
            String::new()
        } else {
            text
        };
        Body {
            mode: if json {
                "json"
            } else {
                raw_body_mode(Some(&media), &text)
            },
            text,
            form: Vec::new(),
            content_type: (media != "application/json" && media != "text/plain")
                .then(|| media.clone()),
        }
    }

    /// One example per documented status code, with the body the spec shows or its schema
    /// suggests, so the mock server can answer right after an import.
    fn examples(&mut self, operation: &'a Value) -> Vec<ExampleInput> {
        let responses = operation.get("responses").and_then(Value::as_object);
        let mut examples = Vec::new();
        for (code, response) in responses.into_iter().flatten() {
            // "default" and ranges such as "2XX" have no single status to answer with.
            let Ok(status) = code.parse::<u16>() else {
                continue;
            };
            if examples.len() >= MAX_EXAMPLES_FROM_SPEC {
                break;
            }
            let response = self.resolve(response);
            let (media, body) = self.response_body(response);
            let reason = ::http::StatusCode::from_u16(status)
                .ok()
                .and_then(|status| status.canonical_reason())
                .unwrap_or("Response");
            let description = text_of(response.get("description"));
            let input = ExampleInput {
                name: clean_name(&description, &format!("{status} {reason}")),
                status,
                headers: media
                    .filter(|_| !body.is_empty())
                    .map(|media| {
                        vec![NameValue {
                            name: "Content-Type".to_owned(),
                            value: media,
                        }]
                    })
                    .unwrap_or_default(),
                body,
            };
            if let Ok(example) = check_example(input) {
                examples.push(example);
            }
        }
        examples
    }

    fn response_body(&mut self, response: &'a Value) -> (Option<String>, String) {
        let (media, example) = match self.version {
            Version::OpenApi3 => {
                let Some(content) = response.get("content").and_then(Value::as_object) else {
                    return (None, String::new());
                };
                let chosen = content
                    .get_key_value("application/json")
                    .or_else(|| content.iter().find(|(media, _)| media.ends_with("+json")))
                    .or_else(|| content.iter().next());
                let Some((media, entry)) = chosen else {
                    return (None, String::new());
                };
                let example = entry
                    .get("example")
                    .cloned()
                    .or_else(|| {
                        let first = entry
                            .get("examples")
                            .and_then(Value::as_object)?
                            .values()
                            .next()?;
                        self.resolve(first).get("value").cloned()
                    })
                    .or_else(|| entry.get("schema").map(|schema| self.sample(schema)));
                (media.clone(), example)
            }
            Version::Swagger2 => {
                let example = response
                    .get("examples")
                    .and_then(|examples| examples.get("application/json"))
                    .cloned()
                    .or_else(|| response.get("schema").map(|schema| self.sample(schema)));
                ("application/json".to_owned(), example)
            }
        };
        let body = match example {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(text)) if !media.contains("json") => text,
            Some(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        };
        (Some(media), body)
    }

    fn sample(&mut self, schema: &'a Value) -> Value {
        let mut budget = EXAMPLE_BUDGET;
        self.example(schema, 0, &mut budget)
    }

    /// A value shaped like the schema: its example, default or first enum value where given.
    fn example(&mut self, schema: &'a Value, depth: usize, budget: &mut usize) -> Value {
        let schema = self.resolve(schema);
        if let Some(found) = ["example", "default"]
            .iter()
            .find_map(|key| schema.get(key))
        {
            return found.clone();
        }
        if let Some(first) = schema
            .pointer("/examples/0")
            .or_else(|| schema.pointer("/enum/0"))
        {
            return first.clone();
        }
        if depth > MAX_EXAMPLE_DEPTH || *budget == 0 {
            return Value::Null;
        }
        *budget -= 1;
        if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
            let mut merged = Map::new();
            for part in parts {
                if let Value::Object(object) = self.example(part, depth + 1, budget) {
                    merged.extend(object);
                }
            }
            return Value::Object(merged);
        }
        if let Some(first) = ["oneOf", "anyOf"]
            .iter()
            .find_map(|key| schema.get(key)?.get(0))
        {
            return self.example(first, depth + 1, budget);
        }
        let kind = match schema.get("type") {
            Some(Value::String(kind)) => kind.as_str(),
            Some(Value::Array(kinds)) => kinds
                .iter()
                .filter_map(Value::as_str)
                .find(|kind| *kind != "null")
                .unwrap_or("null"),
            _ if schema.get("properties").is_some() => "object",
            _ => "",
        };
        match kind {
            "object" => {
                let mut object = Map::new();
                let properties = schema.get("properties").and_then(Value::as_object);
                for (key, property) in properties.into_iter().flatten() {
                    if *budget == 0 {
                        break;
                    }
                    let value = self.example(property, depth + 1, budget);
                    object.insert(key.clone(), value);
                }
                Value::Object(object)
            }
            "array" => match schema.get("items") {
                Some(items) => Value::Array(vec![self.example(items, depth + 1, budget)]),
                None => Value::Array(Vec::new()),
            },
            "string" => Value::String(
                match schema.get("format").and_then(Value::as_str) {
                    Some("date") => "2024-01-01",
                    Some("date-time") => "2024-01-01T00:00:00Z",
                    Some("email") => "user@example.com",
                    Some("uuid") => "00000000-0000-4000-8000-000000000000",
                    Some("uri" | "url") => "https://example.com",
                    Some("binary" | "byte" | "base64") => "",
                    _ => "string",
                }
                .to_owned(),
            ),
            "integer" | "number" => Value::from(0),
            "boolean" => Value::Bool(true),
            _ => Value::Null,
        }
    }

    /// The first scheme of the first requirement becomes the auth; an empty list means none.
    fn auth_for(&mut self, requirements: &'a Value) -> Option<Auth> {
        let requirements = requirements.as_array()?;
        let Some(first) = requirements.first() else {
            return Some(Auth::None);
        };
        let (scheme_name, scopes) = first.as_object()?.iter().next()?;
        let pointer = match self.version {
            Version::OpenApi3 => format!("/components/securitySchemes/{scheme_name}"),
            Version::Swagger2 => format!("/securityDefinitions/{scheme_name}"),
        };
        let scheme = self
            .root
            .pointer(&pointer)
            .map(|scheme| self.resolve(scheme))?;
        let kind = scheme
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let http_scheme = scheme
            .get("scheme")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        match (kind, http_scheme.as_str()) {
            ("http", "bearer") => {
                self.add_variable("bearerToken", String::new(), true);
                Some(Auth::Bearer {
                    token: "{{bearerToken}}".to_owned(),
                    prefix: "Bearer".to_owned(),
                })
            }
            ("http", "basic") | ("basic", _) => {
                self.add_variable("username", String::new(), false);
                self.add_variable("password", String::new(), true);
                Some(Auth::Basic {
                    username: "{{username}}".to_owned(),
                    password: "{{password}}".to_owned(),
                })
            }
            ("http", "digest") => {
                self.add_variable("username", String::new(), false);
                self.add_variable("password", String::new(), true);
                Some(Auth::Digest {
                    username: "{{username}}".to_owned(),
                    password: "{{password}}".to_owned(),
                })
            }
            ("apiKey", _) => {
                let location = match scheme.get("in").and_then(Value::as_str) {
                    Some("header") => ApiKeyLocation::Header,
                    Some("query") => ApiKeyLocation::Query,
                    _ => {
                        self.warn(format!("The “{scheme_name}” API key goes in a cookie; add it as a Cookie header"));
                        return None;
                    }
                };
                self.add_variable("apiKey", String::new(), true);
                Some(Auth::ApiKey {
                    key: text_of(scheme.get("name")),
                    value: "{{apiKey}}".to_owned(),
                    location,
                })
            }
            ("oauth2", _) => self.oauth2(scheme, scopes),
            _ => {
                self.warn(format!("The “{scheme_name}” security scheme ({kind}) is not supported; set auth by hand"));
                None
            }
        }
    }

    fn oauth2(&mut self, scheme: &'a Value, scopes: &Value) -> Option<Auth> {
        let scope = scopes
            .as_array()
            .map(|scopes| {
                scopes
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        // OpenAPI 3 lists flows by name; Swagger 2 has one `flow` with the URLs beside it.
        let (grant, flow) = match self.version {
            Version::OpenApi3 => {
                let flows = scheme.get("flows")?;
                [
                    ("authorizationCode", OAuthGrant::AuthorizationCode),
                    ("clientCredentials", OAuthGrant::ClientCredentials),
                    ("password", OAuthGrant::Password),
                ]
                .into_iter()
                .find_map(|(name, grant)| flows.get(name).map(|flow| (grant, flow)))?
            }
            Version::Swagger2 => {
                let grant = match scheme.get("flow").and_then(Value::as_str) {
                    Some("application") => OAuthGrant::ClientCredentials,
                    Some("password") => OAuthGrant::Password,
                    Some("accessCode") => OAuthGrant::AuthorizationCode,
                    _ => {
                        self.warn("The implicit OAuth 2.0 flow is not supported; use the authorization code flow".to_owned());
                        return None;
                    }
                };
                (grant, scheme)
            }
        };
        self.add_variable("clientId", String::new(), false);
        self.add_variable("clientSecret", String::new(), true);
        Some(Auth::OAuth2(Box::new(OAuth2Settings {
            grant,
            auth_url: text_of(flow.get("authorizationUrl")),
            token_url: text_of(flow.get("tokenUrl")),
            client_id: "{{clientId}}".to_owned(),
            client_secret: "{{clientSecret}}".to_owned(),
            scope,
            redirect_uri: "http://127.0.0.1/callback".to_owned(),
            use_pkce: true,
            username: String::new(),
            password: String::new(),
            client_auth: ClientAuth::Header,
            header_prefix: String::new(),
        })))
    }
}

/// `/users/{id}` becomes `/users/:id`, with the path parameter's example as its value. Names
/// that cannot be a path variable are tidied (`{user.id}` becomes `:user_id`).
fn convert_path(path: &str, values: &[(String, String)]) -> (String, Vec<PathVariable>) {
    let mut variables: Vec<PathVariable> = Vec::new();
    let segments: Vec<String> = path
        .split('/')
        .map(|segment| {
            let Some(name) = segment
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
            else {
                return segment.to_owned();
            };
            let mut key: String = name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            if variable_name(&format!(":{key}")).is_none() {
                key.insert(0, '_');
            }
            if !variables.iter().any(|variable| variable.key == key) {
                let value = values
                    .iter()
                    .find(|(parameter, _)| parameter == name)
                    .map(|(_, value)| value.clone())
                    .unwrap_or_default();
                variables.push(PathVariable {
                    key: key.clone(),
                    value,
                });
            }
            format!(":{key}")
        })
        .collect();
    (segments.join("/"), variables)
}

fn consumes(operation: &Value, root: &Value) -> Vec<String> {
    operation
        .get("consumes")
        .or_else(|| root.get("consumes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn row(key: &str, value: &str, enabled: bool) -> KeyValue {
    KeyValue {
        enabled,
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

fn section_index(sections: &mut Vec<PlannedSection>, tag: &str) -> usize {
    let name = clean_name(tag, "Untitled");
    match sections.iter().position(|section| section.name == name) {
        Some(index) => index,
        None => {
            sections.push(PlannedSection::named(name));
            sections.len() - 1
        }
    }
}

fn drop_unused_sections(
    sections: Vec<PlannedSection>,
    requests: Vec<PlannedRequest>,
    used: &BTreeSet<usize>,
) -> (Vec<PlannedSection>, Vec<PlannedRequest>) {
    let kept: Vec<usize> = (0..sections.len())
        .filter(|index| used.contains(index))
        .collect();
    let renumber = |old: usize| kept.iter().position(|index| *index == old);
    let requests = requests
        .into_iter()
        .map(|request| PlannedRequest {
            section: request.section.and_then(renumber),
            ..request
        })
        .collect();
    let sections = sections
        .into_iter()
        .enumerate()
        .filter(|(index, _)| used.contains(index))
        .map(|(_, section)| section)
        .collect();
    (sections, requests)
}

fn text_of(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn example_text(value: &Value) -> String {
    text_of(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PETSTORE: &str = r#"
openapi: 3.0.3
info:
  title: Pet Store
servers:
  - url: https://{region}.pets.example/v1/
    variables:
      region:
        default: eu
security:
  - bearer: []
tags:
  - name: pets
  - name: unused
paths:
  /pets/{petId}:
    parameters:
      - name: petId
        in: path
        required: true
        schema: { type: integer, example: 42 }
    get:
      tags: [pets]
      summary: Get a pet
      responses:
        200: { description: The pet }
      parameters:
        - name: fields
          in: query
          schema: { type: string, default: name }
        - $ref: '#/components/parameters/Trace'
    delete:
      tags: [pets]
      operationId: deletePet
      security: []
  /pets:
    post:
      tags: [pets]
      summary: Add a pet
      security:
        - oauth: [pets.write]
      requestBody:
        content:
          application/json:
            schema: { $ref: '#/components/schemas/Pet' }
  /upload:
    post:
      requestBody:
        content:
          multipart/form-data:
            schema:
              type: object
              properties:
                caption: { type: string, example: hello }
                file: { type: string, format: binary }
components:
  parameters:
    Trace:
      name: X-Trace
      in: header
      required: true
      schema: { type: string, example: abc }
  schemas:
    Pet:
      type: object
      properties:
        name: { type: string, example: Rex }
        born: { type: string, format: date }
        tags: { type: array, items: { type: string } }
        owner: { $ref: '#/components/schemas/Owner' }
    Owner:
      allOf:
        - type: object
          properties: { id: { type: string, format: uuid } }
        - type: object
          properties: { pet: { $ref: '#/components/schemas/Pet' } }
  securitySchemes:
    bearer: { type: http, scheme: bearer }
    oauth:
      type: oauth2
      flows:
        clientCredentials:
          tokenUrl: https://auth.pets.example/token
          scopes: { pets.write: write }
"#;

    fn find<'p>(plan: &'p ImportPlan, name: &str) -> &'p RequestContent {
        &plan
            .requests
            .iter()
            .find(|request| request.content.name == name)
            .expect("request")
            .content
    }

    #[test]
    fn imports_an_openapi_3_yaml_spec() {
        let spec: Value = serde_saphyr::from_str(PETSTORE).expect("yaml");
        let plan = plan(&spec).expect("plan");
        assert_eq!(plan.source, "OpenAPI 3.0.3");
        assert_eq!(plan.name, "Pet Store");
        assert_eq!(plan.section_names(), ["pets"]);
        assert_eq!(
            plan.variables[0].value.as_deref(),
            Some("https://eu.pets.example/v1")
        );
        assert!(matches!(plan.auth, Auth::Bearer { ref token, .. } if token == "{{bearerToken}}"));
        assert!(
            plan.variables
                .iter()
                .any(|v| v.key == "bearerToken" && v.is_secret)
        );

        let get_examples = &plan
            .requests
            .iter()
            .find(|request| request.content.name == "Get a pet")
            .expect("request")
            .examples;
        assert_eq!(get_examples.len(), 1);
        assert_eq!(get_examples[0].status, 200);
        assert_eq!(get_examples[0].name, "The pet");
        let get = find(&plan, "Get a pet");
        assert_eq!(get.url, "{{baseUrl}}/pets/:petId");
        assert_eq!(
            get.path_variables,
            [PathVariable {
                key: "petId".to_owned(),
                value: "42".to_owned()
            }]
        );
        assert_eq!(get.params[0].key, "fields");
        assert_eq!(get.params[0].value, "name");
        assert!(!get.params[0].enabled);
        assert_eq!(get.headers[0].key, "X-Trace");
        assert_eq!(get.auth, Auth::Inherit);
        assert_eq!(find(&plan, "deletePet").auth, Auth::None);

        let add = find(&plan, "Add a pet");
        assert_eq!(add.body_mode, "json");
        let body: Value = serde_json::from_str(&add.body).expect("json body");
        assert_eq!(body["name"], "Rex");
        assert_eq!(body["born"], "2024-01-01");
        assert_eq!(body["owner"]["id"], "00000000-0000-4000-8000-000000000000");
        assert!(
            matches!(add.auth, Auth::OAuth2(ref s) if s.grant == OAuthGrant::ClientCredentials && s.scope == "pets.write")
        );

        let upload = find(&plan, "POST /upload");
        assert_eq!(upload.body_mode, "formdata");
        assert_eq!(upload.form[0].value, "hello");
        assert_eq!(upload.form[1].kind, FormFieldKind::File);
    }

    #[test]
    fn imports_a_swagger_2_spec() {
        let spec: Value = serde_json::from_str(r#"{
          "swagger": "2.0", "info": { "title": "Legacy" }, "host": "api.legacy.test", "basePath": "/v2",
          "securityDefinitions": { "key": { "type": "apiKey", "in": "query", "name": "api_key" } },
          "security": [{ "key": [] }],
          "paths": { "/users/{user.id}": { "put": {
            "parameters": [
              { "name": "user.id", "in": "path", "type": "string", "required": true },
              { "name": "body", "in": "body", "schema": { "type": "object", "properties": { "age": { "type": "integer" } } } }
            ] } },
            "/login": { "post": { "consumes": ["application/x-www-form-urlencoded"], "parameters": [
              { "name": "user", "in": "formData", "type": "string", "default": "ada" } ] } } }
        }"#).expect("json");
        let plan = plan(&spec).expect("plan");
        assert_eq!(plan.source, "Swagger 2.0");
        assert_eq!(
            plan.variables[0].value.as_deref(),
            Some("https://api.legacy.test/v2")
        );
        assert!(
            matches!(plan.auth, Auth::ApiKey { ref key, location: ApiKeyLocation::Query, .. } if key == "api_key")
        );
        let put = find(&plan, "PUT /users/{user.id}");
        assert_eq!(put.url, "{{baseUrl}}/users/:user_id");
        assert_eq!(put.body, "{\n  \"age\": 0\n}");
        let login = find(&plan, "POST /login");
        assert_eq!(login.body_mode, "urlencoded");
        assert_eq!(login.form[0].value, "ada");
    }

    #[test]
    fn refuses_other_versions() {
        let spec: Value = serde_json::from_str(r#"{ "swagger": "1.2" }"#).expect("json");
        assert!(plan(&spec).is_err());
    }
}
