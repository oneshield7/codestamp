use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use std::collections::BTreeSet;

use super::models::{ImportKind, ImportPlan, PlannedRequest, PlannedSection};
use super::url_parts::split_url;
use crate::auth::models::{Auth as RequestAuth, ClientAuth, OAuth2Settings, OAuthGrant};
use crate::encoding::base64_encode;
use crate::environment::dynamic::is_dynamic;
use crate::environment::models::VariableInput;
use crate::environment::resolve::{collect_names, is_valid_name};
use crate::error::AppError;
use crate::request::body_mode::raw_body_mode;
use crate::request::path_vars;
use crate::request::rules::{MAX_KEY_VALUE_LENGTH, MAX_SCRIPT_BYTES, is_allowed_method};
use crate::request::types::NameValue;
use crate::storage::descriptions::MAX_DESCRIPTION_CHARS;
use crate::storage::examples::{ExampleInput, MAX_EXAMPLES_PER_REQUEST, check_example};
use crate::storage::models::{
    FormField, FormFieldKind, KeyValue, PathVariable, RequestContent, RequestKind,
};
use crate::storage::validate::check_request;

pub const MAX_IMPORT_BYTES: usize = 20 * 1024 * 1024;
pub(super) const MAX_REQUESTS: usize = 2_000;
const MAX_SECTIONS: usize = 500;
const MAX_VARIABLES: usize = 200;
const MAX_NAME_CHARS: usize = 120;
pub(super) const MAX_WARNINGS: usize = 50;

// ---- Postman v2.0/v2.1 shapes (only the parts Codestamp uses) ----

#[derive(Deserialize)]
struct Collection {
    info: Info,
    #[serde(default)]
    item: Vec<Item>,
    #[serde(default)]
    variable: Vec<Variable>,
    #[serde(default)]
    event: Vec<Event>,
    auth: Option<Auth>,
}

#[derive(Deserialize)]
struct Info {
    #[serde(default)]
    name: String,
    description: Option<Value>,
    /// The schema URL, which names the format version.
    #[serde(default)]
    schema: String,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    name: String,
    request: Option<RequestField>,
    item: Option<Vec<Item>>,
    #[serde(default)]
    event: Vec<Event>,
    auth: Option<Auth>,
    /// Saved responses ("examples").
    #[serde(default)]
    response: Vec<SavedResponse>,
    /// A folder's docs; a request's are usually on the request itself.
    description: Option<Value>,
}

#[derive(Deserialize)]
struct SavedResponse {
    #[serde(default)]
    name: String,
    code: Option<u16>,
    #[serde(default)]
    header: Option<Vec<Pair>>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RequestField {
    Url(String),
    Full(Box<Request>),
}

#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    header: HeaderField,
    url: Option<UrlField>,
    body: Option<Body>,
    auth: Option<Auth>,
    description: Option<Value>,
}

#[derive(Deserialize, Default)]
#[serde(untagged)]
enum HeaderField {
    List(Vec<Pair>),
    Raw(String),
    #[default]
    Missing,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum UrlField {
    Raw(String),
    Object(UrlObject),
}

#[derive(Deserialize)]
struct UrlObject {
    #[serde(default)]
    raw: String,
    query: Option<Vec<Pair>>,
    /// Values for `:name` path segments.
    #[serde(default)]
    variable: Vec<Pair>,
}

#[derive(Deserialize)]
struct Pair {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Value,
    #[serde(default)]
    disabled: bool,
}

#[derive(Deserialize)]
struct Body {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    raw: Option<String>,
    #[serde(default)]
    urlencoded: Vec<Pair>,
    #[serde(default)]
    formdata: Vec<FormDataPair>,
    graphql: Option<GraphQl>,
    options: Option<Value>,
}

#[derive(Deserialize)]
struct FormDataPair {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Value,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[derive(Deserialize)]
struct GraphQl {
    #[serde(default)]
    query: String,
    #[serde(default)]
    variables: Option<String>,
}

#[derive(Deserialize)]
struct Event {
    #[serde(default)]
    listen: String,
    script: Option<Script>,
}

#[derive(Deserialize)]
struct Script {
    #[serde(default)]
    exec: Exec,
}

#[derive(Deserialize, Default)]
#[serde(untagged)]
enum Exec {
    Lines(Vec<String>),
    Text(String),
    #[default]
    Missing,
}

#[derive(Deserialize, Clone)]
struct Auth {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    bearer: Vec<AuthParam>,
    #[serde(default)]
    basic: Vec<AuthParam>,
    #[serde(default)]
    apikey: Vec<AuthParam>,
    #[serde(default)]
    digest: Vec<AuthParam>,
    #[serde(default)]
    awsv4: Vec<AuthParam>,
    #[serde(default)]
    oauth2: Vec<AuthParam>,
}

#[derive(Deserialize, Clone)]
struct AuthParam {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: Value,
}

#[derive(Deserialize)]
struct Variable {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: Value,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[derive(Deserialize)]
struct EnvironmentFile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    values: Vec<EnvironmentValue>,
}

#[derive(Deserialize)]
struct EnvironmentValue {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: Value,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

const fn enabled_by_default() -> bool {
    true
}

// ---- Planning ----

/// Reads a parsed collection (v2.0/v2.1) or environment export and turns it into an import
/// plan. Nothing is written; unsupported parts become warnings.
pub fn plan_value(value: Value) -> Result<ImportPlan, AppError> {
    let structure_error =
        || AppError::invalid("The file does not have the expected collection structure");
    if value.get("info").is_some() && value.get("item").is_some() {
        let collection: Collection =
            serde_json::from_value(value).map_err(|_| structure_error())?;
        return Ok(plan_collection(collection));
    }
    if value.get("values").is_some() {
        let environment: EnvironmentFile =
            serde_json::from_value(value).map_err(|_| structure_error())?;
        return Ok(plan_environment(environment));
    }
    if value.get("requests").is_some() {
        return Err(AppError::invalid(
            "This is an old v1 collection. Export it again as Collection v2.1",
        ));
    }
    Err(AppError::invalid(
        "This is not a collection or environment file",
    ))
}

/// Up to this many unsupported dynamic variable names are listed in the warning.
const MAX_LISTED_DYNAMIC: usize = 5;

#[derive(Default)]
struct Notes {
    warnings: Vec<String>,
    require: usize,
    unsupported_dynamic: usize,
    unsupported_names: BTreeSet<String>,
    files: usize,
}

impl Notes {
    fn warn(&mut self, message: String) {
        if self.warnings.len() < MAX_WARNINGS {
            self.warnings.push(message);
        }
    }

    fn finish(mut self) -> Vec<String> {
        if self.require > 0 {
            self.warn(format!(
                "{} request(s) use require(), which is not available in the sandbox",
                self.require
            ));
        }
        if self.unsupported_dynamic > 0 {
            let names: Vec<String> = self
                .unsupported_names
                .iter()
                .take(MAX_LISTED_DYNAMIC)
                .map(|name| format!("{{{{{name}}}}}"))
                .collect();
            self.warn(format!(
                "{} request(s) use dynamic variables Codestamp does not have: {}",
                self.unsupported_dynamic,
                names.join(", ")
            ));
        }
        if self.files > 0 {
            self.warn(format!(
                "{} request(s) upload files. File paths are not imported; choose the files again in the Body tab",
                self.files
            ));
        }
        self.warnings
    }
}

/// Scripts inherited from the collection and folders, run before the request's own.
#[derive(Clone, Default)]
struct ScriptChain {
    pre_request: StageScripts,
    test: StageScripts,
}

/// The scripts of one stage. Once they pass the size limit they are dropped right away, so a
/// huge inherited script is not copied into every request of the collection.
#[derive(Clone, Default)]
struct StageScripts {
    parts: Vec<String>,
    too_large: bool,
}

impl StageScripts {
    fn push(&mut self, code: String) {
        let total = self.parts.iter().map(String::len).sum::<usize>() + code.len();
        if self.too_large || total > MAX_SCRIPT_BYTES {
            self.too_large = true;
            self.parts.clear();
        } else {
            self.parts.push(code);
        }
    }
}

impl ScriptChain {
    fn with(&self, events: &[Event]) -> Self {
        let mut chain = self.clone();
        for event in events {
            let code = event
                .script
                .as_ref()
                .map(|s| exec_text(&s.exec))
                .unwrap_or_default();
            if code.trim().is_empty() {
                continue;
            }
            match event.listen.as_str() {
                "prerequest" => chain.pre_request.push(code),
                "test" => chain.test.push(code),
                _ => {}
            }
        }
        chain
    }
}

struct Planner {
    sections: Vec<PlannedSection>,
    requests: Vec<PlannedRequest>,
    script_count: usize,
    notes: Notes,
}

fn plan_collection(collection: Collection) -> ImportPlan {
    let mut planner = Planner {
        sections: Vec::new(),
        requests: Vec::new(),
        script_count: 0,
        notes: Notes::default(),
    };
    let chain = ScriptChain::default().with(&collection.event);
    planner.walk(
        &collection.item,
        None,
        &[],
        collection.auth.as_ref(),
        &chain,
    );
    let variables = plan_variables(
        collection.variable.iter().map(|v| {
            (
                v.key.as_str(),
                &v.value,
                !v.disabled,
                v.kind.as_deref() == Some("secret"),
            )
        }),
        &mut planner.notes,
    );
    let source = if collection.info.schema.contains("v2.0") {
        "Collection v2.0"
    } else {
        "Collection v2.1"
    };
    ImportPlan {
        kind: ImportKind::Collection,
        source: source.to_owned(),
        // Collection auth is copied into each request, as Postman applies it.
        auth: RequestAuth::None,
        name: clean_name(&collection.info.name, "Imported collection"),
        description: description_of(collection.info.description.as_ref()),
        sections: planner.sections,
        requests: planner.requests,
        variables,
        environments: Vec::new(),
        icon: None,
        script_count: planner.script_count,
        warnings: planner.notes.finish(),
    }
}

fn plan_environment(environment: EnvironmentFile) -> ImportPlan {
    let mut notes = Notes::default();
    let variables = plan_variables(
        environment.values.iter().map(|v| {
            (
                v.key.as_str(),
                &v.value,
                v.enabled,
                v.kind.as_deref() == Some("secret"),
            )
        }),
        &mut notes,
    );
    ImportPlan {
        kind: ImportKind::Environment,
        source: "Environment".to_owned(),
        auth: RequestAuth::None,
        name: clean_name(&environment.name, "Imported environment"),
        description: String::new(),
        sections: Vec::new(),
        requests: Vec::new(),
        variables,
        environments: Vec::new(),
        icon: None,
        script_count: 0,
        warnings: notes.finish(),
    }
}

impl Planner {
    fn walk(
        &mut self,
        items: &[Item],
        section: Option<usize>,
        path: &[String],
        auth: Option<&Auth>,
        chain: &ScriptChain,
    ) {
        for item in items {
            let item_auth = item.auth.as_ref().or(auth);
            if let Some(children) = &item.item {
                let chain = chain.with(&item.event);
                // Top-level folders become sections; deeper folders are folded into the name.
                if section.is_none() && self.sections.len() < MAX_SECTIONS {
                    self.sections.push(PlannedSection {
                        name: clean_name(&item.name, "Untitled folder"),
                        description: description_of(item.description.as_ref()),
                    });
                    let index = self.sections.len() - 1;
                    self.walk(children, Some(index), &[], item_auth, &chain);
                } else {
                    let mut nested = path.to_vec();
                    nested.push(item.name.clone());
                    self.walk(children, section, &nested, item_auth, &chain);
                }
                continue;
            }
            let Some(request) = &item.request else {
                continue;
            };
            if self.requests.len() >= MAX_REQUESTS {
                self.notes.warn(format!(
                    "Only the first {MAX_REQUESTS} requests were imported"
                ));
                return;
            }
            let mut full_name = path.to_vec();
            full_name.push(item.name.clone());
            let name = clean_name(&full_name.join(" / "), "Untitled request");
            let chain = chain.with(&item.event);
            if let Some(content) = self.convert(&name, request, item_auth, &chain) {
                let examples = self.examples(&name, &item.response);
                let own = match request {
                    RequestField::Full(full) => full.description.as_ref(),
                    RequestField::Url(_) => None,
                };
                self.requests.push(PlannedRequest {
                    section,
                    content,
                    examples,
                    description: description_of(own.or(item.description.as_ref())),
                });
            }
        }
    }

    fn convert(
        &mut self,
        name: &str,
        field: &RequestField,
        inherited_auth: Option<&Auth>,
        chain: &ScriptChain,
    ) -> Option<RequestContent> {
        let (method, url_field, header, body, own_auth) = match field {
            RequestField::Url(url) => (
                "GET".to_owned(),
                Some(UrlRef::Raw(url)),
                &HeaderField::Missing,
                None,
                None,
            ),
            RequestField::Full(request) => (
                request.method.as_deref().unwrap_or("GET").to_uppercase(),
                request.url.as_ref().map(UrlRef::from),
                &request.header,
                request.body.as_ref(),
                request.auth.as_ref(),
            ),
        };
        if !is_allowed_method(&method) {
            self.notes.warn(format!(
                "Skipped “{name}”: the {method} method is not supported"
            ));
            return None;
        }

        let (url, params, path_values) = match url_field {
            Some(UrlRef::Raw(raw)) => {
                let (base, params) = split_url(raw);
                (base, params, Vec::new())
            }
            Some(UrlRef::Object(object)) => {
                let (base, parsed) = split_url(&object.raw);
                let params = object.query.as_ref().map_or(parsed, |query| pairs(query));
                (base, params, pairs(&object.variable))
            }
            None => (String::new(), Vec::new(), Vec::new()),
        };
        // Every `:name` in the path gets a row, filled from the export when it has a value.
        let path_variables = path_vars::names(&url)
            .into_iter()
            .map(|key| PathVariable {
                key: key.to_owned(),
                value: path_values
                    .iter()
                    .find(|pair| pair.key == key)
                    .map(|pair| pair.value.clone())
                    .unwrap_or_default(),
            })
            .collect();
        let mut headers = match header {
            HeaderField::List(list) => pairs(list),
            HeaderField::Raw(raw) => raw
                .lines()
                .filter_map(|line| line.split_once(':'))
                .map(|(key, value)| KeyValue {
                    enabled: true,
                    key: key.trim().to_owned(),
                    value: value.trim().to_owned(),
                })
                .collect(),
            HeaderField::Missing => Vec::new(),
        };
        let mut params = params;
        // A GraphQL body sent with POST becomes a GraphQL request; with other methods it stays
        // the JSON it would be sent as.
        let graphql = body
            .filter(|body| body.mode == "graphql" && method == "POST")
            .and_then(|body| body.graphql.as_ref());
        let (kind, converted, graphql_variables) = match graphql {
            Some(graphql) => (
                RequestKind::Graphql,
                ConvertedBody::text("json", graphql.query.clone()),
                graphql.variables.clone().unwrap_or_default(),
            ),
            None => (
                RequestKind::Http,
                self.convert_body(name, body, &mut headers),
                String::new(),
            ),
        };

        let mut pre_request = chain.pre_request.clone();
        let auth = own_auth.or(inherited_auth);
        let request_auth = auth.and_then(typed_auth);
        if request_auth.is_none()
            && let Some(auth) = auth
            && let Some(script) = self.apply_auth(name, auth, &mut headers, &mut params)
        {
            pre_request.parts.insert(0, script);
        }

        let pre_request_script = self.script_text(name, "pre-request", &pre_request);
        let test_script = self.script_text(name, "test", &chain.test);
        let texts = [
            url.as_str(),
            converted.text.as_str(),
            pre_request_script.as_str(),
            test_script.as_str(),
        ]
        .into_iter()
        .chain(
            params
                .iter()
                .chain(&headers)
                .flat_map(|row| [row.key.as_str(), row.value.as_str()]),
        )
        .chain(
            converted
                .form
                .iter()
                .flat_map(|field| [field.key.as_str(), field.value.as_str()]),
        );
        self.note_unsupported_dynamic(texts);

        match check_request(RequestContent {
            kind,
            name: name.to_owned(),
            method,
            url,
            params,
            headers,
            body_mode: converted.mode.to_owned(),
            body: converted.text,
            form: converted.form,
            body_file: String::new(),
            pre_request_script,
            test_script,
            auth: request_auth.unwrap_or(RequestAuth::Inherit),
            mcp_operation: None,
            path_variables,
            graphql_variables,
            grpc: None,
            soap: None,
        }) {
            Ok(content) => Some(content),
            Err(error) => {
                self.notes.warn(format!("Skipped “{name}”: {error}"));
                None
            }
        }
    }

    fn examples(&mut self, name: &str, responses: &[SavedResponse]) -> Vec<ExampleInput> {
        let mut examples = Vec::new();
        for response in responses.iter().take(MAX_EXAMPLES_PER_REQUEST) {
            let input = ExampleInput {
                name: clean_name(&response.name, "Example"),
                status: response.code.unwrap_or(200),
                headers: response
                    .header
                    .as_deref()
                    .map(pairs)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|row| NameValue {
                        name: row.key,
                        value: row.value,
                    })
                    .collect(),
                body: response.body.clone().unwrap_or_default(),
            };
            match check_example(input) {
                Ok(example) => examples.push(example),
                Err(error) => self
                    .notes
                    .warn(format!("“{name}”: an example was left out ({error})")),
            }
        }
        examples
    }

    fn convert_body(
        &mut self,
        name: &str,
        body: Option<&Body>,
        headers: &mut Vec<KeyValue>,
    ) -> ConvertedBody {
        let Some(body) = body else {
            return ConvertedBody::text("none", String::new());
        };
        match body.mode.as_str() {
            "raw" => {
                let text = body.raw.clone().unwrap_or_default();
                let language = body
                    .options
                    .as_ref()
                    .and_then(|o| o.pointer("/raw/language"))
                    .and_then(Value::as_str);
                let media = match language {
                    Some("json") => Some("application/json"),
                    Some("xml") => Some("application/xml"),
                    Some("html") => Some("text/html"),
                    Some(_) => Some("text/plain"),
                    None => None,
                };
                let mode = if text.is_empty() {
                    "none"
                } else {
                    raw_body_mode(media, &text)
                };
                ConvertedBody::text(mode, text)
            }
            "urlencoded" => ConvertedBody::form("urlencoded", form_rows(&body.urlencoded)),
            "formdata" => {
                let mut has_files = false;
                let rows = body
                    .formdata
                    .iter()
                    .filter_map(|pair| {
                        let key = pair.key.as_deref()?.trim();
                        if key.is_empty() {
                            return None;
                        }
                        let is_file = pair.kind.as_deref() == Some("file");
                        has_files |= is_file;
                        Some(FormField {
                            enabled: !pair.disabled,
                            key: key.to_owned(),
                            // A path from another device must never become an upload source.
                            value: if is_file {
                                String::new()
                            } else {
                                value_text(&pair.value)
                            },
                            kind: if is_file {
                                FormFieldKind::File
                            } else {
                                FormFieldKind::Text
                            },
                        })
                    })
                    .collect();
                if has_files {
                    self.notes.files += 1;
                }
                ConvertedBody::form("formdata", rows)
            }
            "file" => {
                self.notes.files += 1;
                ConvertedBody::text("binary", String::new())
            }
            "graphql" => {
                let graphql = body.graphql.as_ref();
                let variables: Value = graphql
                    .and_then(|g| g.variables.as_deref())
                    .and_then(|v| serde_json::from_str(v).ok())
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                let payload = serde_json::json!({
                    "query": graphql.map(|g| g.query.clone()).unwrap_or_default(),
                    "variables": variables,
                });
                add_missing_header(headers, "Content-Type", "application/json");
                ConvertedBody::text(
                    "json",
                    serde_json::to_string_pretty(&payload).unwrap_or_default(),
                )
            }
            "" => ConvertedBody::text("none", String::new()),
            other => {
                self.notes.warn(format!(
                    "“{name}”: the {other} body is not supported yet and was left empty"
                ));
                ConvertedBody::text("none", String::new())
            }
        }
    }

    /// Counts a request that uses `{{$name}}` values Codestamp cannot fill in.
    fn note_unsupported_dynamic<'a>(&mut self, texts: impl Iterator<Item = &'a str>) {
        let mut names = BTreeSet::new();
        for text in texts {
            collect_names(text, &mut names);
        }
        let unsupported: Vec<String> = names
            .into_iter()
            .filter(|name| name.starts_with('$') && !is_dynamic(name))
            .collect();
        if !unsupported.is_empty() {
            self.notes.unsupported_dynamic += 1;
            self.notes.unsupported_names.extend(unsupported);
        }
    }

    /// Turns Postman auth into headers or params. Basic auth that uses variables cannot be
    /// encoded ahead of time, so it becomes a small pre-request script instead.
    fn apply_auth(
        &mut self,
        name: &str,
        auth: &Auth,
        headers: &mut Vec<KeyValue>,
        params: &mut Vec<KeyValue>,
    ) -> Option<String> {
        let has_authorization = headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("authorization"));
        match auth.kind.as_str() {
            "noauth" | "" => None,
            "bearer" if !has_authorization => {
                let token = param(&auth.bearer, "token");
                headers.push(enabled("Authorization", &format!("Bearer {token}")));
                None
            }
            "basic" if !has_authorization => {
                let user = param(&auth.basic, "username");
                let password = param(&auth.basic, "password");
                if user.contains("{{") || password.contains("{{") {
                    let quote = |text: &str| serde_json::to_string(text).unwrap_or_default();
                    return Some(format!(
                        "// Basic auth from the imported collection\nconst basicUser = pm.variables.replaceIn({});\nconst basicPassword = pm.variables.replaceIn({});\npm.request.headers.upsert({{ key: 'Authorization', value: 'Basic ' + btoa(basicUser + ':' + basicPassword) }});",
                        quote(&user),
                        quote(&password)
                    ));
                }
                let encoded = base64_encode(format!("{user}:{password}").as_bytes());
                headers.push(enabled("Authorization", &format!("Basic {encoded}")));
                None
            }
            "apikey" => {
                let key = param(&auth.apikey, "key");
                let value = param(&auth.apikey, "value");
                if param(&auth.apikey, "in") == "query" {
                    params.push(enabled(&key, &value));
                } else if !headers.iter().any(|h| h.key.eq_ignore_ascii_case(&key)) {
                    headers.push(enabled(&key, &value));
                }
                None
            }
            "bearer" | "basic" => None,
            other => {
                self.notes.warn(format!(
                    "“{name}”: {other} auth is not supported yet; add the header manually"
                ));
                None
            }
        }
    }

    fn script_text(&mut self, name: &str, stage: &str, scripts: &StageScripts) -> String {
        let text = scripts.parts.join("\n\n");
        if !scripts.too_large && text.trim().is_empty() {
            return String::new();
        }
        if scripts.too_large || text.len() > MAX_SCRIPT_BYTES {
            self.notes.warn(format!(
                "“{name}”: the {stage} script is larger than 100 KB and was left out"
            ));
            return String::new();
        }
        self.script_count += 1;
        if text.contains("require(") {
            self.notes.require += 1;
        }
        text
    }
}

/// A converted body: raw `text` for JSON/text, `form` rows for URL-encoded and form data.
struct ConvertedBody {
    mode: &'static str,
    text: String,
    form: Vec<FormField>,
}

impl ConvertedBody {
    const fn text(mode: &'static str, text: String) -> Self {
        Self {
            mode,
            text,
            form: Vec::new(),
        }
    }

    const fn form(mode: &'static str, form: Vec<FormField>) -> Self {
        Self {
            mode,
            text: String::new(),
            form,
        }
    }
}

fn form_rows(list: &[Pair]) -> Vec<FormField> {
    pairs(list)
        .into_iter()
        .map(|row| FormField {
            enabled: row.enabled,
            key: row.key,
            value: row.value,
            kind: FormFieldKind::Text,
        })
        .collect()
}

enum UrlRef<'a> {
    Raw(&'a str),
    Object(&'a UrlObject),
}

impl<'a> From<&'a UrlField> for UrlRef<'a> {
    fn from(field: &'a UrlField) -> Self {
        match field {
            UrlField::Raw(raw) => UrlRef::Raw(raw),
            UrlField::Object(object) => UrlRef::Object(object),
        }
    }
}

fn plan_variables<'a>(
    entries: impl Iterator<Item = (&'a str, &'a Value, bool, bool)>,
    notes: &mut Notes,
) -> Vec<VariableInput> {
    // Later duplicates win, like in Postman.
    let mut by_key: HashMap<String, usize> = HashMap::new();
    let mut variables: Vec<VariableInput> = Vec::new();
    for (key, value, enabled, is_secret) in entries {
        let key = key.trim();
        let value = value_text(value);
        if !is_valid_name(key) {
            notes.warn(format!(
                "Skipped variable “{key}”: names may only use letters, numbers, _ - and ."
            ));
            continue;
        }
        if value.len() > MAX_KEY_VALUE_LENGTH {
            notes.warn(format!("Skipped variable “{key}”: the value is too long"));
            continue;
        }
        let input = VariableInput {
            id: None,
            key: key.to_owned(),
            value: Some(value),
            is_secret,
            enabled,
            restricted: None,
        };
        if let Some(&existing) = by_key.get(key) {
            if let Some(slot) = variables.get_mut(existing) {
                *slot = input;
            }
        } else if variables.len() < MAX_VARIABLES {
            by_key.insert(key.to_owned(), variables.len());
            variables.push(input);
        } else {
            notes.warn(format!(
                "Only the first {MAX_VARIABLES} variables were imported"
            ));
            break;
        }
    }
    variables
}

fn pairs(list: &[Pair]) -> Vec<KeyValue> {
    list.iter()
        .filter_map(|pair| {
            let key = pair.key.as_deref()?.trim();
            (!key.is_empty()).then(|| KeyValue {
                enabled: !pair.disabled,
                key: key.to_owned(),
                value: value_text(&pair.value),
            })
        })
        .collect()
}

/// Auth types that become Codestamp auth as they are; the others turn into headers or params.
fn typed_auth(auth: &Auth) -> Option<RequestAuth> {
    match auth.kind.as_str() {
        "digest" => Some(RequestAuth::Digest {
            username: param(&auth.digest, "username"),
            password: param(&auth.digest, "password"),
        }),
        "awsv4" => Some(RequestAuth::AwsSigV4 {
            access_key: param(&auth.awsv4, "accessKey"),
            secret_key: param(&auth.awsv4, "secretKey"),
            session_token: param(&auth.awsv4, "sessionToken"),
            region: param(&auth.awsv4, "region"),
            service: param(&auth.awsv4, "service"),
        }),
        "oauth2" => {
            let get = |key: &str| param(&auth.oauth2, key);
            let grant_type = get("grant_type");
            let grant = match grant_type.as_str() {
                "client_credentials" => OAuthGrant::ClientCredentials,
                "password_credentials" => OAuthGrant::Password,
                _ => OAuthGrant::AuthorizationCode,
            };
            Some(RequestAuth::OAuth2(Box::new(OAuth2Settings {
                grant,
                auth_url: get("authUrl"),
                token_url: get("accessTokenUrl"),
                client_id: get("clientId"),
                client_secret: get("clientSecret"),
                scope: get("scope"),
                redirect_uri: get("redirect_uri"),
                use_pkce: grant_type == "authorization_code_with_pkce",
                username: get("username"),
                password: get("password"),
                client_auth: if get("client_authentication") == "body" {
                    ClientAuth::Body
                } else {
                    ClientAuth::Header
                },
                header_prefix: get("headerPrefix"),
            })))
        }
        _ => None,
    }
}

fn param(params: &[AuthParam], key: &str) -> String {
    params
        .iter()
        .find(|p| p.key == key)
        .map(|p| value_text(&p.value))
        .unwrap_or_default()
}

fn value_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn exec_text(exec: &Exec) -> String {
    match exec {
        Exec::Lines(lines) => lines.join("\n"),
        Exec::Text(text) => text.clone(),
        Exec::Missing => String::new(),
    }
}

fn enabled(key: &str, value: &str) -> KeyValue {
    KeyValue {
        enabled: true,
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

fn add_missing_header(headers: &mut Vec<KeyValue>, name: &str, value: &str) {
    if !headers.iter().any(|h| h.key.eq_ignore_ascii_case(name)) {
        headers.push(enabled(name, value));
    }
}

pub(super) fn clean_name(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return fallback.to_owned();
    }
    trimmed.chars().take(MAX_NAME_CHARS).collect()
}

/// A description from another tool, cut to what Docs keeps and without stray control characters.
pub(super) fn imported_description(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .take(MAX_DESCRIPTION_CHARS)
        .collect()
}

/// Postman writes a description as text, or as `{ "content": "…", "type": "text/markdown" }`.
fn description_of(value: Option<&Value>) -> String {
    let text = match value {
        Some(Value::String(text)) => text.as_str(),
        Some(Value::Object(object)) => object
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        _ => "",
    };
    imported_description(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::file::plan;

    const COLLECTION: &str = r#"{
      "info": { "name": "Users API", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
      "auth": { "type": "bearer", "bearer": [{ "key": "token", "value": "{{authToken}}", "type": "string" }] },
      "event": [{ "listen": "test", "script": { "exec": ["pm.test('collection-level', () => {});"] } }],
      "variable": [
        { "key": "baseUrl", "value": "https://api.example.com" },
        { "key": "apiSecret", "value": "s3", "type": "secret" },
        { "key": "bad name", "value": "x" }
      ],
      "item": [
        {
          "name": "Login",
          "request": {
            "method": "POST",
            "auth": { "type": "noauth" },
            "header": [{ "key": "Content-Type", "value": "application/json" }],
            "url": { "raw": "{{baseUrl}}/auth/login" },
            "body": { "mode": "raw", "raw": "{\"user\":\"rina\"}", "options": { "raw": { "language": "json" } } }
          },
          "event": [{ "listen": "test", "script": { "exec": ["const body = pm.response.json();", "pm.environment.set('authToken', body.token);"] } }]
        },
        {
          "name": "Users",
          "item": [
            {
              "name": "List users",
              "request": {
                "method": "GET",
                "url": { "raw": "{{baseUrl}}/users?page=1&limit=20", "query": [
                  { "key": "page", "value": "1" }, { "key": "limit", "value": "20", "disabled": true }
                ] }
              }
            },
            {
              "name": "Admin",
              "item": [
                { "name": "Purge", "request": { "method": "PURGE", "url": "{{baseUrl}}/cache" } },
                {
                  "name": "Basic with variables",
                  "request": {
                    "method": "GET",
                    "auth": { "type": "basic", "basic": [{ "key": "username", "value": "{{user}}" }, { "key": "password", "value": "{{pass}}" }] },
                    "url": "{{baseUrl}}/admin?id={{$guid}}&who={{$randomFirstName}}"
                  }
                }
              ]
            },
            {
              "name": "Upload",
              "request": { "method": "POST", "url": "{{baseUrl}}/files", "body": { "mode": "formdata", "formdata": [
                { "key": "title", "value": "Report", "type": "text" },
                { "key": "file", "type": "file", "src": "C:\\Users\\someone\\secret.pdf" }
              ] } },
              "event": [{ "listen": "prerequest", "script": { "exec": "pm.sendRequest('x', () => {});" } }]
            }
          ]
        }
      ]
    }"#;

    fn find<'a>(plan: &'a ImportPlan, name: &str) -> &'a PlannedRequest {
        plan.requests
            .iter()
            .find(|r| r.content.name == name)
            .unwrap_or_else(|| panic!("missing request {name}"))
    }

    fn header<'a>(request: &'a RequestContent, name: &str) -> Option<&'a str> {
        request
            .headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }

    #[test]
    fn converts_a_collection_with_folders_auth_scripts_and_variables() {
        let plan = plan(COLLECTION).expect("plan");
        assert_eq!(plan.kind, ImportKind::Collection);
        assert_eq!(plan.name, "Users API");
        assert_eq!(plan.section_names(), ["Users"]);
        assert_eq!(plan.requests.len(), 4, "PURGE is skipped");

        let login = find(&plan, "Login");
        assert_eq!(login.section, None);
        assert_eq!(login.content.body_mode, "json");
        assert_eq!(header(&login.content, "Authorization"), None, "noauth wins");
        assert!(
            login
                .content
                .test_script
                .starts_with("pm.test('collection-level'")
        );
        assert!(
            login
                .content
                .test_script
                .contains("pm.environment.set('authToken'")
        );

        let list = find(&plan, "List users");
        assert_eq!(list.section, Some(0));
        assert_eq!(list.content.url, "{{baseUrl}}/users");
        assert_eq!(list.content.params.len(), 2);
        assert!(
            !list
                .content
                .params
                .iter()
                .any(|p| p.key == "limit" && p.enabled)
        );
        assert_eq!(
            header(&list.content, "Authorization"),
            Some("Bearer {{authToken}}")
        );

        let basic = find(&plan, "Admin / Basic with variables");
        assert!(
            basic
                .content
                .pre_request_script
                .contains("btoa(basicUser + ':' + basicPassword)")
        );

        let upload = find(&plan, "Upload");
        assert_eq!(upload.content.body_mode, "formdata");
        let fields: Vec<_> = upload
            .content
            .form
            .iter()
            .map(|f| (f.key.as_str(), f.value.as_str(), f.kind))
            .collect();
        assert_eq!(
            fields,
            [
                ("title", "Report", FormFieldKind::Text),
                ("file", "", FormFieldKind::File)
            ],
            "file paths from the export are never kept"
        );

        let keys: Vec<_> = plan
            .variables
            .iter()
            .map(|v| (v.key.as_str(), v.is_secret))
            .collect();
        assert_eq!(keys, [("baseUrl", false), ("apiSecret", true)]);

        let warnings = plan.warnings.join("\n");
        assert!(warnings.contains("PURGE"));
        assert!(warnings.contains("choose the files again"));
        assert!(warnings.contains("bad name"));
        assert!(
            !warnings.contains("pm.sendRequest"),
            "pm.sendRequest is supported now"
        );
        assert!(warnings.contains("{{$randomFirstName}}"));
        assert!(!warnings.contains("{{$guid}}"), "$guid is supported now");
    }

    #[test]
    fn converts_url_encoded_and_binary_bodies() {
        let plan = plan(
            r#"{ "info": { "name": "Forms" }, "item": [
              { "name": "Login", "request": { "method": "POST", "url": "https://x.test/login",
                "body": { "mode": "urlencoded", "urlencoded": [
                  { "key": "user", "value": "rina" }, { "key": "debug", "value": "1", "disabled": true }
                ] } } },
              { "name": "Upload", "request": { "method": "PUT", "url": "https://x.test/blob",
                "body": { "mode": "file", "file": { "src": "/home/someone/data.bin" } } } }
            ] }"#,
        )
        .expect("plan");
        let login = find(&plan, "Login");
        assert_eq!(login.content.body_mode, "urlencoded");
        assert_eq!(login.content.form.len(), 2);
        assert!(
            login
                .content
                .form
                .iter()
                .any(|f| f.key == "debug" && !f.enabled)
        );
        let upload = find(&plan, "Upload");
        assert_eq!(upload.content.body_mode, "binary");
        assert!(upload.content.body_file.is_empty());
    }

    #[test]
    fn converts_an_environment_export() {
        let plan = plan(
            r#"{ "name": "Staging", "values": [
            { "key": "baseUrl", "value": "https://staging.test", "enabled": true },
            { "key": "token", "value": "t", "type": "secret", "enabled": false }
        ], "_postman_variable_scope": "environment" }"#,
        )
        .expect("plan");
        assert_eq!(plan.kind, ImportKind::Environment);
        assert_eq!(plan.name, "Staging");
        assert_eq!(plan.variables.len(), 2);
        assert!(
            plan.variables
                .iter()
                .any(|v| v.key == "token" && v.is_secret && !v.enabled)
        );
    }

    #[test]
    fn rejects_files_that_are_not_postman_exports() {
        assert!(plan("not json").is_err());
        assert!(plan(r#"{ "hello": "world" }"#).is_err());
        assert!(plan(r#"{ "requests": [] }"#).is_err_and(|e| e.to_string().contains("v1")));
    }

    #[test]
    fn urlencoded_and_graphql_bodies_are_converted() {
        let plan = plan(r#"{ "info": { "name": "Bodies" }, "item": [
            { "name": "Form", "request": { "method": "POST", "url": "https://a.test", "body": { "mode": "urlencoded", "urlencoded": [
                { "key": "a", "value": "1 2" }, { "key": "skip", "value": "x", "disabled": true } ] } } },
            { "name": "Query", "request": { "method": "POST", "url": "https://a.test/graphql", "body": { "mode": "graphql",
                "graphql": { "query": "{ me { id } }", "variables": "{\"x\":1}" } } } }
        ] }"#)
        .expect("plan");
        let form = find(&plan, "Form");
        assert_eq!(form.content.body_mode, "urlencoded");
        let rows: Vec<_> = form
            .content
            .form
            .iter()
            .map(|f| (f.key.as_str(), f.value.as_str(), f.enabled))
            .collect();
        assert_eq!(rows, [("a", "1 2", true), ("skip", "x", false)]);
        let query = find(&plan, "Query");
        assert_eq!(query.content.kind, RequestKind::Graphql);
        assert_eq!(query.content.body, "{ me { id } }");
        assert_eq!(query.content.graphql_variables, "{\"x\":1}");
    }
}
