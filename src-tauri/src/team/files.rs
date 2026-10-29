use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::access::{self, AccessRequest, GroupFile, SealedValue};
use super::groups;
use crate::auth::apply::check_auth;
use crate::auth::models::Auth;
use crate::environment::repo as environments;
use crate::environment::resolve::is_valid_name;
use crate::error::AppError;
use crate::legacy;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::request::types::NameValue;
use crate::storage::descriptions::{self, Described, clean_description};
use crate::storage::examples::{self, Example, MAX_EXAMPLES_PER_REQUEST, check_example};
use crate::storage::icons;
use crate::storage::models::{
    FormField, FormFieldKind, GrpcSettings, KeyValue, McpOperation, PathVariable, RequestContent,
    RequestKind, SavedRequest, SoapSettings,
};
use crate::storage::repo as collections;
use crate::storage::validate::{check_id, check_request, clean_name};

/// Bumped when older versions would misread the layout or drop data from it.
/// 2: `collection.json` may hold the icon someone chose.
/// 3: requests may be MCP requests with the tool they call.
/// 4: requests may hold values for `:name` path variables, and be GraphQL requests.
/// 5: requests may have XML or HTML bodies, and be SOAP requests.
/// 6: the collection, its sections and its requests may have a Docs description.
/// 7: environments that belong to the collection, one file each in `environments/`.
/// 8: access groups in `access/`, and variables encrypted for them. Only written once a
///    collection has a group, so older versions keep opening the others.
pub const FORMAT: u32 = 8;
const FORMAT_WITHOUT_ACCESS: u32 = 7;
pub const MANIFEST: &str = "codestamp.json";
/// The `codestamp` field of a file that holds one exported collection.
pub const BUNDLE_KIND: &str = "collection";
pub const COLLECTION: &str = "collection.json";
const REQUESTS_DIR: &str = "requests";
const REQUEST_PREFIX: &str = "requests/";
const ENVIRONMENTS_DIR: &str = "environments";
const ENVIRONMENT_PREFIX: &str = "environments/";
const ACCESS_DIR: &str = "access";
const ACCESS_REQUESTS_DIR: &str = "access/requests";
const JSON_SUFFIX: &str = ".json";
const ATTRIBUTES: &str = ".gitattributes";
/// Keeps line endings identical on every member's machine, so files only differ in content.
const ATTRIBUTES_TEXT: &str = "*.json text eol=lf\n";
const APP: &str = "Codestamp";
const MAX_REQUESTS: usize = 2_000;
const MAX_VARIABLES: usize = 200;
const MAX_ENVIRONMENTS: usize = 50;
/// A request body may be up to 10 MB; the rest is room for the other fields.
const MAX_FILE_BYTES: u64 = 12 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub app: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionFile {
    pub id: String,
    pub name: String,
    #[serde(default = "no_auth")]
    pub auth: Auth,
    /// The emoji someone chose; left out while the icon follows the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Docs, in Markdown.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default)]
    pub sections: Vec<SectionEntry>,
    /// Request ids in notebook order.
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default)]
    pub variables: Vec<VariableEntry>,
}

const fn no_auth() -> Auth {
    Auth::None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionEntry {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A collection variable. A secret's value is never written; each member keeps their own. A
/// restricted one travels encrypted for its access group instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableEntry {
    pub key: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub secret: bool,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restricted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed: Option<SealedValue>,
}

const fn enabled_by_default() -> bool {
    true
}

/// An environment that belongs to the collection. Secret values are left out like the
/// collection's own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentFile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub variables: Vec<VariableEntry>,
}

/// One request per file. Local file paths (uploads, `.proto` files) are never written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestFile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: RequestKind,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub section_id: Option<String>,
    #[serde(default)]
    pub params: Vec<KeyValue>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default = "no_body")]
    pub body_mode: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub form: Vec<FormField>,
    #[serde(default)]
    pub pre_request_script: String,
    #[serde(default)]
    pub test_script: String,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_operation: Option<McpOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_variables: Vec<PathVariable>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub graphql_variables: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grpc: Option<GrpcSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soap: Option<SoapSettings>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<ExampleFile>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A saved example as shared: everything but the request it belongs to, which is the file's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExampleFile {
    pub id: String,
    pub name: String,
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<NameValue>,
    #[serde(default)]
    pub body: String,
}

impl ExampleFile {
    fn from_example(example: Example) -> Self {
        Self {
            id: example.id,
            name: example.name,
            status: example.status,
            headers: example.headers,
            body: example.body,
        }
    }

    pub fn to_example(&self, request_id: &str) -> Example {
        Example {
            id: self.id.clone(),
            request_id: request_id.to_owned(),
            name: self.name.clone(),
            status: self.status,
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
}

fn no_body() -> String {
    "none".to_owned()
}

impl RequestFile {
    pub fn from_saved(saved: &SavedRequest) -> Self {
        let content = &saved.content;
        Self {
            id: saved.id.clone(),
            name: content.name.clone(),
            kind: content.kind,
            method: content.method.clone(),
            url: content.url.clone(),
            section_id: saved.section_id.clone(),
            params: content.params.clone(),
            headers: content.headers.clone(),
            body_mode: content.body_mode.clone(),
            body: content.body.clone(),
            form: content.form.iter().map(shared_field).collect(),
            pre_request_script: content.pre_request_script.clone(),
            test_script: content.test_script.clone(),
            auth: content.auth.clone(),
            mcp_operation: content.mcp_operation.clone(),
            path_variables: content.path_variables.clone(),
            graphql_variables: content.graphql_variables.clone(),
            grpc: content.grpc.as_ref().map(|grpc| GrpcSettings {
                proto_file: String::new(),
                ..grpc.clone()
            }),
            soap: content.soap.as_ref().map(|soap| SoapSettings {
                wsdl: if is_web_address(&soap.wsdl) {
                    soap.wsdl.clone()
                } else {
                    String::new()
                },
                ..soap.clone()
            }),
            examples: Vec::new(),
            description: String::new(),
        }
    }

    /// The content to store, keeping the files this device picked for upload rows.
    pub fn to_content(&self, local: Option<&RequestContent>) -> RequestContent {
        let mut local_files: Vec<&FormField> = local
            .map(|content| {
                content
                    .form
                    .iter()
                    .filter(|field| field.kind == FormFieldKind::File && !field.value.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let form = self
            .form
            .iter()
            .map(|field| {
                let mut field = field.clone();
                if field.kind == FormFieldKind::File
                    && let Some(index) = local_files.iter().position(|own| own.key == field.key)
                {
                    field.value = local_files.remove(index).value.clone();
                }
                field
            })
            .collect();
        RequestContent {
            kind: self.kind,
            name: self.name.clone(),
            method: self.method.clone(),
            url: self.url.clone(),
            params: self.params.clone(),
            headers: self.headers.clone(),
            body_mode: self.body_mode.clone(),
            body: self.body.clone(),
            form,
            body_file: local
                .map(|content| content.body_file.clone())
                .unwrap_or_default(),
            pre_request_script: self.pre_request_script.clone(),
            test_script: self.test_script.clone(),
            auth: self.auth.clone(),
            mcp_operation: self.mcp_operation.clone(),
            path_variables: self.path_variables.clone(),
            graphql_variables: self.graphql_variables.clone(),
            grpc: self.grpc.as_ref().map(|grpc| GrpcSettings {
                proto_file: local
                    .and_then(|content| content.grpc.as_ref())
                    .map(|own| own.proto_file.clone())
                    .unwrap_or_default(),
                ..grpc.clone()
            }),
            soap: self.soap.as_ref().map(|soap| SoapSettings {
                // A WSDL file picked on this device stays; a shared WSDL URL is used as it is.
                wsdl: if soap.wsdl.is_empty() {
                    local
                        .and_then(|content| content.soap.as_ref())
                        .map(|own| own.wsdl.clone())
                        .unwrap_or_default()
                } else {
                    soap.wsdl.clone()
                },
                ..soap.clone()
            }),
        }
    }
}

/// Only web addresses are shared; anything else is a path on this device.
fn is_web_address(location: &str) -> bool {
    let lower = location.trim().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

/// An upload row is shared by name only; its path stays on this device.
fn shared_field(field: &FormField) -> FormField {
    FormField {
        value: if field.kind == FormFieldKind::File {
            String::new()
        } else {
            field.value.clone()
        },
        ..field.clone()
    }
}

/// A whole collection as it is written to (or read from) the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub collection: CollectionFile,
    pub requests: BTreeMap<String, RequestFile>,
    pub environments: BTreeMap<String, EnvironmentFile>,
    /// Access groups by id.
    pub groups: BTreeMap<String, GroupFile>,
    /// Requests for access by path.
    pub access_requests: BTreeMap<String, AccessRequest>,
}

pub fn request_path(id: &str) -> String {
    format!("{REQUEST_PREFIX}{id}{JSON_SUFFIX}")
}

/// The request id of a `requests/<id>.json` path.
pub fn request_id_of(path: &str) -> Option<&str> {
    path.strip_prefix(REQUEST_PREFIX)?.strip_suffix(JSON_SUFFIX)
}

pub fn environment_path(id: &str) -> String {
    format!("{ENVIRONMENT_PREFIX}{id}{JSON_SUFFIX}")
}

/// The environment id of an `environments/<id>.json` path.
pub fn environment_id_of(path: &str) -> Option<&str> {
    path.strip_prefix(ENVIRONMENT_PREFIX)?
        .strip_suffix(JSON_SUFFIX)
}

pub fn from_db(conn: &Connection, collection_id: &str) -> Result<Snapshot, AppError> {
    let detail = collections::collection_detail(conn, collection_id)?;
    let mut requests = BTreeMap::new();
    for summary in &detail.requests {
        let saved = collections::load_request(conn, &summary.id)?;
        let mut file = RequestFile::from_saved(&saved);
        file.examples = examples::all_for(conn, &saved.id)?
            .into_iter()
            .map(ExampleFile::from_example)
            .collect();
        file.description = descriptions::get(conn, Described::Request, &saved.id)?;
        requests.insert(saved.id.clone(), file);
    }
    let variables = match environments::find_collection_set(conn, collection_id)? {
        Some(set) => shared_variables(conn, &set)?,
        None => Vec::new(),
    };
    let mut owned = BTreeMap::new();
    for (id, name) in environments::owned(conn, collection_id)? {
        let variables = shared_variables(conn, &id)?;
        owned.insert(
            id.clone(),
            EnvironmentFile {
                id,
                name,
                variables,
            },
        );
    }
    let sections = detail
        .sections
        .into_iter()
        .map(|section| {
            Ok(SectionEntry {
                description: descriptions::get(conn, Described::Section, &section.id)?,
                id: section.id,
                name: section.name,
            })
        })
        .collect::<Result<_, AppError>>()?;
    let (groups, access_requests) = groups::stored_files(conn, collection_id)?;
    Ok(Snapshot {
        collection: CollectionFile {
            icon: detail.icon_custom.then_some(detail.icon),
            description: descriptions::get(conn, Described::Collection, collection_id)?,
            id: detail.id,
            name: detail.name,
            auth: detail.auth,
            sections,
            order: detail
                .requests
                .into_iter()
                .map(|request| request.id)
                .collect(),
            variables,
        },
        requests,
        environments: owned,
        groups,
        access_requests,
    })
}

/// A variable set as it is shared: secrets arrive with an empty value, only their name travels.
/// Restricted values carry the ciphertext the last seal or sync stored.
fn shared_variables(conn: &Connection, set_id: &str) -> Result<Vec<VariableEntry>, AppError> {
    let mut sealed = groups::sealed_values(conn, set_id)?;
    Ok(environments::variables(conn, set_id)?
        .into_iter()
        .map(|variable| VariableEntry {
            sealed: variable
                .restricted
                .as_ref()
                .and_then(|_| sealed.remove(&variable.id)),
            key: variable.key,
            value: if variable.is_secret {
                String::new()
            } else {
                variable.value
            },
            secret: variable.is_secret,
            enabled: variable.enabled,
            restricted: variable.restricted,
        })
        .collect())
}

/// Every file of the snapshot as text, by path relative to the repository root.
pub fn render(snapshot: &Snapshot) -> Result<BTreeMap<String, String>, AppError> {
    let mut files = BTreeMap::new();
    files.insert(
        MANIFEST.to_owned(),
        to_text(&Manifest {
            format: if snapshot.groups.is_empty() {
                FORMAT_WITHOUT_ACCESS
            } else {
                FORMAT
            },
            app: APP.to_owned(),
        })?,
    );
    files.insert(COLLECTION.to_owned(), to_text(&snapshot.collection)?);
    for (id, request) in &snapshot.requests {
        files.insert(request_path(id), to_text(request)?);
    }
    for (id, environment) in &snapshot.environments {
        files.insert(environment_path(id), to_text(environment)?);
    }
    for (id, group) in &snapshot.groups {
        files.insert(access::group_path(id), to_text(group)?);
    }
    for (path, request) in &snapshot.access_requests {
        files.insert(path.clone(), to_text(request)?);
    }
    Ok(files)
}

/// The collection as one file for another Codestamp: `{"codestamp": "collection", "files": {..}}`
/// with the repository's files under their paths. Access groups stay behind, so a restricted
/// value leaves like a secret: its name travels, its value does not.
pub fn bundle(mut snapshot: Snapshot) -> Result<Value, AppError> {
    for variable in snapshot.collection.variables.iter_mut().chain(
        snapshot
            .environments
            .values_mut()
            .flat_map(|environment| environment.variables.iter_mut()),
    ) {
        if variable.restricted.take().is_some() {
            variable.sealed = None;
            variable.value.clear();
            variable.secret = true;
        }
    }
    let mut files = Map::new();
    files.insert(
        MANIFEST.to_owned(),
        to_json(&Manifest {
            format: FORMAT_WITHOUT_ACCESS,
            app: APP.to_owned(),
        })?,
    );
    files.insert(COLLECTION.to_owned(), to_json(&snapshot.collection)?);
    for (id, request) in &snapshot.requests {
        files.insert(request_path(id), to_json(request)?);
    }
    for (id, environment) in &snapshot.environments {
        files.insert(environment_path(id), to_json(environment)?);
    }
    Ok(json!({ "codestamp": BUNDLE_KIND, "files": files }))
}

/// Reads the `files` of an exported collection. It came from outside, so it is checked like a
/// working tree: known paths only, limits, ids, and the same rules as saved requests.
pub fn read_bundle(files: &Map<String, Value>) -> Result<Snapshot, AppError> {
    let manifest: Manifest = bundle_entry(files, MANIFEST)?;
    if manifest.format > FORMAT {
        return Err(AppError::invalid(
            "This collection was exported by a newer Codestamp. Update Codestamp to import it",
        ));
    }
    let collection = check_collection(bundle_entry(files, COLLECTION)?)
        .map_err(|_| invalid_entry(COLLECTION))?;
    let mut requests = BTreeMap::new();
    let mut environments = BTreeMap::new();
    for (path, value) in files {
        if path == MANIFEST || path == COLLECTION {
            continue;
        }
        if request_id_of(path).is_some() {
            if requests.len() == MAX_REQUESTS {
                return Err(AppError::invalid(
                    "The collection has more than 2000 requests",
                ));
            }
            let request = check_request_file(path, entry_value(path, value)?, &collection)
                .map_err(|_| invalid_entry(path))?;
            requests.insert(request.id.clone(), request);
        } else if environment_id_of(path).is_some() {
            if environments.len() == MAX_ENVIRONMENTS {
                return Err(AppError::invalid(
                    "The collection has more than 50 environments",
                ));
            }
            let environment = check_environment_file(path, entry_value(path, value)?)
                .map_err(|_| invalid_entry(path))?;
            environments.insert(environment.id.clone(), environment);
        } else {
            return Err(invalid_entry(path));
        }
    }
    Ok(Snapshot {
        collection,
        requests,
        environments,
        groups: BTreeMap::new(),
        access_requests: BTreeMap::new(),
    })
}

fn bundle_entry<T: DeserializeOwned>(
    files: &Map<String, Value>,
    path: &str,
) -> Result<T, AppError> {
    let value = files.get(path).ok_or_else(|| invalid_entry(path))?;
    entry_value(path, value)
}

fn entry_value<T: DeserializeOwned>(path: &str, value: &Value) -> Result<T, AppError> {
    T::deserialize(value).map_err(|_| invalid_entry(path))
}

fn invalid_entry(path: &str) -> AppError {
    AppError::InvalidInput(format!(
        "The Codestamp file has an entry Codestamp cannot use: {path}"
    ))
}

/// Writes the snapshot into the working tree: only files whose text changed are rewritten, and
/// request files that no longer exist are removed, as is a manifest still under its pre-rename
/// name. Other files (a README, say) are left alone.
pub fn write(dir: &Path, snapshot: &Snapshot) -> Result<(), AppError> {
    let files = render(snapshot)?;
    fs::create_dir_all(dir.join(REQUESTS_DIR)).map_err(|_| AppError::FileAccess)?;
    if !snapshot.environments.is_empty() {
        fs::create_dir_all(dir.join(ENVIRONMENTS_DIR)).map_err(|_| AppError::FileAccess)?;
    }
    if !snapshot.groups.is_empty() {
        fs::create_dir_all(dir.join(ACCESS_DIR)).map_err(|_| AppError::FileAccess)?;
    }
    if !snapshot.access_requests.is_empty() {
        fs::create_dir_all(dir.join(ACCESS_REQUESTS_DIR)).map_err(|_| AppError::FileAccess)?;
    }
    let attributes = dir.join(ATTRIBUTES);
    if !attributes.exists() {
        fs::write(&attributes, ATTRIBUTES_TEXT).map_err(|_| AppError::FileAccess)?;
    }
    for (path, text) in &files {
        write_text(dir, path, text)?;
    }
    let legacy_manifest = dir.join(legacy::TEAM_MANIFEST);
    if legacy_manifest.exists() {
        fs::remove_file(legacy_manifest).map_err(|_| AppError::FileAccess)?;
    }
    for path in request_files(dir)?
        .into_iter()
        .chain(environment_files(dir)?)
        .chain(group_files(dir)?)
        .chain(access_request_files(dir)?)
    {
        if !files.contains_key(&path) {
            fs::remove_file(dir.join(&path)).map_err(|_| AppError::FileAccess)?;
        }
    }
    Ok(())
}

pub fn write_text(dir: &Path, path: &str, text: &str) -> Result<(), AppError> {
    let target = dir.join(path);
    if fs::read_to_string(&target).is_ok_and(|current| current == text) {
        return Ok(());
    }
    fs::write(&target, text).map_err(|_| AppError::FileAccess)
}

/// Reads and checks a working tree. Everything in it came from other people, so it is validated
/// like an import: known paths only, size limits, ids, and the same rules as saved requests.
pub fn read(dir: &Path) -> Result<Snapshot, AppError> {
    // A repository shared before the rename has the old manifest until the next write renames it.
    let manifest_path = if dir.join(MANIFEST).exists() {
        MANIFEST
    } else if dir.join(legacy::TEAM_MANIFEST).exists() {
        legacy::TEAM_MANIFEST
    } else {
        return Err(AppError::invalid(
            "This repository was not shared from Codestamp, so it cannot be joined. To put a collection in it, open the collection and share it from Collaboration",
        ));
    };
    let manifest: Manifest = read_json(dir, manifest_path)?;
    if manifest.format > FORMAT {
        return Err(AppError::invalid(
            "This shared collection was saved by a newer Codestamp. Update Codestamp to open it",
        ));
    }
    let collection = check_collection(read_json(dir, COLLECTION)?)?;
    let paths = request_files(dir)?;
    if paths.len() > MAX_REQUESTS {
        return Err(AppError::invalid(
            "The shared collection has more than 2000 requests",
        ));
    }
    let mut requests = BTreeMap::new();
    for path in paths {
        let request = check_request_file(&path, read_json(dir, &path)?, &collection)?;
        requests.insert(request.id.clone(), request);
    }
    let paths = environment_files(dir)?;
    if paths.len() > MAX_ENVIRONMENTS {
        return Err(AppError::invalid(
            "The shared collection has more than 50 environments",
        ));
    }
    let mut environments = BTreeMap::new();
    for path in paths {
        let environment = check_environment_file(&path, read_json(dir, &path)?)?;
        environments.insert(environment.id.clone(), environment);
    }
    let paths = group_files(dir)?;
    if paths.len() > access::MAX_GROUPS {
        return Err(AppError::invalid(
            "The shared collection has more than 20 access groups",
        ));
    }
    let mut groups = BTreeMap::new();
    for path in paths {
        let group = access::check_group(&path, read_json(dir, &path)?)?;
        groups.insert(group.id.clone(), group);
    }
    let paths = access_request_files(dir)?;
    if paths.len() > access::MAX_REQUESTS {
        return Err(AppError::invalid(
            "The shared collection has more than 200 access requests",
        ));
    }
    let mut access_requests = BTreeMap::new();
    for path in paths {
        let request = access::check_request(&path, read_json(dir, &path)?)?;
        access_requests.insert(path, request);
    }
    Ok(Snapshot {
        collection,
        requests,
        environments,
        groups,
        access_requests,
    })
}

/// Checks a merged file and returns its canonical text.
pub fn normalize(
    path: &str,
    value: &Value,
    collection: &CollectionFile,
) -> Result<String, AppError> {
    let invalid = || invalid_file(path);
    if path == MANIFEST {
        let manifest: Manifest = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
        return to_text(&manifest);
    }
    if path == COLLECTION {
        let parsed: CollectionFile =
            serde_json::from_value(value.clone()).map_err(|_| invalid())?;
        return to_text(&check_collection(parsed)?);
    }
    if environment_id_of(path).is_some() {
        let parsed: EnvironmentFile =
            serde_json::from_value(value.clone()).map_err(|_| invalid())?;
        return to_text(&check_environment_file(path, parsed)?);
    }
    if access::group_id_of(path).is_some() {
        return to_text(&group_from_value(path, value)?);
    }
    if access::request_of(path).is_some() {
        let parsed: AccessRequest = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
        return to_text(&access::check_request(path, parsed)?);
    }
    let parsed: RequestFile = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    to_text(&check_request_file(path, parsed, collection)?)
}

/// A checked group file at some version.
pub fn group_from_value(path: &str, value: &Value) -> Result<GroupFile, AppError> {
    let parsed: GroupFile =
        serde_json::from_value(value.clone()).map_err(|_| invalid_file(path))?;
    access::check_group(path, parsed)
}

/// The checked collection file from a (merged) value.
pub fn collection_from_value(value: &Value) -> Result<CollectionFile, AppError> {
    let parsed: CollectionFile =
        serde_json::from_value(value.clone()).map_err(|_| invalid_file(COLLECTION))?;
    check_collection(parsed)
}

/// A request file at some version, checked like any other file from the repository.
pub fn request_from_text(
    path: &str,
    text: &str,
    collection: &CollectionFile,
) -> Result<RequestFile, AppError> {
    let parsed: RequestFile =
        serde_json::from_value(parse_text(path, text)?).map_err(|_| invalid_file(path))?;
    check_request_file(path, parsed, collection)
}

pub fn parse_text(path: &str, text: &str) -> Result<Value, AppError> {
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(invalid_file(path));
    }
    serde_json::from_str(text).map_err(|_| invalid_file(path))
}

pub fn is_known_path(path: &str) -> bool {
    path == MANIFEST
        || path == COLLECTION
        || request_id_of(path).is_some_and(|id| check_id(id).is_ok())
        || environment_id_of(path).is_some_and(|id| check_id(id).is_ok())
        || access::group_id_of(path).is_some()
        || access::request_of(path).is_some()
}

fn check_collection(collection: CollectionFile) -> Result<CollectionFile, AppError> {
    let invalid = || invalid_file(COLLECTION);
    check_id(&collection.id).map_err(|_| invalid())?;
    let name = clean_name(&collection.name).map_err(|_| invalid())?;
    check_auth(&collection.auth, false).map_err(|_| invalid())?;
    let icon = collection
        .icon
        .as_deref()
        .map(icons::check_icon)
        .transpose()
        .map_err(|_| invalid())?
        .map(str::to_owned);
    let mut sections = Vec::with_capacity(collection.sections.len());
    for section in collection.sections {
        check_id(&section.id).map_err(|_| invalid())?;
        sections.push(SectionEntry {
            name: clean_name(&section.name).map_err(|_| invalid())?,
            description: clean_description(&section.description).map_err(|_| invalid())?,
            id: section.id,
        });
    }
    if collection.order.len() > MAX_REQUESTS
        || collection.order.iter().any(|id| check_id(id).is_err())
    {
        return Err(invalid());
    }
    if !variables_fit(&collection.variables) {
        return Err(invalid());
    }
    Ok(CollectionFile {
        description: clean_description(&collection.description).map_err(|_| invalid())?,
        name,
        icon,
        sections,
        ..collection
    })
}

fn variables_fit(variables: &[VariableEntry]) -> bool {
    variables.len() <= MAX_VARIABLES
        && variables.iter().all(|variable| {
            let restriction_fits = match &variable.restricted {
                Some(group) => {
                    check_id(group).is_ok()
                        && variable.secret
                        && variable.value.is_empty()
                        && variable.sealed.as_ref().is_none_or(access::sealed_fits)
                }
                None => variable.sealed.is_none(),
            };
            is_valid_name(&variable.key)
                && variable.value.len() <= MAX_KEY_VALUE_LENGTH
                && restriction_fits
        })
}

/// An environment file from the repository: its id must match the file name.
fn check_environment_file(
    path: &str,
    environment: EnvironmentFile,
) -> Result<EnvironmentFile, AppError> {
    let invalid = || invalid_file(path);
    check_id(&environment.id).map_err(|_| invalid())?;
    if environment_id_of(path) != Some(environment.id.as_str())
        || !variables_fit(&environment.variables)
    {
        return Err(invalid());
    }
    Ok(EnvironmentFile {
        name: clean_name(&environment.name).map_err(|_| invalid())?,
        ..environment
    })
}

fn check_request_file(
    path: &str,
    request: RequestFile,
    collection: &CollectionFile,
) -> Result<RequestFile, AppError> {
    let invalid = || invalid_file(path);
    if request_id_of(path) != Some(request.id.as_str()) {
        return Err(invalid());
    }
    check_id(&request.id).map_err(|_| invalid())?;
    let checked = check_request(request.to_content(None)).map_err(|_| invalid())?;
    if request.examples.len() > MAX_EXAMPLES_PER_REQUEST {
        return Err(invalid());
    }
    let mut examples = Vec::with_capacity(request.examples.len());
    for example in &request.examples {
        check_id(&example.id).map_err(|_| invalid())?;
        let checked =
            check_example(example.to_example(&request.id).into_input()).map_err(|_| invalid())?;
        examples.push(ExampleFile {
            name: checked.name,
            ..example.clone()
        });
    }
    // A section that is gone (or never existed) just leaves the request outside sections.
    let section_id = request
        .section_id
        .filter(|id| collection.sections.iter().any(|section| &section.id == id));
    Ok(RequestFile {
        name: checked.name,
        section_id,
        form: request.form.iter().map(shared_field).collect(),
        examples,
        description: clean_description(&request.description).map_err(|_| invalid())?,
        ..request
    })
}

/// `requests/<uuid>.json` files that are regular files (links and folders are skipped).
pub(crate) fn request_files(dir: &Path) -> Result<Vec<String>, AppError> {
    json_files(dir, REQUESTS_DIR, REQUEST_PREFIX)
}

/// `environments/<uuid>.json` files, read the same way as request files.
pub(crate) fn environment_files(dir: &Path) -> Result<Vec<String>, AppError> {
    json_files(dir, ENVIRONMENTS_DIR, ENVIRONMENT_PREFIX)
}

/// `access/<uuid>.json` group files.
fn group_files(dir: &Path) -> Result<Vec<String>, AppError> {
    json_files(dir, ACCESS_DIR, access::GROUP_PREFIX)
}

/// `access/requests/<uuid>.<fingerprint>.json` request files.
fn access_request_files(dir: &Path) -> Result<Vec<String>, AppError> {
    json_files(dir, ACCESS_REQUESTS_DIR, access::REQUEST_PREFIX)
}

fn json_files(dir: &Path, folder: &str, prefix: &str) -> Result<Vec<String>, AppError> {
    let folder = dir.join(folder);
    let Ok(entries) = fs::read_dir(&folder) else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| AppError::FileAccess)?;
        let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let path = format!("{prefix}{name}");
        if is_file && is_known_path(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_json<T: DeserializeOwned>(dir: &Path, path: &str) -> Result<T, AppError> {
    let full = dir.join(path);
    let size = fs::metadata(&full).map_err(|_| invalid_file(path))?.len();
    if size > MAX_FILE_BYTES {
        return Err(invalid_file(path));
    }
    let text = fs::read_to_string(&full).map_err(|_| invalid_file(path))?;
    serde_json::from_str(&text).map_err(|_| invalid_file(path))
}

fn to_json<T: Serialize>(value: &T) -> Result<Value, AppError> {
    serde_json::to_value(value).map_err(|_| AppError::Storage)
}

fn to_text<T: Serialize>(value: &T) -> Result<String, AppError> {
    let mut text = serde_json::to_string_pretty(value).map_err(|_| AppError::Storage)?;
    text.push('\n');
    Ok(text)
}

fn invalid_file(path: &str) -> AppError {
    AppError::InvalidInput(format!(
        "The repository has a file Codestamp cannot use: {path}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::{Database, new_id};
    use crate::storage::models::{GrpcSource, KeyValue};

    fn content(name: &str) -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: name.to_owned(),
            method: "POST".to_owned(),
            url: "{{baseUrl}}/upload".to_owned(),
            params: vec![KeyValue {
                enabled: true,
                key: "page".to_owned(),
                value: "1".to_owned(),
            }],
            headers: Vec::new(),
            body_mode: "formdata".to_owned(),
            body: String::new(),
            form: vec![FormField {
                enabled: true,
                key: "doc".to_owned(),
                value: "C:\\Users\\rina\\q3.pdf".to_owned(),
                kind: FormFieldKind::File,
            }],
            body_file: "C:\\Users\\rina\\raw.bin".to_owned(),
            pre_request_script: String::new(),
            test_script: "pm.test('ok', () => {});".to_owned(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    #[test]
    fn writes_one_file_per_request_without_local_paths_and_reads_it_back() {
        let db = Database::in_memory().expect("database");
        let snapshot = db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Users API")?;
                let section = collections::insert_section(conn, &collection.id, "Uploads")?;
                collections::insert_request(
                    conn,
                    &collection.id,
                    Some(&section.id),
                    content("Upload"),
                )?;
                collections::insert_request(conn, &collection.id, None, content("Health"))?;
                from_db(conn, &collection.id)
            })
            .expect("snapshot");
        let dir = std::env::temp_dir().join(format!("codestamp-team-{}", new_id()));

        write(&dir, &snapshot).expect("write");
        let files = render(&snapshot).expect("render");
        let all_text: String = files.values().cloned().collect();
        assert_eq!(files.len(), 4);
        assert!(!all_text.contains("q3.pdf") && !all_text.contains("raw.bin"));
        assert!(all_text.contains("\"testScript\": \"pm.test('ok', () => {});\""));

        let read_back = read(&dir).expect("read");
        assert_eq!(read_back, snapshot);
        let local = content("Upload");
        let upload = read_back
            .requests
            .values()
            .find(|r| r.name == "Upload")
            .expect("upload");
        let restored = upload.to_content(Some(&local));
        assert_eq!(
            restored.form.first().map(|f| f.value.as_str()),
            Some("C:\\Users\\rina\\q3.pdf")
        );
        assert_eq!(restored.body_file, "C:\\Users\\rina\\raw.bin");
        fs::remove_dir_all(&dir).expect("clean up");
    }

    #[test]
    fn bundles_a_collection_into_one_file_and_reads_it_back() {
        let db = Database::in_memory().expect("database");
        let mut snapshot = db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Users API")?;
                let section = collections::insert_section(conn, &collection.id, "Uploads")?;
                collections::insert_request(
                    conn,
                    &collection.id,
                    Some(&section.id),
                    content("Upload"),
                )?;
                from_db(conn, &collection.id)
            })
            .expect("snapshot");
        snapshot.collection.variables = vec![
            VariableEntry {
                key: "baseUrl".to_owned(),
                value: "https://api.example.com".to_owned(),
                secret: false,
                enabled: true,
                restricted: None,
                sealed: None,
            },
            VariableEntry {
                key: "token".to_owned(),
                value: String::new(),
                secret: true,
                enabled: true,
                restricted: Some(new_id()),
                sealed: None,
            },
        ];

        let value = bundle(snapshot.clone()).expect("bundle");
        let text = value.to_string();
        assert!(!text.contains("q3.pdf") && !text.contains("\"restricted\""));
        let files = value["files"].as_object().expect("files");
        let read_back = read_bundle(files).expect("read");
        assert_eq!(read_back.requests, snapshot.requests);
        assert_eq!(read_back.collection.sections, snapshot.collection.sections);
        let token = &read_back.collection.variables[1];
        assert!(token.secret && token.restricted.is_none() && token.value.is_empty());

        let mut extra = files.clone();
        extra.insert("access/group.json".to_owned(), json!({}));
        assert!(read_bundle(&extra).is_err());
        let mut renamed = files.clone();
        let (path, request) = renamed
            .iter()
            .find(|(path, _)| request_id_of(path).is_some())
            .map(|(path, request)| (path.clone(), request.clone()))
            .expect("request file");
        renamed.remove(&path);
        renamed.insert(request_path(&new_id()), request);
        assert!(read_bundle(&renamed).is_err());
    }

    #[test]
    fn shares_the_grpc_method_but_not_the_proto_path() {
        let local = RequestContent {
            kind: RequestKind::Grpc,
            grpc: Some(GrpcSettings {
                method: "shop.Shop/GetOrder".to_owned(),
                source: GrpcSource::Proto,
                proto_file: "C:\\Users\\rina\\shop.proto".to_owned(),
            }),
            ..content("Orders")
        };
        let shared = RequestFile::from_saved(&SavedRequest {
            id: new_id(),
            collection_id: new_id(),
            section_id: None,
            content: local.clone(),
        });
        let text = serde_json::to_string(&shared).expect("json");
        assert!(text.contains("shop.Shop/GetOrder") && !text.contains("shop.proto"));
        assert_eq!(shared.to_content(Some(&local)).grpc, local.grpc);
        assert_eq!(
            shared.to_content(None).grpc.map(|grpc| grpc.proto_file),
            Some(String::new())
        );
    }

    #[test]
    fn refuses_files_that_do_not_belong() {
        let collection = CollectionFile {
            id: new_id(),
            name: "API".to_owned(),
            auth: Auth::None,
            icon: None,
            description: String::new(),
            sections: Vec::new(),
            order: Vec::new(),
            variables: Vec::new(),
        };
        let id = new_id();
        let mut request = RequestFile::from_saved(&SavedRequest {
            id: id.clone(),
            collection_id: collection.id.clone(),
            section_id: Some(new_id()),
            content: content("X"),
        });
        let path = request_path(&id);
        let ok = normalize(
            &path,
            &serde_json::to_value(&request).expect("json"),
            &collection,
        );
        assert!(
            ok.is_ok_and(|text| text.contains("\"sectionId\": null")),
            "unknown section dropped"
        );

        let other = request_path(&new_id());
        assert!(
            normalize(
                &other,
                &serde_json::to_value(&request).expect("json"),
                &collection
            )
            .is_err()
        );
        request.description = "bell\u{7}".to_owned();
        assert!(
            normalize(
                &path,
                &serde_json::to_value(&request).expect("json"),
                &collection
            )
            .is_err(),
            "a description with control characters is refused"
        );
        request.description = String::new();
        request.method = "TRACE".to_owned();
        assert!(
            normalize(
                &path,
                &serde_json::to_value(&request).expect("json"),
                &collection
            )
            .is_err()
        );
        assert!(!is_known_path("requests/../secret.json"));
        assert!(!is_known_path("requests/not-an-id.json"));
    }

    #[test]
    fn carries_a_chosen_icon_and_refuses_unknown_ones() {
        let db = Database::in_memory().expect("database");
        let (automatic, chosen) = db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Shop API")?;
                let automatic = from_db(conn, &collection.id)?;
                collections::set_collection_icon(conn, &collection.id, Some("🦊"))?;
                Ok((automatic, from_db(conn, &collection.id)?))
            })
            .expect("snapshots");
        let collection_text = |snapshot: &Snapshot| {
            render(snapshot)
                .expect("render")
                .get(COLLECTION)
                .cloned()
                .unwrap_or_default()
        };
        assert!(!collection_text(&automatic).contains("\"icon\""));
        assert!(collection_text(&chosen).contains("\"icon\": \"🦊\""));

        let mut value = serde_json::to_value(&chosen.collection).expect("json");
        value["icon"] = Value::String("<img src=x>".to_owned());
        assert!(collection_from_value(&value).is_err());
        value["icon"] = Value::String("🚀".to_owned());
        assert_eq!(
            collection_from_value(&value)
                .expect("valid")
                .icon
                .as_deref(),
            Some("🚀")
        );
    }

    #[test]
    fn reads_older_formats_and_refuses_newer_ones() {
        let db = Database::in_memory().expect("database");
        let snapshot = db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Users API")?;
                from_db(conn, &collection.id)
            })
            .expect("snapshot");
        let dir = std::env::temp_dir().join(format!("codestamp-team-{}", new_id()));
        write(&dir, &snapshot).expect("write");
        let manifest =
            |format: u32| format!("{{\n  \"format\": {format},\n  \"app\": \"Codestamp\"\n}}\n");

        write_text(&dir, MANIFEST, &manifest(1)).expect("older manifest");
        assert_eq!(read(&dir).expect("format 1 still opens"), snapshot);
        write_text(&dir, MANIFEST, &manifest(FORMAT + 1)).expect("newer manifest");
        assert!(read(&dir).is_err());
        fs::remove_dir_all(&dir).expect("clean up");
    }

    #[test]
    fn opens_a_repository_with_the_old_manifest_and_renames_it() {
        let db = Database::in_memory().expect("database");
        let snapshot = db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Users API")?;
                from_db(conn, &collection.id)
            })
            .expect("snapshot");
        let dir = std::env::temp_dir().join(format!("codestamp-team-{}", new_id()));
        write(&dir, &snapshot).expect("write");
        fs::rename(dir.join(MANIFEST), dir.join(legacy::TEAM_MANIFEST)).expect("old manifest");

        assert_eq!(read(&dir).expect("old manifest opens"), snapshot);
        write(&dir, &snapshot).expect("rewrite");
        assert!(dir.join(MANIFEST).exists());
        assert!(!dir.join(legacy::TEAM_MANIFEST).exists());
        fs::remove_dir_all(&dir).expect("clean up");
    }
}
