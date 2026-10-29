use tauri::State;

use super::build;
use super::models::{CatalogDetail, CatalogInfo, CatalogOverview, RunInput};
use super::repo;
use super::validate::{check_offset, check_run, clean_info};
use crate::error::AppError;
use crate::storage::db::{Database, now_ms};
use crate::storage::validate::check_id;

/// `utc_offset_minutes` places the 7 days on the user's calendar (420 for UTC+7).
#[tauri::command]
pub async fn catalog_overview(
    db: State<'_, Database>,
    utc_offset_minutes: i32,
) -> Result<CatalogOverview, AppError> {
    check_offset(utc_offset_minutes)?;
    db.with(|conn| build::overview(conn, now_ms(), utc_offset_minutes))
}

#[tauri::command]
pub async fn catalog_detail(
    db: State<'_, Database>,
    collection_id: String,
    utc_offset_minutes: i32,
) -> Result<CatalogDetail, AppError> {
    check_id(&collection_id)?;
    check_offset(utc_offset_minutes)?;
    db.with(|conn| build::detail(conn, &collection_id, now_ms(), utc_offset_minutes))
}

#[tauri::command]
pub async fn save_catalog_info(
    db: State<'_, Database>,
    collection_id: String,
    owner: String,
    tags: Vec<String>,
) -> Result<CatalogInfo, AppError> {
    check_id(&collection_id)?;
    let info = clean_info(&owner, &tags)?;
    db.with(|conn| repo::save_info(conn, &collection_id, &info))?;
    Ok(info)
}

/// Keeps a finished Run all for the catalog's Tests numbers.
#[tauri::command]
pub async fn record_collection_run(db: State<'_, Database>, run: RunInput) -> Result<(), AppError> {
    check_run(&run)?;
    db.with(|conn| repo::insert_run(conn, &run).map(|_| ()))
}
