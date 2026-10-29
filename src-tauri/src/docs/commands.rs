use tauri::State;

use super::models::{CollectionDocs, DocHit};
use super::{repo, search};
use crate::error::AppError;
use crate::storage::db::Database;
use crate::storage::descriptions::{self, Described, clean_description};
use crate::storage::validate::check_id;

#[tauri::command]
pub async fn collection_docs(
    db: State<'_, Database>,
    collection_id: String,
) -> Result<CollectionDocs, AppError> {
    check_id(&collection_id)?;
    db.with(|conn| repo::load(conn, &collection_id))
}

/// Collections, sections and requests whose name or description holds every word of `query`.
#[tauri::command]
pub async fn search_docs(db: State<'_, Database>, query: String) -> Result<Vec<DocHit>, AppError> {
    let words = search::query_words(&query)?;
    db.with(|conn| search::search(conn, &words))
}

/// Saves the Markdown description of a collection, section or request; returns it as stored.
#[tauri::command]
pub async fn save_description(
    db: State<'_, Database>,
    item: Described,
    id: String,
    text: String,
) -> Result<String, AppError> {
    check_id(&id)?;
    let text = clean_description(&text)?;
    db.with(|conn| descriptions::set(conn, item, &id, &text))?;
    Ok(text)
}
