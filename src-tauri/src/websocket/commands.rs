use tauri::State;
use tauri::ipc::Channel;

use super::connect::open_input;
use super::manager::WsManager;
use super::models::{WsConnectInput, WsEvent, WsSession};
use crate::environment::lookup::resolve_text;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::rules::MAX_REQUEST_BODY_BYTES;
use crate::storage::db::Database;
use crate::storage::validate::{check_id, check_optional_id};

#[tauri::command]
pub async fn ws_connect(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    manager: State<'_, WsManager>,
    input: WsConnectInput,
    on_event: Channel<WsEvent>,
) -> Result<WsSession, AppError> {
    manager.check_capacity()?;
    let opened = open_input(&db, &vault, input).await?;
    let connection_id = manager.start(opened.socket, move |event| on_event.send(event).is_ok())?;
    Ok(WsSession {
        connection_id,
        protocol: opened.protocol,
    })
}

#[tauri::command]
pub async fn ws_send(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    manager: State<'_, WsManager>,
    connection_id: String,
    text: String,
    collection_id: Option<String>,
) -> Result<(), AppError> {
    check_id(&connection_id)?;
    check_optional_id(collection_id.as_deref())?;
    if text.len() > MAX_REQUEST_BODY_BYTES {
        return Err(AppError::invalid("A message can be up to 10 MB"));
    }
    let resolved = resolve_text(&db, &vault, text, collection_id.as_deref())?;
    manager.send(&connection_id, resolved).await
}

#[tauri::command]
pub async fn ws_close(
    manager: State<'_, WsManager>,
    connection_id: String,
) -> Result<(), AppError> {
    check_id(&connection_id)?;
    manager.close(&connection_id);
    Ok(())
}

/// Called when the UI starts, since connections from before a reload can no longer be reached.
#[tauri::command]
pub async fn ws_close_all(manager: State<'_, WsManager>) -> Result<(), AppError> {
    manager.close_all();
    Ok(())
}
