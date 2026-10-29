use tauri::State;

use super::models::{CookieKey, StoredCookie};
use super::repo;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::storage::db::Database;

#[tauri::command]
pub async fn list_cookies(client: State<'_, HttpClient>) -> Result<Vec<StoredCookie>, AppError> {
    client.jar().list()
}

#[tauri::command]
pub async fn reveal_cookie(
    client: State<'_, HttpClient>,
    key: CookieKey,
) -> Result<String, AppError> {
    check_key(&key)?;
    client.jar().value(&key)
}

#[tauri::command]
pub async fn delete_cookie(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    key: CookieKey,
) -> Result<(), AppError> {
    check_key(&key)?;
    client.jar().remove(&key)?;
    db.with(|conn| repo::save(conn, &vault, client.jar()))
}

/// Clears every cookie, or only those of `domain`.
#[tauri::command]
pub async fn clear_cookies(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    domain: Option<String>,
) -> Result<(), AppError> {
    if domain
        .as_ref()
        .is_some_and(|d| d.len() > MAX_KEY_VALUE_LENGTH)
    {
        return Err(AppError::invalid("Unknown cookie domain"));
    }
    client.jar().clear(domain.as_deref())?;
    db.with(|conn| repo::save(conn, &vault, client.jar()))
}

fn check_key(key: &CookieKey) -> Result<(), AppError> {
    let too_long = [&key.domain, &key.path, &key.name]
        .iter()
        .any(|part| part.len() > MAX_KEY_VALUE_LENGTH);
    if too_long {
        return Err(AppError::NotFound("Cookie"));
    }
    Ok(())
}
