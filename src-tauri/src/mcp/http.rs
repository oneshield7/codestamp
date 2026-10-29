use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, RequestBuilder, Response, StatusCode, Url};
use serde_json::Value;

use super::models::{Direction, McpEvent};
use super::protocol::{self, Incoming, RpcError};
use super::session::{Emitter, MAX_MESSAGE_BYTES, traffic};
use super::stdio::log_text;
use crate::error::AppError;
use crate::request::client::root_cause;

const SESSION_HEADER: &str = "mcp-session-id";
const VERSION_HEADER: &str = "mcp-protocol-version";
const MAX_SESSION_ID_CHARS: usize = 256;
const MAX_ERROR_SNIPPET: usize = 300;
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// A server reached over MCP's Streamable HTTP transport: each message is a POST, answered with
/// JSON or with an event stream that carries the answer.
pub struct HttpSession {
    client: Client,
    url: Url,
    headers: HeaderMap,
    session_id: Mutex<Option<HeaderValue>>,
    protocol_version: Mutex<Option<HeaderValue>>,
    next_id: AtomicU64,
    emit: Emitter,
}

impl HttpSession {
    pub fn new(client: Client, url: Url, headers: HeaderMap, emit: Emitter) -> Self {
        Self {
            client,
            url,
            headers,
            session_id: Mutex::new(None),
            protocol_version: Mutex::new(None),
            next_id: AtomicU64::new(1),
            emit,
        }
    }

    /// Sent with every message after `initialize`, as the specification asks.
    pub fn set_protocol_version(&self, version: &str) {
        *lock(&self.protocol_version) = HeaderValue::from_str(version).ok();
    }

    pub async fn request(
        &self,
        method: &str,
        params: Option<&Value>,
        timeout: Duration,
    ) -> Result<Result<Value, RpcError>, AppError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let message = protocol::request(id, method, params);
        let answer = tokio::time::timeout(timeout, self.exchange(&message, Some(id)))
            .await
            .map_err(|_| AppError::Timeout)??;
        answer.ok_or_else(|| AppError::Request("The server did not answer".to_owned()))
    }

    pub async fn notify(&self, method: &str, timeout: Duration) -> Result<(), AppError> {
        let message = protocol::notification(method);
        tokio::time::timeout(timeout, self.exchange(&message, None))
            .await
            .map_err(|_| AppError::Timeout)??;
        Ok(())
    }

    /// Ends the session on the server. It also expires on its own, so a failure is not shown.
    pub async fn close(&self) {
        let session = lock(&self.session_id).clone();
        if let Some(session) = session {
            let ended = self
                .client
                .delete(self.url.clone())
                .headers(self.headers.clone())
                .header(SESSION_HEADER, session)
                .timeout(CLOSE_TIMEOUT)
                .send()
                .await;
            drop(ended);
        }
        (self.emit)(McpEvent::Closed {
            reason: "Disconnected".to_owned(),
        });
    }

    fn post(&self, text: String) -> RequestBuilder {
        let mut request = self
            .client
            .post(self.url.clone())
            .headers(self.headers.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json, text/event-stream")
            .body(text);
        if let Some(session) = lock(&self.session_id).clone() {
            request = request.header(SESSION_HEADER, session);
        }
        if let Some(version) = lock(&self.protocol_version).clone() {
            request = request.header(VERSION_HEADER, version);
        }
        request
    }

    async fn exchange(
        &self,
        message: &Value,
        expect: Option<u64>,
    ) -> Result<Option<Result<Value, RpcError>>, AppError> {
        let text = message.to_string();
        traffic(&self.emit, Direction::Sent, &text);
        let had_session = lock(&self.session_id).is_some();
        let response = self.post(text).send().await.map_err(map_error)?;
        if let Some(session) = response.headers().get(SESSION_HEADER) {
            let usable = session.to_str().is_ok_and(|id| {
                id.len() <= MAX_SESSION_ID_CHARS && id.chars().all(|c| c.is_ascii_graphic())
            });
            if usable {
                *lock(&self.session_id) = Some(session.clone());
            }
        }
        let status = response.status();
        if status == StatusCode::ACCEPTED {
            return Ok(None);
        }
        if status == StatusCode::NOT_FOUND && had_session {
            return Err(AppError::Request(
                "The server ended the session. Connect again".to_owned(),
            ));
        }
        if !status.is_success() {
            return Err(refused(response).await);
        }
        let streamed = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"));
        if streamed {
            self.read_stream(response, expect).await
        } else {
            let body = read_body(response, MAX_MESSAGE_BYTES).await?;
            Ok(self.handle(&String::from_utf8_lossy(&body), expect).await)
        }
    }

    /// Reads server-sent events until the answer to `expect` arrives or the stream ends.
    async fn read_stream(
        &self,
        mut response: Response,
        expect: Option<u64>,
    ) -> Result<Option<Result<Value, RpcError>>, AppError> {
        let mut pending: Vec<u8> = Vec::new();
        let mut data = String::new();
        let mut total = 0_usize;
        while let Some(chunk) = response.chunk().await.map_err(map_error)? {
            total = total.saturating_add(chunk.len());
            if total > MAX_MESSAGE_BYTES {
                return Err(too_large());
            }
            pending.extend_from_slice(&chunk);
            while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
                let raw: Vec<u8> = pending.drain(..=end).collect();
                let line = String::from_utf8_lossy(&raw);
                let line = line.trim_end_matches(['\r', '\n']);
                if line.is_empty() {
                    let event = std::mem::take(&mut data);
                    if !event.is_empty()
                        && let Some(answer) = self.handle(&event, expect).await
                    {
                        return Ok(Some(answer));
                    }
                } else if let Some(value) = line.strip_prefix("data:") {
                    if !data.is_empty() {
                        data.push('\n');
                    }
                    data.push_str(value.strip_prefix(' ').unwrap_or(value));
                }
            }
        }
        if data.is_empty() {
            Ok(None)
        } else {
            Ok(self.handle(&data, expect).await)
        }
    }

    /// Shows the messages in `text`, answers the server's own requests, and returns the answer
    /// to `expect` if it is among them.
    async fn handle(&self, text: &str, expect: Option<u64>) -> Option<Result<Value, RpcError>> {
        traffic(&self.emit, Direction::Received, text);
        let mut answer = None;
        for message in protocol::parse_messages(text) {
            match message {
                Incoming::Response { id, outcome } if Some(id) == expect => answer = Some(outcome),
                Incoming::Response { .. } => {}
                Incoming::Request { id, method } => {
                    let reply = protocol::reply_to_server(&id, &method).to_string();
                    traffic(&self.emit, Direction::Sent, &reply);
                    // The server only needs to see the reply; a failure shows up on the next call.
                    drop(self.post(reply).send().await);
                }
                Incoming::Notification { method, params } => {
                    if method == "notifications/message"
                        && let Some(text) = log_text(params.as_ref())
                    {
                        (self.emit)(McpEvent::Log { text });
                    }
                }
            }
        }
        answer
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn map_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::Timeout
    } else if error.is_connect() {
        AppError::Connect(root_cause(&error))
    } else {
        AppError::Request(root_cause(&error))
    }
}

fn too_large() -> AppError {
    AppError::Request("A message from the server is larger than 16 MB".to_owned())
}

async fn read_body(mut response: Response, max_bytes: usize) -> Result<Vec<u8>, AppError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_error)? {
        if body.len() + chunk.len() > max_bytes {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// A refusal such as 401 or 405, with the start of the server's explanation.
async fn refused(response: Response) -> AppError {
    let status = response.status();
    let body = read_body(response, MAX_ERROR_SNIPPET * 4)
        .await
        .unwrap_or_default();
    let snippet: String = String::from_utf8_lossy(&body)
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_ERROR_SNIPPET)
        .collect();
    let hint = if status == StatusCode::METHOD_NOT_ALLOWED {
        " (it may use the older HTTP+SSE transport, which Codestamp does not support)"
    } else {
        ""
    };
    let detail = if snippet.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", snippet.trim())
    };
    AppError::Request(format!("The server answered {status}{hint}{detail}"))
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    use super::*;

    /// Answers MCP messages the way a Streamable HTTP server does, and records the session
    /// header of every request it saw. Each connection carries one exchange and says so, so the
    /// client never reuses a connection this server is about to close.
    fn serve(count: usize) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local port");
        let address = listener.local_addr().expect("local address");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&seen);
        thread::spawn(move || {
            for _ in 0..count {
                let (stream, _) = listener.accept().expect("accept");
                let mut reader = BufReader::new(stream);
                let mut length = 0;
                let mut session = String::from("-");
                let mut first = String::new();
                reader.read_line(&mut first).expect("request line");
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) <= 2 {
                        break;
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(value) = lower.strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                    if let Some(value) = lower.strip_prefix("mcp-session-id:") {
                        value.trim().clone_into(&mut session);
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).expect("body");
                let body = String::from_utf8_lossy(&body).into_owned();
                lock(&record).push(session);
                let reply = if first.starts_with("DELETE") {
                    "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n".to_owned()
                } else if body.contains("\"initialize\"") {
                    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","serverInfo":{"name":"fake","version":"1"},"capabilities":{"tools":{}}}}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/json\r\nMcp-Session-Id: s-42\r\nContent-Length: {}\r\n\r\n{json}",
                        json.len()
                    )
                } else if body.contains("notifications/initialized") {
                    "HTTP/1.1 202 Accepted\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
                        .to_owned()
                } else if body.contains("tools/list") {
                    let events = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{\"level\":\"info\",\"data\":\"listing\"}}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\n";
                    let rest = "data: \"result\":{\"tools\":[{\"name\":\"echo\"}]}}\n\n";
                    format!(
                        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{events}{rest}",
                        events.len() + rest.len()
                    )
                } else {
                    let json = r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32602,"message":"Unknown tool"}}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{json}",
                        json.len()
                    )
                };
                let mut stream = reader.into_inner();
                stream.write_all(reply.as_bytes()).expect("reply");
            }
        });
        (format!("http://{address}/mcp"), seen)
    }

    #[tokio::test]
    async fn speaks_json_and_event_streams_and_keeps_the_session() {
        let (url, seen) = serve(5);
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let emit: Emitter = Arc::new(move |event| {
            lock(&sink).push(event);
            true
        });
        let session = HttpSession::new(
            Client::new(),
            Url::parse(&url).expect("url"),
            HeaderMap::new(),
            emit,
        );
        let timeout = Duration::from_secs(5);

        let init = session
            .request("initialize", Some(&protocol::initialize_params()), timeout)
            .await
            .expect("answer")
            .expect("result");
        assert_eq!(protocol::server_info(&init).expect("info").name, "fake");
        session
            .notify("notifications/initialized", timeout)
            .await
            .expect("accepted");

        let tools = session
            .request("tools/list", None, timeout)
            .await
            .expect("answer")
            .expect("result");
        assert_eq!(tools["tools"][0]["name"], "echo");

        let refused = session
            .request(
                "tools/call",
                Some(&serde_json::json!({ "name": "nope" })),
                timeout,
            )
            .await
            .expect("answer");
        assert!(matches!(refused, Err(error) if error.code == -32602));
        session.close().await;

        assert_eq!(*lock(&seen), ["-", "s-42", "s-42", "s-42", "s-42"]);
        let events = lock(&events);
        assert!(events.contains(&McpEvent::Log {
            text: "info: listing".to_owned()
        }));
        assert!(matches!(events.last(), Some(McpEvent::Closed { .. })));
    }
}
