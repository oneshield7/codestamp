use std::path::{Path, PathBuf};
use std::time::Duration;

use ::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use ::http::{HeaderMap, HeaderName, HeaderValue, Method};
use url::Url;

use super::rules::{
    MAX_KEY_VALUE_LENGTH, MAX_KEY_VALUE_ROWS, MAX_REQUEST_BODY_BYTES, MAX_UPLOAD_BYTES,
    MAX_URL_LENGTH, is_allowed_method,
};
use super::types::{FormPart, HttpRequestInput, NameValue, RequestBody, encode_form};
use super::uploads::file_label;
use crate::error::AppError;

pub const HTTP_SCHEMES: [&str; 2] = ["http", "https"];
const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];
pub const MIN_TIMEOUT_MS: u64 = 1_000;
pub const MAX_TIMEOUT_MS: u64 = 300_000;
const MULTIPART: &str = "multipart/form-data";

#[derive(Debug, Clone)]
pub struct ValidatedRequest {
    pub method: Method,
    pub url: Url,
    pub headers: HeaderMap,
    pub body: Option<ValidatedBody>,
    /// `None` uses the default timeout from Settings › Requests.
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum ValidatedBody {
    Text(String),
    Multipart(Vec<ValidatedPart>),
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub enum ValidatedPart {
    Text { name: String, value: String },
    File { name: String, path: PathBuf },
}

pub fn validate(input: HttpRequestInput) -> Result<ValidatedRequest, AppError> {
    let method = parse_method(&input.method)?;
    let url = build_url(&input.url, &input.params, &HTTP_SCHEMES)?;
    let mut headers = parse_headers(&input.headers)?;
    let body = match input.body {
        Some(body) => Some(check_body(body, &mut headers)?),
        None => None,
    };
    Ok(ValidatedRequest {
        method,
        url,
        headers,
        body,
        timeout: parse_timeout(input.timeout_ms)?,
    })
}

/// A URL Codestamp fetches by itself (an OAuth token endpoint, an OpenAPI spec): HTTPS, with plain
/// HTTP allowed only on this machine for local test servers.
pub fn secure_url(raw: &str, what: &str) -> Result<Url, AppError> {
    let url = Url::parse(raw.trim())
        .map_err(|_| AppError::InvalidInput(format!("Enter a valid {what}")))?;
    let local = url.host_str().is_some_and(is_loopback_host);
    match url.scheme() {
        "https" => Ok(url),
        "http" if local => Ok(url),
        _ => Err(AppError::InvalidInput(format!(
            "The {what} must use https:// (http:// works for localhost only)"
        ))),
    }
}

/// This machine, where plain `http://` reaches only local test servers.
pub fn is_loopback_host(host: &str) -> bool {
    LOOPBACK_HOSTS.contains(&host)
}

fn parse_method(raw: &str) -> Result<Method, AppError> {
    if !is_allowed_method(raw) {
        return Err(AppError::invalid("Unsupported HTTP method"));
    }
    Method::from_bytes(raw.as_bytes()).map_err(|_| AppError::invalid("Unsupported HTTP method"))
}

fn parse_url(raw: &str, schemes: &[&str]) -> Result<Url, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid("Enter a URL"));
    }
    if trimmed.len() > MAX_URL_LENGTH {
        return Err(AppError::invalid("The URL is too long"));
    }
    let url = Url::parse(trimmed).map_err(|_| AppError::invalid("The URL is not valid"))?;
    if !schemes.contains(&url.scheme()) {
        let allowed: Vec<String> = schemes
            .iter()
            .map(|scheme| format!("{scheme}://"))
            .collect();
        return Err(AppError::InvalidInput(format!(
            "The URL must start with {}",
            allowed.join(" or ")
        )));
    }
    if url.host_str().is_none() {
        return Err(AppError::invalid("The URL has no host"));
    }
    Ok(url)
}

/// A base URL kept as a variable, such as the one given when a collection is created.
pub fn check_base_url(raw: &str) -> Result<String, AppError> {
    web_url(raw).map(|_| raw.trim().to_owned())
}

/// An address read for the user the same way their requests are sent: http or https, any host.
pub fn web_url(raw: &str) -> Result<Url, AppError> {
    parse_url(raw, &HTTP_SCHEMES)
}

/// Parses the URL (with one of `schemes`) and appends the encoded query parameters.
pub fn build_url(raw: &str, params: &[NameValue], schemes: &[&str]) -> Result<Url, AppError> {
    let mut url = parse_url(raw, schemes)?;
    if params.is_empty() {
        return Ok(url);
    }
    if params.len() > MAX_KEY_VALUE_ROWS {
        return Err(AppError::invalid("Too many query parameters"));
    }
    let invalid = params.iter().any(|param| {
        param.name.trim().is_empty()
            || param.name.len() > MAX_KEY_VALUE_LENGTH
            || param.value.len() > MAX_KEY_VALUE_LENGTH
    });
    if invalid {
        return Err(AppError::invalid("A query parameter is empty or too long"));
    }
    url.query_pairs_mut().extend_pairs(
        params
            .iter()
            .map(|param| (param.name.trim(), param.value.as_str())),
    );
    if url.as_str().len() > MAX_URL_LENGTH {
        return Err(AppError::invalid("The URL is too long"));
    }
    Ok(url)
}

pub fn parse_headers(entries: &[NameValue]) -> Result<HeaderMap, AppError> {
    if entries.len() > MAX_KEY_VALUE_ROWS {
        return Err(AppError::invalid("Too many headers"));
    }
    let mut headers = HeaderMap::with_capacity(entries.len());
    for entry in entries {
        let name = HeaderName::from_bytes(entry.name.trim().as_bytes())
            .map_err(|_| AppError::invalid("A header name is not valid"))?;
        if entry.value.len() > MAX_KEY_VALUE_LENGTH {
            return Err(AppError::invalid("A header value is too long"));
        }
        // Rejects CR/LF and other control characters, which blocks header injection.
        let value = HeaderValue::from_str(&entry.value).map_err(|_| {
            AppError::invalid("A header value contains characters that are not allowed")
        })?;
        headers.append(name, value);
    }
    Ok(headers)
}

fn check_body(body: RequestBody, headers: &mut HeaderMap) -> Result<ValidatedBody, AppError> {
    match body {
        RequestBody::Raw { text } => checked_text(text),
        RequestBody::Urlencoded { fields } => {
            if fields.len() > MAX_KEY_VALUE_ROWS {
                return Err(AppError::invalid("Too many form fields"));
            }
            checked_text(encode_form(&fields))
        }
        RequestBody::FormData { parts } => check_parts(parts, headers),
        RequestBody::Binary { path } => {
            let size = upload_size(&path)?;
            check_upload_size(size)?;
            // The file is streamed, so its length is announced up front instead of chunking.
            headers.insert(CONTENT_LENGTH, HeaderValue::from(size));
            Ok(ValidatedBody::File(PathBuf::from(path)))
        }
        RequestBody::Graphql { query, variables } => {
            headers
                .entry(CONTENT_TYPE)
                .or_insert(HeaderValue::from_static("application/json"));
            checked_text(graphql_json(query, &variables)?)
        }
    }
}

/// The JSON body of a GraphQL request; blank variables are left out.
pub fn graphql_json(query: String, variables: &str) -> Result<String, AppError> {
    let mut body = serde_json::Map::new();
    body.insert("query".to_owned(), serde_json::Value::String(query));
    if !variables.trim().is_empty() {
        let parsed: serde_json::Value = serde_json::from_str(variables)
            .map_err(|_| AppError::invalid("The GraphQL variables are not valid JSON"))?;
        if !parsed.is_object() {
            return Err(AppError::invalid(
                "The GraphQL variables must be a JSON object",
            ));
        }
        body.insert("variables".to_owned(), parsed);
    }
    Ok(serde_json::Value::Object(body).to_string())
}

fn checked_text(text: String) -> Result<ValidatedBody, AppError> {
    if text.len() > MAX_REQUEST_BODY_BYTES {
        return Err(AppError::invalid("The request body is larger than 10 MB"));
    }
    Ok(ValidatedBody::Text(text))
}

fn check_parts(parts: Vec<FormPart>, headers: &mut HeaderMap) -> Result<ValidatedBody, AppError> {
    if parts.len() > MAX_KEY_VALUE_ROWS {
        return Err(AppError::invalid("Too many form fields"));
    }
    claim_multipart_content_type(headers)?;
    let mut text_bytes: usize = 0;
    let mut file_bytes: u64 = 0;
    let mut checked = Vec::with_capacity(parts.len());
    for part in parts {
        let name = part.name.trim().to_owned();
        // The name goes into the part's own headers, so it must be a plain one-line value.
        if name.is_empty()
            || name.len() > MAX_KEY_VALUE_LENGTH
            || name.chars().any(char::is_control)
        {
            return Err(AppError::invalid(
                "A form field name is empty, too long or contains line breaks",
            ));
        }
        if part.file {
            file_bytes = file_bytes.saturating_add(upload_size(&part.value)?);
            checked.push(ValidatedPart::File {
                name,
                path: PathBuf::from(part.value),
            });
        } else {
            text_bytes = text_bytes.saturating_add(part.value.len());
            checked.push(ValidatedPart::Text {
                name,
                value: part.value,
            });
        }
    }
    if text_bytes > MAX_REQUEST_BODY_BYTES {
        return Err(AppError::invalid("The form text is larger than 10 MB"));
    }
    check_upload_size(file_bytes)?;
    Ok(ValidatedBody::Multipart(checked))
}

/// The multipart Content-Type carries the boundary, so the client sets it. A typed one is only
/// accepted when it asks for the same type; anything else would describe a different body.
fn claim_multipart_content_type(headers: &mut HeaderMap) -> Result<(), AppError> {
    let conflicting = headers.get_all(CONTENT_TYPE).iter().any(|value| {
        !value.to_str().is_ok_and(|text| {
            text.trim_start()
                .to_ascii_lowercase()
                .starts_with(MULTIPART)
        })
    });
    if conflicting {
        return Err(AppError::invalid(
            "Form data is sent as multipart/form-data. Remove the Content-Type header or pick another body type",
        ));
    }
    headers.remove(CONTENT_TYPE);
    Ok(())
}

fn upload_size(path: &str) -> Result<u64, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid("Choose a file to upload"));
    }
    let unreadable =
        || AppError::InvalidInput(format!("The file “{}” cannot be read", file_label(path)));
    let metadata = Path::new(path).metadata().map_err(|_| unreadable())?;
    if !metadata.is_file() {
        return Err(unreadable());
    }
    Ok(metadata.len())
}

fn check_upload_size(total: u64) -> Result<(), AppError> {
    if total > MAX_UPLOAD_BYTES {
        return Err(AppError::invalid("Uploads can be up to 100 MB in total"));
    }
    Ok(())
}

fn parse_timeout(timeout_ms: Option<u64>) -> Result<Option<Duration>, AppError> {
    let Some(millis) = timeout_ms else {
        return Ok(None);
    };
    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&millis) {
        return Err(AppError::invalid(
            "The timeout must be between 1 and 300 seconds",
        ));
    }
    Ok(Some(Duration::from_millis(millis)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(method: &str, url: &str) -> HttpRequestInput {
        HttpRequestInput {
            method: method.to_owned(),
            url: url.to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body: None,
            timeout_ms: None,
            collection_id: None,
            request_id: None,
            name: None,
            pre_request_script: None,
            test_script: None,
            auth: None,
            record_history: true,
            path_variables: Vec::new(),
            iteration_data: Vec::new(),
            iteration: Default::default(),
        }
    }

    #[test]
    fn appends_encoded_query_parameters() {
        let mut request = input("GET", "https://api.example.com/v1/users?page=1");
        request.params = vec![header("q", "rina & budi"), header("limit", "20")];

        let validated = validate(request);

        assert!(validated.is_ok_and(|v| v.url.as_str()
            == "https://api.example.com/v1/users?page=1&q=rina+%26+budi&limit=20"));
    }

    fn header(name: &str, value: &str) -> NameValue {
        NameValue {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn accepts_a_valid_request() {
        let mut request = input("POST", " https://api.example.com/v1/users?page=1 ");
        request.headers = vec![header("Content-Type", "application/json")];
        request.body = Some(RequestBody::Raw {
            text: "{}".to_owned(),
        });

        let validated = validate(request);

        assert!(validated.is_ok_and(|v| v.method == Method::POST
            && v.url.as_str() == "https://api.example.com/v1/users?page=1"
            && v.headers.len() == 1
            && v.timeout.is_none()));
    }

    #[test]
    fn encodes_url_encoded_fields() {
        let mut request = input("POST", "https://example.com");
        request.body = Some(RequestBody::Urlencoded {
            fields: vec![header("name", "Rina Saputra"), header("note", "a&b=c")],
        });

        let validated = validate(request).expect("valid request");

        assert!(matches!(
            validated.body,
            Some(ValidatedBody::Text(text)) if text == "name=Rina+Saputra&note=a%26b%3Dc"
        ));
    }

    fn part(name: &str, value: &str, file: bool) -> FormPart {
        FormPart {
            name: name.to_owned(),
            value: value.to_owned(),
            file,
        }
    }

    #[test]
    fn form_data_takes_over_a_multipart_content_type_and_refuses_others() {
        let mut typed_multipart = input("POST", "https://example.com");
        typed_multipart.headers = vec![header("Content-Type", "multipart/form-data")];
        typed_multipart.body = Some(RequestBody::FormData {
            parts: vec![part("name", "Rina", false)],
        });
        let validated = validate(typed_multipart).expect("valid request");
        assert!(validated.headers.get(CONTENT_TYPE).is_none());

        let mut typed_json = input("POST", "https://example.com");
        typed_json.headers = vec![header("Content-Type", "application/json")];
        typed_json.body = Some(RequestBody::FormData {
            parts: vec![part("name", "Rina", false)],
        });
        assert!(validate(typed_json).is_err());
    }

    #[test]
    fn rejects_form_field_names_with_line_breaks() {
        let mut request = input("POST", "https://example.com");
        request.body = Some(RequestBody::FormData {
            parts: vec![part("name\r\nX-Injected: 1", "v", false)],
        });
        assert!(validate(request).is_err());
    }

    #[test]
    fn checks_upload_files_and_announces_their_length() {
        let path =
            std::env::temp_dir().join(format!("codestamp-{}.bin", crate::storage::db::new_id()));
        std::fs::write(&path, [1_u8, 2, 3]).expect("write file");
        let mut request = input("POST", "https://example.com");
        request.body = Some(RequestBody::Binary {
            path: path.display().to_string(),
        });
        let validated = validate(request).expect("valid request");
        assert_eq!(
            validated.headers.get(CONTENT_LENGTH),
            Some(&HeaderValue::from(3_u64))
        );
        std::fs::remove_file(&path).expect("remove file");

        let mut missing = input("POST", "https://example.com");
        missing.body = Some(RequestBody::Binary {
            path: path.display().to_string(),
        });
        assert!(validate(missing).is_err());

        let mut empty = input("POST", "https://example.com");
        empty.body = Some(RequestBody::FormData {
            parts: vec![part("file", "", true)],
        });
        assert!(validate(empty).is_err());
    }

    #[test]
    fn rejects_methods_outside_the_whitelist() {
        assert!(validate(input("get", "https://example.com")).is_err());
        assert!(validate(input("TRACE", "https://example.com")).is_err());
    }

    #[test]
    fn rejects_non_http_schemes_and_missing_scheme() {
        assert!(validate(input("GET", "file:///C:/Windows/win.ini")).is_err());
        assert!(validate(input("GET", "ftp://example.com")).is_err());
        assert!(validate(input("GET", "example.com")).is_err());
        assert!(validate(input("GET", "   ")).is_err());
    }

    #[test]
    fn rejects_header_injection() {
        let mut request = input("GET", "https://example.com");
        request.headers = vec![header("X-Test", "ok\r\nInjected: yes")];
        assert!(validate(request).is_err());
    }

    #[test]
    fn rejects_invalid_header_names() {
        let mut request = input("GET", "https://example.com");
        request.headers = vec![header("Bad Header", "value")];
        assert!(validate(request).is_err());
    }

    #[test]
    fn rejects_out_of_range_timeouts() {
        let mut too_short = input("GET", "https://example.com");
        too_short.timeout_ms = Some(10);
        assert!(validate(too_short).is_err());

        let mut too_long = input("GET", "https://example.com");
        too_long.timeout_ms = Some(MAX_TIMEOUT_MS + 1);
        assert!(validate(too_long).is_err());
    }
}
