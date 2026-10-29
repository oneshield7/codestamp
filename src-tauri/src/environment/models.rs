use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSummary {
    pub id: String,
    pub name: String,
    pub is_global: bool,
    /// The collection it belongs to and is shared with; `None` keeps it on this device.
    pub collection_id: Option<String>,
    pub variable_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentList {
    pub environments: Vec<EnvironmentSummary>,
    pub active_id: Option<String>,
}

/// What the UI sees for a variable. Secret values are never included; use `reveal_secret`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableView {
    pub id: String,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub enabled: bool,
    /// The access group of a restricted variable; its value is a secret here.
    pub restricted: Option<String>,
    /// Restricted to a group this device cannot read: the value is empty and cannot be changed.
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableScope {
    Global,
    Collection,
    Environment,
}

/// Where an autocomplete suggestion comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HintSource {
    Global,
    Collection,
    Environment,
    /// A built-in `{{$name}}`, made fresh on every send.
    Dynamic,
}

impl From<VariableScope> for HintSource {
    fn from(scope: VariableScope) -> Self {
        match scope {
            VariableScope::Global => Self::Global,
            VariableScope::Collection => Self::Collection,
            VariableScope::Environment => Self::Environment,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableHint {
    pub name: String,
    pub scope: HintSource,
    pub is_secret: bool,
    pub preview: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlPreview {
    pub url: String,
    pub missing: Vec<String>,
}

/// `value: None` on a secret variable means "keep the stored secret unchanged".
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariableInput {
    pub id: Option<String>,
    pub key: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub enabled: bool,
    /// An access group of the collection; only for secrets of a shared collection.
    #[serde(default)]
    pub restricted: Option<String>,
}
