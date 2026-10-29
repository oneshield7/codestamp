use tauri::State;

use super::models::{HistoryEntry, HistoryFilter};
use super::repo;
use crate::error::AppError;
use crate::storage::db::Database;
use crate::storage::models::SavedRequest;
use crate::storage::repo as collections;
use crate::storage::validate::{check_id, check_optional_id, check_request};

#[tauri::command]
pub async fn list_history(
    db: State<'_, Database>,
    filter: HistoryFilter,
) -> Result<Vec<HistoryEntry>, AppError> {
    check_optional_id(filter.collection_id.as_deref())?;
    db.with(|conn| repo::list(conn, &filter))
}

#[tauri::command]
pub async fn delete_history_entry(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    check_id(&id)?;
    db.with(|conn| repo::delete(conn, &id))
}

/// Clears one collection's history, or all of it when no collection is given.
#[tauri::command]
pub async fn clear_history(
    db: State<'_, Database>,
    collection_id: Option<String>,
) -> Result<(), AppError> {
    check_optional_id(collection_id.as_deref())?;
    db.with(|conn| repo::clear(conn, collection_id.as_deref()))
}

#[tauri::command]
pub async fn save_history_to_collection(
    db: State<'_, Database>,
    id: String,
    collection_id: String,
    section_id: Option<String>,
) -> Result<SavedRequest, AppError> {
    check_id(&id)?;
    check_id(&collection_id)?;
    check_optional_id(section_id.as_deref())?;
    db.with(|conn| {
        let content = check_request(repo::request_content(conn, &id)?)?;
        collections::insert_request(conn, &collection_id, section_id.as_deref(), content)
    })
}
