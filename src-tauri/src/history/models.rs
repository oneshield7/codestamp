use serde::{Deserialize, Serialize};

use crate::auth::models::Auth;
use crate::request::body_mode::raw_body_mode;
use crate::request::types::{HttpRequestInput, NameValue, RequestBody};
use crate::request::validate::graphql_json;
use crate::storage::models::{FormField, FormFieldKind};

/// Larger request bodies are not copied into history, to keep the database small.
const MAX_HISTORY_BODY_BYTES: usize = 256 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub request_id: Option<String>,
    pub name: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub body_size: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusFilter {
    All,
    Success,
    Redirect,
    ClientError,
    ServerError,
    Failed,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryFilter {
    pub query: Option<String>,
    pub status: StatusFilter,
    /// Only what was sent from this collection; history lives in each collection.
    #[serde(default)]
    pub collection_id: Option<String>,
}

/// The request as the user wrote it (with `{{variables}}`), captured before variables are
/// filled in. Resolved values, including secrets, are therefore never written to history.
#[derive(Debug)]
pub struct SentRequest {
    pub collection_id: Option<String>,
    pub request_id: Option<String>,
    pub method: String,
    pub url: String,
    pub params: Vec<NameValue>,
    pub headers: Vec<NameValue>,
    /// The body mode as stored for a request (`none`, `json`, `text`, `urlencoded`, …).
    pub body_mode: &'static str,
    /// Raw text, form rows as JSON, or the file path, depending on the mode.
    pub body: Option<String>,
    pub body_omitted: bool,
    /// The auth as configured (inheritance already applied), with variables still in place.
    pub auth: Auth,
    pub path_variables: Vec<NameValue>,
}

impl SentRequest {
    pub fn capture(input: &HttpRequestInput) -> Self {
        let (body_mode, stored) = stored_body(input.body.as_ref(), &input.headers);
        let body_omitted = stored
            .as_ref()
            .is_some_and(|body| body.len() > MAX_HISTORY_BODY_BYTES);
        Self {
            collection_id: input.collection_id.clone(),
            request_id: input.request_id.clone(),
            method: input.method.clone(),
            url: input.url.clone(),
            params: input.params.clone(),
            headers: input.headers.clone(),
            body_mode,
            body: if body_omitted { None } else { stored },
            body_omitted,
            auth: input.auth.clone().unwrap_or(Auth::None),
            path_variables: input.path_variables.clone(),
        }
    }
}

/// The body in the shape a saved request keeps it, so it can be reopened from history.
fn stored_body(
    body: Option<&RequestBody>,
    headers: &[NameValue],
) -> (&'static str, Option<String>) {
    let form_json = |fields: Vec<FormField>| serde_json::to_string(&fields).ok();
    match body {
        None => ("none", None),
        // Reopened from history it is a plain JSON request with the body as it was built.
        Some(RequestBody::Graphql { query, variables }) => (
            "json",
            Some(graphql_json(query.clone(), variables).unwrap_or_else(|_| query.clone())),
        ),
        Some(RequestBody::Raw { text }) => {
            let content_type = headers
                .iter()
                .find(|header| header.name.eq_ignore_ascii_case("content-type"))
                .map(|header| header.value.as_str());
            (raw_body_mode(content_type, text), Some(text.clone()))
        }
        Some(RequestBody::Urlencoded { fields }) => (
            "urlencoded",
            form_json(
                fields
                    .iter()
                    .map(|field| FormField {
                        enabled: true,
                        key: field.name.clone(),
                        value: field.value.clone(),
                        kind: FormFieldKind::Text,
                    })
                    .collect(),
            ),
        ),
        Some(RequestBody::FormData { parts }) => (
            "formdata",
            form_json(
                parts
                    .iter()
                    .map(|part| FormField {
                        enabled: true,
                        key: part.name.clone(),
                        value: part.value.clone(),
                        kind: if part.file {
                            FormFieldKind::File
                        } else {
                            FormFieldKind::Text
                        },
                    })
                    .collect(),
            ),
        ),
        Some(RequestBody::Binary { path }) => ("binary", Some(path.clone())),
    }
}
