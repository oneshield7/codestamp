use tauri::WebviewWindow;

use super::parse::{RunData, read};
use crate::dialogs::pick_file;
use crate::error::AppError;

/// Lets the user pick a CSV or JSON data file for a collection run; `None` if they cancel.
/// The file is read here, so the webview never touches the disk.
#[tauri::command]
pub async fn pick_run_data(window: WebviewWindow) -> Result<Option<RunData>, AppError> {
    let Some(path) = pick_file(
        &window,
        "Choose a data file",
        "CSV or JSON",
        &["csv", "json"],
    )
    .await
    else {
        return Ok(None);
    };
    read(&path).map(Some)
}
