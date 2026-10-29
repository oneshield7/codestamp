//! Run results and the reports written from them: JSON shaped like the app's Export report,
//! and JUnit XML, which CI systems read.

use serde::Serialize;

use crate::script::models::TestResult;
use crate::text::escape_xml;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// Counted from 1, as in the app's report.
    pub iteration: u32,
    pub name: String,
    /// The method, or the kind's stamp (GQL, WS, MCP, gRPC).
    pub method: String,
    pub url: String,
    pub status: StepStatus,
    pub code: Option<u16>,
    /// Printed with the code; the app's report leaves it out too.
    #[serde(skip)]
    pub status_text: String,
    pub elapsed_ms: Option<u64>,
    pub tests: Vec<TestResult>,
    pub errors: Vec<String>,
    pub note: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub tests: usize,
    pub failed_tests: usize,
}

impl Summary {
    pub fn of(results: &[StepResult]) -> Self {
        let count = |status| results.iter().filter(|step| step.status == status).count();
        let tests = results.iter().flat_map(|step| &step.tests);
        Self {
            total: results.len(),
            passed: count(StepStatus::Passed),
            failed: count(StepStatus::Failed),
            skipped: count(StepStatus::Skipped),
            tests: tests.clone().count(),
            failed_tests: tests.filter(|test| !test.passed).count(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport<'a> {
    pub app: &'static str,
    pub collection: &'a str,
    pub environment: Option<&'a str>,
    pub data_file: Option<&'a str>,
    pub iterations: u32,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    pub summary: Summary,
    pub results: &'a [StepResult],
}

/// A request passes when it got a response, its scripts ran cleanly and every test passed.
/// Without tests, an error status (4xx/5xx) counts as a failure, as in the app's runner.
pub fn passed(code: Option<u16>, tests: &[TestResult], errors: &[String]) -> bool {
    let Some(code) = code else {
        return false;
    };
    if !errors.is_empty() || tests.iter().any(|test| !test.passed) {
        return false;
    }
    !tests.is_empty() || code < 400
}

/// One test suite per request run, one test case per test; a request without tests is one
/// case of its own.
pub fn junit(collection: &str, results: &[StepResult], duration_ms: u64) -> String {
    let summary = Summary::of(results);
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    line(
        &mut xml,
        &format!(
            "<testsuites name=\"{}\" tests=\"{}\" failures=\"{}\" skipped=\"{}\" time=\"{}\">",
            escape_xml(collection),
            results.iter().map(case_count).sum::<usize>(),
            results.iter().map(failure_count).sum::<usize>(),
            summary.skipped,
            seconds(duration_ms),
        ),
    );
    let several = results.iter().any(|step| step.iteration > 1);
    for step in results {
        let suite = if several {
            format!("{} (iteration {})", step.name, step.iteration)
        } else {
            step.name.clone()
        };
        let time = seconds(step.elapsed_ms.unwrap_or(0));
        line(
            &mut xml,
            &format!(
                "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" skipped=\"{}\" time=\"{time}\">",
                escape_xml(&suite),
                case_count(step),
                failure_count(step),
                usize::from(step.status == StepStatus::Skipped),
            ),
        );
        let class = escape_xml(&format!("{collection}.{}", step.name));
        if step.status == StepStatus::Skipped {
            line(
                &mut xml,
                &format!(
                    "    <testcase name=\"{}\" classname=\"{class}\"><skipped message=\"{}\"/></testcase>",
                    escape_xml(&format!("{} {}", step.method, step.url)),
                    escape_xml(step.note.as_deref().unwrap_or("Skipped")),
                ),
            );
        } else if step.tests.is_empty() {
            let name = escape_xml(&format!("{} {}", step.method, step.url));
            match failure_text(step) {
                Some(message) => {
                    line(
                        &mut xml,
                        &format!(
                            "    <testcase name=\"{name}\" classname=\"{class}\" time=\"{time}\"><failure message=\"{}\"/></testcase>",
                            escape_xml(&message)
                        ),
                    );
                }
                None => {
                    line(
                        &mut xml,
                        &format!(
                            "    <testcase name=\"{name}\" classname=\"{class}\" time=\"{time}\"/>"
                        ),
                    );
                }
            }
        } else {
            for test in &step.tests {
                let name = escape_xml(&test.name);
                if test.passed {
                    line(
                        &mut xml,
                        &format!("    <testcase name=\"{name}\" classname=\"{class}\"/>"),
                    );
                } else {
                    line(
                        &mut xml,
                        &format!(
                            "    <testcase name=\"{name}\" classname=\"{class}\"><failure message=\"{}\"/></testcase>",
                            escape_xml(test.error.as_deref().unwrap_or("Failed"))
                        ),
                    );
                }
            }
            if !step.errors.is_empty() {
                line(
                    &mut xml,
                    &format!(
                        "    <testcase name=\"Scripts\" classname=\"{class}\"><error message=\"{}\"/></testcase>",
                        escape_xml(&step.errors.join("; "))
                    ),
                );
            }
        }
        xml.push_str("  </testsuite>\n");
    }
    xml.push_str("</testsuites>\n");
    xml
}

fn line(xml: &mut String, text: &str) {
    xml.push_str(text);
    xml.push('\n');
}

fn case_count(step: &StepResult) -> usize {
    if step.tests.is_empty() || step.status == StepStatus::Skipped {
        1
    } else {
        step.tests.len() + usize::from(!step.errors.is_empty())
    }
}

fn failure_count(step: &StepResult) -> usize {
    match step.status {
        StepStatus::Skipped | StepStatus::Passed => 0,
        StepStatus::Failed if step.tests.is_empty() => 1,
        StepStatus::Failed => {
            step.tests.iter().filter(|test| !test.passed).count()
                + usize::from(!step.errors.is_empty())
        }
    }
}

/// Why a request without tests failed.
fn failure_text(step: &StepResult) -> Option<String> {
    if step.status != StepStatus::Failed {
        return None;
    }
    if !step.errors.is_empty() {
        return Some(step.errors.join("; "));
    }
    Some(match step.code {
        Some(code) => format!("The server answered {code}"),
        None => "No response".to_owned(),
    })
}

fn seconds(millis: u64) -> String {
    format!("{}.{:03}", millis / 1000, millis % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(status: StepStatus, tests: Vec<TestResult>) -> StepResult {
        StepResult {
            iteration: 1,
            name: "List <users>".to_owned(),
            method: "GET".to_owned(),
            url: "{{baseUrl}}/users?a=1&b=2".to_owned(),
            status,
            code: Some(200),
            status_text: "OK".to_owned(),
            elapsed_ms: Some(1234),
            tests,
            errors: Vec::new(),
            note: None,
        }
    }

    fn test(name: &str, passed: bool) -> TestResult {
        TestResult {
            name: name.to_owned(),
            passed,
            error: (!passed).then(|| "expected 200 to equal \"201\"".to_owned()),
        }
    }

    #[test]
    fn decides_like_the_app() {
        assert!(passed(Some(200), &[], &[]));
        assert!(!passed(Some(404), &[], &[]));
        assert!(passed(Some(404), &[test("not found is fine", true)], &[]));
        assert!(!passed(Some(200), &[test("t", false)], &[]));
        assert!(!passed(Some(200), &[], &["ReferenceError".to_owned()]));
        assert!(!passed(None, &[], &[]));
    }

    #[test]
    fn writes_junit_with_escaped_text() {
        let results = [
            step(StepStatus::Passed, vec![test("status is 200", true)]),
            step(StepStatus::Failed, vec![test("created", false)]),
            StepResult {
                note: Some("WebSocket requests are not part of a run".to_owned()),
                ..step(StepStatus::Skipped, Vec::new())
            },
        ];
        let xml = junit("Users & co", &results, 2500);
        assert!(xml.contains(
            "<testsuites name=\"Users &amp; co\" tests=\"3\" failures=\"1\" skipped=\"1\" time=\"2.500\">"
        ));
        assert!(xml.contains("name=\"List &lt;users&gt;\""));
        assert!(xml.contains("<failure message=\"expected 200 to equal &quot;201&quot;\"/>"));
        assert!(xml.contains("<skipped message=\"WebSocket requests are not part of a run\"/>"));
        assert!(!xml.contains("a=1&b"));
        let summary = Summary::of(&results);
        assert_eq!((summary.passed, summary.failed, summary.skipped), (1, 1, 1));
        assert_eq!((summary.tests, summary.failed_tests), (2, 1));
    }
}
