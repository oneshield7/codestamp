//! Runs a flow from Start to End: each step's request in order, with its scripts, checks and
//! extracts. Extracted values reach the steps after it as `{{name}}`, winning over every saved
//! scope for this run only; nothing is written to environments or history.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde::Serialize;

use super::evaluate::{Answer, CheckResult, evaluate, extract};
use super::models::{FlowDetail, FlowStepView, OnFailure};
use crate::environment::vault::Vault;
use crate::request::client::HttpClient;
use crate::request::pipeline;
use crate::request::saved::input_from_content;
use crate::request::types::NameValue;
use crate::script::models::TestResult;
use crate::storage::db::Database;
use crate::storage::repo as collections;
use crate::text::clip;

/// The response body kept for the step's Response tab.
const MAX_BODY_SNIPPET_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassedValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutcome {
    pub step_id: String,
    pub status: StepStatus,
    pub code: Option<u16>,
    pub status_text: String,
    pub elapsed_ms: Option<u64>,
    pub checks: Vec<CheckResult>,
    /// `pm.test` results from the request's own scripts.
    pub tests: Vec<TestResult>,
    pub passed_on: Vec<PassedValue>,
    pub errors: Vec<String>,
    pub note: Option<String>,
    pub headers: Vec<NameValue>,
    pub body: String,
}

/// What happens during a run, as it happens.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FlowEvent {
    #[serde(rename_all = "camelCase")]
    StepStarted {
        step_id: String,
    },
    StepFinished(Box<StepOutcome>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowRunSummary {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
    /// The step that failed and stopped the run.
    pub stopped_at: Option<String>,
    /// Set when the user pressed Stop.
    pub cancelled: bool,
}

pub struct Context<'a> {
    pub client: &'a HttpClient,
    pub db: &'a Database,
    pub vault: &'a Vault,
    pub stop: &'a AtomicBool,
}

pub async fn run(
    context: &Context<'_>,
    flow: &FlowDetail,
    emit: impl Fn(FlowEvent),
) -> FlowRunSummary {
    let started = Instant::now();
    let mut values: Vec<NameValue> = Vec::new();
    let mut summary = FlowRunSummary {
        passed: 0,
        failed: 0,
        skipped: 0,
        duration_ms: 0,
        stopped_at: None,
        cancelled: false,
    };
    for step in &flow.steps {
        summary.cancelled |= context.stop.load(Ordering::Relaxed);
        let reason = if summary.cancelled {
            Some("Stopped before this step")
        } else if summary.stopped_at.is_some() {
            Some("Skipped: an earlier step failed")
        } else {
            None
        };
        let outcome = match reason {
            Some(reason) => StepOutcome {
                status: StepStatus::Skipped,
                note: Some(reason.to_owned()),
                ..empty(&step.id)
            },
            None => {
                emit(FlowEvent::StepStarted {
                    step_id: step.id.clone(),
                });
                run_step(context, step, &mut values).await
            }
        };
        match outcome.status {
            StepStatus::Passed => summary.passed += 1,
            StepStatus::Skipped => summary.skipped += 1,
            StepStatus::Failed => {
                summary.failed += 1;
                if step.on_failure == OnFailure::Stop {
                    summary.stopped_at = Some(step.id.clone());
                }
            }
        }
        emit(FlowEvent::StepFinished(Box::new(outcome)));
    }
    summary.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    summary
}

async fn run_step(
    context: &Context<'_>,
    step: &FlowStepView,
    values: &mut Vec<NameValue>,
) -> StepOutcome {
    let mut outcome = StepOutcome {
        status: StepStatus::Failed,
        ..empty(&step.id)
    };
    let Some(request) = &step.request else {
        outcome
            .errors
            .push("The request was deleted. Choose another one or remove this step".to_owned());
        return outcome;
    };
    let saved = match context
        .db
        .with(|conn| collections::load_request(conn, &request.id))
    {
        Ok(saved) => saved,
        Err(error) => {
            outcome.errors.push(error.to_string());
            return outcome;
        }
    };
    let mut input = input_from_content(&saved.content, &saved.collection_id, &saved.id);
    input.iteration_data = values.clone();
    let mut response = match pipeline::send(context.client, context.db, context.vault, input).await
    {
        Ok(response) => response,
        Err(error) => {
            outcome.errors.push(error.to_string());
            return outcome;
        }
    };
    // A flow does not wait for a stream that may never end; dropping it closes it.
    if response.stream.take().is_some() {
        outcome.note = Some("Event stream: stopped once its headers arrived".to_owned());
    }
    let answer = Answer {
        status: response.status,
        headers: &response.headers,
        body: &response.body_preview,
        body_truncated: response.body_truncated,
        elapsed_ms: response.elapsed_ms,
    };
    outcome.checks = step
        .checks
        .iter()
        .map(|check| evaluate(check, &answer))
        .collect();
    for rule in &step.extracts {
        match extract(rule, &answer) {
            Ok(value) => {
                values.retain(|existing| existing.name != rule.name);
                values.push(NameValue {
                    name: rule.name.clone(),
                    value: value.clone(),
                });
                outcome.passed_on.push(PassedValue {
                    name: rule.name.clone(),
                    value,
                });
            }
            Err(reason) => outcome.errors.push(format!("Could not pass on {reason}")),
        }
    }
    if let Some(script) = response.script.take() {
        outcome.tests = script.tests;
        outcome.errors.extend(script.errors);
    }
    outcome.code = Some(response.status);
    outcome.elapsed_ms = Some(response.elapsed_ms);
    outcome.body = clip(&response.body_preview, MAX_BODY_SNIPPET_BYTES)
        .0
        .to_owned();
    outcome.status_text = response.status_text;
    outcome.headers = response.headers;
    if passed(&outcome) {
        outcome.status = StepStatus::Passed;
    }
    outcome
}

/// A step passes when it got a response, its scripts and extracts worked, and every check and
/// test passed. With neither checks nor tests, an error status (4xx/5xx) fails it, as in runs.
fn passed(outcome: &StepOutcome) -> bool {
    let Some(code) = outcome.code else {
        return false;
    };
    if !outcome.errors.is_empty()
        || outcome.checks.iter().any(|check| !check.passed)
        || outcome.tests.iter().any(|test| !test.passed)
    {
        return false;
    }
    !outcome.checks.is_empty() || !outcome.tests.is_empty() || code < 400
}

fn empty(step_id: &str) -> StepOutcome {
    StepOutcome {
        step_id: step_id.to_owned(),
        status: StepStatus::Skipped,
        code: None,
        status_text: String::new(),
        elapsed_ms: None,
        checks: Vec::new(),
        tests: Vec::new(),
        passed_on: Vec::new(),
        errors: Vec::new(),
        note: None,
        headers: Vec::new(),
        body: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::thread;

    use super::*;
    use crate::auth::models::Auth;
    use crate::environment::vault::EphemeralKey;
    use crate::flow::models::{Check, CheckOp, CheckSubject, Extract, ExtractSource, StepRequest};
    use crate::storage::models::{KeyValue, RequestContent, RequestKind};

    /// Answers `count` requests: `/login` gives a token, `/me` echoes the Authorization header,
    /// anything else fails with 500.
    fn serve(count: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local port");
        let address = listener.local_addr().expect("local address");
        thread::spawn(move || {
            for _ in 0..count {
                let (stream, _) = listener.accept().expect("accept connection");
                let mut reader = BufReader::new(stream);
                let mut first = String::new();
                reader.read_line(&mut first).expect("request line");
                let mut authorization = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) <= 2 {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("authorization:") {
                        authorization = value.trim().to_owned();
                    }
                }
                let (status, body) = if first.contains("/login") {
                    ("200 OK", r#"{"token":"t-123"}"#.to_owned())
                } else if first.contains("/me") {
                    ("200 OK", format!(r#"{{"auth":"{authorization}"}}"#))
                } else {
                    ("500 Internal Server Error", "{}".to_owned())
                };
                let mut stream = reader.into_inner();
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(head.as_bytes()).expect("write head");
                stream.write_all(body.as_bytes()).expect("write body");
            }
        });
        format!("http://{address}")
    }

    fn content(name: &str, url: String, headers: Vec<KeyValue>) -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: name.to_owned(),
            method: "GET".to_owned(),
            url,
            params: Vec::new(),
            headers,
            body_mode: "none".to_owned(),
            body: String::new(),
            form: Vec::new(),
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: String::new(),
            auth: Auth::None,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    fn step(db: &Database, collection_id: &str, content: RequestContent) -> FlowStepView {
        let saved = db
            .with(|conn| collections::insert_request(conn, collection_id, None, content))
            .expect("request");
        FlowStepView {
            id: crate::storage::db::new_id(),
            request: Some(StepRequest {
                id: saved.id,
                collection_id: collection_id.to_owned(),
                collection_name: "Shop".to_owned(),
                kind: RequestKind::Http,
                name: saved.content.name,
                method: saved.content.method,
                url: saved.content.url,
                variables: Vec::new(),
            }),
            extracts: Vec::new(),
            checks: Vec::new(),
            on_failure: OnFailure::Stop,
        }
    }

    #[tokio::test]
    async fn passes_values_on_and_stops_at_a_failure() {
        let base = serve(3);
        let db = Database::in_memory().expect("database");
        let collection = db
            .with(|conn| collections::insert_collection(conn, "Shop"))
            .expect("collection");
        let mut login = step(
            &db,
            &collection.id,
            content("Login", format!("{base}/login"), Vec::new()),
        );
        login.extracts.push(Extract {
            name: "token".to_owned(),
            source: ExtractSource::Body,
            path: "token".to_owned(),
        });
        let header = KeyValue {
            enabled: true,
            key: "Authorization".to_owned(),
            value: "Bearer {{token}}".to_owned(),
        };
        let mut me = step(
            &db,
            &collection.id,
            content("Me", format!("{base}/me"), vec![header]),
        );
        me.checks.push(Check {
            subject: CheckSubject::Body,
            path: "auth".to_owned(),
            op: CheckOp::Equals,
            value: "bearer t-123".to_owned(),
        });
        let broken = step(
            &db,
            &collection.id,
            content("Broken", format!("{base}/fail"), Vec::new()),
        );
        let after = step(
            &db,
            &collection.id,
            content("After", format!("{base}/me"), Vec::new()),
        );
        let flow = FlowDetail {
            id: crate::storage::db::new_id(),
            name: "Signup".to_owned(),
            steps: vec![login, me, broken, after],
        };

        let client = HttpClient::new().expect("client");
        let vault = Vault::new(Box::new(EphemeralKey::generate()));
        let stop = AtomicBool::new(false);
        let context = Context {
            client: &client,
            db: &db,
            vault: &vault,
            stop: &stop,
        };
        let events = Mutex::new(Vec::new());
        let summary = run(&context, &flow, |event| {
            events.lock().expect("events").push(event);
        })
        .await;

        let outcomes: Vec<StepOutcome> = events
            .into_inner()
            .expect("events")
            .into_iter()
            .filter_map(|event| match event {
                FlowEvent::StepFinished(outcome) => Some(*outcome),
                FlowEvent::StepStarted { .. } => None,
            })
            .collect();
        assert_eq!(outcomes.len(), 4);
        assert_eq!(outcomes[0].status, StepStatus::Passed);
        assert_eq!(outcomes[0].passed_on[0].value, "t-123");
        assert_eq!(
            outcomes[1].status,
            StepStatus::Passed,
            "{:?}",
            outcomes[1].checks
        );
        assert_eq!(outcomes[2].status, StepStatus::Failed);
        assert_eq!(outcomes[2].code, Some(500));
        assert_eq!(outcomes[3].status, StepStatus::Skipped);
        assert_eq!((summary.passed, summary.failed, summary.skipped), (2, 1, 1));
        assert_eq!(
            summary.stopped_at.as_deref(),
            Some(flow.steps[2].id.as_str())
        );
        assert!(!summary.cancelled);
    }
}
