use std::collections::{BTreeSet, HashMap};

use super::dynamic;
use crate::error::AppError;

const OPEN: &str = "{{";
const CLOSE: &str = "}}";
const MAX_NAME_LENGTH: usize = 100;
/// Room for a name plus the spaces allowed around it, as in `{{ name }}`.
const MAX_REFERENCE_SPAN: usize = MAX_NAME_LENGTH + 64;
const DYNAMIC_PREFIX: char = '$';

/// A name a variable can have. `$` is kept for dynamic variables such as `{{$guid}}`.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_LENGTH
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn is_reference_name(name: &str) -> bool {
    is_valid_name(name) || name.strip_prefix(DYNAMIC_PREFIX).is_some_and(is_valid_name)
}

/// What to do with `{{$name}}` references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dynamic {
    /// Fill in a fresh value for each one (when sending).
    Generate,
    /// Leave known ones in place (previews, where the value would change on every refresh).
    Keep,
}

/// Adds every `{{name}}` found in `text` to `names`. Braces around anything that is not a
/// valid name (for example a template in a JSON body) are left alone.
pub fn collect_names(text: &str, names: &mut BTreeSet<String>) {
    for_each_reference(text, |_, name| {
        names.insert(name.to_owned());
    });
}

/// Replaces each `{{name}}` with its value and fails on the first unknown name.
pub fn substitute(text: &str, values: &HashMap<String, String>) -> Result<String, AppError> {
    let (output, missing) = replace_known(text, values, Dynamic::Generate);
    match missing.first() {
        Some(name) if name.starts_with(DYNAMIC_PREFIX) => Err(AppError::InvalidInput(format!(
            "{{{{{name}}}}} is not a dynamic variable Codestamp supports"
        ))),
        Some(name) => Err(AppError::InvalidInput(format!(
            "The variable {{{{{name}}}}} is not defined in Globals, this collection or the active environment"
        ))),
        None => Ok(output),
    }
}

/// Replaces the names it knows, leaves unknown `{{name}}` in place and lists them.
/// Values are inserted as-is and are not scanned again, so a value containing `{{...}}`
/// cannot trigger further lookups.
pub fn replace_known(
    text: &str,
    values: &HashMap<String, String>,
    dynamic: Dynamic,
) -> (String, Vec<String>) {
    let mut output = String::with_capacity(text.len());
    let mut copied_until = 0;
    let mut missing = Vec::new();
    for_each_reference(text, |range, name| {
        let generated;
        let value = if name.starts_with(DYNAMIC_PREFIX) {
            match dynamic {
                Dynamic::Generate => {
                    generated = dynamic::generate(name);
                    generated.as_deref()
                }
                Dynamic::Keep if dynamic::is_dynamic(name) => return,
                Dynamic::Keep => None,
            }
        } else {
            values.get(name).map(String::as_str)
        };
        match value {
            Some(value) => {
                output.push_str(&text[copied_until..range.0]);
                output.push_str(value);
                copied_until = range.1;
            }
            None => missing.push(name.to_owned()),
        }
    });
    output.push_str(&text[copied_until..]);
    (output, missing)
}

fn for_each_reference(text: &str, mut visit: impl FnMut((usize, usize), &str)) {
    let mut cursor = 0;
    // The first `}}` after the last search. Searching again from every `{{` would make texts
    // full of `{{` take quadratic time, so it is only looked for again once the scan passes it.
    let mut next_close: Option<usize> = None;
    while let Some(open) = text[cursor..].find(OPEN).map(|offset| cursor + offset) {
        let inner_start = open + OPEN.len();
        let close = match next_close {
            Some(close) if close >= inner_start => close,
            _ => match text[inner_start..].find(CLOSE) {
                Some(offset) => inner_start + offset,
                None => return,
            },
        };
        next_close = Some(close);
        if close + CLOSE.len() - inner_start > MAX_REFERENCE_SPAN {
            cursor = inner_start;
            continue;
        }
        let name = text[inner_start..close].trim();
        let end = close + CLOSE.len();
        if is_reference_name(name) {
            visit((open, end), name);
            cursor = end;
        } else {
            cursor = inner_start;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> HashMap<String, String> {
        HashMap::from([
            (
                "baseUrl".to_owned(),
                "https://api.dev.example.com".to_owned(),
            ),
            ("token".to_owned(), "abc{{baseUrl}}".to_owned()),
        ])
    }

    #[test]
    fn collects_only_valid_names() {
        let mut names = BTreeSet::new();
        collect_names(
            "{{baseUrl}}/users?q={{ token }}&x={{not valid}}{{",
            &mut names,
        );
        assert_eq!(names.into_iter().collect::<Vec<_>>(), ["baseUrl", "token"]);
    }

    #[test]
    fn substitutes_known_variables() {
        let result = substitute("{{baseUrl}}/v1/users", &values());
        assert_eq!(
            result.ok().as_deref(),
            Some("https://api.dev.example.com/v1/users")
        );
    }

    #[test]
    fn does_not_expand_variables_inside_values() {
        let result = substitute("Bearer {{token}}", &values());
        assert_eq!(result.ok().as_deref(), Some("Bearer abc{{baseUrl}}"));
    }

    #[test]
    fn reports_undefined_variables() {
        let result = substitute("{{missing}}/x", &values());
        assert!(
            matches!(result, Err(AppError::InvalidInput(message)) if message.contains("{{missing}}"))
        );
    }

    #[test]
    fn scans_large_texts_in_linear_time() {
        let braces = "{{".repeat(500_000);
        let closed_far_away = format!("{braces}}}}} {{{{token}}}}");
        let started = std::time::Instant::now();
        let mut names = BTreeSet::new();
        collect_names(&closed_far_away, &mut names);
        collect_names(&braces, &mut names);
        assert_eq!(names.into_iter().collect::<Vec<_>>(), ["token"]);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn ignores_braces_that_close_too_far_away() {
        let long_gap = " ".repeat(MAX_REFERENCE_SPAN);
        let mut names = BTreeSet::new();
        collect_names(&format!("{{{{{long_gap}far}}}} {{{{ near }}}}"), &mut names);
        assert_eq!(names.into_iter().collect::<Vec<_>>(), ["near"]);
    }

    #[test]
    fn fills_in_dynamic_variables_fresh_for_each_reference() {
        let text = "{{$guid}}|{{ $guid }}|{{baseUrl}}";
        let result = substitute(text, &values()).expect("substituted");
        let parts: Vec<&str> = result.split('|').collect();
        assert_eq!(parts.len(), 3);
        assert_ne!(parts.first(), parts.get(1));
        assert_eq!(parts.get(2), Some(&"https://api.dev.example.com"));

        let unknown = substitute("{{$randomFirstName}}", &values());
        assert!(
            matches!(unknown, Err(AppError::InvalidInput(message)) if message.contains("dynamic variable"))
        );
    }

    #[test]
    fn previews_keep_known_dynamic_variables() {
        let (text, missing) = replace_known("{{$guid}} {{$nope}}", &values(), Dynamic::Keep);
        assert_eq!(text, "{{$guid}} {{$nope}}");
        assert_eq!(missing, ["$nope"]);
    }

    #[test]
    fn leaves_non_variable_braces_untouched() {
        let text = r#"{"template": "{{ not a name }}", "open": "{{"}"#;
        assert_eq!(substitute(text, &values()).ok().as_deref(), Some(text));
    }
}
