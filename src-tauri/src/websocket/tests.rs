use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};

use super::connect::open;
use super::manager::WsManager;
use super::models::WsEvent;
use crate::preferences::models::TlsVersion;
use crate::request::types::NameValue;

const WAIT: Duration = Duration::from_secs(5);

fn pair(name: &str, value: &str) -> NameValue {
    NameValue {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

#[expect(
    clippy::result_large_err,
    reason = "the signature is fixed by tungstenite's handshake callback"
)]
fn accept_protocol(request: &Request, mut response: Response) -> Result<Response, ErrorResponse> {
    if let Some(protocol) = request.headers().get("sec-websocket-protocol") {
        response
            .headers_mut()
            .insert("sec-websocket-protocol", protocol.clone());
    }
    Ok(response)
}

/// Echoes text as `echo:<text>` and accepts whatever subprotocol the client asks for.
async fn echo_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut socket = tokio_tungstenite::accept_hdr_async(stream, accept_protocol)
            .await
            .expect("handshake");
        while let Some(Ok(message)) = socket.next().await {
            if let Message::Text(text) = message {
                let reply = Message::text(format!("echo:{}", text.as_str()));
                socket.send(reply).await.expect("echo");
            }
        }
    });
    format!("ws://{address}/chat")
}

async fn next_event(events: &mut mpsc::UnboundedReceiver<WsEvent>) -> WsEvent {
    tokio::time::timeout(WAIT, events.recv())
        .await
        .expect("event in time")
        .expect("event")
}

#[tokio::test]
async fn sends_receives_and_closes() {
    let url = echo_server().await;
    let opened = open(
        &url,
        &[pair("v", "1")],
        &[pair("Sec-WebSocket-Protocol", "chat.v1")],
        TlsVersion::Tls12,
    )
    .await
    .expect("connect");
    assert_eq!(opened.protocol.as_deref(), Some("chat.v1"));

    let manager = WsManager::default();
    let (sender, mut events) = mpsc::unbounded_channel();
    let id = manager
        .start(opened.socket, move |event| sender.send(event).is_ok())
        .expect("start");

    manager.send(&id, "hi".to_owned()).await.expect("send");
    assert_eq!(
        next_event(&mut events).await,
        WsEvent::Message {
            text: "echo:hi".to_owned(),
            binary: false,
            size: 7,
            truncated: false
        }
    );

    manager.close(&id);
    assert!(matches!(
        next_event(&mut events).await,
        WsEvent::Closed {
            code: Some(1000),
            ..
        }
    ));
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(manager.send(&id, "late".to_owned()).await.is_err());
}

#[tokio::test]
async fn close_all_ends_every_connection() {
    let manager = WsManager::default();
    let (sender, mut events) = mpsc::unbounded_channel();
    for _ in 0..2 {
        let opened = open(&echo_server().await, &[], &[], TlsVersion::Tls12)
            .await
            .expect("connect");
        let events = sender.clone();
        manager
            .start(opened.socket, move |event| events.send(event).is_ok())
            .expect("start");
    }

    manager.close_all();

    for _ in 0..2 {
        assert!(matches!(
            next_event(&mut events).await,
            WsEvent::Closed {
                code: Some(1000),
                ..
            }
        ));
    }
    assert!(manager.check_capacity().is_ok());
}

#[tokio::test]
async fn rejects_other_schemes_and_reserved_headers() {
    let tls = TlsVersion::Tls12;
    assert!(open("http://127.0.0.1:9/", &[], &[], tls).await.is_err());
    assert!(open("file:///etc/passwd", &[], &[], tls).await.is_err());
    assert!(
        open("ws://127.0.0.1:9/", &[], &[pair("Upgrade", "h2c")], tls)
            .await
            .is_err()
    );
    assert!(
        open("ws://127.0.0.1:9/", &[], &[pair("X-Test", "a\r\nb")], tls)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn reports_servers_that_refuse_the_upgrade() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buffer = [0_u8; 1024];
        let read = stream.read(&mut buffer).await.expect("read");
        assert!(read > 0);
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            .await
            .expect("write");
    });

    let error = open(&format!("ws://{address}/"), &[], &[], TlsVersion::Tls12)
        .await
        .err();
    assert!(error.is_some_and(|error| error.to_string().contains("HTTP 404")));
}
