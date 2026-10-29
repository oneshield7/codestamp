use std::error::Error;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use tokio::time::timeout;

use reqwest::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Client, Response, Url};

use super::bodies::MAX_KEPT_BODY;
use super::network::{Clients, NetworkConfig};
use super::types::{HttpResponseOutput, NameValue};
use super::uploads::file_label;
use super::validate::{ValidatedBody, ValidatedPart, ValidatedRequest};
use crate::cookies::jar::{CookieJar, response_cookies};
use crate::error::AppError;

/// Only this much of a body reaches the UI, so a huge response never sits in the webview.
const BODY_PREVIEW_LIMIT: usize = 2 * 1024 * 1024;
const BYTES_PER_MB: u64 = 1024 * 1024;

/// Shared clients so connections and TLS sessions are pooled across requests. Cloning is cheap:
/// clones share the clients, the current settings and the cookie jar.
#[derive(Clone)]
pub struct HttpClient {
    jar: Arc<CookieJar>,
    current: Arc<RwLock<Arc<Clients>>>,
}

impl HttpClient {
    pub fn new() -> Result<Self, AppError> {
        let jar = Arc::new(CookieJar::default());
        let clients = Clients::build(&jar, &NetworkConfig::default())?;
        Ok(Self {
            jar,
            current: Arc::new(RwLock::new(Arc::new(clients))),
        })
    }

    pub fn jar(&self) -> &CookieJar {
        &self.jar
    }

    /// Builds clients for `config` first, so a configuration that does not work leaves the
    /// current one in place. Requests already on their way finish with the old clients.
    pub fn configure(&self, config: &NetworkConfig) -> Result<(), AppError> {
        let clients = Arc::new(Clients::build(&self.jar, config)?);
        *self.current.write().map_err(|_| AppError::ClientInit)? = clients;
        Ok(())
    }

    /// Checks that `config` can be used, without switching to it.
    pub fn check(&self, config: &NetworkConfig) -> Result<(), AppError> {
        Clients::build(&self.jar, config).map(|_| ())
    }

    /// The client that requests to `url` go through (with its client certificate, if any).
    pub fn client_for(&self, url: &Url) -> Result<Client, AppError> {
        Ok(self.clients()?.for_url(url).clone())
    }

    /// The timeout from Settings › Requests.
    pub fn default_timeout(&self) -> Result<Duration, AppError> {
        Ok(self.clients()?.timeout)
    }

    fn clients(&self) -> Result<Arc<Clients>, AppError> {
        let current = self.current.read().map_err(|_| AppError::ClientInit)?;
        Ok(Arc::clone(&current))
    }

    pub async fn send(
        &self,
        mut request: ValidatedRequest,
    ) -> Result<HttpResponseOutput, AppError> {
        let clients = self.clients()?;
        if clients.no_cache && !request.headers.contains_key(CACHE_CONTROL) {
            request
                .headers
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        }
        let started = Instant::now();
        let limit = request.timeout.unwrap_or(clients.timeout);
        let builder = clients
            .for_url(&request.url)
            .request(request.method, request.url)
            .headers(request.headers);
        let builder = match request.body {
            None => builder,
            Some(ValidatedBody::Text(text)) => builder.body(text),
            Some(ValidatedBody::Multipart(parts)) => {
                builder.multipart(multipart_form(parts).await?)
            }
            // Streamed from disk, so a large upload is never held in memory.
            Some(ValidatedBody::File(path)) => builder.body(open_upload(&path).await?),
        };
        // The limit covers the whole exchange, except that an event stream may go on for as
        // long as the user keeps it open once its headers have arrived.
        let response = timeout(limit, builder.send())
            .await
            .map_err(|_| AppError::Timeout)?
            .map_err(map_error)?;
        if is_event_stream(&response) {
            return Ok(stream_output(response, started));
        }
        let left = limit.saturating_sub(started.elapsed());
        timeout(
            left,
            read_response(response, started, clients.max_response_bytes),
        )
        .await
        .map_err(|_| AppError::Timeout)?
    }
}

async fn multipart_form(parts: Vec<ValidatedPart>) -> Result<Form, AppError> {
    let mut form = Form::new();
    for part in parts {
        form = match part {
            ValidatedPart::Text { name, value } => form.text(name, value),
            ValidatedPart::File { name, path } => {
                let file = Part::file(&path).await.map_err(|_| unreadable(&path))?;
                form.part(name, file)
            }
        };
    }
    Ok(form)
}

async fn open_upload(path: &Path) -> Result<Body, AppError> {
    tokio::fs::File::open(path)
        .await
        .map(Body::from)
        .map_err(|_| unreadable(path))
}

fn unreadable(path: &Path) -> AppError {
    AppError::InvalidInput(format!(
        "The file “{}” cannot be read",
        file_label(&path.to_string_lossy())
    ))
}

fn is_event_stream(response: &Response) -> bool {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .trim()
                .to_ascii_lowercase()
                .starts_with("text/event-stream")
        })
}

/// Status, headers and cookies, which every response has before its body.
struct Head {
    status: reqwest::StatusCode,
    http_version: String,
    headers: Vec<NameValue>,
    content_type: Option<String>,
    cookies: Vec<crate::cookies::models::ResponseCookie>,
}

fn head(response: &Response) -> Head {
    Head {
        status: response.status(),
        http_version: format!("{:?}", response.version()),
        content_type: response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        headers: response
            .headers()
            .iter()
            .map(|(name, value)| NameValue {
                name: name.as_str().to_owned(),
                value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
            })
            .collect(),
        cookies: response_cookies(response.headers()),
    }
}

/// An event stream returns as soon as its headers arrive; its body is read as it comes.
fn stream_output(response: Response, started: Instant) -> HttpResponseOutput {
    let Head {
        status,
        http_version,
        headers,
        content_type,
        cookies,
    } = head(&response);
    HttpResponseOutput {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or_default().to_owned(),
        http_version,
        headers,
        content_type,
        body_preview: String::new(),
        body_size: 0,
        body_truncated: false,
        body_id: None,
        whole_body: None,
        stream_id: None,
        stream: Some(response),
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        cookies,
        warning: None,
        script: None,
    }
}

async fn read_response(
    mut response: Response,
    started: Instant,
    max_bytes: Option<u64>,
) -> Result<HttpResponseOutput, AppError> {
    let Head {
        status,
        http_version,
        headers,
        content_type,
        cookies,
    } = head(&response);

    let mut kept = Vec::new();
    let mut body_size: u64 = 0;
    let mut stopped = false;
    while let Some(chunk) = response.chunk().await.map_err(map_error)? {
        body_size = body_size.saturating_add(chunk.len() as u64);
        let room = MAX_KEPT_BODY.saturating_sub(kept.len());
        kept.extend_from_slice(&chunk[..room.min(chunk.len())]);
        if max_bytes.is_some_and(|max| body_size >= max) {
            stopped = true;
            break;
        }
    }
    let preview = &kept[..kept.len().min(BODY_PREVIEW_LIMIT)];
    let whole = !stopped && body_size > 0 && body_size == kept.len() as u64;

    Ok(HttpResponseOutput {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or_default().to_owned(),
        http_version,
        headers,
        content_type,
        body_truncated: body_size > preview.len() as u64,
        body_preview: String::from_utf8_lossy(preview).into_owned(),
        body_id: None,
        whole_body: whole.then_some(kept),
        stream_id: None,
        stream: None,
        body_size,
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        cookies,
        warning: stopped.then(|| {
            format!(
                "The download stopped at {} MB, the largest response set in Settings › Requests",
                max_bytes.unwrap_or_default() / BYTES_PER_MB
            )
        }),
        script: None,
    })
}

pub fn map_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        return AppError::Timeout;
    }
    let reason = root_cause(&error);
    if error.is_connect() {
        AppError::Connect(reason)
    } else {
        AppError::Request(reason)
    }
}

/// The innermost error ("connection refused", "invalid peer certificate") is the useful part.
pub fn root_cause(error: &(dyn Error + 'static)) -> String {
    let mut current = error;
    while let Some(source) = current.source() {
        current = source;
    }
    current.to_string()
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;
    use crate::request::types::{FormPart, HttpRequestInput, RequestBody};
    use crate::request::validate::validate;

    /// Serves `count` requests on a random local port. Each response sets a cookie and echoes the
    /// request it got (head and body), which only works for bodies with a Content-Length.
    fn serve_echo(count: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local port");
        let address = listener.local_addr().expect("local address");
        thread::spawn(move || {
            for _ in 0..count {
                let (stream, _) = listener.accept().expect("accept connection");
                let mut reader = BufReader::new(stream);
                let mut echoed = Vec::new();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) <= 2 {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                    echoed.extend_from_slice(line.as_bytes());
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).expect("read body");
                echoed.extend_from_slice(b"\r\n");
                echoed.extend_from_slice(&body);
                let mut stream = reader.into_inner();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nSet-Cookie: sid=abc; Path=/\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    echoed.len()
                );
                stream.write_all(head.as_bytes()).expect("write head");
                stream.write_all(&echoed).expect("write body");
            }
        });
        format!("http://{address}/")
    }

    fn post(url: &str, body: RequestBody) -> ValidatedRequest {
        validate(HttpRequestInput {
            method: "POST".to_owned(),
            url: url.to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body: Some(body),
            timeout_ms: Some(5_000),
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
        })
        .expect("valid request")
    }

    #[tokio::test]
    async fn uploads_form_fields_and_files_with_a_known_length() {
        let path =
            std::env::temp_dir().join(format!("codestamp-{}.txt", crate::storage::db::new_id()));
        std::fs::write(&path, "hello file").expect("write file");
        let url = serve_echo(2);
        let client = HttpClient::new().expect("client");

        let form = RequestBody::FormData {
            parts: vec![
                FormPart {
                    name: "name".to_owned(),
                    value: "Rina".to_owned(),
                    file: false,
                },
                FormPart {
                    name: "doc".to_owned(),
                    value: path.display().to_string(),
                    file: true,
                },
            ],
        };
        let sent = client.send(post(&url, form)).await.expect("response");
        let echoed = sent.body_preview.to_ascii_lowercase();
        assert!(echoed.contains("content-type: multipart/form-data; boundary="));
        assert!(echoed.contains("name=\"name\"") && echoed.contains("rina"));
        assert!(echoed.contains("name=\"doc\"; filename=") && echoed.contains("hello file"));

        let raw = RequestBody::Binary {
            path: path.display().to_string(),
        };
        let sent = client.send(post(&url, raw)).await.expect("response");
        assert!(
            sent.body_preview
                .to_ascii_lowercase()
                .contains("content-length: 10")
        );
        assert!(sent.body_preview.ends_with("hello file"));
        std::fs::remove_file(&path).expect("remove file");
    }

    #[tokio::test]
    async fn sends_back_cookies_that_a_response_set() {
        let url = serve_echo(2);
        let client = HttpClient::new().expect("client");

        let first = client.send(get(url.clone())).await.expect("response");
        assert!(
            first
                .cookies
                .iter()
                .any(|c| c.name == "sid" && c.value == "abc")
        );
        let second = client.send(get(url)).await.expect("response");
        assert!(
            second
                .body_preview
                .to_ascii_lowercase()
                .contains("cookie: sid=abc")
        );
        assert!(client.jar().take_changed());
    }

    /// Serves exactly one HTTP/1.1 response on a random local port and returns its URL.
    fn serve_once(content_type: &'static str, body: Vec<u8>) -> String {
        serve_raw(format!("200 OK\r\nContent-Type: {content_type}"), body)
    }

    /// `head` is the status (without `HTTP/1.1`) followed by extra header lines.
    fn serve_raw(head: String, body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local port");
        let address = listener.local_addr().expect("local address");
        thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept connection");
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                line.clear();
            }
            let mut stream = reader.into_inner();
            let head = format!(
                "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).expect("write head");
            stream.write_all(&body).expect("write body");
        });
        format!("http://{address}/")
    }

    fn get(url: String) -> ValidatedRequest {
        validate(HttpRequestInput {
            method: "GET".to_owned(),
            url,
            params: Vec::new(),
            headers: Vec::new(),
            body: None,
            timeout_ms: Some(5_000),
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
        })
        .expect("valid request")
    }

    #[tokio::test]
    async fn returns_status_headers_and_body() {
        let url = serve_once("application/json", br#"{"ok":true}"#.to_vec());
        let client = HttpClient::new().expect("client");

        let response = client.send(get(url)).await.expect("response");

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type.as_deref(), Some("application/json"));
        assert_eq!(response.body_preview, r#"{"ok":true}"#);
        assert_eq!(response.body_size, 11);
        assert!(!response.body_truncated);
    }

    #[tokio::test]
    async fn keeps_only_a_preview_of_large_bodies() {
        let total = BODY_PREVIEW_LIMIT + 4_096;
        let url = serve_once("text/plain", vec![b'a'; total]);
        let client = HttpClient::new().expect("client");

        let response = client.send(get(url)).await.expect("response");

        assert!(response.body_truncated);
        assert_eq!(response.body_size, total as u64);
        assert_eq!(response.body_preview.len(), BODY_PREVIEW_LIMIT);
    }

    #[tokio::test]
    async fn does_not_follow_redirects_to_another_host() {
        let elsewhere = serve_once("text/plain", b"should not be reached".to_vec());
        let url = serve_raw(format!("302 Found\r\nLocation: {elsewhere}"), Vec::new());
        let client = HttpClient::new().expect("client");

        let response = client.send(get(url)).await.expect("response");

        assert_eq!(response.status, 302);
        assert!(
            response
                .headers
                .iter()
                .any(|h| h.name == "location" && h.value == elsewhere)
        );
    }

    #[tokio::test]
    async fn reports_connection_failures() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local port");
        let address = listener.local_addr().expect("local address");
        drop(listener);
        let client = HttpClient::new().expect("client");

        let result = client.send(get(format!("http://{address}/"))).await;

        assert!(matches!(result, Err(AppError::Connect(_))));
    }
}
