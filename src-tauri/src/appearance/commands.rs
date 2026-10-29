use tauri::ipc::Response;
use tauri::{Manager, State, Webview, WebviewWindow};

use super::fonts::{FONT_EXTENSIONS, FontStore};
use super::models::{Appearance, AppearanceState, ImportedFont};
use super::repo;
use super::settings::{current, paint_background, window_theme, zoom_factor};
use super::validate::check_appearance;
use crate::dialogs::pick_file;
use crate::error::AppError;
use crate::storage::db::Database;

#[tauri::command]
pub async fn get_appearance(db: State<'_, Database>) -> Result<AppearanceState, AppError> {
    db.with(|conn| {
        Ok(AppearanceState {
            appearance: current(conn)?,
            fonts: repo::list_fonts(conn)?,
        })
    })
}

#[tauri::command]
pub async fn save_appearance(
    webview: Webview,
    db: State<'_, Database>,
    appearance: Appearance,
) -> Result<Appearance, AppError> {
    let saved = db.with(|conn| {
        let checked = check_appearance(conn, appearance)?;
        repo::save_appearance(conn, &checked)?;
        Ok(checked)
    })?;
    webview
        .set_zoom(zoom_factor(&saved))
        .map_err(|_| AppError::Window)?;
    webview
        .window()
        .set_theme(window_theme(&saved))
        .map_err(|_| AppError::Window)?;
    if let Some(window) = webview.get_webview_window(webview.label()) {
        paint_background(&window, &saved)?;
    }
    Ok(saved)
}

/// Opens a native file picker; `None` when the user cancels.
#[tauri::command]
pub async fn import_font(
    window: WebviewWindow,
    db: State<'_, Database>,
    store: State<'_, FontStore>,
) -> Result<Option<ImportedFont>, AppError> {
    let picked = pick_file(&window, "Import a font", "Font files", &FONT_EXTENSIONS).await;
    let Some(path) = picked else {
        return Ok(None);
    };
    store.import(&db, &path).map(Some)
}

/// The font file as raw bytes, so the UI can load it without a file URL.
#[tauri::command]
pub async fn read_font(
    db: State<'_, Database>,
    store: State<'_, FontStore>,
    id: String,
) -> Result<Response, AppError> {
    store.read(&db, &id).map(Response::new)
}

#[tauri::command]
pub async fn delete_font(
    db: State<'_, Database>,
    store: State<'_, FontStore>,
    id: String,
) -> Result<AppearanceState, AppError> {
    store.remove(&db, &id)
}
