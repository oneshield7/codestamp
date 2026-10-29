//! A small HTTP/1.1 server on 127.0.0.1 that answers with a collection's saved examples. Each
//! connection carries one exchange; sizes, time and connection counts are capped, since any
//! program on this machine can reach it.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, oneshot};
use tokio::time::timeout;

use super::matcher;
use crate::error::AppError;
use crate::storage::db::{Database, now_ms};
use crate::storage::examples::{self, Example};

const MAX_HEAD_BYTES: usize = 64 * 1024;
const MAX_IGNORED_BODY_BYTES: usize = 10 * 1024 * 1024;
const MAX_CONNECTIONS: usize = 64;
const READ_TIMEOUT: Duration = Duration::from_secs(10);
/// Headers the mock sets itself, or that would describe a framing it does not use.
const OWN_HEADERS: [&str; 7] = [
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "content-encoding",
    "upgrade",
    "trailer",
];
pub const HIT_EVENT: &str = "mock://hit";

/// One answered request, for the mock panel's log.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockHit {
    pub collection_id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub example: Option<String>,
    pub at: i64,
}

pub struct RunningMock {
    pub port: u16,
    // Dropping the sender stops the accept loop.
    _stop: oneshot::Sender<()>,
}

pub async fn start(
    app: AppHandle,
    collection_id: String,
    port: u16,
) -> Result<RunningMock, AppError> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|_| AppError::InvalidInput(format!("Port {port} is in use; choose another")))?;
    let port = listener
        .local_addr()
        .map_err(|_| AppError::invalid("The mock server could not start"))?
        .port();
    let (stop, mut stopped) = oneshot::channel::<()>();
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stopped => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { continue };
                    // When every slot is busy the connection is simply closed.
                    let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else { continue };
                    let app = app.clone();
                    let collection_id = collection_id.clone();
                    tauri::async_runtime::spawn(async move {
                        let _slot = slot;
                        serve(app, collection_id, stream).await;
                    });
                }
            }
        }
    });
    Ok(RunningMock { port, _stop: stop })
}

struct Incoming {
    method: String,
    path: String,
    example_name: Option<String>,
    example_code: Option<u16>,
}

async fn serve(app: AppHandle, collection_id: String, mut stream: TcpStream) {
    let Ok(Some(incoming)) = timeout(READ_TIMEOUT, read_request(&mut stream)).await else {
        return;
    };
    let (status, headers, body, example) = answer(&app, &collection_id, &incoming);
    let response = response_bytes(status, &headers, &body);
    // A client that went away cannot be told anything more.
    let _ = stream.write_all(&response).await;
    let _ = stream.shutdown().await;
    let hit = MockHit {
        collection_id,
        method: incoming.method,
        path: incoming.path,
        status,
        example,
        at: now_ms(),
    };
    // The panel may be closed; the log is only for display.
    let _ = app.emit(HIT_EVENT, hit);
}

/// Reads the request line and headers, and drains a body up to the limit. `None` for anything
/// that is not a readable HTTP request.
async fn read_request(stream: &mut TcpStream) -> Option<Incoming> {
    let mut buffer = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        if buffer.len() > MAX_HEAD_BYTES {
            return None;
        }
    };
    let head = std::str::from_utf8(&buffer[..head_end]).ok()?;
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next()?.split(' ');
    let method = request_line.next()?.to_ascii_uppercase();
    let target = request_line.next()?;
    let path = target.split(['?', '#']).next().unwrap_or("/").to_owned();
    let mut incoming = Incoming {
        method,
        path,
        example_name: None,
        example_code: None,
    };
    let mut length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = value.parse().unwrap_or(0),
            "x-mock-response-name" => incoming.example_name = Some(value.to_owned()),
            "x-mock-response-code" => incoming.example_code = value.parse().ok(),
            _ => {}
        }
    }
    let mut remaining = length
        .min(MAX_IGNORED_BODY_BYTES)
        .saturating_sub(buffer.len() - head_end - 4);
    while remaining > 0 {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            break;
        }
        remaining = remaining.saturating_sub(read);
    }
    Some(incoming)
}

type Answer = (u16, Vec<(String, String)>, Vec<u8>, Option<String>);

fn answer(app: &AppHandle, collection_id: &str, incoming: &Incoming) -> Answer {
    let db = app.state::<Database>();
    let found = db.with(|conn| {
        let candidates = examples::owners(conn, collection_id)?;
        let Some(candidate) = matcher::best(&candidates, &incoming.method, &incoming.path) else {
            return Ok(None);
        };
        let kept = examples::all_for(conn, &candidate.request_id)?;
        Ok(matcher::choose(
            &kept,
            incoming.example_name.as_deref(),
            incoming.example_code,
        )
        .cloned())
    });
    match found {
        Ok(Some(example)) => from_example(example),
        Ok(None) => json_error(
            404,
            &format!(
                "No saved example matches {} {}",
                incoming.method, incoming.path
            ),
        ),
        Err(_) => json_error(500, "The mock server could not read the collection"),
    }
}

fn from_example(example: Example) -> Answer {
    let headers = example
        .headers
        .into_iter()
        .filter(|header| {
            let name = header.name.trim().to_ascii_lowercase();
            !OWN_HEADERS.contains(&name.as_str())
                && is_token(&name)
                && header
                    .value
                    .chars()
                    .all(|c| c == '\t' || (' '..='~').contains(&c))
        })
        .map(|header| (header.name.trim().to_owned(), header.value))
        .collect();
    (
        example.status,
        headers,
        example.body.into_bytes(),
        Some(example.name),
    )
}

fn json_error(status: u16, message: &str) -> Answer {
    let body = serde_json::json!({ "error": message }).to_string();
    (
        status,
        vec![("Content-Type".to_owned(), "application/json".to_owned())],
        body.into_bytes(),
        None,
    )
}

fn response_bytes(status: u16, headers: &[(String, String)], body: &[u8]) -> Vec<u8> {
    let reason = ::http::StatusCode::from_u16(status)
        .ok()
        .and_then(|status| status.canonical_reason())
        .unwrap_or("Response");
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\nX-Codestamp-Mock: 1\r\n\r\n",
        body.len()
    ));
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn is_token(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}
