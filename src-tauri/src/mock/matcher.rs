//! Which saved example answers a mock request: the request with the same method whose path
//! matches (`:id` and `{{variable}}` segments match anything), then the example asked for by
//! header, or the first successful one.

use crate::request::path_vars::{path_template, variable_name};
use crate::storage::examples::{Example, ExampleOwner};

/// The request for a method and path; with several, the one with the fewest wildcards.
pub fn best<'c>(
    candidates: &'c [ExampleOwner],
    method: &str,
    path: &str,
) -> Option<&'c ExampleOwner> {
    candidates
        .iter()
        .filter(|candidate| candidate.method.eq_ignore_ascii_case(method))
        .filter_map(|candidate| {
            wildcards_matching(path_template(&candidate.url), path).map(|count| (count, candidate))
        })
        .min_by_key(|(count, _)| *count)
        .map(|(_, candidate)| candidate)
}

/// How many wildcard segments the pattern used to match, or `None` when it does not match.
fn wildcards_matching(pattern: &str, path: &str) -> Option<usize> {
    let pattern: Vec<&str> = pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let path: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if pattern.len() != path.len() {
        return None;
    }
    let mut wildcards = 0;
    for (expected, actual) in pattern.iter().zip(&path) {
        if variable_name(expected).is_some() || expected.contains("{{") {
            wildcards += 1;
        } else if expected != actual {
            return None;
        }
    }
    Some(wildcards)
}

/// The example named by `x-mock-response-name`, or with the status in `x-mock-response-code`
/// (Postman's mock headers), otherwise the first successful one, otherwise the first.
pub fn choose<'e>(
    examples: &'e [Example],
    name: Option<&str>,
    code: Option<u16>,
) -> Option<&'e Example> {
    if let Some(name) = name {
        return examples
            .iter()
            .find(|example| example.name.eq_ignore_ascii_case(name.trim()));
    }
    if let Some(code) = code {
        return examples.iter().find(|example| example.status == code);
    }
    examples
        .iter()
        .find(|example| (200..300).contains(&example.status))
        .or_else(|| examples.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(method: &str, url: &str) -> ExampleOwner {
        ExampleOwner {
            request_id: url.to_owned(),
            method: method.to_owned(),
            url: url.to_owned(),
        }
    }

    fn example(name: &str, status: u16) -> Example {
        Example {
            id: name.to_owned(),
            request_id: String::new(),
            name: name.to_owned(),
            status,
            headers: Vec::new(),
            body: String::new(),
        }
    }

    #[test]
    fn picks_the_most_specific_request() {
        let candidates = [
            candidate("GET", "{{baseUrl}}/users/:id"),
            candidate("GET", "{{baseUrl}}/users/me"),
            candidate("POST", "{{baseUrl}}/users"),
            candidate("GET", "https://api.test/{{version}}/health?full=1"),
        ];
        assert_eq!(
            best(&candidates, "GET", "/users/42").map(|c| c.url.as_str()),
            Some("{{baseUrl}}/users/:id")
        );
        assert_eq!(
            best(&candidates, "get", "/users/me/").map(|c| c.url.as_str()),
            Some("{{baseUrl}}/users/me")
        );
        assert_eq!(
            best(&candidates, "POST", "/users").map(|c| c.method.as_str()),
            Some("POST")
        );
        assert!(best(&candidates, "GET", "/v2/health").is_some());
        assert!(best(&candidates, "DELETE", "/users/42").is_none());
        assert!(best(&candidates, "GET", "/users/42/orders").is_none());
    }

    #[test]
    fn chooses_the_asked_for_or_successful_example() {
        let examples = [
            example("Missing", 404),
            example("Found", 200),
            example("Created", 201),
        ];
        assert_eq!(choose(&examples, None, None).map(|e| e.status), Some(200));
        assert_eq!(
            choose(&examples, Some("missing"), None).map(|e| e.status),
            Some(404)
        );
        assert_eq!(
            choose(&examples, None, Some(201)).map(|e| e.name.as_str()),
            Some("Created")
        );
        assert!(choose(&examples, Some("nope"), None).is_none());
        assert_eq!(
            choose(&examples[..1], None, None).map(|e| e.status),
            Some(404)
        );
    }
}
