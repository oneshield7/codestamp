use serde::{Deserialize, Serialize};
use url::form_urlencoded;

use crate::auth::models::Auth;
use crate::cookies::models::ResponseCookie;
use crate::script::models::ScriptReport;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HttpRequestInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub params: Vec<NameValue>,
    #[serde(default)]
    pub headers: Vec<NameValue>,
    pub body: Option<RequestBody>,
    pub timeout_ms: Option<u64>,
    /// Where the request lives, for collection variables and history. Both are optional.
    #[serde(default)]
    pub collection_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub pre_request_script: Option<String>,
    #[serde(default)]
    pub test_script: Option<String>,
    /// `None` sends no auth; `Inherit` takes the collection's auth.
    #[serde(default)]
    pub auth: Option<Auth>,
    /// Collection runs leave history alone so a run does not flood it.
    #[serde(default = "record_by_default")]
    pub record_history: bool,
    /// Values for `:name` path segments, with `{{variables}}` still in place.
    #[serde(default)]
    pub path_variables: Vec<NameValue>,
    /// The current row of a run's data file; it wins over every saved scope.
    #[serde(default)]
    pub iteration_data: Vec<NameValue>,
    /// Which pass of a run this is, for `pm.info.iteration`.
    #[serde(default)]
    pub iteration: Iteration,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Iteration {
    pub index: u32,
    pub count: u32,
}

impl Default for Iteration {
    fn default() -> Self {
        Self { index: 0, count: 1 }
    }
}

const fn record_by_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NameValue {
    pub name: String,
    pub value: String,
}

/// The body as the user built it, with `{{variables}}` still in place. File paths are checked
/// against the files picked in the system dialog before anything is read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RequestBody {
    Raw {
        text: String,
    },
    Urlencoded {
        fields: Vec<NameValue>,
    },
    FormData {
        parts: Vec<FormPart>,
    },
    Binary {
        path: String,
    },
    /// Sent as `{"query", "variables"}` JSON once `{{variables}}` in both are filled in.
    Graphql {
        query: String,
        variables: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormPart {
    pub name: String,
    /// The text, or the path of the file to upload when `file` is set.
    pub value: String,
    #[serde(default)]
    pub file: bool,
}

impl RequestBody {
    /// Texts that may hold `{{variables}}`. File paths are never templates.
    pub fn templates(&self) -> Vec<&String> {
        match self {
            Self::Raw { text } => vec![text],
            Self::Graphql { query, variables } => vec![query, variables],
            Self::Urlencoded { fields } => fields
                .iter()
                .flat_map(|field| [&field.name, &field.value])
                .collect(),
            Self::FormData { parts } => parts
                .iter()
                .flat_map(|part| {
                    let value = (!part.file).then_some(&part.value);
                    std::iter::once(&part.name).chain(value)
                })
                .collect(),
            Self::Binary { .. } => Vec::new(),
        }
    }

    pub fn templates_mut(&mut self) -> Vec<&mut String> {
        match self {
            Self::Raw { text } => vec![text],
            Self::Graphql { query, variables } => vec![query, variables],
            Self::Urlencoded { fields } => fields
                .iter_mut()
                .flat_map(|field| [&mut field.name, &mut field.value])
                .collect(),
            Self::FormData { parts } => parts
                .iter_mut()
                .flat_map(|part| {
                    let value = (!part.file).then_some(&mut part.value);
                    std::iter::once(&mut part.name).chain(value)
                })
                .collect(),
            Self::Binary { .. } => Vec::new(),
        }
    }

    /// Every local file this body would read.
    pub fn file_paths(&self) -> Vec<&str> {
        match self {
            Self::FormData { parts } => parts
                .iter()
                .filter(|part| part.file)
                .map(|part| part.value.as_str())
                .collect(),
            Self::Binary { path } => vec![path.as_str()],
            Self::Raw { .. } | Self::Urlencoded { .. } | Self::Graphql { .. } => Vec::new(),
        }
    }

    /// What `pm.request.body.raw` shows: the text of a raw body or the encoded form; files and
    /// multipart bodies have no text form.
    pub fn script_text(&self) -> Option<String> {
        match self {
            Self::Raw { text } => Some(text.clone()),
            Self::Urlencoded { fields } => Some(encode_form(fields)),
            Self::Graphql { query, .. } => Some(query.clone()),
            Self::FormData { .. } | Self::Binary { .. } => None,
        }
    }
}

pub fn encode_form(fields: &[NameValue]) -> String {
    form_urlencoded::Serializer::new(String::new())
        .extend_pairs(
            fields
                .iter()
                .map(|field| (field.name.trim(), field.value.as_str())),
        )
        .finish()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseOutput {
    pub status: u16,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<NameValue>,
    pub content_type: Option<String>,
    pub body_preview: String,
    pub body_size: u64,
    pub body_truncated: bool,
    /// Set when the whole body is kept in the Rust core, so the user can save it to a file.
    pub body_id: Option<String>,
    /// The whole body while it fits the kept-body limit; moved into the body store, never sent.
    #[serde(skip)]
    pub whole_body: Option<Vec<u8>>,
    /// Set for an event stream (`text/event-stream`): its events arrive one by one afterwards.
    pub stream_id: Option<String>,
    /// The open event stream, handed to the stream reader; never sent.
    #[serde(skip)]
    pub stream: Option<reqwest::Response>,
    pub elapsed_ms: u64,
    /// The cookies set by this response, read from its `Set-Cookie` headers.
    pub cookies: Vec<ResponseCookie>,
    /// Set when the response arrived but a side task (such as history) failed.
    pub warning: Option<String>,
    /// Present when a pre-request or test script ran.
    pub script: Option<ScriptReport>,
}
