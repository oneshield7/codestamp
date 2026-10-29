use tauri::State;

use super::lookup::preview_url as preview;
use super::models::{
    EnvironmentList, EnvironmentSummary, UrlPreview, VariableHint, VariableInput, VariableView,
};
use super::repo;
use super::validate::check_variables;
use super::vault::Vault;
use crate::error::AppError;
use crate::request::path_vars::MAX_PATH_VARIABLES;
use crate::request::rules::{MAX_KEY_VALUE_LENGTH, MAX_KEY_VALUE_ROWS, MAX_URL_LENGTH};
use crate::request::types::NameValue;
use crate::storage::db::Database;
use crate::storage::validate::{check_id, check_optional_id, clean_name};
use crate::team::groups;

#[tauri::command]
pub async fn list_environments(db: State<'_, Database>) -> Result<EnvironmentList, AppError> {
    db.with(repo::list)
}

#[tauri::command]
pub async fn create_environment(
    db: State<'_, Database>,
    name: String,
) -> Result<EnvironmentSummary, AppError> {
    let name = clean_name(&name)?;
    db.with(|conn| repo::insert(conn, &name))
}

#[tauri::command]
pub async fn rename_environment(
    db: State<'_, Database>,
    id: String,
    name: String,
) -> Result<(), AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    db.with(|conn| repo::rename(conn, &id, &name))
}

#[tauri::command]
pub async fn delete_environment(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    check_id(&id)?;
    db.with(|conn| repo::delete(conn, &id))
}

/// `collection_id: None` keeps the environment on this device only.
#[tauri::command]
pub async fn set_environment_collection(
    db: State<'_, Database>,
    id: String,
    collection_id: Option<String>,
) -> Result<(), AppError> {
    check_id(&id)?;
    check_optional_id(collection_id.as_deref())?;
    db.with(|conn| repo::set_owner(conn, &id, collection_id.as_deref()))
}

#[tauri::command]
pub async fn set_active_environment(
    db: State<'_, Database>,
    id: Option<String>,
) -> Result<(), AppError> {
    if let Some(id) = &id {
        check_id(id)?;
    }
    db.with(|conn| repo::set_active(conn, id.as_deref()))
}

#[tauri::command]
pub async fn get_environment_variables(
    db: State<'_, Database>,
    id: String,
) -> Result<Vec<VariableView>, AppError> {
    check_id(&id)?;
    db.with(|conn| repo::variables(conn, &id))
}

#[tauri::command]
pub async fn save_environment_variables(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    id: String,
    variables: Vec<VariableInput>,
) -> Result<Vec<VariableView>, AppError> {
    check_id(&id)?;
    let variables = check_variables(variables)?;
    db.with(|conn| {
        groups::check_edit(conn, &id, &variables)?;
        repo::replace_variables(conn, &vault, &id, variables)
    })
}

#[tauri::command]
pub async fn get_collection_variable_set(
    db: State<'_, Database>,
    collection_id: String,
) -> Result<String, AppError> {
    check_id(&collection_id)?;
    db.with(|conn| repo::collection_set_id(conn, &collection_id))
}

#[tauri::command]
pub async fn variable_hints(
    db: State<'_, Database>,
    collection_id: Option<String>,
) -> Result<Vec<VariableHint>, AppError> {
    check_optional_id(collection_id.as_deref())?;
    db.with(|conn| repo::hints(conn, collection_id.as_deref()))
}

#[tauri::command]
pub async fn preview_url(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    url: String,
    params: Vec<NameValue>,
    path_variables: Vec<NameValue>,
    collection_id: Option<String>,
) -> Result<UrlPreview, AppError> {
    check_optional_id(collection_id.as_deref())?;
    let too_long = url.len() > MAX_URL_LENGTH
        || params.len() > MAX_KEY_VALUE_ROWS
        || path_variables.len() > MAX_PATH_VARIABLES
        || params
            .iter()
            .chain(&path_variables)
            .any(|p| p.name.len() > MAX_KEY_VALUE_LENGTH || p.value.len() > MAX_KEY_VALUE_LENGTH);
    if too_long {
        return Err(AppError::invalid("The URL or its parameters are too long"));
    }
    preview(
        &db,
        &vault,
        &url,
        &params,
        &path_variables,
        collection_id.as_deref(),
    )
}

#[tauri::command]
pub async fn reveal_secret(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    id: String,
) -> Result<String, AppError> {
    check_id(&id)?;
    db.with(|conn| repo::reveal(conn, &vault, &id))
}
