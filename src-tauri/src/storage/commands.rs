use tauri::State;

use super::db::Database;
use super::examples::{self, Example, ExampleInput, ExampleSummary, check_example};
use super::icons::{self, IconEntry, IconSuggestion};
use super::models::{
    CollectionDetail, CollectionSummary, RequestContent, RequestGroup, RequestSummary,
    SavedRequest, SearchEntry, SectionInfo,
};
use super::repo;
use super::validate::{MAX_NAME_CHARS, check_id, check_request, clean_name};
use crate::auth::apply::check_auth;
use crate::auth::models::Auth;
use crate::environment::models::VariableInput;
use crate::environment::repo as environments;
use crate::environment::validate::check_variables;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::validate::check_base_url;
use crate::trash::repo::{self as trash, DeletedItem};

/// The collection variable a base URL given at creation is saved as.
const BASE_URL_KEY: &str = "baseUrl";

#[tauri::command]
pub async fn list_collections(db: State<'_, Database>) -> Result<Vec<CollectionSummary>, AppError> {
    db.with(repo::list_collections)
}

/// `icon` is `None` to follow the name; `base_url` becomes the collection's `{{baseUrl}}`.
#[tauri::command]
pub async fn create_collection(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    name: String,
    icon: Option<String>,
    base_url: Option<String>,
) -> Result<CollectionSummary, AppError> {
    let name = clean_name(&name)?;
    let icon = icon.as_deref().map(icons::check_icon).transpose()?;
    let base_url = match base_url.as_deref().map(str::trim) {
        Some(url) if !url.is_empty() => Some(check_base_url(url)?),
        _ => None,
    };
    // Checked before anything is written, so a bad value never leaves a half-made collection.
    let variables = base_url
        .map(|url| {
            check_variables(vec![VariableInput {
                id: None,
                key: BASE_URL_KEY.to_owned(),
                value: Some(url),
                is_secret: false,
                enabled: true,
                restricted: None,
            }])
        })
        .transpose()?;
    db.with(|conn| {
        let created = repo::insert_collection(conn, &name)?;
        if icon.is_some() {
            repo::set_collection_icon(conn, &created.id, icon)?;
        }
        if let Some(variables) = variables {
            let set = environments::collection_set_id(conn, &created.id)?;
            environments::replace_variables(conn, &vault, &set, variables)?;
        }
        repo::collection_summary(conn, &created.id)
    })
}

/// Returns the summary, whose automatic icon may change with the name.
#[tauri::command]
pub async fn rename_collection(
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<CollectionSummary, AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    db.with(|conn| {
        repo::rename_collection(conn, &id, &name)?;
        repo::collection_summary(conn, &id)
    })
}

/// `icon` is an emoji from the catalogue, or `None` to follow the name again.
#[tauri::command]
pub async fn set_collection_icon(
    db: State<'_, Database>,
    id: String,
    icon: Option<String>,
) -> Result<CollectionSummary, AppError> {
    check_id(&id)?;
    let icon = icon.as_deref().map(icons::check_icon).transpose()?;
    db.with(|conn| {
        repo::set_collection_icon(conn, &id, icon)?;
        repo::collection_summary(conn, &id)
    })
}

#[tauri::command]
pub fn collection_icons() -> Vec<IconEntry> {
    icons::catalogue()
}

/// The icon a name would get, while the user types it.
#[tauri::command]
pub fn suggest_collection_icon(name: String) -> Result<IconSuggestion, AppError> {
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(AppError::invalid("The name is too long"));
    }
    Ok(icons::suggest(&name))
}

/// Kept in Recently deleted for 30 days.
#[tauri::command]
pub async fn delete_collection(
    db: State<'_, Database>,
    id: String,
) -> Result<DeletedItem, AppError> {
    check_id(&id)?;
    db.with(|conn| trash::delete_collection(conn, &id))
}

#[tauri::command]
pub async fn list_examples(
    db: State<'_, Database>,
    request_id: String,
) -> Result<Vec<ExampleSummary>, AppError> {
    check_id(&request_id)?;
    db.with(|conn| examples::list(conn, &request_id))
}

#[tauri::command]
pub async fn get_example(db: State<'_, Database>, id: String) -> Result<Example, AppError> {
    check_id(&id)?;
    db.with(|conn| examples::get(conn, &id))
}

/// Keeps a response with its request, for reference and for the mock server.
#[tauri::command]
pub async fn save_example(
    db: State<'_, Database>,
    request_id: String,
    example: ExampleInput,
) -> Result<Example, AppError> {
    check_id(&request_id)?;
    let example = check_example(example)?;
    db.with(|conn| examples::insert(conn, &request_id, &example))
}

#[tauri::command]
pub async fn rename_example(
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<(), AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    db.with(|conn| examples::rename(conn, &id, &name))
}

#[tauri::command]
pub async fn delete_example(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    check_id(&id)?;
    db.with(|conn| examples::delete(conn, &id))
}

/// Every request across collections, for quick search (Ctrl+Space).
#[tauri::command]
pub async fn search_index(db: State<'_, Database>) -> Result<Vec<SearchEntry>, AppError> {
    db.with(repo::search_entries)
}

#[tauri::command]
pub async fn get_collection(
    db: State<'_, Database>,
    id: String,
) -> Result<CollectionDetail, AppError> {
    check_id(&id)?;
    db.with(|conn| repo::collection_detail(conn, &id))
}

#[tauri::command]
pub async fn set_collection_auth(
    db: State<'_, Database>,
    id: String,
    auth: Auth,
) -> Result<(), AppError> {
    check_id(&id)?;
    check_auth(&auth, false)?;
    db.with(|conn| repo::set_collection_auth(conn, &id, &auth))
}

#[tauri::command]
pub async fn create_section(
    db: State<'_, Database>,
    collection_id: String,
    name: String,
) -> Result<SectionInfo, AppError> {
    check_id(&collection_id)?;
    let name = clean_name(&name)?;
    db.with(|conn| repo::insert_section(conn, &collection_id, &name))
}

#[tauri::command]
pub async fn rename_section(
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<(), AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    db.with(|conn| repo::rename_section(conn, &id, &name))
}

#[tauri::command]
pub async fn delete_section(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    check_id(&id)?;
    db.with(|conn| repo::delete_section(conn, &id))
}

#[tauri::command]
pub async fn create_request(
    db: State<'_, Database>,
    collection_id: String,
    section_id: Option<String>,
    content: RequestContent,
) -> Result<SavedRequest, AppError> {
    check_id(&collection_id)?;
    if let Some(section) = &section_id {
        check_id(section)?;
    }
    let content = check_request(content)?;
    db.with(|conn| repo::insert_request(conn, &collection_id, section_id.as_deref(), content))
}

#[tauri::command]
pub async fn get_request(db: State<'_, Database>, id: String) -> Result<SavedRequest, AppError> {
    check_id(&id)?;
    db.with(|conn| repo::load_request(conn, &id))
}

#[tauri::command]
pub async fn update_request(
    db: State<'_, Database>,
    id: String,
    content: RequestContent,
) -> Result<RequestSummary, AppError> {
    check_id(&id)?;
    let content = check_request(content)?;
    db.with(|conn| repo::update_request(conn, &id, &content))
}

#[tauri::command]
pub async fn reorder_collection(
    db: State<'_, Database>,
    collection_id: String,
    sections: Vec<String>,
    groups: Vec<RequestGroup>,
) -> Result<CollectionDetail, AppError> {
    check_id(&collection_id)?;
    for id in sections
        .iter()
        .chain(groups.iter().filter_map(|g| g.section_id.as_ref()))
        .chain(groups.iter().flat_map(|g| &g.request_ids))
    {
        check_id(id)?;
    }
    db.with(|conn| {
        repo::reorder(conn, &collection_id, &sections, &groups)?;
        repo::collection_detail(conn, &collection_id)
    })
}

/// Kept in Recently deleted for 30 days.
#[tauri::command]
pub async fn delete_request(db: State<'_, Database>, id: String) -> Result<DeletedItem, AppError> {
    check_id(&id)?;
    db.with(|conn| trash::delete_request(conn, &id))
}

#[tauri::command]
pub async fn duplicate_request(
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<SavedRequest, AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    db.with(|conn| repo::duplicate_request(conn, &id, &name))
}

#[tauri::command]
pub async fn duplicate_section(
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<CollectionDetail, AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    db.with(|conn| repo::duplicate_section(conn, &id, &name))
}

/// Copies the collection, then its variables. If the variables cannot be copied, the copy is
/// removed again so no half-finished collection is left behind.
#[tauri::command]
pub async fn duplicate_collection(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    id: String,
    name: String,
) -> Result<CollectionSummary, AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    db.with(|conn| {
        let copy = repo::duplicate_collection(conn, &id, &name)?;
        if let Err(error) = environments::copy_collection_variables(conn, &vault, &id, &copy.id) {
            // The original error is what the user needs; if the cleanup fails as well, an extra
            // copy is left behind, which they can delete.
            let _ = repo::delete_collection(conn, &copy.id);
            return Err(error);
        }
        Ok(copy)
    })
}
