use serde::{Deserialize, Serialize};

use crate::request::types::NameValue;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WsConnectInput {
    pub url: String,
    #[serde(default)]
    pub params: Vec<NameValue>,
    #[serde(default)]
    pub headers: Vec<NameValue>,
    /// Used for collection variables only.
    #[serde(default)]
    pub collection_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsSession {
    pub connection_id: String,
    /// The subprotocol the server picked from `Sec-WebSocket-Protocol`, if any.
    pub protocol: Option<String>,
}

/// Pushed to the UI while a connection is open; `Closed` is always the last event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WsEvent {
    Message {
        text: String,
        binary: bool,
        size: usize,
        truncated: bool,
    },
    Error {
        message: String,
    },
    Closed {
        code: Option<u16>,
        reason: String,
    },
}
