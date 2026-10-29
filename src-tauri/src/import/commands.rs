use tauri::State;

use super::curl;
use super::fetch::{checked_url, fetch_text};
use super::file;
use super::models::{CurlImport, ImportPlan, ImportPreview, ImportResult};
use super::run;
use super::wsdl;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::soap::WsdlSource;
use crate::soap::commands::read_definition;
use crate::storage::db::Database;
use crate::storage::validate::{check_id, clean_name};

const IMPORT_ACCEPT: &str = "application/json, application/yaml;q=0.9, text/yaml;q=0.9, */*;q=0.5";

/// Parsing large files is CPU work, so it runs off the async runtime's worker threads.
#[tauri::command]
pub async fn preview_import(content: String) -> Result<ImportPreview, AppError> {
    tauri::async_runtime::spawn_blocking(move || file::plan(&content).map(|plan| plan.preview()))
        .await
        .map_err(|_| AppError::invalid("The file could not be read"))?
}

#[tauri::command]
pub async fn run_import(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    content: String,
    name: String,
) -> Result<ImportResult, AppError> {
    let name = clean_name(&name)?;
    let plan = tauri::async_runtime::spawn_blocking(move || file::plan(&content))
        .await
        .map_err(|_| AppError::invalid("The file could not be read"))??;
    db.with(|conn| run::apply(conn, &vault, plan, &name))
}

/// Replaces the content of an existing collection with a collection file; see `run::replace`.
#[tauri::command]
pub async fn replace_with_import(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    content: String,
    collection_id: String,
) -> Result<ImportResult, AppError> {
    check_id(&collection_id)?;
    let plan = tauri::async_runtime::spawn_blocking(move || file::plan(&content))
        .await
        .map_err(|_| AppError::invalid("The file could not be read"))??;
    db.with(|conn| run::replace(conn, &vault, plan, &collection_id))
}

/// Downloads a spec or collection to import, through the same network settings as requests.
#[tauri::command]
pub async fn fetch_import(client: State<'_, HttpClient>, url: String) -> Result<String, AppError> {
    let url = checked_url(&url, "file URL")?;
    fetch_text(&client, &url, IMPORT_ACCEPT).await
}

#[tauri::command]
pub async fn parse_curl(command: String) -> Result<CurlImport, AppError> {
    curl::parse(&command)
}

/// A WSDL is planned from where it lives, not from pasted text, so its imports can be read.
#[tauri::command]
pub async fn preview_wsdl_import(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    source: WsdlSource,
) -> Result<ImportPreview, AppError> {
    Ok(wsdl_plan(&client, &db, &source).await?.preview())
}

#[tauri::command]
pub async fn run_wsdl_import(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    source: WsdlSource,
    name: String,
) -> Result<ImportResult, AppError> {
    let name = clean_name(&name)?;
    let plan = wsdl_plan(&client, &db, &source).await?;
    db.with(|conn| run::apply(conn, &vault, plan, &name))
}

async fn wsdl_plan(
    client: &HttpClient,
    db: &Database,
    source: &WsdlSource,
) -> Result<ImportPlan, AppError> {
    let definition = read_definition(client, db, source).await?;
    let location = match source {
        WsdlSource::Url { url } => url.as_str(),
        WsdlSource::File { path } => path.as_str(),
        WsdlSource::Content { .. } => "",
    };
    wsdl::plan(definition, location)
}
