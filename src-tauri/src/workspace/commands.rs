use rusqlite::Connection;
use tauri::State;

use super::icon::{check_icon, check_website};
use super::logo;
use super::models::{LogoCandidate, Workspace, WorkspaceIcon, WorkspaceList};
use super::repo::{self, Look};
use super::validate::check_hue;
use crate::cookies::jar::CookieJar;
use crate::cookies::repo as cookies;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::storage::db::Database;
use crate::storage::validate::{check_id, clean_name};

#[tauri::command]
pub async fn list_workspaces(db: State<'_, Database>) -> Result<WorkspaceList, AppError> {
    db.with(repo::list)
}

#[tauri::command]
pub async fn create_workspace(
    db: State<'_, Database>,
    name: String,
    hue: String,
    icon: WorkspaceIcon,
) -> Result<Workspace, AppError> {
    let name = clean_name(&name)?;
    check_hue(&hue)?;
    let icon = check_icon(icon)?;
    db.with(|conn| {
        repo::insert(
            conn,
            &Look {
                name: &name,
                hue: &hue,
                icon: &icon,
            },
        )
    })
}

#[tauri::command]
pub async fn update_workspace(
    db: State<'_, Database>,
    id: String,
    name: String,
    hue: String,
    icon: WorkspaceIcon,
) -> Result<Workspace, AppError> {
    check_id(&id)?;
    let name = clean_name(&name)?;
    check_hue(&hue)?;
    let icon = check_icon(icon)?;
    db.with(|conn| {
        repo::update(
            conn,
            &id,
            &Look {
                name: &name,
                hue: &hue,
                icon: &icon,
            },
        )
    })
}

/// Logos for the websites a name suggests. Sends those website names to Google's icon service.
#[tauri::command]
pub async fn find_workspace_logos(name: String) -> Result<Vec<LogoCandidate>, AppError> {
    let name = clean_name(&name)?;
    logo::find(&logo::guesses(&name)).await
}

/// The logo of a website the user typed, if Google's icon service has one.
#[tauri::command]
pub async fn find_website_logo(website: String) -> Result<Vec<LogoCandidate>, AppError> {
    let website = check_website(&website)?;
    logo::find(&[website]).await
}

/// The app reloads after this, so every view reads the new workspace.
#[tauri::command]
pub async fn open_workspace(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    id: String,
) -> Result<(), AppError> {
    check_id(&id)?;
    db.with(|conn| open(conn, &vault, client.jar(), &id))
}

/// Deleting the open workspace opens the first other one.
#[tauri::command]
pub async fn delete_workspace(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    client: State<'_, HttpClient>,
    id: String,
) -> Result<WorkspaceList, AppError> {
    check_id(&id)?;
    db.with(|conn| {
        repo::check_deletable(conn, &id)?;
        if repo::current(conn)? == id {
            open(conn, &vault, client.jar(), &repo::other_than(conn, &id)?)?;
        }
        repo::delete(conn, &id)?;
        repo::list(conn)
    })
}

#[tauri::command]
pub async fn move_collection_to_workspace(
    db: State<'_, Database>,
    collection_id: String,
    workspace_id: String,
) -> Result<(), AppError> {
    check_id(&collection_id)?;
    check_id(&workspace_id)?;
    db.with(|conn| repo::move_collection(conn, &collection_id, &workspace_id))
}

/// The open workspace keeps its cookies and the jar gets the ones `id` saved.
fn open(conn: &Connection, vault: &Vault, jar: &CookieJar, id: &str) -> Result<(), AppError> {
    if repo::current(conn)? == id {
        return Ok(());
    }
    repo::check_exists(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    cookies::switch(&tx, vault, jar, id)?;
    repo::set_current(&tx, id)?;
    tx.commit()?;
    Ok(())
}
