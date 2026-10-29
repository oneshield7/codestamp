use serde::{Deserialize, Serialize};

use crate::storage::models::RequestKind;

/// Healthy, needs attention or critical, from the health score; no data when nothing was
/// measured yet (an empty collection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    NeedsAttention,
    Critical,
    NoData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    /// 0 to 100.
    pub score: Option<u8>,
    pub status: HealthStatus,
}

/// The three parts the health score is made of; a part is `None` when it has nothing to measure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CategoryScores {
    pub runtime: Option<u8>,
    pub tests: Option<u8>,
    pub quality: Option<u8>,
}

/// Each part graded on the same bands as the whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryHealth {
    pub runtime: Health,
    pub tests: Health,
    pub quality: Health,
}

/// How a whole Run all ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RunOutcome {
    Passed,
    Failed,
    Stopped,
}

impl RunOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }

    pub fn from_stored(text: &str) -> Self {
        match text {
            "passed" => Self::Passed,
            "stopped" => Self::Stopped,
            _ => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
    Passed,
    Failed,
    Skipped,
}

/// One request's results in a run, added up over its iterations.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEndpoint {
    pub request_id: String,
    pub name: String,
    pub method: String,
    pub kind: RequestKind,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub tests_passed: u32,
    pub tests_total: u32,
    /// Average time of the iterations that got a response.
    pub avg_ms: Option<u64>,
    /// The status code of the last iteration that got a response.
    pub last_code: Option<u16>,
}

/// A stored Run all, without its per-request rows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunBrief {
    pub id: String,
    pub environment: Option<String>,
    pub started_at: i64,
    pub finished_at: i64,
    pub outcome: RunOutcome,
    /// Request sends (iterations count separately) that passed and failed.
    pub passed: u32,
    pub failed: u32,
    pub tests_passed: u32,
    pub tests_total: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    #[serde(flatten)]
    pub brief: RunBrief,
    pub endpoints: Vec<RunEndpoint>,
}

/// One send of a request in a run, as the runner reports it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunStepInput {
    pub request_id: String,
    pub status: StepStatus,
    pub code: Option<u16>,
    pub elapsed_ms: Option<u64>,
    pub tests_passed: u32,
    pub tests_total: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunInput {
    pub collection_id: String,
    pub environment: Option<String>,
    pub started_at: i64,
    pub finished_at: i64,
    /// Stop was pressed before every request had its turn.
    pub stopped: bool,
    pub steps: Vec<RunStepInput>,
}

/// The owner and tags the user gives an API.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogInfo {
    pub owner: String,
    pub tags: Vec<String>,
}

/// Requests sent in a window and the window before it, for "up 12% on the week before".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trend {
    pub current: i64,
    pub previous: i64,
}

/// One row of the catalog list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub icon: String,
    #[serde(flatten)]
    pub info: CatalogInfo,
    pub request_count: i64,
    /// Requests Run all sends, and those of them with a test script.
    pub runnable_count: i64,
    pub tested_count: i64,
    pub example_count: i64,
    pub health: Health,
    pub last_run: Option<RunBrief>,
    /// Percent of the last 7 days' sends that got no answer or a 5xx.
    pub error_rate: Option<f64>,
    pub p95_ms: Option<i64>,
    pub last_activity: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSummary {
    pub apis: usize,
    pub endpoints: i64,
    pub requests: Trend,
    pub errors: Trend,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogOverview {
    pub summary: CatalogSummary,
    pub apis: Vec<CatalogEntry>,
    /// Every tag in use, for the filter.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KindCount {
    pub kind: RequestKind,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assets {
    pub kinds: Vec<KindCount>,
    pub sections: i64,
    pub examples: i64,
    pub variables: i64,
    pub flows: i64,
}

/// Sends that started on one local day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayTraffic {
    pub start: i64,
    pub requests: i64,
    pub errors: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointTraffic {
    pub request_id: String,
    pub name: String,
    pub method: String,
    pub kind: RequestKind,
    pub requests: i64,
    pub errors: i64,
    pub error_rate: f64,
    pub p95_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub requests: Trend,
    /// No answer, or a 5xx.
    pub errors: Trend,
    pub client_errors: i64,
    pub error_rate: Option<f64>,
    pub p95_ms: Option<i64>,
    pub avg_ms: Option<i64>,
    pub days: Vec<DayTraffic>,
    /// The requests with the highest error rate, at most five.
    pub endpoints: Vec<EndpointTraffic>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tests {
    pub runnable: i64,
    pub tested: i64,
    pub coverage: Option<f64>,
    pub latest: Option<RunRecord>,
    /// Newest first.
    pub runs: Vec<RunBrief>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingRequest {
    pub id: String,
    pub name: String,
    pub method: String,
    pub kind: RequestKind,
}

/// Where a rule failed: a request, or the collection's own auth when `request` is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub request: Option<FindingRequest>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleResult {
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub severity: Severity,
    /// How many requests (and the collection auth, for secrets) the rule looked at.
    pub checked: usize,
    pub findings: Vec<Finding>,
}

/// A flow with steps from this API, and the other APIs it calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowUse {
    pub id: String,
    pub name: String,
    pub steps_here: i64,
    pub steps_total: i64,
    pub other_apis: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDetail {
    pub entry: CatalogEntry,
    pub scores: CategoryHealth,
    pub assets: Assets,
    pub runtime: Runtime,
    pub tests: Tests,
    pub rules: Vec<RuleResult>,
    pub flows: Vec<FlowUse>,
}
