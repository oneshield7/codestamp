//! Reads values out of a response and checks it, for the Extract and Checks of a flow step.

use serde::Serialize;
use serde_json::Value;

use super::models::{Check, CheckOp, CheckSubject, Extract, ExtractSource};
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::request::types::NameValue;

/// Shown values are cut to this many characters; the full value is still passed on.
const MAX_SHOWN_CHARS: usize = 120;
const TRUNCATED_BODY: &str = "the body is larger than the 2 MB that is kept, so it cannot be read";

/// The parts of a response that extracts and checks look at.
pub struct Answer<'a> {
    pub status: u16,
    pub headers: &'a [NameValue],
    pub body: &'a str,
    pub body_truncated: bool,
    pub elapsed_ms: u64,
}

impl Answer<'_> {
    fn json(&self) -> Result<Value, String> {
        if self.body_truncated {
            return Err(TRUNCATED_BODY.to_owned());
        }
        serde_json::from_str(self.body).map_err(|_| "the body is not JSON".to_owned())
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    /// The check in words, such as `status equals 201`.
    pub label: String,
    pub passed: bool,
    /// What was found instead, when it failed.
    pub detail: Option<String>,
}

/// The value to pass on, or why it could not be read.
pub fn extract(rule: &Extract, answer: &Answer<'_>) -> Result<String, String> {
    let value = match rule.source {
        ExtractSource::Header => answer
            .header(&rule.path)
            .map(str::to_owned)
            .ok_or_else(|| format!("{}: the response has no {} header", rule.name, rule.path))?,
        ExtractSource::Body => {
            let json = answer
                .json()
                .map_err(|reason| format!("{}: {reason}", rule.name))?;
            lookup(&json, &rule.path)
                .map(text)
                .ok_or_else(|| format!("{}: {} is not in the body", rule.name, rule.path))?
        }
    };
    if value.len() > MAX_KEY_VALUE_LENGTH {
        return Err(format!("{}: the value is longer than 8 KB", rule.name));
    }
    Ok(value)
}

pub fn evaluate(check: &Check, answer: &Answer<'_>) -> CheckResult {
    let label = describe(check);
    let outcome = match check.subject {
        CheckSubject::Status => compare_number(f64::from(answer.status), check)
            .map(|passed| (passed, format!("got {}", answer.status))),
        CheckSubject::Time => compare_number(lossy(answer.elapsed_ms), check)
            .map(|passed| (passed, format!("took {} ms", answer.elapsed_ms))),
        CheckSubject::Header => Ok(compare_text(answer.header(&check.path), check)),
        CheckSubject::Body if check.path.is_empty() => {
            if answer.body_truncated {
                Err(TRUNCATED_BODY.to_owned())
            } else {
                Ok(compare_text(
                    Some(answer.body).filter(|body| !body.is_empty()),
                    check,
                ))
            }
        }
        CheckSubject::Body => answer.json().map(|json| {
            let found = lookup(&json, &check.path).map(text);
            if check.op == CheckOp::LessThan {
                match found.as_deref().map(str::parse::<f64>) {
                    Some(Ok(number)) => compare_number(number, check)
                        .map_or((false, "not a number".to_owned()), |passed| {
                            (passed, format!("got {}", shown(&number.to_string())))
                        }),
                    Some(Err(_)) => (false, "not a number".to_owned()),
                    None => (false, "missing".to_owned()),
                }
            } else {
                compare_text(found.as_deref(), check)
            }
        }),
    };
    match outcome {
        Ok((passed, detail)) => CheckResult {
            label,
            passed,
            detail: (!passed).then_some(detail),
        },
        Err(reason) => CheckResult {
            label,
            passed: false,
            detail: Some(reason),
        },
    }
}

/// Follows a path such as `data.items[0].id`; a leading `$.` is allowed.
pub fn lookup<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.strip_prefix('$').unwrap_or(path);
    let mut current = value;
    for segment in path.split('.').filter(|segment| !segment.is_empty()) {
        let (key, mut rest) = segment
            .find('[')
            .map_or((segment, ""), |at| (&segment[..at], &segment[at..]));
        if !key.is_empty() {
            current = current.get(key)?;
        }
        while let Some(inside) = rest.strip_prefix('[') {
            let end = inside.find(']')?;
            current = current.get(inside[..end].trim().parse::<usize>().ok()?)?;
            rest = &inside[end + 1..];
        }
        if !rest.is_empty() {
            return None;
        }
    }
    Some(current)
}

/// Text as a `{{variable}}` gets it: strings without quotes, everything else as JSON.
fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn compare_number(actual: f64, check: &Check) -> Result<bool, String> {
    let expected = check
        .value
        .parse::<f64>()
        .map_err(|_| "the expected value is not a number".to_owned())?;
    Ok(match check.op {
        CheckOp::Equals => (actual - expected).abs() < f64::EPSILON,
        CheckOp::NotEquals => (actual - expected).abs() >= f64::EPSILON,
        CheckOp::LessThan => actual < expected,
        CheckOp::Contains | CheckOp::Exists => false,
    })
}

fn compare_text(found: Option<&str>, check: &Check) -> (bool, String) {
    let Some(actual) = found else {
        return (check.op == CheckOp::NotEquals, "missing".to_owned());
    };
    let passed = match check.op {
        CheckOp::Exists => true,
        CheckOp::Equals => actual == check.value,
        CheckOp::NotEquals => actual != check.value,
        CheckOp::Contains => actual.contains(&check.value),
        CheckOp::LessThan => false,
    };
    (passed, format!("got {}", shown(actual)))
}

fn describe(check: &Check) -> String {
    let subject = match check.subject {
        CheckSubject::Status => "status".to_owned(),
        CheckSubject::Time => "time".to_owned(),
        CheckSubject::Header => format!("header {}", check.path),
        CheckSubject::Body if check.path.is_empty() => "body".to_owned(),
        CheckSubject::Body => check.path.clone(),
    };
    let unit = if check.subject == CheckSubject::Time {
        " ms"
    } else {
        ""
    };
    match check.op {
        CheckOp::Exists => format!("{subject} exists"),
        CheckOp::Equals => format!("{subject} equals {}{unit}", check.value),
        CheckOp::NotEquals => format!("{subject} is not {}{unit}", check.value),
        CheckOp::Contains => format!("{subject} contains {}", check.value),
        CheckOp::LessThan => format!("{subject} is less than {}{unit}", check.value),
    }
}

fn shown(text: &str) -> String {
    if text.chars().count() <= MAX_SHOWN_CHARS {
        return text.to_owned();
    }
    let cut: String = text.chars().take(MAX_SHOWN_CHARS).collect();
    format!("{cut}…")
}

/// Milliseconds compare well as floating point long before precision is lost.
fn lossy(millis: u64) -> f64 {
    u32::try_from(millis).map_or(f64::MAX, f64::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answer<'a>(body: &'a str, headers: &'a [NameValue]) -> Answer<'a> {
        Answer {
            status: 201,
            headers,
            body,
            body_truncated: false,
            elapsed_ms: 120,
        }
    }

    fn check(subject: CheckSubject, path: &str, op: CheckOp, value: &str) -> Check {
        Check {
            subject,
            path: path.to_owned(),
            op,
            value: value.to_owned(),
        }
    }

    #[test]
    fn follows_paths() {
        let value = json!({ "data": { "items": [{ "id": 7 }, { "id": 8 }] }, "list": [[1, 2]] });
        assert_eq!(lookup(&value, "data.items[1].id"), Some(&json!(8)));
        assert_eq!(lookup(&value, "$.data.items[0].id"), Some(&json!(7)));
        assert_eq!(lookup(&value, "list[0][1]"), Some(&json!(2)));
        assert_eq!(lookup(&value, "data.items[5].id"), None);
        assert_eq!(lookup(&value, "data.items[x]"), None);
        assert_eq!(lookup(&json!([{ "id": 3 }]), "[0].id"), Some(&json!(3)));
    }

    #[test]
    fn extracts_from_body_and_headers() {
        let headers = [NameValue {
            name: "X-Request-Id".to_owned(),
            value: "abc".to_owned(),
        }];
        let body = r#"{"data":{"token":"eyJ","user":{"id":7}}}"#;
        let reply = answer(body, &headers);
        let rule = |source, path: &str| Extract {
            name: "v".to_owned(),
            source,
            path: path.to_owned(),
        };
        assert_eq!(
            extract(&rule(ExtractSource::Body, "data.token"), &reply).as_deref(),
            Ok("eyJ")
        );
        assert_eq!(
            extract(&rule(ExtractSource::Body, "data.user.id"), &reply).as_deref(),
            Ok("7")
        );
        assert_eq!(
            extract(&rule(ExtractSource::Header, "x-request-id"), &reply).as_deref(),
            Ok("abc")
        );
        assert!(extract(&rule(ExtractSource::Body, "data.nope"), &reply).is_err());
        assert!(extract(&rule(ExtractSource::Body, "a"), &answer("not json", &[])).is_err());
        let large = Answer {
            body_truncated: true,
            ..answer(body, &headers)
        };
        assert!(extract(&rule(ExtractSource::Body, "data.token"), &large).is_err());
    }

    #[test]
    fn evaluates_checks_with_readable_labels() {
        let headers = [NameValue {
            name: "Content-Type".to_owned(),
            value: "application/json".to_owned(),
        }];
        let body = r#"{"id":7,"total":"49.5","name":"Rina"}"#;
        let reply = answer(body, &headers);
        let passes = [
            check(CheckSubject::Status, "", CheckOp::Equals, "201"),
            check(CheckSubject::Status, "", CheckOp::LessThan, "300"),
            check(CheckSubject::Time, "", CheckOp::LessThan, "500"),
            check(
                CheckSubject::Header,
                "content-type",
                CheckOp::Contains,
                "json",
            ),
            check(CheckSubject::Body, "id", CheckOp::Exists, ""),
            check(CheckSubject::Body, "id", CheckOp::Equals, "7"),
            check(CheckSubject::Body, "name", CheckOp::Equals, "Rina"),
            check(CheckSubject::Body, "total", CheckOp::LessThan, "50"),
            check(CheckSubject::Body, "missing", CheckOp::NotEquals, "x"),
            check(CheckSubject::Body, "", CheckOp::Contains, "Rina"),
        ];
        for rule in &passes {
            let result = evaluate(rule, &reply);
            assert!(result.passed, "{}: {:?}", result.label, result.detail);
        }
        let failed = evaluate(
            &check(CheckSubject::Status, "", CheckOp::Equals, "200"),
            &reply,
        );
        assert_eq!(failed.label, "status equals 200");
        assert_eq!(failed.detail.as_deref(), Some("got 201"));
        let missing = evaluate(
            &check(CheckSubject::Body, "cartId", CheckOp::Exists, ""),
            &reply,
        );
        assert_eq!(
            (missing.passed, missing.detail.as_deref()),
            (false, Some("missing"))
        );
        let slow = evaluate(
            &check(CheckSubject::Time, "", CheckOp::LessThan, "100"),
            &reply,
        );
        assert_eq!(slow.label, "time is less than 100 ms");
        assert_eq!(slow.detail.as_deref(), Some("took 120 ms"));
    }
}
