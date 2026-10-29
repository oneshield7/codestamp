use tauri::State;

use super::repo::{self, DeletedItem, Restored};
use crate::error::AppError;
use crate::storage::db::Database;
use crate::storage::validate::check_id;

/// Collections and requests deleted in the last 30 days, newest first.
#[tauri::command]
pub async fn list_deleted(db: State<'_, Database>) -> Result<Vec<DeletedItem>, AppError> {
    db.with(repo::list)
}

#[tauri::command]
pub async fn restore_deleted(db: State<'_, Database>, id: String) -> Result<Restored, AppError> {
    check_id(&id)?;
    db.with(|conn| repo::restore(conn, &id))
}

/// Deletes one kept copy for good.
#[tauri::command]
pub async fn forget_deleted(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    check_id(&id)?;
    db.with(|conn| repo::forget(conn, &id))
}

/// Deletes every kept copy for good.
#[tauri::command]
pub async fn forget_all_deleted(db: State<'_, Database>) -> Result<(), AppError> {
    db.with(repo::forget_all)
}
