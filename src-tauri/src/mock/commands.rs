use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, State};

use super::server::{self, RunningMock};
use crate::error::AppError;
use crate::storage::validate::check_id;

/// The mock servers running now, one per collection at most.
#[derive(Default)]
pub struct MockServers {
    running: Mutex<HashMap<String, RunningMock>>,
}

impl MockServers {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, RunningMock>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockStatus {
    pub port: u16,
    pub url: String,
}

fn status(port: u16) -> MockStatus {
    MockStatus {
        port,
        url: format!("http://127.0.0.1:{port}"),
    }
}

/// Starts (or restarts on another port) the mock server of a collection. Port 0 picks a free
/// one; ports below 1024 need extra rights and are refused.
#[tauri::command]
pub async fn mock_start(
    app: AppHandle,
    servers: State<'_, MockServers>,
    collection_id: String,
    port: u16,
) -> Result<MockStatus, AppError> {
    check_id(&collection_id)?;
    if port != 0 && port < 1024 {
        return Err(AppError::invalid(
            "Choose a port from 1024 to 65535, or 0 for any free one",
        ));
    }
    {
        let mut running = servers.lock();
        match running.get(&collection_id) {
            Some(current) if port == 0 || current.port == port => return Ok(status(current.port)),
            Some(_) => {
                running.remove(&collection_id);
            }
            None => {}
        }
    }
    let started = server::start(app, collection_id.clone(), port).await?;
    let port = started.port;
    servers.lock().insert(collection_id, started);
    Ok(status(port))
}

#[tauri::command]
pub async fn mock_stop(
    servers: State<'_, MockServers>,
    collection_id: String,
) -> Result<(), AppError> {
    check_id(&collection_id)?;
    servers.lock().remove(&collection_id);
    Ok(())
}

#[tauri::command]
pub async fn mock_status(
    servers: State<'_, MockServers>,
    collection_id: String,
) -> Result<Option<MockStatus>, AppError> {
    check_id(&collection_id)?;
    Ok(servers
        .lock()
        .get(&collection_id)
        .map(|running| status(running.port)))
}
