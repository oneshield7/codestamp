use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::AppError;

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const SUPPORTED_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const MAX_INSTRUCTIONS_CHARS: usize = 4_000;

/// What the UI may ask a server. Anything else could change the server in ways Codestamp does
/// not show, so it is refused.
pub const CALLABLE_METHODS: [&str; 8] = [
    "ping",
    "tools/list",
    "tools/call",
    "prompts/list",
    "prompts/get",
    "resources/list",
    "resources/templates/list",
    "resources/read",
];

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// One message from a server.
#[derive(Debug, PartialEq)]
pub enum Incoming {
    Response {
        id: u64,
        outcome: Result<Value, RpcError>,
    },
    /// The server asks the client something, such as `ping`.
    Request { id: Value, method: String },
    Notification {
        method: String,
        params: Option<Value>,
    },
}

/// Every message in a JSON text; servers may send a batch as an array.
pub fn parse_messages(text: &str) -> Vec<Incoming> {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(items)) => items.into_iter().filter_map(parse_message).collect(),
        Ok(value) => parse_message(value).into_iter().collect(),
        Err(_) => Vec::new(),
    }
}

fn parse_message(value: Value) -> Option<Incoming> {
    let method = value
        .get("method")
        .and_then(Value::as_str)
        .map(str::to_owned);
    match (value.get("id"), method) {
        (Some(id), Some(method)) => Some(Incoming::Request {
            id: id.clone(),
            method,
        }),
        (None, Some(method)) => Some(Incoming::Notification {
            method,
            params: value.get("params").cloned(),
        }),
        (Some(id), None) => {
            let id = id.as_u64()?;
            let outcome = match value.get("error") {
                Some(error) => Err(serde_json::from_value(error.clone()).unwrap_or(RpcError {
                    code: -32603,
                    message: "The server sent an error Codestamp cannot read".to_owned(),
                    data: None,
                })),
                None => Ok(value.get("result").cloned().unwrap_or(Value::Null)),
            };
            Some(Incoming::Response { id, outcome })
        }
        (None, None) => None,
    }
}

pub fn request(id: u64, method: &str, params: Option<&Value>) -> Value {
    let mut message = json!({ "jsonrpc": "2.0", "id": id, "method": method });
    if let Some(params) = params {
        message["params"] = params.clone();
    }
    message
}

pub fn notification(method: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": method })
}

/// The answer to a server's own request: `ping` is answered, everything else is declined, since
/// Codestamp offers no sampling, roots or elicitation.
pub fn reply_to_server(id: &Value, method: &str) -> Value {
    if method == "ping" {
        json!({ "jsonrpc": "2.0", "id": id, "result": {} })
    } else {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": "Codestamp does not support this request" }
        })
    }
}

pub fn initialize_params() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": { "name": "Codestamp", "version": env!("CARGO_PKG_VERSION") }
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub instructions: Option<String>,
    pub tools: bool,
    pub prompts: bool,
    pub resources: bool,
}

pub fn server_info(result: &Value) -> Result<ServerInfo, AppError> {
    let version = result
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !SUPPORTED_VERSIONS.contains(&version) {
        return Err(AppError::Request(format!(
            "The server speaks MCP version “{version}”, which Codestamp does not support"
        )));
    }
    let text = |pointer: &str| {
        result
            .pointer(pointer)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let has = |capability: &str| {
        result
            .pointer(&format!("/capabilities/{capability}"))
            .is_some()
    };
    Ok(ServerInfo {
        name: text("/serverInfo/name"),
        version: text("/serverInfo/version"),
        protocol_version: version.to_owned(),
        instructions: result
            .get("instructions")
            .and_then(Value::as_str)
            .map(|text| text.chars().take(MAX_INSTRUCTIONS_CHARS).collect()),
        tools: has("tools"),
        prompts: has("prompts"),
        resources: has("resources"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_responses_requests_and_notifications_apart() {
        let messages = parse_messages(
            r#"[{"jsonrpc":"2.0","id":3,"result":{"tools":[]}},
                {"jsonrpc":"2.0","id":4,"error":{"code":-32602,"message":"Unknown tool"}},
                {"jsonrpc":"2.0","id":"s1","method":"ping"},
                {"jsonrpc":"2.0","method":"notifications/message","params":{"level":"info"}}]"#,
        );
        assert_eq!(messages.len(), 4);
        assert!(matches!(
            &messages[0],
            Incoming::Response {
                id: 3,
                outcome: Ok(_)
            }
        ));
        assert!(matches!(
            &messages[1],
            Incoming::Response { id: 4, outcome: Err(error) } if error.code == -32602
        ));
        assert!(matches!(&messages[2], Incoming::Request { method, .. } if method == "ping"));
        assert!(
            matches!(&messages[3], Incoming::Notification { method, .. } if method == "notifications/message")
        );
        assert!(parse_messages("not json").is_empty());
    }

    #[test]
    fn answers_ping_and_declines_other_server_requests() {
        let id = json!("s1");
        assert_eq!(reply_to_server(&id, "ping")["result"], json!({}));
        assert_eq!(
            reply_to_server(&id, "sampling/createMessage")["error"]["code"],
            json!(-32601)
        );
    }

    #[test]
    fn reads_the_server_and_refuses_unknown_versions() {
        let info = server_info(&json!({
            "protocolVersion": "2025-03-26",
            "serverInfo": { "name": "demo", "version": "1.2.0" },
            "capabilities": { "tools": {}, "resources": { "subscribe": true } }
        }))
        .expect("info");
        assert_eq!(info.name, "demo");
        assert!(info.tools && info.resources && !info.prompts);
        assert!(server_info(&json!({ "protocolVersion": "1999-01-01" })).is_err());
    }
}
