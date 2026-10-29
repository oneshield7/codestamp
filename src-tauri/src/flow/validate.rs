use std::collections::HashSet;

use super::models::{Check, CheckOp, CheckSubject, Extract, ExtractSource, FlowInput};
use crate::environment::resolve::is_valid_name;
use crate::error::AppError;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::storage::validate::{check_id, clean_name};

pub const MAX_STEPS: usize = 100;
const MAX_EXTRACTS: usize = 20;
const MAX_CHECKS: usize = 20;
const MAX_PATH_CHARS: usize = 512;
const MAX_TIME_MS: u64 = 600_000;

/// Checks a flow before it is saved; names are trimmed, values a check ignores are cleared.
pub fn check_flow(input: FlowInput) -> Result<FlowInput, AppError> {
    check_id(&input.id)?;
    let name = clean_name(&input.name)?;
    if input.steps.len() > MAX_STEPS {
        return Err(AppError::invalid("A flow can have up to 100 steps"));
    }
    let mut seen = HashSet::with_capacity(input.steps.len());
    let mut steps = Vec::with_capacity(input.steps.len());
    for mut step in input.steps {
        check_id(&step.id)?;
        if !seen.insert(step.id.clone()) {
            return Err(AppError::invalid("Two steps have the same id"));
        }
        if let Some(request_id) = &step.request_id {
            check_id(request_id)?;
        }
        if step.extracts.len() > MAX_EXTRACTS || step.checks.len() > MAX_CHECKS {
            return Err(AppError::invalid(
                "A step can have up to 20 values to pass on and 20 checks",
            ));
        }
        step.extracts = step
            .extracts
            .into_iter()
            .map(check_extract)
            .collect::<Result<_, _>>()?;
        step.checks = step
            .checks
            .into_iter()
            .map(check_check)
            .collect::<Result<_, _>>()?;
        steps.push(step);
    }
    Ok(FlowInput {
        id: input.id,
        name,
        steps,
    })
}

fn check_extract(extract: Extract) -> Result<Extract, AppError> {
    let name = extract.name.trim().to_owned();
    if !is_valid_name(&name) {
        return Err(AppError::InvalidInput(format!(
            "“{name}” cannot be a variable name: use letters, digits, - _ and ."
        )));
    }
    let path = clean_path(&extract.path)?;
    if path.is_empty() {
        return Err(AppError::InvalidInput(match extract.source {
            ExtractSource::Body => format!("Say where {name} is in the body, such as data.token"),
            ExtractSource::Header => format!("Say which header {name} comes from"),
        }));
    }
    Ok(Extract {
        name,
        source: extract.source,
        path,
    })
}

fn check_check(check: Check) -> Result<Check, AppError> {
    let path = clean_path(&check.path)?;
    if check.value.len() > MAX_KEY_VALUE_LENGTH {
        return Err(AppError::invalid("A check value is too long"));
    }
    let allowed = match check.subject {
        CheckSubject::Status => matches!(
            check.op,
            CheckOp::Equals | CheckOp::NotEquals | CheckOp::LessThan
        ),
        CheckSubject::Time => check.op == CheckOp::LessThan,
        CheckSubject::Body => true,
        CheckSubject::Header => check.op != CheckOp::LessThan,
    };
    if !allowed {
        return Err(AppError::invalid("This check cannot compare that way"));
    }
    if check.subject == CheckSubject::Header && path.is_empty() {
        return Err(AppError::invalid("Say which header to check"));
    }
    let value = if check.op == CheckOp::Exists {
        String::new()
    } else {
        check.value.trim().to_owned()
    };
    let number = |max: u64| value.parse::<u64>().ok().filter(|n| (1..=max).contains(n));
    match check.subject {
        CheckSubject::Status if number(999).is_none() => {
            return Err(AppError::invalid(
                "A status check needs a code, such as 201",
            ));
        }
        CheckSubject::Time if number(MAX_TIME_MS).is_none() => {
            return Err(AppError::invalid(
                "A time check needs a number of milliseconds, up to 600000",
            ));
        }
        CheckSubject::Body if check.op == CheckOp::LessThan && value.parse::<f64>().is_err() => {
            return Err(AppError::invalid("“Less than” needs a number"));
        }
        _ => {}
    }
    let keeps_path = matches!(check.subject, CheckSubject::Body | CheckSubject::Header);
    Ok(Check {
        subject: check.subject,
        path: if keeps_path { path } else { String::new() },
        op: check.op,
        value,
    })
}

fn clean_path(raw: &str) -> Result<String, AppError> {
    let path = raw.trim();
    if path.chars().count() > MAX_PATH_CHARS || path.chars().any(char::is_control) {
        return Err(AppError::invalid(
            "A path or header name is too long or not valid",
        ));
    }
    Ok(path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::models::{FlowStepInput, OnFailure};

    const ID: &str = "8f2c1a7e-0000-4000-8000-000000000001";
    const STEP: &str = "8f2c1a7e-0000-4000-8000-000000000002";

    fn flow(extracts: Vec<Extract>, checks: Vec<Check>) -> FlowInput {
        FlowInput {
            id: ID.to_owned(),
            name: "  Register to checkout ".to_owned(),
            steps: vec![FlowStepInput {
                id: STEP.to_owned(),
                request_id: None,
                extracts,
                checks,
                on_failure: OnFailure::Stop,
            }],
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
    fn accepts_and_cleans_a_flow() {
        let extract = Extract {
            name: " token ".to_owned(),
            source: ExtractSource::Body,
            path: "data.token".to_owned(),
        };
        let checks = vec![
            check(CheckSubject::Status, "ignored", CheckOp::Equals, " 201 "),
            check(CheckSubject::Body, "data.id", CheckOp::Exists, "ignored"),
            check(CheckSubject::Time, "", CheckOp::LessThan, "500"),
        ];
        let checked = check_flow(flow(vec![extract], checks)).expect("valid");
        assert_eq!(checked.name, "Register to checkout");
        let step = &checked.steps[0];
        assert_eq!(step.extracts[0].name, "token");
        assert_eq!(step.checks[0].path, "");
        assert_eq!(step.checks[0].value, "201");
        assert_eq!(step.checks[1].value, "");
    }

    #[test]
    fn refuses_what_cannot_run() {
        let bad_name = Extract {
            name: "my token".to_owned(),
            source: ExtractSource::Body,
            path: "token".to_owned(),
        };
        let no_path = Extract {
            name: "token".to_owned(),
            source: ExtractSource::Header,
            path: " ".to_owned(),
        };
        assert!(check_flow(flow(vec![bad_name], Vec::new())).is_err());
        assert!(check_flow(flow(vec![no_path], Vec::new())).is_err());
        for bad in [
            check(CheckSubject::Status, "", CheckOp::Contains, "2"),
            check(CheckSubject::Status, "", CheckOp::Equals, "ok"),
            check(CheckSubject::Time, "", CheckOp::Equals, "10"),
            check(CheckSubject::Header, "", CheckOp::Exists, ""),
            check(CheckSubject::Body, "total", CheckOp::LessThan, "many"),
        ] {
            assert!(
                check_flow(flow(Vec::new(), vec![bad.clone()])).is_err(),
                "{bad:?}"
            );
        }
        let mut twice = flow(Vec::new(), Vec::new());
        let copy = FlowStepInput {
            id: STEP.to_owned(),
            request_id: None,
            extracts: Vec::new(),
            checks: Vec::new(),
            on_failure: OnFailure::Continue,
        };
        twice.steps.push(copy);
        assert!(check_flow(twice).is_err());
    }
}
