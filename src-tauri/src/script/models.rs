use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::request::types::{Iteration, NameValue};

/// The prelude stops at the same number; the host enforces it again.
pub const MAX_SCRIPT_CHANGES: usize = 200;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    PreRequest,
    Test,
}

/// Enabled variables per scope, as plain values, for `pm.globals` / `pm.environment` / ….
/// `data` is the current row of a run's data file (`pm.iterationData`), read-only.
#[derive(Debug, Default, Serialize)]
pub struct VariableSnapshot {
    pub globals: HashMap<String, String>,
    pub collection: HashMap<String, String>,
    pub environment: HashMap<String, String>,
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<NameValue>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptResponse {
    pub code: u16,
    pub status: String,
    pub response_time: u64,
    pub size: u64,
    pub headers: Vec<NameValue>,
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInput {
    pub phase: Phase,
    pub request_name: String,
    pub variables: VariableSnapshot,
    pub request: ScriptRequest,
    pub response: Option<ScriptResponse>,
    pub iteration: Iteration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeScope {
    Globals,
    Collection,
    Environment,
}

/// `value: None` means the script unset the variable.
#[derive(Debug, Deserialize)]
pub struct VariableChange {
    pub scope: ChangeScope,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ScriptOutput {
    pub changes: Vec<VariableChange>,
    pub tests: Vec<TestResult>,
    pub logs: Vec<LogLine>,
    pub request: ScriptRequest,
    pub local: HashMap<String, String>,
}

/// What the UI shows after a send: test results, console output and script problems.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptReport {
    pub tests: Vec<TestResult>,
    pub logs: Vec<LogLine>,
    pub errors: Vec<String>,
}
