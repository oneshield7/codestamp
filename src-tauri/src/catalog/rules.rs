use std::collections::HashMap;

use url::Url;

use super::models::{Finding, FindingRequest, RuleResult, Severity};
use crate::auth::models::Auth;
use crate::request::validate::is_loopback_host;
use crate::sensitive::{is_sensitive_name, is_written_out};
use crate::storage::models::{KeyValue, RequestKind};

/// What the rules need to know about one saved request.
pub struct RequestFacts {
    pub id: String,
    pub name: String,
    pub method: String,
    pub kind: RequestKind,
    pub url: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub auth: Auth,
    pub has_tests: bool,
    pub examples: i64,
}

/// Schemes that send everything, credentials included, unencrypted.
const PLAIN_SCHEMES: [&str; 3] = ["http", "ws", "grpc"];

/// Every rule, checked against one collection.
pub fn check(requests: &[RequestFacts], collection_auth: &Auth) -> Vec<RuleResult> {
    vec![
        secrets(requests, collection_auth),
        encrypted(requests),
        address_variable(requests),
        tests(requests),
        examples(requests),
        unique_names(requests),
    ]
}

fn secrets(requests: &[RequestFacts], collection_auth: &Auth) -> RuleResult {
    let mut findings: Vec<Finding> = requests
        .iter()
        .filter_map(|request| {
            let mut found = written_rows(&request.headers, "header");
            found.extend(written_rows(&request.params, "parameter"));
            found.extend(written_auth(&request.auth));
            (!found.is_empty())
                .then(|| finding(request, format!("Written out: {}", found.join(", "))))
        })
        .collect();
    let in_collection = written_auth(collection_auth);
    if !in_collection.is_empty() {
        findings.push(Finding {
            request: None,
            detail: format!("Written out: {}", in_collection.join(", ")),
        });
    }
    RuleResult {
        key: "secrets",
        title: "Secrets are kept in variables",
        description: "Tokens, passwords and API keys typed into a request are saved as plain text and shared with the collection. Keep them in a secret variable and write {{name}} instead.",
        severity: Severity::Error,
        checked: requests.len() + 1,
        findings,
    }
}

fn encrypted(requests: &[RequestFacts]) -> RuleResult {
    let mut checked = 0;
    let mut findings = Vec::new();
    for request in requests {
        let Some(url) = written_address(request) else {
            continue;
        };
        checked += 1;
        let host = url.host_str().unwrap_or_default();
        if PLAIN_SCHEMES.contains(&url.scheme()) && !is_loopback_host(host) {
            findings.push(finding(
                request,
                format!("Sent over {}:// to {host}", url.scheme()),
            ));
        }
    }
    RuleResult {
        key: "encrypted",
        title: "Requests use encrypted connections",
        description: "http://, ws:// and grpc:// send everything, tokens included, unencrypted. Requests to this machine (localhost) are fine.",
        severity: Severity::Warning,
        checked,
        findings,
    }
}

fn address_variable(requests: &[RequestFacts]) -> RuleResult {
    let applicable: Vec<&RequestFacts> = requests
        .iter()
        .filter(|request| has_address(request) && !request.url.trim().is_empty())
        .collect();
    let findings = applicable
        .iter()
        .filter(|request| !request.url.trim_start().starts_with("{{"))
        .map(|request| {
            let detail = written_address(request).map_or_else(
                || "Starts with a written-out address".to_owned(),
                |url| format!("Starts with {}", url.origin().ascii_serialization()),
            );
            finding(request, detail)
        })
        .collect();
    RuleResult {
        key: "address",
        title: "Server addresses come from a variable",
        description: "A URL that starts with a variable such as {{baseUrl}} lets one environment move every request to another server, staging or production.",
        severity: Severity::Warning,
        checked: applicable.len(),
        findings,
    }
}

fn tests(requests: &[RequestFacts]) -> RuleResult {
    let runnable = runnable(requests);
    let findings = runnable
        .iter()
        .filter(|request| !request.has_tests)
        .map(|request| finding(request, "No test script".to_owned()))
        .collect();
    RuleResult {
        key: "tests",
        title: "Requests have tests",
        description: "A test script (pm.test) is how Run all and codestamp-cli tell a right answer from a wrong one.",
        severity: Severity::Warning,
        checked: runnable.len(),
        findings,
    }
}

fn examples(requests: &[RequestFacts]) -> RuleResult {
    let runnable = runnable(requests);
    let findings = runnable
        .iter()
        .filter(|request| request.examples == 0)
        .map(|request| finding(request, "No saved example".to_owned()))
        .collect();
    RuleResult {
        key: "examples",
        title: "Requests have a saved example",
        description: "An example shows what a good answer looks like, and the mock server answers with it.",
        severity: Severity::Warning,
        checked: runnable.len(),
        findings,
    }
}

fn unique_names(requests: &[RequestFacts]) -> RuleResult {
    let mut uses: HashMap<String, usize> = HashMap::new();
    for request in requests {
        *uses.entry(name_key(&request.name)).or_default() += 1;
    }
    let findings = requests
        .iter()
        .filter_map(|request| {
            let others = uses.get(&name_key(&request.name))?.saturating_sub(1);
            (others > 0).then(|| {
                let noun = if others == 1 { "request" } else { "requests" };
                finding(request, format!("Same name as {others} other {noun}"))
            })
        })
        .collect();
    RuleResult {
        key: "names",
        title: "Request names are unique",
        description: "Requests that share a name are hard to tell apart in reports, search and Orchestration.",
        severity: Severity::Warning,
        checked: requests.len(),
        findings,
    }
}

fn finding(request: &RequestFacts, detail: String) -> Finding {
    Finding {
        request: Some(FindingRequest {
            id: request.id.clone(),
            name: request.name.clone(),
            method: request.method.clone(),
            kind: request.kind,
        }),
        detail,
    }
}

fn runnable(requests: &[RequestFacts]) -> Vec<&RequestFacts> {
    requests
        .iter()
        .filter(|request| request.kind.run_skip_reason().is_none())
        .collect()
}

/// Kinds whose URL is a server address; an MCP request's is one only over HTTP.
fn has_address(request: &RequestFacts) -> bool {
    request.kind != RequestKind::Mcp || request.method == "HTTP"
}

/// The address when the URL spells it out; `None` when it starts with a variable.
fn written_address(request: &RequestFacts) -> Option<Url> {
    let url = request.url.trim();
    if !has_address(request) || url.starts_with("{{") {
        return None;
    }
    Url::parse(url)
        .ok()
        .filter(|parsed| parsed.host_str().is_some())
}

fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Enabled rows that carry a credential typed in as it is.
fn written_rows(rows: &[KeyValue], what: &str) -> Vec<String> {
    rows.iter()
        .filter(|row| row.enabled && is_sensitive_name(&row.key) && is_written_out(&row.value))
        .map(|row| format!("{} {what}", row.key.trim()))
        .collect()
}

fn written_auth(auth: &Auth) -> Vec<String> {
    let fields: Vec<(&str, &String)> = match auth {
        Auth::Inherit | Auth::None => Vec::new(),
        Auth::Bearer { token, .. } => vec![("Bearer token", token)],
        Auth::Basic { password, .. } => vec![("Basic auth password", password)],
        Auth::Digest { password, .. } => vec![("Digest auth password", password)],
        Auth::ApiKey { value, .. } => vec![("API key value", value)],
        Auth::AwsSigV4 {
            secret_key,
            session_token,
            ..
        } => vec![
            ("AWS secret key", secret_key),
            ("AWS session token", session_token),
        ],
        Auth::OAuth2(settings) => vec![
            ("OAuth 2.0 client secret", &settings.client_secret),
            ("OAuth 2.0 password", &settings.password),
        ],
    };
    fields
        .into_iter()
        .filter(|(_, value)| is_written_out(value))
        .map(|(label, _)| label.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(name: &str, kind: RequestKind, url: &str) -> RequestFacts {
        RequestFacts {
            id: format!("id-{name}"),
            name: name.to_owned(),
            method: "GET".to_owned(),
            kind,
            url: url.to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            auth: Auth::Inherit,
            has_tests: true,
            examples: 1,
        }
    }

    fn row(key: &str, value: &str) -> KeyValue {
        KeyValue {
            enabled: true,
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    fn rule<'a>(results: &'a [RuleResult], key: &str) -> &'a RuleResult {
        results
            .iter()
            .find(|result| result.key == key)
            .expect("rule")
    }

    fn failing(rule: &RuleResult) -> Vec<&str> {
        rule.findings
            .iter()
            .filter_map(|finding| finding.request.as_ref())
            .map(|request| request.name.as_str())
            .collect()
    }

    #[test]
    fn finds_secrets_typed_into_requests_and_the_collection() {
        let mut typed = request("Typed", RequestKind::Http, "{{baseUrl}}/a");
        typed.headers = vec![
            row("Authorization", "Bearer abc"),
            row("X-Trace", "1"),
            KeyValue {
                enabled: false,
                key: "X-Api-Key".to_owned(),
                value: "off".to_owned(),
            },
        ];
        typed.params = vec![row("api_key", "k-123")];
        let mut variable = request("Variable", RequestKind::Http, "{{baseUrl}}/b");
        variable.headers = vec![row("Authorization", "Bearer {{token}}")];
        variable.auth = Auth::Basic {
            username: "me".to_owned(),
            password: "{{password}}".to_owned(),
        };
        let collection = Auth::Bearer {
            token: "plain".to_owned(),
            prefix: "Bearer".to_owned(),
        };

        let results = check(&[typed, variable], &collection);
        let secrets = rule(&results, "secrets");
        assert_eq!(secrets.checked, 3);
        assert_eq!(secrets.findings.len(), 2);
        assert_eq!(
            secrets.findings[0].detail,
            "Written out: Authorization header, api_key parameter"
        );
        assert_eq!(failing(secrets), ["Typed"]);
        assert_eq!(secrets.findings[1].request, None);
        assert_eq!(secrets.findings[1].detail, "Written out: Bearer token");
    }

    #[test]
    fn plain_connections_fail_except_on_this_machine() {
        let results = check(
            &[
                request("Plain", RequestKind::Http, "http://api.example.com/a"),
                request("Local", RequestKind::Http, "http://localhost:3000/a"),
                request("Socket", RequestKind::Websocket, "ws://chat.example.com"),
                request("Secure", RequestKind::Http, "https://api.example.com/a"),
                request("Variable", RequestKind::Http, "{{baseUrl}}/a"),
            ],
            &Auth::None,
        );
        let encrypted = rule(&results, "encrypted");
        assert_eq!(encrypted.checked, 4);
        assert_eq!(failing(encrypted), ["Plain", "Socket"]);
        assert_eq!(
            encrypted.findings[0].detail,
            "Sent over http:// to api.example.com"
        );

        let address = rule(&results, "address");
        assert_eq!(address.checked, 5);
        assert_eq!(address.findings.len(), 4);
        assert_eq!(
            address.findings[0].detail,
            "Starts with http://api.example.com"
        );
    }

    #[test]
    fn tests_and_examples_only_count_requests_that_run() {
        let mut untested = request("Untested", RequestKind::Graphql, "{{gql}}");
        untested.has_tests = false;
        untested.examples = 0;
        let mut socket = request("Socket", RequestKind::Websocket, "{{ws}}");
        socket.has_tests = false;
        socket.examples = 0;
        let mut stdio = request("Server", RequestKind::Mcp, "npx server");
        stdio.method = "STDIO".to_owned();
        let results = check(
            &[
                untested,
                socket,
                stdio,
                request("Tested", RequestKind::Soap, "{{soap}}"),
            ],
            &Auth::None,
        );
        assert_eq!(rule(&results, "tests").checked, 2);
        assert_eq!(failing(rule(&results, "tests")), ["Untested"]);
        assert_eq!(rule(&results, "examples").findings.len(), 1);
        // The STDIO server's "URL" is a command, not an address.
        assert_eq!(rule(&results, "address").checked, 3);
    }

    #[test]
    fn names_are_compared_without_case() {
        let results = check(
            &[
                request("Login", RequestKind::Http, "{{a}}"),
                request(" login", RequestKind::Http, "{{a}}"),
                request("Logout", RequestKind::Http, "{{a}}"),
            ],
            &Auth::None,
        );
        let names = rule(&results, "names");
        assert_eq!(names.findings.len(), 2);
        assert_eq!(names.findings[0].detail, "Same name as 1 other request");
    }
}
