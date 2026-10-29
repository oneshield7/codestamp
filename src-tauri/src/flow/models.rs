use serde::{Deserialize, Serialize};

use crate::storage::models::RequestKind;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowSummary {
    pub id: String,
    pub name: String,
    pub step_count: i64,
}

/// A flow always runs from Start to End; only the steps between them are stored.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDetail {
    pub id: String,
    pub name: String,
    pub steps: Vec<FlowStepView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowStepView {
    pub id: String,
    /// `None` once the request was deleted; the step stays so the user sees the gap.
    pub request: Option<StepRequest>,
    pub extracts: Vec<Extract>,
    pub checks: Vec<Check>,
    pub on_failure: OnFailure,
}

/// The request a step calls, with where it lives.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepRequest {
    pub id: String,
    pub collection_id: String,
    pub collection_name: String,
    pub kind: RequestKind,
    pub name: String,
    pub method: String,
    pub url: String,
    /// Every `{{name}}` the request sends, so the editor can show which passed values reach it.
    pub variables: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowInput {
    pub id: String,
    pub name: String,
    pub steps: Vec<FlowStepInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowStepInput {
    pub id: String,
    pub request_id: Option<String>,
    #[serde(default)]
    pub extracts: Vec<Extract>,
    #[serde(default)]
    pub checks: Vec<Check>,
    #[serde(default)]
    pub on_failure: OnFailure,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnFailure {
    #[default]
    Stop,
    Continue,
}

impl OnFailure {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Continue => "continue",
        }
    }

    pub fn from_stored(text: &str) -> Self {
        if text == "continue" {
            Self::Continue
        } else {
            Self::Stop
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractSource {
    Body,
    Header,
}

/// A value taken from a step's response and passed on as `{{name}}` to the steps after it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Extract {
    pub name: String,
    pub source: ExtractSource,
    /// A JSON path such as `data.items[0].id` for the body, or a header name.
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckSubject {
    Status,
    Body,
    Header,
    /// Response time in milliseconds.
    Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckOp {
    Equals,
    NotEquals,
    Contains,
    Exists,
    LessThan,
}

/// A condition a step's response must meet, written without a script.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Check {
    pub subject: CheckSubject,
    /// The JSON path (body) or header name; empty for status and time.
    #[serde(default)]
    pub path: String,
    pub op: CheckOp,
    #[serde(default)]
    pub value: String,
}
