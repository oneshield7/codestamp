use std::collections::HashSet;

use super::models::{CatalogInfo, RunInput};
use crate::error::AppError;
use crate::storage::validate::{MAX_NAME_CHARS, check_id};

const MAX_OWNER_CHARS: usize = 80;
const MAX_TAGS: usize = 12;
const MAX_TAG_CHARS: usize = 30;
/// The runner's own limit on requests times iterations.
const MAX_RUN_STEPS: usize = 10_000;
const MAX_TESTS_PER_SEND: u32 = 10_000;
/// UTC−12:00 to UTC+14:00, with room to spare.
const MAX_UTC_OFFSET_MINUTES: i32 = 14 * 60;

pub fn clean_info(owner: &str, tags: &[String]) -> Result<CatalogInfo, AppError> {
    let owner = owner.trim();
    if owner.chars().count() > MAX_OWNER_CHARS {
        return Err(AppError::invalid("The owner can be up to 80 characters"));
    }
    if owner.chars().any(char::is_control) {
        return Err(AppError::invalid(
            "The owner has characters that are not allowed",
        ));
    }
    if tags.len() > MAX_TAGS {
        return Err(AppError::invalid("An API can have up to 12 tags"));
    }
    let mut seen = HashSet::new();
    let mut cleaned = Vec::with_capacity(tags.len());
    for tag in tags {
        let tag = tag.trim();
        let length = tag.chars().count();
        if length == 0 || length > MAX_TAG_CHARS {
            return Err(AppError::invalid("Tags are 1 to 30 characters long"));
        }
        if !tag.chars().all(is_tag_char) {
            return Err(AppError::invalid(
                "Tags can have letters, digits, spaces and - _ . /",
            ));
        }
        if !seen.insert(tag.to_lowercase()) {
            return Err(AppError::invalid("Each tag can be added once"));
        }
        cleaned.push(tag.to_owned());
    }
    Ok(CatalogInfo {
        owner: owner.to_owned(),
        tags: cleaned,
    })
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '/')
}

pub fn check_offset(minutes: i32) -> Result<(), AppError> {
    if minutes.abs() > MAX_UTC_OFFSET_MINUTES {
        return Err(AppError::invalid("Unknown time zone"));
    }
    Ok(())
}

pub fn check_run(run: &RunInput) -> Result<(), AppError> {
    check_id(&run.collection_id)?;
    if let Some(environment) = &run.environment
        && (environment.chars().count() > MAX_NAME_CHARS
            || environment.chars().any(char::is_control))
    {
        return Err(AppError::invalid("The environment name is not valid"));
    }
    if run.started_at <= 0 || run.finished_at < run.started_at {
        return Err(AppError::invalid("The run times are not valid"));
    }
    if run.steps.len() > MAX_RUN_STEPS {
        return Err(AppError::invalid("A run has at most 10,000 steps"));
    }
    for step in &run.steps {
        check_id(&step.request_id)?;
        if step.tests_passed > step.tests_total || step.tests_total > MAX_TESTS_PER_SEND {
            return Err(AppError::invalid("The test counts are not valid"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::models::{RunStepInput, StepStatus};

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn keeps_owner_and_tags_trimmed() {
        let info = clean_info(
            "  Payments team ",
            &tags(&[" public ", "v2.1", "team/core"]),
        )
        .expect("valid");
        assert_eq!(info.owner, "Payments team");
        assert_eq!(info.tags, ["public", "v2.1", "team/core"]);
    }

    #[test]
    fn rejects_bad_tags() {
        assert!(clean_info("", &tags(&[""])).is_err());
        assert!(clean_info("", &tags(&["<script>"])).is_err());
        assert!(clean_info("", &tags(&["Public", "public"])).is_err());
        assert!(clean_info("", &tags(&["x"; 13])).is_err());
        assert!(clean_info(&"o".repeat(81), &[]).is_err());
        assert!(clean_info("a\u{7}b", &[]).is_err());
    }

    #[test]
    fn checks_runs() {
        let id = "8a0f4d2e-3c1b-4b5a-9d6e-7f8091a2b3c4";
        let step = |passed: u32, total: u32| RunStepInput {
            request_id: id.to_owned(),
            status: StepStatus::Passed,
            code: Some(200),
            elapsed_ms: Some(10),
            tests_passed: passed,
            tests_total: total,
        };
        let run = |steps: Vec<RunStepInput>, started: i64, finished: i64| RunInput {
            collection_id: id.to_owned(),
            environment: Some("Staging".to_owned()),
            started_at: started,
            finished_at: finished,
            stopped: false,
            steps,
        };
        assert!(check_run(&run(vec![step(1, 2)], 1, 2)).is_ok());
        assert!(check_run(&run(vec![step(3, 2)], 1, 2)).is_err());
        assert!(check_run(&run(vec![], 5, 2)).is_err());
        assert!(check_offset(420).is_ok());
        assert!(check_offset(-900).is_err());
    }
}
