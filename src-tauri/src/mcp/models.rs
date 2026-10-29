use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::protocol::{RpcError, ServerInfo};
use crate::storage::models::KeyValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Http,
    Stdio,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpConnectInput {
    pub transport: McpTransport,
    /// The server URL (HTTP) or the command that starts it (STDIO).
    pub target: String,
    /// HTTP headers, or environment variables for a STDIO server.
    #[serde(default)]
    pub pairs: Vec<KeyValue>,
    /// Used for collection variables only.
    #[serde(default)]
    pub collection_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpRequestInput {
    pub session_id: String,
    pub method: String,
    /// JSON text; its `{{variables}}` are filled in first.
    pub params: Option<String>,
    pub collection_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSession {
    pub session_id: String,
    pub server: ServerInfo,
}

/// A program Codestamp has not run before with exactly these settings, shown for approval.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLaunch {
    pub program: String,
    pub args: Vec<String>,
    /// Names only; values may be secrets.
    pub env: Vec<String>,
    /// Sent back with the approval, so only what was shown can be started.
    pub fingerprint: String,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum McpConnectOutcome {
    Connected { session: McpSession },
    NeedsApproval { launch: McpLaunch },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpReply {
    pub result: Option<Value>,
    pub error: Option<RpcError>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Sent,
    Received,
}

/// Pushed to the UI while a session is open; `Closed` is always the last event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum McpEvent {
    Traffic {
        direction: Direction,
        text: String,
        size: usize,
        truncated: bool,
    },
    /// A line the server wrote to stderr, or a logging notification.
    Log {
        text: String,
    },
    Closed {
        reason: String,
    },
}
