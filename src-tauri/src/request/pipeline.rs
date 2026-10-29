use std::collections::HashMap;

use serde::Serialize;
use tokio::runtime::Handle;

use super::client::HttpClient;
use super::path_vars::{self, MAX_PATH_VARIABLES};
use super::rules::{MAX_KEY_VALUE_LENGTH, MAX_KEY_VALUE_ROWS, MAX_SCRIPT_BYTES};
use super::types::{HttpRequestInput, HttpResponseOutput, Iteration, NameValue, RequestBody};
use super::uploads::check_granted;
use super::validate::{HTTP_SCHEMES, build_url, graphql_json, validate};
use crate::auth::apply::{apply_auth, check_auth, effective_auth};
use crate::auth::flow;
use crate::auth::models::Auth;
use crate::auth::oauth;
use crate::cookies::repo as cookies;
use crate::environment::lookup::{resolve_request, save_script_changes, script_variables};
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::history::models::SentRequest;
use crate::history::repo as history;
use crate::script::models::{
    LogLine, Phase, ScriptInput, ScriptReport, ScriptRequest, ScriptResponse, VariableSnapshot,
};
use crate::script::network::Network;
use crate::script::runner::{self, RunResult};
use crate::storage::db::Database;
use crate::storage::validate::{MAX_NAME_CHARS, check_optional_id};

const HISTORY_WARNING: &str = "The response arrived, but it could not be added to history";
const COOKIE_WARNING: &str = "The response arrived, but its cookies could not be saved";
const STREAM_TESTS: &str =
    "The test script did not run: this response is an event stream, shown as events arrive";
const RAW_BODY_ONLY: &str =
    "pm.request.body changes apply to raw bodies only; this request's body was sent unchanged";

/// The request as it would go out (variables filled in, auth added), for "Copy as code".
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<NameValue>,
    pub body: Option<RequestBody>,
}

/// Sends one request: pre-request script → variables → auth → validation → network → history →
/// test script. Requests rejected before sending (bad input, unknown variable, failing
/// pre-request script, incomplete auth) are not added to history; anything that reached the
/// network is, unless the request comes from a collection run.
pub async fn send(
    client: &HttpClient,
    db: &Database,
    vault: &Vault,
    mut request: HttpRequestInput,
) -> Result<HttpResponseOutput, AppError> {
    check_input(&request)?;
    let collection_id = request.collection_id.clone();
    let token_owner = oauth::token_owner(
        request.auth.as_ref(),
        request.request_id.as_deref(),
        collection_id.as_deref(),
    );
    request.auth = Some(effective_auth(
        db,
        request.auth.take(),
        collection_id.as_deref(),
    )?);
    let sent = SentRequest::capture(&request);
    let typed_body_text = request.body.as_ref().and_then(RequestBody::script_text);
    let record_history = request.record_history;
    let name = request.name.clone().unwrap_or_default();
    let pre_request_script = non_empty(request.pre_request_script.take());
    let test_script = non_empty(request.test_script.take());
    let iteration = request.iteration;
    let data = request.iteration_data.clone();
    let mut report = ScriptReport::default();
    let mut local = HashMap::new();
    let network = Network::new(client.clone(), Handle::current());

    if let Some(script) = pre_request_script.clone() {
        let input = ScriptInput {
            phase: Phase::PreRequest,
            request_name: name.clone(),
            variables: snapshot(db, vault, collection_id.as_deref(), &data)?,
            request: ScriptRequest {
                method: request.method.clone(),
                url: request.url.clone(),
                headers: request.headers.clone(),
                body: typed_body_text.clone(),
            },
            response: None,
            iteration,
        };
        let result = run_script(script, input, network.clone()).await?;
        record_run(db, vault, collection_id.as_deref(), &result, &mut report)?;
        if let Some(error) = result.error {
            return Err(AppError::Script(format!(
                "Pre-request script failed: {error}"
            )));
        }
        let edited = result.output.request;
        request.method = edited.method;
        request.url = edited.url;
        request.headers = edited.headers;
        if matches!(request.body, None | Some(RequestBody::Raw { .. })) {
            request.body = edited.body.map(|text| RequestBody::Raw { text });
        } else if edited.body != typed_body_text {
            report.logs.push(LogLine {
                level: "warn".to_owned(),
                message: RAW_BODY_ONLY.to_owned(),
            });
        }
        local = result.output.local;
    }

    let mut resolved = resolve_request(db, vault, request, &local)?;
    resolved.url = path_vars::fill(&resolved.url, &resolved.path_variables)?;
    if let Some(Auth::OAuth2(settings)) = &resolved.auth {
        let token =
            oauth::access_token(client, db, vault, token_owner.as_deref(), settings).await?;
        resolved.auth = Some(oauth::bearer(settings, token));
    }
    let auth = resolved.auth.take();
    if let Some(auth) = &auth {
        apply_auth(&mut resolved, auth)?;
    }
    if let Some(body) = &resolved.body {
        db.with(|conn| check_granted(conn, body))?;
    }
    let validated = validate(resolved)?;
    let outcome = flow::send(client, validated, auth.as_ref()).await;
    let recorded = if record_history {
        db.with(|conn| history::record(conn, &sent, outcome.as_ref()))
    } else {
        Ok(())
    };
    let mut response = outcome?;
    if recorded.is_err() {
        response.warning = Some(HISTORY_WARNING.to_owned());
    }

    if response.stream.is_some() && test_script.is_some() {
        report.logs.push(LogLine {
            level: "warn".to_owned(),
            message: STREAM_TESTS.to_owned(),
        });
    } else if let Some(script) = test_script.clone() {
        let request = ScriptRequest {
            method: sent.method.clone(),
            url: sent.url.clone(),
            headers: sent.headers.clone(),
            body: typed_body_text,
        };
        let scope = TestScope {
            db,
            vault,
            collection_id: collection_id.as_deref(),
            network,
            data: &data,
            iteration,
        };
        // The request already reached the server, so a failing test phase must not hide the
        // response: its problems are reported next to it instead.
        if let Err(error) = run_tests(&scope, script, name, request, &response, &mut report).await {
            report
                .errors
                .push(format!("The test script could not run: {error}"));
        }
    }

    if pre_request_script.is_some() || test_script.is_some() {
        response.script = Some(report);
    }
    // After the test script, so cookies set by its pm.sendRequest calls are saved too.
    if cookies::save_if_changed(db, vault, client.jar()).is_err() {
        response.warning = Some(COOKIE_WARNING.to_owned());
    }
    Ok(response)
}

/// Fills in variables and adds auth the way `send` does, without running scripts or sending.
/// Secrets are included: the user asked to see the request as it would really go out.
pub fn resolve_for_code(
    db: &Database,
    vault: &Vault,
    mut request: HttpRequestInput,
) -> Result<ResolvedRequest, AppError> {
    check_input(&request)?;
    let token_owner = oauth::token_owner(
        request.auth.as_ref(),
        request.request_id.as_deref(),
        request.collection_id.as_deref(),
    );
    request.auth = Some(effective_auth(
        db,
        request.auth.take(),
        request.collection_id.as_deref(),
    )?);
    let mut resolved = resolve_request(db, vault, request, &HashMap::new())?;
    resolved.url = path_vars::fill(&resolved.url, &resolved.path_variables)?;
    if let Some(Auth::OAuth2(settings)) = &resolved.auth {
        let token = oauth::current_token(db, vault, token_owner.as_deref())?;
        resolved.auth = Some(oauth::bearer(settings, token));
    }
    let auth = resolved.auth.take();
    if let Some(auth) = &auth {
        apply_auth(&mut resolved, auth)?;
    }
    let url = build_url(&resolved.url, &resolved.params, &HTTP_SCHEMES)?;
    // Digest needs the server's challenge, so code shows it without credentials.
    if let Some(auth @ Auth::AwsSigV4 { .. }) = &auth {
        flow::sign_for_code(
            &resolved.method,
            &url,
            &mut resolved.headers,
            resolved.body.as_ref(),
            auth,
        )?;
    }
    // Code shows the GraphQL body as the JSON it is sent as.
    let body = match resolved.body {
        Some(RequestBody::Graphql { query, variables }) => Some(RequestBody::Raw {
            text: graphql_json(query, &variables)?,
        }),
        other => other,
    };
    Ok(ResolvedRequest {
        method: resolved.method,
        url: url.to_string(),
        headers: resolved.headers,
        body,
    })
}

fn check_input(request: &HttpRequestInput) -> Result<(), AppError> {
    check_optional_id(request.collection_id.as_deref())?;
    check_optional_id(request.request_id.as_deref())?;
    if let Some(auth) = &request.auth {
        check_auth(auth, true)?;
    }
    let script_too_large = [&request.pre_request_script, &request.test_script]
        .into_iter()
        .flatten()
        .any(|script| script.len() > MAX_SCRIPT_BYTES);
    if script_too_large {
        return Err(AppError::invalid("Scripts can be up to 100 KB"));
    }
    if request
        .name
        .as_ref()
        .is_some_and(|name| name.chars().count() > MAX_NAME_CHARS)
    {
        return Err(AppError::invalid("Names can be up to 120 characters"));
    }
    let too_many = request.path_variables.len() > MAX_PATH_VARIABLES
        || request.iteration_data.len() > MAX_KEY_VALUE_ROWS;
    let too_long = request
        .path_variables
        .iter()
        .chain(&request.iteration_data)
        .any(|pair| {
            pair.name.len() > MAX_KEY_VALUE_LENGTH || pair.value.len() > MAX_KEY_VALUE_LENGTH
        });
    if too_many || too_long {
        return Err(AppError::invalid(
            "There are too many path variables or data values, or one is too long",
        ));
    }
    Ok(())
}

/// The saved scopes for a script, plus the run's data row.
fn snapshot(
    db: &Database,
    vault: &Vault,
    collection_id: Option<&str>,
    data: &[NameValue],
) -> Result<VariableSnapshot, AppError> {
    let mut variables = script_variables(db, vault, collection_id)?;
    variables.data = data
        .iter()
        .map(|pair| (pair.name.clone(), pair.value.clone()))
        .collect();
    Ok(variables)
}

fn non_empty(script: Option<String>) -> Option<String> {
    script.filter(|text| !text.trim().is_empty())
}

struct TestScope<'a> {
    db: &'a Database,
    vault: &'a Vault,
    collection_id: Option<&'a str>,
    network: Network,
    data: &'a [NameValue],
    iteration: Iteration,
}

async fn run_tests(
    scope: &TestScope<'_>,
    script: String,
    name: String,
    request: ScriptRequest,
    response: &HttpResponseOutput,
    report: &mut ScriptReport,
) -> Result<(), AppError> {
    let input = ScriptInput {
        phase: Phase::Test,
        request_name: name,
        variables: snapshot(scope.db, scope.vault, scope.collection_id, scope.data)?,
        request,
        response: Some(ScriptResponse {
            code: response.status,
            status: response.status_text.clone(),
            response_time: response.elapsed_ms,
            size: response.body_size,
            headers: response.headers.clone(),
            body: response.body_preview.clone(),
        }),
        iteration: scope.iteration,
    };
    let result = run_script(script, input, scope.network.clone()).await?;
    record_run(scope.db, scope.vault, scope.collection_id, &result, report)?;
    if let Some(error) = result.error {
        report.errors.push(format!("Test script failed: {error}"));
    }
    Ok(())
}

/// QuickJS is synchronous, so each run gets its own blocking thread and runtime.
async fn run_script(
    script: String,
    input: ScriptInput,
    network: Network,
) -> Result<RunResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || runner::run(&script, &input, Some(network)))
        .await
        .map_err(|_| AppError::Script("The script engine stopped unexpectedly".to_owned()))?
}

/// Saves variable changes and adds the run's tests and logs to the report.
fn record_run(
    db: &Database,
    vault: &Vault,
    collection_id: Option<&str>,
    result: &RunResult,
    report: &mut ScriptReport,
) -> Result<(), AppError> {
    let skipped = save_script_changes(db, vault, collection_id, &result.output.changes)?;
    report.tests.extend(result.output.tests.iter().cloned());
    report.logs.extend(result.output.logs.iter().cloned());
    report
        .logs
        .extend(skipped.into_iter().map(|message| LogLine {
            level: "warn".to_owned(),
            message,
        }));
    Ok(())
}
