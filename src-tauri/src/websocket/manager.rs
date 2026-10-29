use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use super::connect::Socket;
use super::models::WsEvent;
use crate::error::AppError;
use crate::request::client::root_cause;
use crate::storage::db::new_id;
use crate::text::clip;

const MAX_CONNECTIONS: usize = 20;
const MAX_PREVIEW_BYTES: usize = 64 * 1024;
const CLOSE_GRACE: Duration = Duration::from_secs(5);
/// A peer that stops reading would otherwise block a send (and the connection) forever.
const SEND_TIMEOUT: Duration = Duration::from_secs(10);
const COMMAND_QUEUE: usize = 32;

struct Outgoing {
    text: String,
    reply: oneshot::Sender<Result<(), AppError>>,
}

/// Dropping a connection's sender (removing it here) tells its task to close.
type Registry = Arc<Mutex<HashMap<String, mpsc::Sender<Outgoing>>>>;

/// Open connections by id. Each one runs in its own task that owns the socket.
#[derive(Default)]
pub struct WsManager {
    connections: Registry,
}

impl WsManager {
    pub fn check_capacity(&self) -> Result<(), AppError> {
        if lock(&self.connections).len() >= MAX_CONNECTIONS {
            return Err(capacity_error());
        }
        Ok(())
    }

    /// `emit` returns false once nobody listens anymore, which closes the connection.
    pub fn start(
        &self,
        socket: Socket,
        emit: impl FnMut(WsEvent) -> bool + Send + 'static,
    ) -> Result<String, AppError> {
        let id = new_id();
        let (sender, receiver) = mpsc::channel(COMMAND_QUEUE);
        {
            // Checked and inserted under one lock so parallel connects cannot exceed the limit.
            let mut connections = lock(&self.connections);
            if connections.len() >= MAX_CONNECTIONS {
                return Err(capacity_error());
            }
            connections.insert(id.clone(), sender);
        }

        let registry = Arc::clone(&self.connections);
        let task_id = id.clone();
        tokio::spawn(async move {
            run(socket, receiver, emit).await;
            lock(&registry).remove(&task_id);
        });
        Ok(id)
    }

    pub async fn send(&self, id: &str, text: String) -> Result<(), AppError> {
        let sender = lock(&self.connections)
            .get(id)
            .cloned()
            .ok_or(AppError::NotFound("The connection"))?;
        let (reply, outcome) = oneshot::channel();
        sender
            .send(Outgoing { text, reply })
            .await
            .map_err(|_| AppError::NotFound("The connection"))?;
        outcome
            .await
            .map_err(|_| AppError::NotFound("The connection"))?
    }

    /// Closing a connection that has already ended is not an error.
    pub fn close(&self, id: &str) {
        lock(&self.connections).remove(id);
    }

    /// Closes every connection, for example after the UI reloaded and forgot their ids.
    pub fn close_all(&self) {
        lock(&self.connections).clear();
    }
}

fn capacity_error() -> AppError {
    AppError::invalid("Up to 20 WebSocket connections can be open at once")
}

/// The map stays consistent even if a holder panicked, so a poisoned lock is still usable.
fn lock(registry: &Registry) -> MutexGuard<'_, HashMap<String, mpsc::Sender<Outgoing>>> {
    registry.lock().unwrap_or_else(PoisonError::into_inner)
}

async fn run(
    mut socket: Socket,
    mut commands: mpsc::Receiver<Outgoing>,
    mut emit: impl FnMut(WsEvent) -> bool,
) {
    let mut close_frame = None;
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(Outgoing { text, reply }) => {
                    let sent = tokio::time::timeout(SEND_TIMEOUT, socket.send(Message::text(text))).await;
                    let Ok(outcome) = sent else {
                        // Nobody waits for the reply if the UI call was cancelled.
                        let _ = reply.send(Err(AppError::Timeout));
                        emit(WsEvent::Error { message: "The server stopped reading messages".to_owned() });
                        break;
                    };
                    let _ = reply.send(outcome.map_err(|error| AppError::Request(root_cause(&error))));
                }
                None => {
                    finish(&mut socket, &mut close_frame).await;
                    break;
                }
            },
            frame = socket.next() => match frame {
                Some(Ok(message)) => {
                    let Some(event) = incoming(message, &mut close_frame) else {
                        continue;
                    };
                    if !emit(event) {
                        finish(&mut socket, &mut close_frame).await;
                        break;
                    }
                }
                Some(Err(error)) => {
                    emit(WsEvent::Error { message: root_cause(&error) });
                    break;
                }
                None => break,
            },
        }
    }
    emit(closed_event(close_frame));
}

/// Sends our close frame and waits briefly for the server's, so its code can be shown.
async fn finish(socket: &mut Socket, close_frame: &mut Option<CloseFrame>) {
    let goodbye = CloseFrame {
        code: CloseCode::Normal,
        reason: "".into(),
    };
    let closed = tokio::time::timeout(CLOSE_GRACE, socket.close(Some(goodbye))).await;
    if !matches!(closed, Ok(Ok(()))) {
        return;
    }
    let drain = async {
        while let Some(Ok(message)) = socket.next().await {
            if let Message::Close(frame) = message {
                *close_frame = frame;
            }
        }
    };
    // A server that never answers just ends the wait; the connection is closed either way.
    let _ = tokio::time::timeout(CLOSE_GRACE, drain).await;
}

fn incoming(message: Message, close_frame: &mut Option<CloseFrame>) -> Option<WsEvent> {
    match message {
        Message::Text(text) => {
            let (preview, truncated) = clip(text.as_str(), MAX_PREVIEW_BYTES);
            Some(WsEvent::Message {
                text: preview.to_owned(),
                binary: false,
                size: text.len(),
                truncated,
            })
        }
        Message::Binary(bytes) => Some(WsEvent::Message {
            text: String::new(),
            binary: true,
            size: bytes.len(),
            truncated: false,
        }),
        Message::Close(frame) => {
            *close_frame = frame;
            None
        }
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => None,
    }
}

fn closed_event(frame: Option<CloseFrame>) -> WsEvent {
    match frame {
        Some(frame) => WsEvent::Closed {
            code: Some(u16::from(frame.code)),
            reason: frame.reason.to_string(),
        },
        None => WsEvent::Closed {
            code: None,
            reason: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_close_codes() {
        let frame = CloseFrame {
            code: CloseCode::Away,
            reason: "bye".into(),
        };
        assert_eq!(
            closed_event(Some(frame)),
            WsEvent::Closed {
                code: Some(1001),
                reason: "bye".to_owned()
            }
        );
    }
}
