//! Connecting to a gRPC server and calling a method with messages built from JSON.

use std::time::{Duration, Instant};

use futures_util::StreamExt;
use prost_reflect::{DynamicMessage, MethodDescriptor, SerializeOptions};
use serde::Serialize;
use tonic::client::Grpc;
use tonic::codegen::http::uri::PathAndQuery;
use tonic::metadata::{MetadataKey, MetadataMap, MetadataValue};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};
use tonic::{Code, Request, Status};
use url::Url;

use super::codec::DynamicCodec;
use crate::error::AppError;
use crate::request::client::root_cause;
use crate::request::network::NetworkConfig;
use crate::request::types::NameValue;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;
/// A server stream is read until it ends or this many messages arrived.
const MAX_STREAM_MESSAGES: usize = 200;

/// A server address: `grpc://` is plaintext HTTP/2, `grpcs://` (or `https://`) uses TLS.
pub struct Address {
    uri: String,
    tls: bool,
    url: Url,
}

pub fn address(raw: &str) -> Result<Address, AppError> {
    let wrong = || {
        AppError::invalid(
            "Enter the server address with grpc:// (plaintext) or grpcs:// (TLS), such as grpcs://api.example.com:443",
        )
    };
    let url = Url::parse(raw.trim()).map_err(|_| wrong())?;
    let tls = match url.scheme() {
        "grpcs" | "https" => true,
        "grpc" | "http" => false,
        _ => return Err(wrong()),
    };
    let host = url
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(wrong)?;
    let port = url.port().unwrap_or(if tls { 443 } else { 80 });
    let scheme = if tls { "https" } else { "http" };
    let uri = format!("{scheme}://{host}:{port}");
    let url = Url::parse(&uri).map_err(|_| wrong())?;
    Ok(Address { uri, tls, url })
}

/// TLS trusts the operating system's roots plus the CA certificates from Settings, and sends a
/// client certificate set for this host. Proxies are not used for gRPC.
pub async fn connect(
    address: &Address,
    config: &NetworkConfig,
    timeout: Duration,
) -> Result<Channel, AppError> {
    let mut endpoint = Endpoint::from_shared(address.uri.clone())
        .map_err(|_| AppError::invalid("The server address is not valid"))?
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(timeout)
        .user_agent(config.user_agent.clone())
        .map_err(|_| AppError::invalid("The User-Agent from Settings cannot be used"))?;
    if address.tls {
        let mut tls = ClientTlsConfig::new()
            .with_native_roots()
            .domain_name(address.url.host_str().unwrap_or_default().to_owned());
        for pem in &config.ca_pems {
            tls = tls.ca_certificate(Certificate::from_pem(pem));
        }
        if let Some(identity) = config
            .identities
            .iter()
            .find(|identity| identity.host.matches(&address.url))
        {
            // The stored PEM holds the certificate chain and its key together.
            tls = tls.identity(Identity::from_pem(&identity.pem, &identity.pem));
        }
        endpoint = endpoint
            .tls_config(tls)
            .map_err(|_| AppError::invalid("The TLS settings cannot be used for gRPC"))?;
    }
    endpoint
        .connect()
        .await
        .map_err(|error| AppError::Connect(root_cause(&error)))
}

/// Metadata from `name: value` rows; names are lowercased as gRPC requires.
pub fn metadata(rows: &[NameValue]) -> Result<MetadataMap, AppError> {
    let mut map = MetadataMap::new();
    for row in rows {
        let name = row.name.trim().to_ascii_lowercase();
        if name.ends_with("-bin") {
            return Err(AppError::InvalidInput(format!(
                "Binary metadata ({name}) is not supported yet"
            )));
        }
        let key = MetadataKey::from_bytes(name.as_bytes()).map_err(|_| {
            AppError::InvalidInput(format!("“{name}” is not a valid metadata name"))
        })?;
        let value = MetadataValue::try_from(row.value.as_str()).map_err(|_| {
            AppError::InvalidInput(format!("The value of “{name}” cannot be sent as metadata"))
        })?;
        map.append(key, value);
    }
    Ok(map)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcReply {
    /// The gRPC status code (0 is OK).
    pub code: i32,
    pub status: String,
    pub message: String,
    /// Each answer as proto3 JSON.
    pub messages: Vec<String>,
    pub headers: Vec<NameValue>,
    pub trailers: Vec<NameValue>,
    pub elapsed_ms: u64,
    /// Set when a long server stream was cut short.
    pub warning: Option<String>,
}

/// Calls the method. Every kind of method goes out as a stream, which is the same on the wire,
/// so headers and trailers stay apart: streams from the client send every message given, and
/// answers are read until the server ends them.
pub async fn invoke(
    channel: Channel,
    method: &MethodDescriptor,
    messages: Vec<DynamicMessage>,
    metadata: MetadataMap,
) -> Result<GrpcReply, AppError> {
    let started = Instant::now();
    let path = PathAndQuery::from_maybe_shared(format!(
        "/{}/{}",
        method.parent_service().full_name(),
        method.name()
    ))
    .map_err(|_| AppError::invalid("The method name is not valid"))?;
    let messages = if method.is_client_streaming() {
        messages
    } else {
        vec![first(messages)?]
    };
    let codec = DynamicCodec::new(method.output());
    let mut grpc = Grpc::new(channel).max_decoding_message_size(MAX_MESSAGE_BYTES);
    grpc.ready()
        .await
        .map_err(|error| AppError::Connect(root_cause(&error)))?;
    let mut request = Request::new(futures_util::stream::iter(messages));
    *request.metadata_mut() = metadata;
    let outcome = match grpc.streaming(request, path, codec).await {
        Ok(mut response) => {
            let headers = rows(response.metadata());
            let stream = response.get_mut();
            let mut answers = Vec::new();
            let mut warning = None;
            let mut failure = None;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(message) => answers.push(message),
                    Err(status) => {
                        failure = Some(status);
                        break;
                    }
                }
                if answers.len() >= MAX_STREAM_MESSAGES {
                    warning = Some(format!("Stopped after {MAX_STREAM_MESSAGES} messages"));
                    break;
                }
            }
            let trailers = if failure.is_none() && warning.is_none() {
                match stream.trailers().await {
                    Ok(trailers) => trailers.as_ref().map(rows).unwrap_or_default(),
                    Err(status) => {
                        failure = Some(status);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            };
            Ok((answers, headers, trailers, failure, warning))
        }
        Err(status) => Err(status),
    };
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match outcome {
        Ok((answers, headers, trailers, failure, warning)) => {
            let status = failure.unwrap_or_else(|| Status::new(Code::Ok, ""));
            Ok(GrpcReply {
                code: status.code() as i32,
                status: code_name(status.code()),
                message: status.message().to_owned(),
                messages: answers.iter().map(to_json).collect(),
                headers,
                trailers,
                elapsed_ms,
                warning,
            })
        }
        Err(status) => Ok(GrpcReply {
            code: status.code() as i32,
            status: code_name(status.code()),
            message: status.message().to_owned(),
            messages: Vec::new(),
            headers: Vec::new(),
            trailers: rows(status.metadata()),
            elapsed_ms,
            warning: None,
        }),
    }
}

fn first(messages: Vec<DynamicMessage>) -> Result<DynamicMessage, AppError> {
    messages
        .into_iter()
        .next()
        .ok_or_else(|| AppError::invalid("Write the message to send"))
}

fn to_json(message: &DynamicMessage) -> String {
    let options = SerializeOptions::new().skip_default_fields(true);
    message
        .serialize_with_options(serde_json::value::Serializer, &options)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_default()
}

fn rows(metadata: &MetadataMap) -> Vec<NameValue> {
    metadata
        .clone()
        .into_headers()
        .iter()
        .map(|(name, value)| NameValue {
            name: name.as_str().to_owned(),
            value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
        })
        .collect()
}

/// The status name as gRPC documents it, such as `NOT_FOUND`.
fn code_name(code: Code) -> String {
    match code {
        Code::Ok => "OK",
        Code::Cancelled => "CANCELLED",
        Code::Unknown => "UNKNOWN",
        Code::InvalidArgument => "INVALID_ARGUMENT",
        Code::DeadlineExceeded => "DEADLINE_EXCEEDED",
        Code::NotFound => "NOT_FOUND",
        Code::AlreadyExists => "ALREADY_EXISTS",
        Code::PermissionDenied => "PERMISSION_DENIED",
        Code::ResourceExhausted => "RESOURCE_EXHAUSTED",
        Code::FailedPrecondition => "FAILED_PRECONDITION",
        Code::Aborted => "ABORTED",
        Code::OutOfRange => "OUT_OF_RANGE",
        Code::Unimplemented => "UNIMPLEMENTED",
        Code::Internal => "INTERNAL",
        Code::Unavailable => "UNAVAILABLE",
        Code::DataLoss => "DATA_LOSS",
        Code::Unauthenticated => "UNAUTHENTICATED",
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_server_addresses() {
        let plain = address("grpc://localhost:50051").expect("address");
        assert_eq!(plain.uri, "http://localhost:50051");
        assert!(!plain.tls);
        let secure = address("grpcs://api.example.test").expect("address");
        assert_eq!(secure.uri, "https://api.example.test:443");
        assert!(secure.tls);
        assert!(address("api.example.test:443").is_err());
        assert!(address("ftp://api.example.test").is_err());
    }

    #[test]
    fn builds_metadata() {
        let rows = vec![NameValue {
            name: "X-Trace".to_owned(),
            value: "abc".to_owned(),
        }];
        let map = metadata(&rows).expect("metadata");
        assert_eq!(
            map.get("x-trace").and_then(|v| v.to_str().ok()),
            Some("abc")
        );
        let binary = vec![NameValue {
            name: "data-bin".to_owned(),
            value: "x".to_owned(),
        }];
        assert!(metadata(&binary).is_err());
    }

    #[test]
    fn names_statuses_as_grpc_does() {
        assert_eq!(code_name(Code::Ok), "OK");
        assert_eq!(code_name(Code::NotFound), "NOT_FOUND");
        assert_eq!(code_name(Code::Unauthenticated), "UNAUTHENTICATED");
    }
}
