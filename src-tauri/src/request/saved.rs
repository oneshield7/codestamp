//! A saved request turned into what is sent, as the app's request card builds it (see
//! `buildRequestInput` in the UI); the command-line runner uses it.

use super::path_vars;
use super::types::{FormPart, HttpRequestInput, Iteration, NameValue, RequestBody};
use crate::soap::SoapVersion;
use crate::storage::models::{FormFieldKind, KeyValue, RequestContent, RequestKind, SoapSettings};

/// `{{variables}}` stay in place; the pipeline fills them in when sending.
pub fn input_from_content(
    content: &RequestContent,
    collection_id: &str,
    request_id: &str,
) -> HttpRequestInput {
    let mut headers = active(&content.headers);
    let is_soap = content.kind == RequestKind::Soap;
    if is_soap {
        headers.extend(soap_defaults(content.soap.as_ref(), &content.headers));
    } else if let Some(content_type) = default_content_type(&content.body_mode)
        && !headers
            .iter()
            .any(|header| header.name.eq_ignore_ascii_case("content-type"))
    {
        headers.push(NameValue {
            name: "Content-Type".to_owned(),
            value: content_type.to_owned(),
        });
    }
    HttpRequestInput {
        method: if is_soap {
            "POST".to_owned()
        } else {
            content.method.clone()
        },
        url: content.url.trim().to_owned(),
        params: active(&content.params),
        headers,
        body: body(content),
        timeout_ms: None,
        collection_id: Some(collection_id.to_owned()),
        request_id: Some(request_id.to_owned()),
        name: Some(content.name.clone()),
        pre_request_script: script(&content.pre_request_script),
        test_script: script(&content.test_script),
        auth: Some(content.auth.clone()),
        record_history: false,
        path_variables: path_vars::names(&content.url)
            .into_iter()
            .map(|name| NameValue {
                name: name.to_owned(),
                value: content
                    .path_variables
                    .iter()
                    .find(|variable| variable.key == name)
                    .map(|variable| variable.value.clone())
                    .unwrap_or_default(),
            })
            .collect(),
        iteration_data: Vec::new(),
        iteration: Iteration::default(),
    }
}

/// The headers a SOAP request adds unless they are typed: SOAP 1.1 names the action in a
/// SOAPAction header, SOAP 1.2 in the Content-Type.
pub fn soap_defaults(soap: Option<&SoapSettings>, typed: &[KeyValue]) -> Vec<NameValue> {
    let has = |name: &str| {
        typed
            .iter()
            .any(|row| row.enabled && row.key.trim().eq_ignore_ascii_case(name))
    };
    let version = soap.map(|soap| soap.version).unwrap_or_default();
    let action = soap.map(|soap| soap.action.as_str()).unwrap_or_default();
    let mut added = Vec::new();
    if !has("content-type") {
        let value = match version {
            SoapVersion::V11 => "text/xml; charset=utf-8".to_owned(),
            SoapVersion::V12 if action.is_empty() => {
                "application/soap+xml; charset=utf-8".to_owned()
            }
            SoapVersion::V12 => format!("application/soap+xml; charset=utf-8; action=\"{action}\""),
        };
        added.push(NameValue {
            name: "Content-Type".to_owned(),
            value,
        });
    }
    if version == SoapVersion::V11 && !has("soapaction") {
        added.push(NameValue {
            name: "SOAPAction".to_owned(),
            value: format!("\"{action}\""),
        });
    }
    added
}

/// The Content-Type a body gets unless one is typed; form data's is set with its boundary.
fn default_content_type(body_mode: &str) -> Option<&'static str> {
    match body_mode {
        "json" => Some("application/json"),
        "text" => Some("text/plain; charset=utf-8"),
        "xml" => Some("application/xml"),
        "html" => Some("text/html; charset=utf-8"),
        "urlencoded" => Some("application/x-www-form-urlencoded"),
        "binary" => Some("application/octet-stream"),
        _ => None,
    }
}

/// URL-encoded bodies carry text only, so file rows are left out of them.
fn body(content: &RequestContent) -> Option<RequestBody> {
    if content.kind == RequestKind::Graphql {
        return Some(RequestBody::Graphql {
            query: content.body.clone(),
            variables: content.graphql_variables.clone(),
        });
    }
    let rows = content
        .form
        .iter()
        .filter(|field| field.enabled && !field.key.trim().is_empty());
    match content.body_mode.as_str() {
        "json" | "text" | "xml" | "html" => Some(RequestBody::Raw {
            text: content.body.clone(),
        }),
        "urlencoded" => Some(RequestBody::Urlencoded {
            fields: rows
                .filter(|field| field.kind == FormFieldKind::Text)
                .map(|field| NameValue {
                    name: field.key.trim().to_owned(),
                    value: field.value.clone(),
                })
                .collect(),
        }),
        "formdata" => Some(RequestBody::FormData {
            parts: rows
                .map(|field| FormPart {
                    name: field.key.trim().to_owned(),
                    value: field.value.clone(),
                    file: field.kind == FormFieldKind::File,
                })
                .collect(),
        }),
        "binary" => Some(RequestBody::Binary {
            path: content.body_file.clone(),
        }),
        _ => None,
    }
}

fn active(rows: &[KeyValue]) -> Vec<NameValue> {
    rows.iter()
        .filter(|row| row.enabled && !row.key.trim().is_empty())
        .map(|row| NameValue {
            name: row.key.trim().to_owned(),
            value: row.value.clone(),
        })
        .collect()
}

fn script(text: &str) -> Option<String> {
    (!text.trim().is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::models::Auth;
    use crate::storage::models::{FormField, PathVariable};

    fn content() -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: "Create user".to_owned(),
            method: "POST".to_owned(),
            url: " {{baseUrl}}/teams/:team/users ".to_owned(),
            params: vec![
                KeyValue {
                    enabled: true,
                    key: " page ".to_owned(),
                    value: "1".to_owned(),
                },
                KeyValue {
                    enabled: false,
                    key: "off".to_owned(),
                    value: "x".to_owned(),
                },
            ],
            headers: Vec::new(),
            body_mode: "json".to_owned(),
            body: r#"{"name":"Rina"}"#.to_owned(),
            form: Vec::new(),
            body_file: String::new(),
            pre_request_script: "  ".to_owned(),
            test_script: "pm.test('ok', () => {});".to_owned(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: vec![PathVariable {
                key: "team".to_owned(),
                value: "{{teamId}}".to_owned(),
            }],
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    #[test]
    fn builds_what_the_request_card_sends() {
        let input = input_from_content(&content(), "c", "r");
        assert_eq!(input.url, "{{baseUrl}}/teams/:team/users");
        assert_eq!(input.params.len(), 1);
        assert_eq!(input.params[0].name, "page");
        assert_eq!(input.headers[0].value, "application/json");
        assert!(matches!(input.body, Some(RequestBody::Raw { .. })));
        assert!(input.pre_request_script.is_none() && input.test_script.is_some());
        assert_eq!(input.path_variables[0].value, "{{teamId}}");
        assert_eq!(input.collection_id.as_deref(), Some("c"));
        assert!(!input.record_history);
    }

    #[test]
    fn keeps_a_typed_content_type_and_drops_files_from_urlencoded_bodies() {
        let form = RequestContent {
            body_mode: "urlencoded".to_owned(),
            headers: vec![KeyValue {
                enabled: true,
                key: "content-type".to_owned(),
                value: "application/x-www-form-urlencoded; charset=utf-8".to_owned(),
            }],
            form: vec![
                FormField {
                    enabled: true,
                    key: "name".to_owned(),
                    value: "Rina".to_owned(),
                    kind: FormFieldKind::Text,
                },
                FormField {
                    enabled: true,
                    key: "avatar".to_owned(),
                    value: "C:\\photo.png".to_owned(),
                    kind: FormFieldKind::File,
                },
            ],
            ..content()
        };
        let input = input_from_content(&form, "c", "r");
        assert_eq!(input.headers.len(), 1);
        let Some(RequestBody::Urlencoded { fields }) = input.body else {
            panic!("urlencoded body expected");
        };
        assert_eq!(fields.len(), 1);

        let graphql = RequestContent {
            kind: RequestKind::Graphql,
            body: "{ me { id } }".to_owned(),
            ..content()
        };
        assert!(matches!(
            input_from_content(&graphql, "c", "r").body,
            Some(RequestBody::Graphql { .. })
        ));
    }
}
