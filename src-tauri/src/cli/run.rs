//! Runs the loaded collection: its HTTP and GraphQL requests in notebook order, once per
//! iteration, with scripts and tests, as the app's Run all does. Prints each result and writes
//! the reports.

use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use rusqlite::Connection;

use super::args::RunOptions;
use super::load::{Loaded, load};
use super::report::{RunReport, StepResult, StepStatus, Summary, junit, passed};
use crate::environment::dynamic::iso_timestamp;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::request::pipeline;
use crate::request::saved::input_from_content;
use crate::request::types::{Iteration, NameValue};
use crate::run_data::parse::{RunData, read};
use crate::script::models::LogLine;
use crate::storage::db::now_ms;
use crate::storage::models::{CollectionDetail, RequestSummary};
use crate::storage::repo as collections;

/// Requests times iterations; the app stops at the same number.
const MAX_RUN_STEPS: usize = 10_000;
const FAILED: u8 = 1;
const NOT_RUN: u8 = 2;

pub async fn run(options: RunOptions) -> ExitCode {
    let paint = Paint::detect();
    let (loaded, data, requests, client) = match prepare(&options) {
        Ok(ready) => ready,
        Err(message) => {
            eprintln!("{} {message}", paint.red("error:"));
            return ExitCode::from(NOT_RUN);
        }
    };
    for warning in &loaded.warnings {
        eprintln!("{} {warning}", paint.yellow("warning:"));
    }
    print_header(&paint, &loaded, data.as_ref());

    let wanted = data.as_ref().map_or(options.iterations, |data| {
        u32::try_from(data.rows.len()).unwrap_or(u32::MAX)
    });
    let most = u32::try_from(MAX_RUN_STEPS / requests.len().max(1)).unwrap_or(u32::MAX);
    let passes = wanted.min(most.max(1));
    if passes < wanted {
        eprintln!(
            "{} only the first {passes} of {wanted} iterations run: a run has at most {MAX_RUN_STEPS} steps",
            paint.yellow("warning:")
        );
    }

    let started_at = now_ms();
    let mut results = Vec::new();
    let mut stopped = false;
    let mut first = true;
    for iteration in 0..passes {
        if passes > 1 {
            println!(
                "\n{}",
                paint.bold(&format!("Iteration {}/{passes}", iteration + 1))
            );
        }
        let row = data
            .as_ref()
            .map(|data| row_values(data, iteration))
            .unwrap_or_default();
        for request in &requests {
            let reason = request
                .kind
                .run_skip_reason()
                .or_else(|| stopped.then_some("Skipped after a failure"));
            let (step, logs) = match reason {
                Some(reason) => (skipped(request, iteration, reason), Vec::new()),
                None => {
                    if !first && options.delay_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(options.delay_ms)).await;
                    }
                    first = false;
                    let call = Call {
                        iteration,
                        count: passes,
                        data: &row,
                        timeout_ms: options.timeout_ms,
                    };
                    send(&client, &loaded, request, &call).await
                }
            };
            print_step(&paint, &step, &logs);
            stopped |= options.bail && step.status == StepStatus::Failed;
            results.push(step);
        }
    }
    let finished_at = now_ms();
    let duration_ms = u64::try_from(finished_at - started_at).unwrap_or(0);
    let summary = Summary::of(&results);
    print_summary(&paint, &summary, duration_ms);

    let report = RunReport {
        app: "Codestamp",
        collection: &loaded.collection_name,
        environment: loaded.environment_name.as_deref(),
        data_file: data.as_ref().map(|data| data.name.as_str()),
        iterations: passes,
        started_at: iso_timestamp(started_at),
        finished_at: iso_timestamp(finished_at),
        duration_ms,
        summary: Summary::of(&results),
        results: &results,
    };
    let written = write_reports(&options, &report, &loaded.collection_name, duration_ms);
    if let Err(message) = written {
        eprintln!("{} {message}", paint.red("error:"));
        return ExitCode::from(NOT_RUN);
    }
    if summary.failed > 0 {
        ExitCode::from(FAILED)
    } else {
        ExitCode::SUCCESS
    }
}

type Ready = (Loaded, Option<RunData>, Vec<RequestSummary>, HttpClient);

fn prepare(options: &RunOptions) -> Result<Ready, String> {
    let loaded = load(options)?;
    let data = options
        .data
        .as_deref()
        .map(|path| read(path).map_err(|error| format!("{}: {error}", path.display())))
        .transpose()?;
    let requests = loaded
        .db
        .with(|conn| notebook_order(conn, &loaded.collection_id))
        .map_err(|error| error.to_string())?;
    if requests
        .iter()
        .all(|request| request.kind.run_skip_reason().is_some())
    {
        return Err("nothing to run: the collection has no HTTP or GraphQL requests".to_owned());
    }
    let client = HttpClient::new().map_err(|error| error.to_string())?;
    Ok((loaded, data, requests, client))
}

/// Requests without a section first, then each section in order, as the app shows them.
fn notebook_order(conn: &Connection, collection_id: &str) -> Result<Vec<RequestSummary>, AppError> {
    let CollectionDetail {
        sections, requests, ..
    } = collections::collection_detail(conn, collection_id)?;
    let (mut ordered, mut rest): (Vec<_>, Vec<_>) = requests
        .into_iter()
        .partition(|request| request.section_id.is_none());
    for section in &sections {
        let (inside, others): (Vec<_>, Vec<_>) = rest
            .into_iter()
            .partition(|request| request.section_id.as_deref() == Some(section.id.as_str()));
        ordered.extend(inside);
        rest = others;
    }
    Ok(ordered)
}

/// Which pass this is and what it brings along.
struct Call<'a> {
    iteration: u32,
    count: u32,
    data: &'a [NameValue],
    timeout_ms: Option<u64>,
}

async fn send(
    client: &HttpClient,
    loaded: &Loaded,
    request: &RequestSummary,
    call: &Call<'_>,
) -> (StepResult, Vec<LogLine>) {
    let mut step = StepResult {
        status: StepStatus::Failed,
        ..skipped(request, call.iteration, "")
    };
    step.note = None;
    let saved = match loaded
        .db
        .with(|conn| collections::load_request(conn, &request.id))
    {
        Ok(saved) => saved,
        Err(error) => {
            step.errors.push(error.to_string());
            return (step, Vec::new());
        }
    };
    let mut input = input_from_content(&saved.content, &loaded.collection_id, &request.id);
    input.timeout_ms = call.timeout_ms;
    input.iteration_data = call.data.to_vec();
    input.iteration = Iteration {
        index: call.iteration,
        count: call.count,
    };
    let mut logs = Vec::new();
    match pipeline::send(client, &loaded.db, &loaded.vault, input).await {
        Ok(mut response) => {
            // A run does not wait for a stream that may never end; dropping it closes it.
            if response.stream.take().is_some() {
                step.note = Some("Event stream: stopped once its headers arrived".to_owned());
            }
            step.code = Some(response.status);
            step.status_text = response.status_text;
            step.elapsed_ms = Some(response.elapsed_ms);
            if let Some(script) = response.script {
                step.tests = script.tests;
                step.errors = script.errors;
                logs = script.logs;
            }
            if passed(step.code, &step.tests, &step.errors) {
                step.status = StepStatus::Passed;
            }
        }
        Err(error) => step.errors.push(error.to_string()),
    }
    (step, logs)
}

fn skipped(request: &RequestSummary, iteration: u32, reason: &str) -> StepResult {
    StepResult {
        iteration: iteration + 1,
        name: request.name.clone(),
        method: request.kind.stamp(&request.method).to_owned(),
        url: request.url.clone(),
        status: StepStatus::Skipped,
        code: None,
        status_text: String::new(),
        elapsed_ms: None,
        tests: Vec::new(),
        errors: Vec::new(),
        note: Some(reason.to_owned()),
    }
}

fn row_values(data: &RunData, iteration: u32) -> Vec<NameValue> {
    let Some(row) = usize::try_from(iteration)
        .ok()
        .and_then(|index| data.rows.get(index))
    else {
        return Vec::new();
    };
    data.columns
        .iter()
        .zip(row)
        .map(|(name, value)| NameValue {
            name: name.clone(),
            value: value.clone(),
        })
        .collect()
}

fn write_reports(
    options: &RunOptions,
    report: &RunReport<'_>,
    collection: &str,
    duration_ms: u64,
) -> Result<(), String> {
    if let Some(path) = &options.json {
        let json = serde_json::to_string_pretty(report)
            .map_err(|_| "the JSON report could not be built".to_owned())?;
        save(path, &json)?;
        println!("JSON report: {}", path.display());
    }
    if let Some(path) = &options.junit {
        save(path, &junit(collection, report.results, duration_ms))?;
        println!("JUnit report: {}", path.display());
    }
    Ok(())
}

fn save(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text)
        .map_err(|_| format!("{}: the report could not be written", path.display()))
}

fn print_header(paint: &Paint, loaded: &Loaded, data: Option<&RunData>) {
    println!(
        "{} {}",
        paint.bold(&format!("Codestamp CLI {}", env!("CARGO_PKG_VERSION"))),
        paint.dim(&format!("· {}", loaded.collection_name))
    );
    let mut context = Vec::new();
    if let Some(name) = &loaded.environment_name {
        context.push(format!("Environment: {name}"));
    }
    if let Some(data) = data {
        context.push(format!("Data: {} ({} rows)", data.name, data.rows.len()));
    }
    if !context.is_empty() {
        println!("{}", paint.dim(&context.join(" · ")));
    }
}

fn print_step(paint: &Paint, step: &StepResult, logs: &[LogLine]) {
    let (mark, detail) = match step.status {
        StepStatus::Passed => (paint.green("✓"), response_detail(step)),
        StepStatus::Failed if step.code.is_some() => (paint.red("✗"), response_detail(step)),
        StepStatus::Failed => (paint.red("✗"), String::new()),
        StepStatus::Skipped => (
            paint.dim("-"),
            paint.dim(&format!(
                " · skipped: {}",
                step.note.as_deref().unwrap_or("")
            )),
        ),
    };
    println!("  {mark} {:<6} {}{detail}", step.method, step.name);
    for test in &step.tests {
        match (&test.passed, &test.error) {
            (true, _) => println!("      {} {}", paint.green("✓"), test.name),
            (false, Some(error)) => println!("      {} {}: {error}", paint.red("✗"), test.name),
            (false, None) => println!("      {} {}", paint.red("✗"), test.name),
        }
    }
    for error in &step.errors {
        println!("      {} {error}", paint.red("!"));
    }
    for log in logs {
        println!(
            "      {}",
            paint.dim(&format!("│ {}: {}", log.level, log.message))
        );
    }
    if step.status != StepStatus::Skipped
        && let Some(note) = &step.note
    {
        println!("      {}", paint.dim(note));
    }
}

fn response_detail(step: &StepResult) -> String {
    let code = step.code.map(|code| code.to_string()).unwrap_or_default();
    let elapsed = step
        .elapsed_ms
        .map(|ms| format!(" · {ms} ms"))
        .unwrap_or_default();
    format!(" · {code} {}{elapsed}", step.status_text)
        .trim_end()
        .to_owned()
}

fn print_summary(paint: &Paint, summary: &Summary, duration_ms: u64) {
    let failed = |count: usize| {
        let text = format!("{count} failed");
        if count > 0 { paint.red(&text) } else { text }
    };
    println!();
    println!(
        "Requests  {} · {} passed · {} · {} skipped",
        summary.total,
        summary.passed,
        failed(summary.failed),
        summary.skipped
    );
    println!(
        "Tests     {} · {}",
        summary.tests,
        failed(summary.failed_tests)
    );
    println!(
        "Time      {}.{} s",
        duration_ms / 1000,
        duration_ms % 1000 / 100
    );
}

/// Colours only for a terminal, and never when `NO_COLOR` is set (no-color.org).
struct Paint {
    on: bool,
}

impl Paint {
    fn detect() -> Self {
        Self {
            on: std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    fn wrap(&self, code: &str, text: &str) -> String {
        if self.on {
            format!("\u{1b}[{code}m{text}\u{1b}[0m")
        } else {
            text.to_owned()
        }
    }

    fn green(&self, text: &str) -> String {
        self.wrap("32", text)
    }

    fn red(&self, text: &str) -> String {
        self.wrap("31", text)
    }

    fn yellow(&self, text: &str) -> String {
        self.wrap("33", text)
    }

    fn dim(&self, text: &str) -> String {
        self.wrap("2", text)
    }

    fn bold(&self, text: &str) -> String {
        self.wrap("1", text)
    }
}
