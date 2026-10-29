use std::fs;

use tauri::{AppHandle, Manager, State, WebviewWindow};

use super::backup::{self, BACKUP_EXTENSION, RESTORE_EXTENSIONS};
use super::certificates::{self, CERTIFICATE_EXTENSIONS};
use super::models::{AboutInfo, CertificateInfo, Preferences, ProxyInput, ProxyView};
use super::network::{self, config_with, proxy_password};
use super::repo::{self, PROXY_PASSWORD_KEY};
use super::validate::{check_preferences, check_proxy, check_proxy_password};
use crate::DATABASE_FILE;
use crate::cookies::repo as cookies;
use crate::dialogs::{pick_file, pick_save_path};
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::history::repo as history;
use crate::request::client::HttpClient;
use crate::request::network::NetworkConfig;
use crate::storage::db::Database;
use crate::storage::validate::check_id;
use crate::team::sync::TeamState;

#[tauri::command]
pub async fn get_preferences(db: State<'_, Database>) -> Result<Preferences, AppError> {
    db.with(repo::load_preferences)
}

/// Checks the new request settings against the HTTP client before saving them, so a setting
/// that cannot work is refused instead of breaking every request.
#[tauri::command]
pub async fn save_preferences(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    preferences: Preferences,
) -> Result<Preferences, AppError> {
    let checked = check_preferences(preferences)?;
    let config = db.with(|conn| {
        let proxy = repo::load_proxy(conn)?;
        let password = proxy_password(conn, &vault)?;
        config_with(conn, &vault, &checked.requests, &proxy, password)
    })?;
    client.check(&config)?;
    db.with(|conn| {
        repo::save_preferences(conn, &checked)?;
        history::prune(conn, &checked.history)
    })?;
    client.configure(&config)?;
    Ok(checked)
}

#[tauri::command]
pub async fn get_proxy(db: State<'_, Database>) -> Result<ProxyView, AppError> {
    db.with(|conn| {
        Ok(ProxyView {
            settings: repo::load_proxy(conn)?,
            has_password: repo::load_secret(conn, PROXY_PASSWORD_KEY)?.is_some(),
        })
    })
}

#[tauri::command]
pub async fn save_proxy(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    input: ProxyInput,
) -> Result<ProxyView, AppError> {
    let settings = check_proxy(input.settings)?;
    if let Some(password) = &input.password {
        check_proxy_password(password)?;
    }
    let password = match &input.password {
        Some(password) => password.clone(),
        None => db.with(|conn| proxy_password(conn, &vault))?,
    };
    let config = db.with(|conn| {
        let requests = repo::load_preferences(conn)?.requests;
        config_with(conn, &vault, &requests, &settings, password.clone())
    })?;
    client.check(&config)?;
    let encrypted = if password.is_empty() {
        None
    } else {
        Some(vault.encrypt(PROXY_PASSWORD_KEY, &password)?)
    };
    db.with(|conn| {
        let tx = conn.unchecked_transaction()?;
        repo::save_proxy(&tx, &settings)?;
        match &encrypted {
            Some(blob) => repo::save_secret(&tx, PROXY_PASSWORD_KEY, blob)?,
            None => repo::delete_secret(&tx, PROXY_PASSWORD_KEY)?,
        }
        tx.commit()?;
        Ok(())
    })?;
    client.configure(&config)?;
    Ok(ProxyView {
        settings,
        has_password: encrypted.is_some(),
    })
}

/// Why the saved network settings cannot be used right now, if they cannot.
#[tauri::command]
pub async fn network_problem(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
) -> Result<Option<String>, AppError> {
    Ok(network::problem(&db, &vault, &client))
}

#[tauri::command]
pub async fn list_certificates(db: State<'_, Database>) -> Result<Vec<CertificateInfo>, AppError> {
    db.with(repo::list_certificates)
}

/// Opens a file picker; `None` when the user cancels.
#[tauri::command]
pub async fn add_ca_certificate(
    window: WebviewWindow,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
) -> Result<Option<CertificateInfo>, AppError> {
    let picked = pick_file(
        &window,
        "Add a CA certificate",
        "PEM certificates",
        &CERTIFICATE_EXTENSIONS,
    )
    .await;
    let Some(path) = picked else {
        return Ok(None);
    };
    certificates::add_ca(&db, &vault, &client, &path).map(Some)
}

/// Asks for the certificate, then for its key when the certificate file has none.
#[tauri::command]
pub async fn add_client_certificate(
    window: WebviewWindow,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    host: String,
) -> Result<Option<CertificateInfo>, AppError> {
    super::validate::parse_host_pattern(&host)?;
    let picked = pick_file(
        &window,
        "Choose the client certificate",
        "PEM certificates",
        &CERTIFICATE_EXTENSIONS,
    )
    .await;
    let Some(certificate) = picked else {
        return Ok(None);
    };
    let has_key = fs::read_to_string(&certificate)
        .is_ok_and(|text| !certificates::split_pem(&text).keys.is_empty());
    let key = if has_key {
        None
    } else {
        let picked = pick_file(
            &window,
            "Choose its private key",
            "PEM keys",
            &CERTIFICATE_EXTENSIONS,
        )
        .await;
        let Some(key) = picked else {
            return Ok(None);
        };
        Some(key)
    };
    certificates::add_client(&db, &vault, &client, &host, &certificate, key.as_deref()).map(Some)
}

#[tauri::command]
pub async fn delete_certificate(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    id: String,
) -> Result<(), AppError> {
    check_id(&id)?;
    certificates::remove(&db, &vault, &client, &id)
}

/// Opens a save dialog; `None` when the user cancels, otherwise the file name it was saved as.
#[tauri::command]
pub async fn backup_data(
    app: AppHandle,
    window: WebviewWindow,
    db: State<'_, Database>,
) -> Result<Option<String>, AppError> {
    let picked = pick_save_path(
        &window,
        "Back up Codestamp",
        "codestamp-backup.codestamp",
        "Codestamp backup",
        &[BACKUP_EXTENSION],
    )
    .await;
    let Some(target) = picked else {
        return Ok(None);
    };
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::FileAccess)?;
    backup::backup(&db, &data_dir, &target)?;
    Ok(target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned()))
}

/// Opens a file picker and replaces all data with the chosen backup; `false` when cancelled.
#[tauri::command]
pub async fn restore_data(
    window: WebviewWindow,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    team: State<'_, TeamState>,
) -> Result<bool, AppError> {
    let picked = pick_file(
        &window,
        "Restore a Codestamp backup",
        "Codestamp backup",
        &RESTORE_EXTENSIONS,
    )
    .await;
    let Some(source) = picked else {
        return Ok(false);
    };
    backup::restore(&db, &source)?;
    client.jar().clear(None)?;
    db.with(|conn| cookies::restore(conn, &vault, client.jar()))?;
    team.cleanup(&db)?;
    // A backup from another device cannot open its secrets here; requests then use defaults
    // until the settings are saved again, and Settings shows why.
    if network::apply(&db, &vault, &client).is_err() {
        client.configure(&NetworkConfig::default())?;
    }
    Ok(true)
}

#[tauri::command]
pub async fn about_app(app: AppHandle) -> Result<AboutInfo, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::FileAccess)?;
    let database_bytes = [DATABASE_FILE.to_owned(), format!("{DATABASE_FILE}-wal")]
        .iter()
        .filter_map(|name| fs::metadata(data_dir.join(name)).ok())
        .map(|metadata| metadata.len())
        .sum();
    Ok(AboutInfo {
        version: app.package_info().version.to_string(),
        data_dir: data_dir.display().to_string(),
        database_bytes,
    })
}
