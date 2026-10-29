use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use super::http::HttpSession;
use super::models::{Direction, McpEvent};
use super::protocol::RpcError;
use super::stdio::StdioSession;
use crate::error::AppError;
use crate::text::clip;

/// The largest message Codestamp reads from a server.
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_PREVIEW_BYTES: usize = 64 * 1024;

/// Sends an event to the UI; false once nobody listens anymore.
pub type Emitter = Arc<dyn Fn(McpEvent) -> bool + Send + Sync>;

/// Shows a message in the UI's message log, clipped so a huge one cannot flood it.
pub fn traffic(emit: &Emitter, direction: Direction, text: &str) {
    let (preview, truncated) = clip(text, MAX_PREVIEW_BYTES);
    emit(McpEvent::Traffic {
        direction,
        text: preview.to_owned(),
        size: text.len(),
        truncated,
    });
}

pub enum Session {
    Http(Box<HttpSession>),
    Stdio(StdioSession),
}

impl Session {
    /// The server's answer: its result, or the JSON-RPC error it returned.
    pub async fn request(
        &self,
        method: &str,
        params: Option<&Value>,
        timeout: Duration,
    ) -> Result<Result<Value, RpcError>, AppError> {
        match self {
            Self::Http(session) => session.request(method, params, timeout).await,
            Self::Stdio(session) => session.request(method, params, timeout).await,
        }
    }

    pub async fn notify(&self, method: &str, timeout: Duration) -> Result<(), AppError> {
        match self {
            Self::Http(session) => session.notify(method, timeout).await,
            Self::Stdio(session) => session.notify(method).await,
        }
    }

    pub async fn close(&self) {
        match self {
            Self::Http(session) => session.close().await,
            Self::Stdio(session) => session.close(),
        }
    }
}
