use serde::{Deserialize, Serialize};

use crate::auth::models::Auth;
use crate::soap::SoapVersion;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummary {
    pub id: String,
    pub name: String,
    pub request_count: i64,
    /// The chosen emoji, or the one picked from the name.
    pub icon: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestKind {
    #[default]
    Http,
    Websocket,
    Mcp,
    Graphql,
    Grpc,
    Soap,
}

impl RequestKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Websocket => "websocket",
            Self::Mcp => "mcp",
            Self::Graphql => "graphql",
            Self::Grpc => "grpc",
            Self::Soap => "soap",
        }
    }

    /// What reports and Docs show in place of the method, the same stamps as the app.
    pub const fn stamp(self, method: &str) -> &str {
        match self {
            Self::Http => method,
            Self::Graphql => "GQL",
            Self::Websocket => "WS",
            Self::Mcp => "MCP",
            Self::Grpc => "gRPC",
            Self::Soap => "SOAP",
        }
    }

    /// Why Run all and the command-line runner pass over this kind; `None` for kinds they send.
    pub const fn run_skip_reason(self) -> Option<&'static str> {
        match self {
            Self::Http | Self::Graphql | Self::Soap => None,
            Self::Websocket => Some("WebSocket requests are not part of a run"),
            Self::Mcp => Some("MCP requests are not part of a run"),
            Self::Grpc => Some("gRPC requests are not part of a run"),
        }
    }

    /// Unknown values from older or edited databases fall back to plain HTTP.
    pub fn from_stored(text: &str) -> Self {
        match text {
            "websocket" => Self::Websocket,
            "mcp" => Self::Mcp,
            "graphql" => Self::Graphql,
            "grpc" => Self::Grpc,
            "soap" => Self::Soap,
            _ => Self::Http,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum McpOperationKind {
    Tool,
    Prompt,
    Resource,
}

/// What an MCP request calls: a tool or prompt by name, or a resource by URI. Its arguments
/// are the request body (JSON).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpOperation {
    pub kind: McpOperationKind,
    pub name: String,
}

/// What a SOAP request calls; the endpoint is `url` and the envelope `body` (XML).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoapSettings {
    /// Where the WSDL is read from: a URL, or a file on this device (not shared).
    pub wsdl: String,
    pub version: SoapVersion,
    /// The WSDL binding the operation belongs to.
    pub binding: String,
    pub operation: String,
    /// Sent as the SOAPAction header (1.1) or the Content-Type action (1.2).
    pub action: String,
}

/// Where a gRPC request learns the server's services: by asking the server, or from a file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GrpcSource {
    #[default]
    Reflection,
    Proto,
}

/// What a gRPC request calls. Its address is `url`, its metadata `headers` and its message
/// (JSON) `body`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrpcSettings {
    /// `package.Service/Method`; empty until one is chosen.
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub source: GrpcSource,
    /// The `.proto` file picked on this device.
    #[serde(default)]
    pub proto_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSummary {
    pub id: String,
    pub section_id: Option<String>,
    pub kind: RequestKind,
    pub name: String,
    pub method: String,
    pub url: String,
}

/// One request as quick search lists it, with where it lives.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntry {
    pub request_id: String,
    pub collection_id: String,
    pub collection_name: String,
    pub collection_icon: String,
    pub section_name: Option<String>,
    pub kind: RequestKind,
    pub name: String,
    pub method: String,
    pub url: String,
}

/// Only summaries are sent for a collection; full request content loads when a card opens.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDetail {
    pub id: String,
    pub name: String,
    /// Used by requests whose auth is `Inherit`.
    pub auth: Auth,
    pub icon: String,
    /// False while the icon follows the name.
    pub icon_custom: bool,
    pub sections: Vec<SectionInfo>,
    pub requests: Vec<RequestSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyValue {
    pub enabled: bool,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FormFieldKind {
    #[default]
    Text,
    File,
}

/// One row of a URL-encoded or form-data body. For a file row, `value` is the path picked in
/// the system dialog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormField {
    pub enabled: bool,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub kind: FormFieldKind,
}

/// `body` holds raw text (JSON or plain), `form` the rows of URL-encoded and form-data bodies,
/// and `body_file` the file sent as a binary body. Each keeps its content when the mode changes.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestContent {
    #[serde(default)]
    pub kind: RequestKind,
    pub name: String,
    pub method: String,
    pub url: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body_mode: String,
    pub body: String,
    #[serde(default)]
    pub form: Vec<FormField>,
    #[serde(default)]
    pub body_file: String,
    #[serde(default)]
    pub pre_request_script: String,
    #[serde(default)]
    pub test_script: String,
    #[serde(default)]
    pub auth: Auth,
    /// MCP requests only.
    #[serde(default)]
    pub mcp_operation: Option<McpOperation>,
    /// Values for the `:name` segments of the URL path.
    #[serde(default)]
    pub path_variables: Vec<PathVariable>,
    /// GraphQL requests only: the variables (JSON); the query is `body`.
    #[serde(default)]
    pub graphql_variables: String,
    /// gRPC requests only.
    #[serde(default)]
    pub grpc: Option<GrpcSettings>,
    /// SOAP requests only.
    #[serde(default)]
    pub soap: Option<SoapSettings>,
}

/// The value of a `:name` path segment; `{{variables}}` in it are filled in when sending.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PathVariable {
    pub key: String,
    pub value: String,
}

/// One group in a collection's new order: requests of a section (or with no section).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestGroup {
    pub section_id: Option<String>,
    pub request_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRequest {
    pub id: String,
    pub collection_id: String,
    pub section_id: Option<String>,
    #[serde(flatten)]
    pub content: RequestContent,
}
