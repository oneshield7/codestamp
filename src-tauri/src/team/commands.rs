use std::collections::HashMap;

use tauri::State;

use super::files::is_known_path;
use super::git::{configured_author, version};
use super::groups::{self, AccessOverview, RestrictedUse};
use super::models::{
    Account, AccountCheck, CommitDetail, CommitPage, Decision, FieldChange, GitAccountView,
    NewAccount, RepoSummary, RequestActivity, SharePreview, SyncReport, TeamSetup, TeamStatus,
    Version,
};
use super::sync::{self, TeamState};
use super::{accounts, history, repo, report};
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::db::Database;
use crate::storage::models::{CollectionSummary, SavedRequest};

/// A decision per conflicting file; more than this cannot come from one sync.
const MAX_DECISIONS: usize = 2_100;

#[tauri::command]
pub async fn team_setup(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
) -> Result<TeamSetup, AppError> {
    let git_version = version(state.root()).await;
    let author = match db.with(repo::author)? {
        Some(author) => Some(author),
        None if git_version.is_some() => configured_author(state.root()).await,
        None => None,
    };
    Ok(TeamSetup {
        git_version,
        author,
    })
}

#[tauri::command]
pub async fn set_team_account(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
    account: Account,
) -> Result<TeamStatus, AppError> {
    sync::set_account(&db, &state, &collection_id, &account)
}

#[tauri::command]
pub async fn list_team_links(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
) -> Result<Vec<TeamStatus>, AppError> {
    sync::statuses(&db, &state)
}

#[tauri::command]
pub async fn share_collection(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
    remote: String,
    account: Account,
    replace_head: Option<String>,
) -> Result<TeamStatus, AppError> {
    sync::share(
        &db,
        &state,
        &collection_id,
        &remote,
        &account,
        replace_head.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn preview_share(
    state: State<'_, TeamState>,
    remote: String,
    username: Option<String>,
) -> Result<SharePreview, AppError> {
    sync::preview_share(&state, &remote, username.as_deref()).await
}

#[tauri::command]
pub async fn join_team_collection(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    remote: String,
    account: Account,
) -> Result<CollectionSummary, AppError> {
    sync::join(&db, &vault, &state, &remote, &account).await
}

#[tauri::command]
pub async fn sync_team_collection(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    collection_id: String,
    decisions: Vec<Decision>,
) -> Result<SyncReport, AppError> {
    if decisions.len() > MAX_DECISIONS || decisions.iter().any(|d| !is_known_path(&d.file)) {
        return Err(AppError::invalid("The conflict choices are not valid"));
    }
    let decisions: HashMap<String, _> = decisions.into_iter().map(|d| (d.file, d.choice)).collect();
    sync::sync(&db, &vault, &state, &collection_id, &decisions).await
}

#[tauri::command]
pub async fn unlink_team_collection(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
) -> Result<(), AppError> {
    sync::unlink(&db, &state, &collection_id).await
}

#[tauri::command]
pub async fn request_versions(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
    request_id: String,
) -> Result<Vec<Version>, AppError> {
    history::versions(&db, &state, &collection_id, &request_id).await
}

#[tauri::command]
pub async fn request_version_changes(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
    request_id: String,
    commit: String,
) -> Result<Vec<FieldChange>, AppError> {
    history::changes(&db, &state, &collection_id, &request_id, &commit).await
}

#[tauri::command]
pub async fn restore_request_version(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
    request_id: String,
    commit: String,
) -> Result<SavedRequest, AppError> {
    history::restore(&db, &state, &collection_id, &request_id, &commit).await
}

#[tauri::command]
pub async fn list_git_accounts(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
) -> Result<Vec<GitAccountView>, AppError> {
    accounts::list(&db, &state).await
}

#[tauri::command]
pub async fn add_git_account(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    account: NewAccount,
) -> Result<Vec<GitAccountView>, AppError> {
    accounts::add(&db, &account)?;
    accounts::list(&db, &state).await
}

#[tauri::command]
pub async fn remove_git_account(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    id: String,
) -> Result<Vec<GitAccountView>, AppError> {
    accounts::remove(&db, &id)?;
    accounts::list(&db, &state).await
}

#[tauri::command]
pub async fn check_git_account(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    host: String,
    username: Option<String>,
) -> Result<AccountCheck, AppError> {
    accounts::check(&db, &state, &host, username.as_deref()).await
}

#[tauri::command]
pub async fn sign_in_git_account(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    host: String,
    username: Option<String>,
) -> Result<AccountCheck, AppError> {
    accounts::sign_in(&db, &state, &host, username.as_deref()).await
}

#[tauri::command]
pub async fn sign_in_team_collection(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
) -> Result<AccountCheck, AppError> {
    accounts::sign_in_collection(&db, &state, &collection_id).await
}

#[tauri::command]
pub async fn sign_out_git_account(
    state: State<'_, TeamState>,
    host: String,
    username: String,
) -> Result<(), AppError> {
    accounts::sign_out(&state, &host, &username).await
}

#[tauri::command]
pub async fn git_changes_overview(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
) -> Result<Vec<RepoSummary>, AppError> {
    report::overview(&db, &vault, &state).await
}

#[tauri::command]
pub async fn git_commits(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
    skip: u32,
) -> Result<CommitPage, AppError> {
    report::commits(&db, &state, &collection_id, skip).await
}

#[tauri::command]
pub async fn git_commit_detail(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
    commit: String,
) -> Result<CommitDetail, AppError> {
    report::commit_detail(&db, &state, &collection_id, &commit).await
}

#[tauri::command]
pub async fn git_request_activity(
    db: State<'_, Database>,
    state: State<'_, TeamState>,
    collection_id: String,
) -> Result<Vec<RequestActivity>, AppError> {
    report::request_activity(&db, &state, &collection_id).await
}

#[tauri::command]
pub async fn access_overview(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    collection_id: String,
) -> Result<AccessOverview, AppError> {
    db.with(|conn| groups::overview(conn, &vault, state.device(), &collection_id))
}

#[tauri::command]
pub async fn create_access_group(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    collection_id: String,
    name: String,
) -> Result<(), AppError> {
    db.with(|conn| groups::create_group(conn, &vault, state.device(), &collection_id, &name))
}

#[tauri::command]
pub async fn request_group_access(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    collection_id: String,
    group_id: String,
) -> Result<(), AppError> {
    db.with(|conn| groups::request_access(conn, &vault, state.device(), &collection_id, &group_id))
}

#[tauri::command]
pub async fn grant_group_access(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    collection_id: String,
    group_id: String,
    fingerprint: String,
) -> Result<(), AppError> {
    db.with(|conn| groups::grant(conn, &vault, &collection_id, &group_id, &fingerprint))
}

#[tauri::command]
pub async fn decline_group_access(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    collection_id: String,
    group_id: String,
    fingerprint: String,
) -> Result<(), AppError> {
    db.with(|conn| {
        groups::decline(
            conn,
            &vault,
            state.device(),
            &collection_id,
            &group_id,
            &fingerprint,
        )
    })
}

#[tauri::command]
pub async fn remove_group_member(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    collection_id: String,
    group_id: String,
    fingerprint: String,
) -> Result<Vec<RestrictedUse>, AppError> {
    db.with(|conn| {
        groups::remove(
            conn,
            &vault,
            state.device(),
            &collection_id,
            &group_id,
            &fingerprint,
        )
    })
}

#[tauri::command]
pub async fn repair_access_group(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    state: State<'_, TeamState>,
    collection_id: String,
    group_id: String,
) -> Result<(), AppError> {
    db.with(|conn| groups::repair(conn, &vault, state.device(), &collection_id, &group_id))
}
