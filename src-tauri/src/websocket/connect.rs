use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, SupportedProtocolVersion};
use rustls_platform_verifier::BuilderVerifierExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{WebSocketStream, client_async_with_config};
use url::Url;

use super::models::WsConnectInput;
use crate::environment::lookup::resolve_request;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::preferences::models::TlsVersion;
use crate::preferences::repo::load_preferences;
use crate::request::client::root_cause;
use crate::request::types::{HttpRequestInput, NameValue};
use crate::request::validate::{build_url, parse_headers};
use crate::storage::db::Database;
use crate::storage::validate::check_optional_id;

const WS_SCHEMES: [&str; 2] = ["ws", "wss"];
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
/// The handshake sets these itself; overriding them would break or spoof the upgrade.
const RESERVED_HEADERS: [&str; 6] = [
    "host",
    "connection",
    "upgrade",
    "sec-websocket-key",
    "sec-websocket-version",
    "sec-websocket-extensions",
];

pub trait Transport: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Transport for T {}

pub type Socket = WebSocketStream<Box<dyn Transport>>;

pub struct Opened {
    pub socket: Socket,
    pub protocol: Option<String>,
}

/// Fills in `{{variables}}` the same way HTTP requests do, then opens the connection.
pub async fn open_input(
    db: &Database,
    vault: &Vault,
    input: WsConnectInput,
) -> Result<Opened, AppError> {
    check_optional_id(input.collection_id.as_deref())?;
    let template = HttpRequestInput {
        method: "GET".to_owned(),
        url: input.url,
        params: input.params,
        headers: input.headers,
        body: None,
        timeout_ms: None,
        collection_id: input.collection_id,
        request_id: None,
        name: None,
        pre_request_script: None,
        test_script: None,
        auth: None,
        record_history: true,
        path_variables: Vec::new(),
        iteration_data: Vec::new(),
        iteration: Default::default(),
    };
    let resolved = resolve_request(db, vault, template, &HashMap::new())?;
    let min_tls = db.with(load_preferences)?.requests.min_tls_version;
    open(&resolved.url, &resolved.params, &resolved.headers, min_tls).await
}

pub async fn open(
    raw_url: &str,
    params: &[NameValue],
    headers: &[NameValue],
    min_tls: TlsVersion,
) -> Result<Opened, AppError> {
    let url = build_url(raw_url, params, &WS_SCHEMES)?;
    let extra_headers = parse_headers(headers)?;
    if extra_headers
        .keys()
        .any(|name| RESERVED_HEADERS.contains(&name.as_str()))
    {
        return Err(AppError::invalid(
            "Host, Connection, Upgrade and Sec-WebSocket-Key/Version/Extensions are set automatically",
        ));
    }
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|_| AppError::invalid("The URL is not valid"))?;
    for (name, value) in &extra_headers {
        request.headers_mut().append(name, value.clone());
    }
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_MESSAGE_BYTES));

    let handshake = async {
        let stream = transport(&url, min_tls).await?;
        client_async_with_config(request, stream, Some(config))
            .await
            .map_err(handshake_error)
    };
    let (socket, response) = tokio::time::timeout(CONNECT_TIMEOUT, handshake)
        .await
        .map_err(|_| AppError::Timeout)??;
    let protocol = response
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    Ok(Opened { socket, protocol })
}

async fn transport(url: &Url, min_tls: TlsVersion) -> Result<Box<dyn Transport>, AppError> {
    let host = url
        .host_str()
        .map(|host| {
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned()
        })
        .ok_or_else(|| AppError::invalid("The URL has no host"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| AppError::invalid("The URL has no port"))?;
    let tcp = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|error| AppError::Connect(root_cause(&error)))?;
    if url.scheme() == "ws" {
        return Ok(Box::new(tcp));
    }
    let server_name =
        ServerName::try_from(host).map_err(|_| AppError::invalid("The URL has no host"))?;
    let tls = tls_connector(min_tls)?
        .connect(server_name, tcp)
        .await
        .map_err(|error| AppError::Connect(root_cause(&error)))?;
    Ok(Box::new(tls))
}

/// Certificates are checked against the operating system's trust store, like browsers do.
fn tls_connector(min_tls: TlsVersion) -> Result<TlsConnector, AppError> {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let versions: &[&SupportedProtocolVersion] = match min_tls {
        TlsVersion::Tls12 => &[&TLS13, &TLS12],
        TlsVersion::Tls13 => &[&TLS13],
    };
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(versions)
        .and_then(BuilderVerifierExt::with_platform_verifier)
        .map_err(|_| AppError::ClientInit)?
        .with_no_client_auth();
    // The WebSocket upgrade only exists in HTTP/1.1, so ALPN must not pick h2.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(TlsConnector::from(Arc::new(config)))
}

fn handshake_error(error: WsError) -> AppError {
    match error {
        WsError::Http(response) => AppError::Connect(format!(
            "the server answered HTTP {} instead of switching to WebSocket",
            response.status().as_u16()
        )),
        other => AppError::Connect(root_cause(&other)),
    }
}
