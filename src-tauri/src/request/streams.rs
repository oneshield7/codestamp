//! Server-Sent Events: a `text/event-stream` response is read as it arrives and each event is
//! passed to the UI, until the server ends the stream or the user stops it.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};

use super::client::map_error;
use crate::error::AppError;
use crate::storage::db::{new_id, now_ms};

pub const EVENT: &str = "stream://event";
pub const END: &str = "stream://end";
const MAX_STREAMS: usize = 10;
/// A line or event longer than this ends the stream, so a broken server cannot fill memory.
const MAX_EVENT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEvent {
    pub stream_id: String,
    /// `message` unless the server named the event.
    pub event: String,
    pub data: String,
    pub id: Option<String>,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEnd {
    pub stream_id: String,
    pub error: Option<String>,
}

/// The event streams being read now.
#[derive(Default)]
pub struct EventStreams {
    running: Mutex<HashMap<String, JoinHandle<()>>>,
}

impl EventStreams {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, JoinHandle<()>>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts reading a stream; its events go to the UI as `stream://event`.
    pub fn start(&self, app: AppHandle, response: reqwest::Response) -> Result<String, AppError> {
        let mut running = self.lock();
        running.retain(|_, task| !task.inner().is_finished());
        if running.len() >= MAX_STREAMS {
            return Err(AppError::invalid(
                "Ten event streams are open; stop one before starting another",
            ));
        }
        let id = new_id();
        let stream_id = id.clone();
        let task = tauri::async_runtime::spawn(async move {
            let error = read(&app, &stream_id, response)
                .await
                .err()
                .map(|error| error.to_string());
            // The window may be closing; then nobody is listening.
            let _ = app.emit(
                END,
                StreamEnd {
                    stream_id: stream_id.clone(),
                    error,
                },
            );
            app.state::<EventStreams>().lock().remove(&stream_id);
        });
        running.insert(id.clone(), task);
        Ok(id)
    }

    pub fn stop(&self, id: &str) {
        if let Some(task) = self.lock().remove(id) {
            task.abort();
        }
    }
}

async fn read(
    app: &AppHandle,
    stream_id: &str,
    response: reqwest::Response,
) -> Result<(), AppError> {
    let mut parser = Parser::default();
    let mut body = response.bytes_stream();
    while let Some(chunk) = body.next().await {
        for event in parser.feed(&chunk.map_err(map_error)?)? {
            let event = StreamEvent {
                stream_id: stream_id.to_owned(),
                event: event.name,
                data: event.data,
                id: event.id,
                at: now_ms(),
            };
            // A closed window stops listening; the stream ends with it.
            if app.emit(EVENT, event).is_err() {
                return Ok(());
            }
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Event {
    name: String,
    data: String,
    id: Option<String>,
}

/// The event-stream format (WHATWG HTML §9.2): `field: value` lines, events separated by a blank
/// line, comments starting with `:`. Chunks may split lines anywhere.
#[derive(Default)]
struct Parser {
    pending: Vec<u8>,
    name: String,
    data: Vec<String>,
    id: Option<String>,
    skip_newline: bool,
}

impl Parser {
    fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Event>, AppError> {
        let mut events = Vec::new();
        for &byte in chunk {
            // A CRLF pair ends one line, not two.
            if std::mem::take(&mut self.skip_newline) && byte == b'\n' {
                continue;
            }
            if byte == b'\r' || byte == b'\n' {
                self.skip_newline = byte == b'\r';
                let line = String::from_utf8_lossy(&std::mem::take(&mut self.pending)).into_owned();
                if let Some(event) = self.line(&line) {
                    events.push(event);
                }
                continue;
            }
            self.pending.push(byte);
            if self.pending.len() + self.data.iter().map(String::len).sum::<usize>()
                > MAX_EVENT_BYTES
            {
                return Err(AppError::invalid(
                    "An event in the stream is larger than 1 MB",
                ));
            }
        }
        Ok(events)
    }

    fn line(&mut self, line: &str) -> Option<Event> {
        if line.is_empty() {
            if self.data.is_empty() {
                self.name.clear();
                return None;
            }
            let name = std::mem::take(&mut self.name);
            return Some(Event {
                name: if name.is_empty() {
                    "message".to_owned()
                } else {
                    name
                },
                data: std::mem::take(&mut self.data).join("\n"),
                id: self.id.clone(),
            });
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => value.clone_into(&mut self.name),
            "data" => self.data.push(value.to_owned()),
            "id" if !value.contains('\0') => self.id = Some(value.to_owned()),
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_events_split_across_chunks() {
        let mut parser = Parser::default();
        let mut events = parser
            .feed(b": hello\r\ndata: one\r\n\r\nevent: tick\ndata: a")
            .expect("feed");
        events.extend(
            parser
                .feed(b"\ndata: b\nid: 7\n\ndata:nospace\r\r")
                .expect("feed"),
        );
        events.extend(parser.feed(b"retry: 100\n\n").expect("feed"));
        assert_eq!(
            events,
            [
                Event {
                    name: "message".to_owned(),
                    data: "one".to_owned(),
                    id: None
                },
                Event {
                    name: "tick".to_owned(),
                    data: "a\nb".to_owned(),
                    id: Some("7".to_owned())
                },
                Event {
                    name: "message".to_owned(),
                    data: "nospace".to_owned(),
                    id: Some("7".to_owned())
                },
            ]
        );
    }

    #[test]
    fn refuses_huge_events() {
        let mut parser = Parser::default();
        assert!(parser.feed(&vec![b'x'; MAX_EVENT_BYTES + 1]).is_err());
    }
}
