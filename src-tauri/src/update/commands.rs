use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use super::models::{DownloadProgress, UpdateCheck};
use super::settings::{describe, is_configured, release_date, release_notes};
use crate::error::AppError;

const CHECK_TIMEOUT: Duration = Duration::from_secs(20);
// The installer is a few megabytes; slow office links still get there within this.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const PROGRESS_STEP_BYTES: u64 = 128 * 1024;

/// The update found by the last check, so installing never asks the server a second time.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

impl PendingUpdate {
    fn lock(&self) -> MutexGuard<'_, Option<Update>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<UpdateCheck, AppError> {
    let current = app.package_info().version.to_string();
    if !is_configured(app.config().plugins.0.get("updater")) {
        return Ok(UpdateCheck::NotConfigured { current });
    }
    let updater = app
        .updater_builder()
        .timeout(CHECK_TIMEOUT)
        .build()
        .map_err(describe)?;
    let found = updater.check().await.map_err(describe)?;
    let check = match &found {
        None => UpdateCheck::UpToDate { current },
        Some(update) => UpdateCheck::Available {
            current,
            version: update.version.clone(),
            notes: release_notes(update.body.as_deref()),
            date: release_date(&update.raw_json),
        },
    };
    *pending.lock() = found;
    Ok(check)
}

/// Downloads and checks the installer, then runs it. On Windows the app closes once the
/// installer has started, so this only returns when something went wrong.
#[tauri::command]
pub async fn install_update(
    pending: State<'_, PendingUpdate>,
    on_progress: Channel<DownloadProgress>,
) -> Result<(), AppError> {
    let Some(mut update) = pending.lock().clone() else {
        return Err(AppError::invalid("Check for updates first."));
    };
    update.timeout = Some(DOWNLOAD_TIMEOUT);
    let mut received = 0;
    let mut reported = 0;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                // The first chunk is reported at once, so the size is known from the start.
                if reported == 0
                    || received - reported >= PROGRESS_STEP_BYTES
                    || Some(received) == total
                {
                    reported = received;
                    let _ = on_progress.send(DownloadProgress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(describe)?;
    update.install(bytes).map_err(describe)
}
