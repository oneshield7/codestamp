use std::sync::Arc;
use std::time::Instant;

use serde_json::Value;
use tauri::State;
use tauri::ipc::Channel;

use super::approvals;
use super::command::{check_env, resolve_program, split_command};
use super::http::HttpSession;
use super::manager::McpManager;
use super::models::{
    McpConnectInput, McpConnectOutcome, McpEvent, McpLaunch, McpReply, McpRequestInput, McpSession,
    McpTransport,
};
use super::protocol::{CALLABLE_METHODS, initialize_params, server_info};
use super::session::{Emitter, Session};
use super::stdio::StdioSession;
use crate::environment::lookup::resolve_text;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::request::rules::{MAX_REQUEST_BODY_BYTES, MAX_URL_LENGTH};
use crate::request::types::NameValue;
use crate::request::validate::{HTTP_SCHEMES, build_url, parse_headers};
use crate::storage::db::Database;
use crate::storage::validate::{check_id, check_optional_id};

/// Connects and starts the MCP session. A program that was never approved with exactly these
/// settings is not started: the UI gets what would run and asks first, then calls again with
/// the fingerprint it showed.
#[tauri::command]
pub async fn mcp_connect(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    manager: State<'_, McpManager>,
    input: McpConnectInput,
    approved: Option<String>,
    on_event: Channel<McpEvent>,
) -> Result<McpConnectOutcome, AppError> {
    manager.check_capacity()?;
    let collection_id = input.collection_id.as_deref();
    check_optional_id(collection_id)?;
    if input.target.len() > MAX_URL_LENGTH {
        return Err(AppError::invalid("The URL or command is too long"));
    }
    let target = resolve_text(&db, &vault, input.target, collection_id)?;
    let mut pairs = Vec::new();
    for row in input
        .pairs
        .into_iter()
        .filter(|row| row.enabled && !row.key.trim().is_empty())
    {
        let value = resolve_text(&db, &vault, row.value, collection_id)?;
        pairs.push((row.key.trim().to_owned(), value));
    }
    let emit: Emitter = Arc::new(move |event| on_event.send(event).is_ok());

    let session = match input.transport {
        McpTransport::Http => {
            let url = build_url(&target, &[], &HTTP_SCHEMES)?;
            let headers = parse_headers(
                &pairs
                    .into_iter()
                    .map(|(name, value)| NameValue { name, value })
                    .collect::<Vec<_>>(),
            )?;
            Session::Http(Box::new(HttpSession::new(
                client.client_for(&url)?,
                url,
                headers,
                emit,
            )))
        }
        McpTransport::Stdio => {
            let mut words = split_command(&target)?;
            check_env(&pairs)?;
            let program = resolve_program(&words.remove(0));
            let program_text = program.display().to_string();
            let fingerprint = approvals::fingerprint(&program_text, &words, &pairs);
            let known = db.with(|conn| approvals::is_approved(conn, &fingerprint))?;
            if !known {
                if approved.as_deref() != Some(fingerprint.as_str()) {
                    return Ok(McpConnectOutcome::NeedsApproval {
                        launch: McpLaunch {
                            program: program_text,
                            args: words,
                            env: pairs.into_iter().map(|(name, _)| name).collect(),
                            fingerprint,
                        },
                    });
                }
                db.with(|conn| approvals::approve(conn, &fingerprint))?;
            }
            Session::Stdio(StdioSession::spawn(&program, &words, &pairs, emit)?)
        }
    };

    let timeout = client.default_timeout()?;
    let server = match start(&session, timeout).await {
        Ok(server) => server,
        Err(error) => {
            session.close().await;
            return Err(error);
        }
    };
    let session_id = manager.insert(session)?;
    Ok(McpConnectOutcome::Connected {
        session: McpSession { session_id, server },
    })
}

async fn start(
    session: &Session,
    timeout: std::time::Duration,
) -> Result<super::protocol::ServerInfo, AppError> {
    let result = session
        .request("initialize", Some(&initialize_params()), timeout)
        .await?
        .map_err(|error| {
            AppError::Request(format!(
                "The server refused to start a session: {}",
                error.message
            ))
        })?;
    let server = server_info(&result)?;
    if let Session::Http(http) = session {
        http.set_protocol_version(&server.protocol_version);
    }
    session.notify("notifications/initialized", timeout).await?;
    Ok(server)
}

/// Asks the server one of the methods the UI offers (`tools/call`, `resources/read`, …).
/// `params` is JSON text whose `{{variables}}` are filled in first, like a request body.
#[tauri::command]
pub async fn mcp_request(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    manager: State<'_, McpManager>,
    input: McpRequestInput,
) -> Result<McpReply, AppError> {
    let McpRequestInput {
        session_id,
        method,
        params,
        collection_id,
    } = input;
    check_id(&session_id)?;
    check_optional_id(collection_id.as_deref())?;
    if !CALLABLE_METHODS.contains(&method.as_str()) {
        return Err(AppError::invalid("Codestamp does not send that MCP method"));
    }
    let params = match params {
        Some(text) => {
            if text.len() > MAX_REQUEST_BODY_BYTES {
                return Err(AppError::invalid("The arguments can be up to 10 MB"));
            }
            let resolved = resolve_text(&db, &vault, text, collection_id.as_deref())?;
            Some(serde_json::from_str::<Value>(&resolved).map_err(|_| {
                AppError::invalid("The arguments are not valid JSON once variables are filled in")
            })?)
        }
        None => None,
    };
    let session = manager.get(&session_id)?;
    let timeout = client.default_timeout()?;
    let started = Instant::now();
    let outcome = session.request(&method, params.as_ref(), timeout).await?;
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(match outcome {
        Ok(result) => McpReply {
            result: Some(result),
            error: None,
            elapsed_ms,
        },
        Err(error) => McpReply {
            result: None,
            error: Some(error),
            elapsed_ms,
        },
    })
}

#[tauri::command]
pub async fn mcp_close(manager: State<'_, McpManager>, session_id: String) -> Result<(), AppError> {
    check_id(&session_id)?;
    manager.close(&session_id).await;
    Ok(())
}

/// Called when the UI starts, since sessions from before a reload can no longer be reached.
#[tauri::command]
pub async fn mcp_close_all(manager: State<'_, McpManager>) -> Result<(), AppError> {
    manager.close_all().await;
    Ok(())
}
