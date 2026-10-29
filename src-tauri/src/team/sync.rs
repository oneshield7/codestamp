use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::access::{self, DeviceKey, GroupFile};
use super::accounts;
use super::apply::{apply, apply_variables};
use super::files::{self, COLLECTION, Snapshot, environment_id_of, request_id_of, request_path};
use super::git::{Git, GitError, check_author, check_remote, check_username};
use super::groups;
use super::history::check_commit;
use super::merge::{FieldConflict, Merged, merge_file};
use super::models::{
    Account, Author, Change, ChangedRequest, Choice, ConflictField, ConflictView, SharePreview,
    SyncReport, SyncState, TeamStatus,
};
use super::present::{display, members_of};
use super::repo::{self, TeamLink};
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::legacy;
use crate::storage::db::{Database, new_id, now_ms};
use crate::storage::models::CollectionSummary;
use crate::storage::repo as collections;
use crate::storage::validate::{MAX_NAME_CHARS, check_id};

const BRANCH: &str = "main";
const REMOTE: &str = "origin";
/// A push refused because a teammate pushed first is retried after merging their work.
const MAX_PUSH_ATTEMPTS: usize = 3;
const JOINING_PREFIX: &str = "joining-";
const COPY_SUFFIX: &str = " (my copy)";
const MAX_BRANCH_CHARS: usize = 100;
/// Files a share preview lists before it only counts the rest.
const MAX_PREVIEW_FILES: usize = 200;

/// Local clones live in `<app data>/repos/<collection id>`. One sync runs per collection at a
/// time; conflicts waiting for a decision are kept until the next sync.
pub struct TeamState {
    root: PathBuf,
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pending: Mutex<HashMap<String, Vec<ConflictView>>>,
    device: DeviceKey,
}

impl TeamState {
    pub fn new(root: PathBuf, device: DeviceKey) -> Self {
        Self {
            root,
            locks: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            device,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn device(&self) -> &DeviceKey {
        &self.device
    }

    /// Callers pass a checked id, so the path stays inside the root.
    pub(crate) fn dir(&self, collection_id: &str) -> PathBuf {
        self.root.join(collection_id)
    }

    fn lock(&self, collection_id: &str) -> Result<Arc<tokio::sync::Mutex<()>>, AppError> {
        let mut locks = self.locks.lock().map_err(|_| AppError::Storage)?;
        Ok(Arc::clone(
            locks.entry(collection_id.to_owned()).or_default(),
        ))
    }

    fn pending(&self, collection_id: &str) -> Vec<ConflictView> {
        self.pending
            .lock()
            .map(|pending| pending.get(collection_id).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    fn set_pending(
        &self,
        collection_id: &str,
        conflicts: Vec<ConflictView>,
    ) -> Result<(), AppError> {
        let mut pending = self.pending.lock().map_err(|_| AppError::Storage)?;
        if conflicts.is_empty() {
            pending.remove(collection_id);
        } else {
            pending.insert(collection_id.to_owned(), conflicts);
        }
        Ok(())
    }

    /// Removes clones that no longer belong to a shared collection: the collection was deleted,
    /// or a join stopped half-way. Only folders Codestamp itself names are touched.
    pub fn cleanup(&self, db: &Database) -> Result<(), AppError> {
        let linked: BTreeSet<String> = db
            .with(repo::links)?
            .into_iter()
            .map(|link| link.collection_id)
            .collect();
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Ok(());
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let ours = check_id(&name).is_ok()
                || name
                    .strip_prefix(JOINING_PREFIX)
                    .is_some_and(|rest| check_id(rest).is_ok());
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            if ours && is_dir && !linked.contains(&name) {
                fs::remove_dir_all(entry.path()).map_err(|_| AppError::FileAccess)?;
            }
        }
        Ok(())
    }
}

pub fn statuses(db: &Database, state: &TeamState) -> Result<Vec<TeamStatus>, AppError> {
    let profile = db.with(repo::author)?;
    Ok(db
        .with(repo::links)?
        .into_iter()
        .map(|link| status_of(state, link, profile.as_ref()))
        .collect())
}

/// `profile` stands in for the author of links made before collections had their own.
fn status_of(state: &TeamState, link: TeamLink, profile: Option<&Author>) -> TeamStatus {
    TeamStatus {
        conflicts: state.pending(&link.collection_id),
        collection_id: link.collection_id,
        remote: link.remote,
        branch: link.branch,
        synced_at: link.synced_at,
        author: link.author.or_else(|| profile.cloned()),
        username: link.username,
    }
}

/// The account with its author and username checked for `remote`.
fn check_account(account: &Account, remote: &str) -> Result<Account, AppError> {
    Ok(Account {
        author: check_author(&account.author)?,
        username: check_username(account.username.as_deref(), remote)?,
    })
}

/// Puts a local collection into an empty repository and links the two.
/// Looks into a repository before sharing into it: a shallow copy without a working tree, read
/// and removed again. Signing in may open the credential manager's window.
pub async fn preview_share(
    state: &TeamState,
    remote: &str,
    username: Option<&str>,
) -> Result<SharePreview, AppError> {
    let remote = check_remote(remote)?;
    let username = check_username(username, &remote)?;
    fs::create_dir_all(&state.root).map_err(|_| AppError::FileAccess)?;
    let temp_name = format!("{JOINING_PREFIX}{}", new_id());
    let temp = state.root.join(&temp_name);
    let result = read_preview(state, &remote, username.as_deref(), &temp_name, &temp).await;
    // A folder left behind is removed by the next cleanup; the preview is what matters here.
    let _ = fs::remove_dir_all(&temp);
    result
}

async fn read_preview(
    state: &TeamState,
    remote: &str,
    username: Option<&str>,
    temp_name: &str,
    temp: &Path,
) -> Result<SharePreview, AppError> {
    Git::signed_in_as(state.root.clone(), username)
        .network(&[
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--no-checkout",
            "--origin",
            REMOTE,
            "--",
            remote,
            temp_name,
        ])
        .await?;
    let git = Git::new(temp.to_path_buf());
    // A repository without commits has no HEAD to read.
    let Ok(head) = git.run(&["rev-parse", "--verify", "--quiet", "HEAD"]).await else {
        return Ok(SharePreview {
            empty: true,
            codestamp: false,
            branch: None,
            head: None,
            files: Vec::new(),
            more: 0,
        });
    };
    let branch = check_branch(
        git.run(&["rev-parse", "--abbrev-ref", "HEAD"])
            .await?
            .trim(),
    )?;
    let listed = git
        .run(&["ls-tree", "-r", "-z", "--name-only", "HEAD"])
        .await?;
    let all: Vec<&str> = listed.split('\0').filter(|path| !path.is_empty()).collect();
    let codestamp = all
        .iter()
        .any(|path| *path == files::MANIFEST || *path == legacy::TEAM_MANIFEST);
    Ok(SharePreview {
        empty: false,
        codestamp,
        branch: Some(branch),
        head: Some(head.trim().to_owned()),
        more: u32::try_from(all.len().saturating_sub(MAX_PREVIEW_FILES)).unwrap_or(u32::MAX),
        files: all
            .into_iter()
            .take(MAX_PREVIEW_FILES)
            .map(str::to_owned)
            .collect(),
    })
}

/// Puts a local collection into a repository and links the two. An empty repository is simply
/// filled. One with content is replaced only when `replace_head` names the commit the person
/// reviewed: its files are removed in one ordinary commit, never a force push, so they stay in
/// the history.
pub async fn share(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    remote: &str,
    account: &Account,
    replace_head: Option<&str>,
) -> Result<TeamStatus, AppError> {
    check_id(collection_id)?;
    let remote = check_remote(remote)?;
    let account = check_account(account, &remote)?;
    if let Some(head) = replace_head {
        check_commit(head)?;
    }
    if db.with(|conn| repo::link(conn, collection_id))?.is_some() {
        return Err(AppError::invalid("This collection is already shared"));
    }
    let snapshot = db.with(|conn| files::from_db(conn, collection_id))?;
    // The profile fills the form the next time a collection is shared or joined.
    db.with(|conn| repo::save_author(conn, &account.author))?;
    let lock = state.lock(collection_id)?;
    let _guard = lock.lock().await;
    let dir = state.dir(collection_id);
    reset_dir(&dir)?;
    let username = account.username.as_deref();
    let pushed = match replace_head {
        None => first_push(
            &Git::signed_in_as(dir.clone(), username),
            &remote,
            &snapshot,
            &account.author,
        )
        .await
        .map(|()| BRANCH.to_owned()),
        Some(head) => {
            replace_push(
                state,
                collection_id,
                &remote,
                username,
                &snapshot,
                &account.author,
                head,
            )
            .await
        }
    };
    let branch = match pushed {
        Ok(branch) => branch,
        Err(error) => {
            // The original error is what the user needs; a folder left behind is removed later.
            let _ = fs::remove_dir_all(&dir);
            return Err(error);
        }
    };
    let link = TeamLink {
        collection_id: collection_id.to_owned(),
        remote,
        branch,
        synced_at: Some(now_ms()),
        username: account.username,
        author: Some(account.author),
        synced_commit: rev(&Git::new(dir), "HEAD").await.ok(),
    };
    db.with(|conn| {
        repo::insert_link(conn, &link)?;
        accounts::remember(conn, &link.remote, link.username.as_deref())
    })?;
    Ok(status_of(state, link, None))
}

/// Changes who this device works as on one shared collection; the next sync uses it.
pub fn set_account(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    account: &Account,
) -> Result<TeamStatus, AppError> {
    check_id(collection_id)?;
    let link = db
        .with(|conn| repo::link(conn, collection_id))?
        .ok_or(AppError::NotFound("Shared collection"))?;
    let account = check_account(account, &link.remote)?;
    db.with(|conn| {
        repo::set_account(conn, collection_id, &account)?;
        accounts::remember(conn, &link.remote, account.username.as_deref())
    })?;
    let link = TeamLink {
        username: account.username,
        author: Some(account.author),
        ..link
    };
    Ok(status_of(state, link, None))
}

async fn first_push(
    git: &Git,
    remote: &str,
    snapshot: &Snapshot,
    author: &Author,
) -> Result<(), AppError> {
    git.run(&["init", "--quiet", "--initial-branch", BRANCH])
        .await?;
    git.run(&["remote", "add", REMOTE, remote]).await?;
    let heads = git.network(&["ls-remote", "--heads", REMOTE]).await?;
    if !heads.trim().is_empty() {
        return Err(AppError::invalid(
            "This repository has content now. Check it again before sharing",
        ));
    }
    files::write(git.dir(), snapshot)?;
    git.run(&["add", "--all"]).await?;
    let message = format!("Share “{}” from Codestamp", snapshot.collection.name);
    git.commit(&["commit", "--quiet", "-m", &message], author)
        .await?;
    git.network(&["push", "--quiet", "--set-upstream", REMOTE, BRANCH])
        .await?;
    Ok(())
}

/// Clones a repository with content and replaces all of its files with the collection, in one
/// commit on its own branch. `head` is the commit the person reviewed; if the branch has moved
/// since, nothing is replaced. Returns the branch.
async fn replace_push(
    state: &TeamState,
    collection_id: &str,
    remote: &str,
    username: Option<&str>,
    snapshot: &Snapshot,
    author: &Author,
    head: &str,
) -> Result<String, AppError> {
    // The share's empty folder makes way for the clone.
    let dir = state.dir(collection_id);
    fs::remove_dir_all(&dir).map_err(|_| AppError::FileAccess)?;
    Git::signed_in_as(state.root.clone(), username)
        .network(&[
            "clone",
            "--quiet",
            "--origin",
            REMOTE,
            "--",
            remote,
            collection_id,
        ])
        .await?;
    let git = Git::signed_in_as(dir, username);
    let now = git
        .run(&["rev-parse", "--verify", "--quiet", "HEAD"])
        .await
        .map_err(|_| moved())?;
    if !now.trim().starts_with(head) {
        return Err(moved());
    }
    let branch = check_branch(
        git.run(&["rev-parse", "--abbrev-ref", "HEAD"])
            .await?
            .trim(),
    )?;
    git.run(&["rm", "-r", "--quiet", "--ignore-unmatch", "--", "."])
        .await?;
    files::write(git.dir(), snapshot)?;
    git.run(&["add", "--all"]).await?;
    let message = format!(
        "Replace the repository's files with “{}” from Codestamp",
        snapshot.collection.name
    );
    git.commit(&["commit", "--quiet", "-m", &message], author)
        .await?;
    let target = format!("HEAD:{branch}");
    match git.network(&["push", "--quiet", REMOTE, &target]).await {
        Ok(_) => Ok(branch),
        Err(GitError::Rejected) => Err(moved()),
        Err(error) => Err(error.into()),
    }
}

fn moved() -> AppError {
    AppError::invalid(
        "The repository changed after you checked it. Check it again before replacing",
    )
}

/// Clones a shared collection and adds it to Codestamp with the ids it has in the repository.
pub async fn join(
    db: &Database,
    vault: &Vault,
    state: &TeamState,
    remote: &str,
    account: &Account,
) -> Result<CollectionSummary, AppError> {
    let remote = check_remote(remote)?;
    let account = check_account(account, &remote)?;
    db.with(|conn| repo::save_author(conn, &account.author))?;
    fs::create_dir_all(&state.root).map_err(|_| AppError::FileAccess)?;
    let temp_name = format!("{JOINING_PREFIX}{}", new_id());
    let temp = state.root.join(&temp_name);
    let result = clone_and_add(db, vault, state, &remote, account, &temp_name, &temp).await;
    if result.is_err() {
        // As above: the original error matters more than a leftover folder.
        let _ = fs::remove_dir_all(&temp);
    }
    result
}

async fn clone_and_add(
    db: &Database,
    vault: &Vault,
    state: &TeamState,
    remote: &str,
    account: Account,
    temp_name: &str,
    temp: &Path,
) -> Result<CollectionSummary, AppError> {
    Git::signed_in_as(state.root.clone(), account.username.as_deref())
        .network(&[
            "clone", "--quiet", "--origin", REMOTE, "--", remote, temp_name,
        ])
        .await?;
    let git = Git::new(temp.to_path_buf());
    let branch = git
        .run(&["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map_err(|_| {
            AppError::invalid(
                "The repository is empty. Ask the owner to share the collection first",
            )
        })?;
    let branch = check_branch(branch.trim())?;
    let head = rev(&git, "HEAD").await?;
    let snapshot = files::read(temp)?;
    let id = snapshot.collection.id.clone();
    if db.with(|conn| collections::has_collection(conn, &id))? {
        return Err(AppError::invalid(
            "This collection is already in Codestamp on this device",
        ));
    }
    let lock = state.lock(&id)?;
    let _guard = lock.lock().await;
    let dir = state.dir(&id);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|_| AppError::FileAccess)?;
    }
    fs::rename(temp, &dir).map_err(|_| AppError::FileAccess)?;
    let link = TeamLink {
        collection_id: id.clone(),
        remote: remote.to_owned(),
        branch,
        synced_at: Some(now_ms()),
        username: account.username,
        author: Some(account.author),
        synced_commit: Some(head),
    };
    let added = db.with(|conn| {
        apply(conn, &snapshot, None)?;
        groups::absorb(conn, vault, &state.device, &snapshot, &BTreeMap::new())?;
        apply_variables(conn, vault, &snapshot)?;
        repo::insert_link(conn, &link)?;
        accounts::remember(conn, &link.remote, link.username.as_deref())
    });
    if let Err(error) = added {
        // Undo the half-added collection; the error that caused it is reported.
        let _ = db.with(|conn| collections::delete_collection(conn, &id));
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }
    db.with(|conn| collections::collection_summary(conn, &id))
}

/// Stops syncing on this device; the collection stays as a local one and the remote is untouched.
pub async fn unlink(db: &Database, state: &TeamState, collection_id: &str) -> Result<(), AppError> {
    check_id(collection_id)?;
    let lock = state.lock(collection_id)?;
    let _guard = lock.lock().await;
    db.with(|conn| repo::delete_link(conn, collection_id))?;
    state.set_pending(collection_id, Vec::new())?;
    let dir = state.dir(collection_id);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|_| AppError::FileAccess)?;
    }
    Ok(())
}

#[derive(Default)]
struct Outcome {
    changed: Vec<ChangedRequest>,
    collection_changed: bool,
    environments_changed: bool,
    access_changed: bool,
    authors: Vec<String>,
}

impl Outcome {
    fn absorb(&mut self, other: Self) {
        self.changed.extend(other.changed);
        self.collection_changed |= other.collection_changed;
        self.environments_changed |= other.environments_changed;
        self.access_changed |= other.access_changed;
        for author in other.authors {
            if !self.authors.contains(&author) {
                self.authors.push(author);
            }
        }
    }
}

enum Merge {
    Done(Outcome),
    Conflicts(Vec<ConflictView>),
}

/// One sync: record local edits as a commit, fetch, bring in the team's commits (merging field
/// by field), then push. Offline or refused pushes keep the local commits for next time.
pub async fn sync(
    db: &Database,
    vault: &Vault,
    state: &TeamState,
    collection_id: &str,
    decisions: &HashMap<String, Choice>,
) -> Result<SyncReport, AppError> {
    check_id(collection_id)?;
    let link = db
        .with(|conn| repo::link(conn, collection_id))?
        .ok_or(AppError::NotFound("Shared collection"))?;
    let author = match link.author.clone() {
        Some(author) => author,
        None => db
            .with(repo::author)?
            .ok_or_else(|| AppError::invalid("Add your name and email for commits first"))?,
    };
    let lock = state.lock(collection_id)?;
    let _guard = lock.lock().await;
    let git = Git::signed_in_as(state.dir(collection_id), link.username.as_deref());
    let tracking = format!("{REMOTE}/{}", link.branch);
    if !usable(&git).await
        && let Err(error) = rebuild(state, &link).await?
    {
        return failure(&git, &tracking, error, Outcome::default()).await;
    }
    let own = db.with(|conn| {
        groups::seal_values(conn, vault, collection_id)?;
        files::from_db(conn, collection_id)
    })?;
    files::write(git.dir(), &own)?;
    commit_local(&git, &own, &author).await?;

    let push_target = format!("HEAD:{}", link.branch);
    let mut outcome = Outcome::default();
    for _ in 0..MAX_PUSH_ATTEMPTS {
        if let Err(error) = git
            .network(&["fetch", "--quiet", REMOTE, &link.branch])
            .await
        {
            return failure(&git, &tracking, error, outcome).await;
        }
        let (ahead, behind) = counts(&git, &tracking).await?;
        if behind > 0 {
            let before = files::read(git.dir())?;
            let keys = Keys {
                vault,
                device: &state.device,
            };
            let step = if ahead == 0 {
                fast_forward(db, keys, &git, &tracking, &before).await?
            } else {
                match merge(db, keys, &git, &tracking, &before, decisions, &author).await? {
                    Merge::Done(step) => step,
                    Merge::Conflicts(conflicts) => {
                        state.set_pending(collection_id, conflicts.clone())?;
                        return Ok(report(
                            SyncState::Conflict,
                            None,
                            ahead,
                            outcome,
                            conflicts,
                            link.synced_at,
                        ));
                    }
                }
            };
            outcome.absorb(step);
        }
        let (ahead, _) = counts(&git, &tracking).await?;
        if ahead > 0 {
            match git
                .network(&["push", "--quiet", REMOTE, &push_target])
                .await
            {
                Ok(_) => {}
                Err(GitError::Rejected) => continue,
                Err(error) => return failure(&git, &tracking, error, outcome).await,
            }
        }
        let now = now_ms();
        let head = rev(&git, "HEAD").await?;
        db.with(|conn| repo::mark_synced(conn, collection_id, now, &head))?;
        state.set_pending(collection_id, Vec::new())?;
        return Ok(report(
            SyncState::Synced,
            None,
            0,
            outcome,
            Vec::new(),
            Some(now),
        ));
    }
    let ahead = counts(&git, &tracking).await.map_or(0, |(ahead, _)| ahead);
    Ok(report(
        SyncState::Error,
        Some("The repository kept changing while syncing. Try again".to_owned()),
        ahead,
        outcome,
        Vec::new(),
        link.synced_at,
    ))
}

fn report(
    status: SyncState,
    message: Option<String>,
    ahead: u32,
    outcome: Outcome,
    conflicts: Vec<ConflictView>,
    synced_at: Option<i64>,
) -> SyncReport {
    SyncReport {
        status,
        message,
        ahead,
        changed: outcome.changed,
        collection_changed: outcome.collection_changed,
        environments_changed: outcome.environments_changed,
        access_changed: outcome.access_changed,
        authors: outcome.authors,
        conflicts,
        synced_at,
    }
}

/// A sync that could not reach or update the remote. Local commits stay for the next try.
async fn failure(
    git: &Git,
    tracking: &str,
    error: GitError,
    outcome: Outcome,
) -> Result<SyncReport, AppError> {
    let status = match error {
        GitError::Offline | GitError::Timeout => SyncState::Offline,
        GitError::SignIn | GitError::NotFound => SyncState::SignIn,
        GitError::ReadOnly => SyncState::ReadOnly,
        GitError::Missing | GitError::Rejected | GitError::Failed(_) => SyncState::Error,
    };
    let ahead = counts(git, tracking).await.map_or(0, |(ahead, _)| ahead);
    let message = AppError::from(error).to_string();
    Ok(report(
        status,
        Some(message),
        ahead,
        outcome,
        Vec::new(),
        None,
    ))
}

async fn counts(git: &Git, tracking: &str) -> Result<(u32, u32), AppError> {
    let range = format!("HEAD...{tracking}");
    let text = git
        .run(&["rev-list", "--left-right", "--count", &range])
        .await?;
    let mut numbers = text
        .split_whitespace()
        .map(|n| n.parse::<u32>().unwrap_or(0));
    Ok((numbers.next().unwrap_or(0), numbers.next().unwrap_or(0)))
}

async fn rev(git: &Git, name: &str) -> Result<String, AppError> {
    Ok(git
        .run(&["rev-parse", "--verify", "--quiet", name])
        .await?
        .trim()
        .to_owned())
}

/// A clone is usable while Git still finds its repository and a current commit.
async fn usable(git: &Git) -> bool {
    git.dir().join(".git").is_dir() && rev(git, "HEAD").await.is_ok()
}

/// Clones the repository again when this device's copy is gone or broken. The outer error is
/// local; the inner one is Git's (sign-in, offline), reported like any failed sync.
async fn rebuild(state: &TeamState, link: &TeamLink) -> Result<Result<(), GitError>, AppError> {
    fs::create_dir_all(&state.root).map_err(|_| AppError::FileAccess)?;
    let temp_name = format!("{JOINING_PREFIX}{}", new_id());
    let temp = state.root.join(&temp_name);
    let cloned = clone_at_base(state, link, &temp_name, &temp).await;
    if !matches!(cloned, Ok(Ok(()))) {
        let _ = fs::remove_dir_all(&temp);
        return cloned;
    }
    let dir = state.dir(&link.collection_id);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|_| AppError::FileAccess)?;
    }
    fs::rename(&temp, &dir).map_err(|_| AppError::FileAccess)?;
    Ok(Ok(()))
}

/// The new clone starts at the commit this device last synced, so changes saved here since then
/// are committed on top of it and merge with the remote like any other local edit.
async fn clone_at_base(
    state: &TeamState,
    link: &TeamLink,
    temp_name: &str,
    temp: &Path,
) -> Result<Result<(), GitError>, AppError> {
    let cloned = Git::signed_in_as(state.root.clone(), link.username.as_deref())
        .network(&[
            "clone",
            "--quiet",
            "--origin",
            REMOTE,
            "--branch",
            &link.branch,
            "--",
            &link.remote,
            temp_name,
        ])
        .await;
    if let Err(error) = cloned {
        return Ok(Err(error));
    }
    let git = Git::new(temp.to_path_buf());
    let base = base_commit(&git, link).await?;
    git.run(&["reset", "--hard", "--quiet", &base]).await?;
    Ok(Ok(()))
}

/// The commit last synced; for links older than schema 24 the newest commit from before the last
/// sync, or else the first commit, so differences come up as conflicts instead of being lost.
async fn base_commit(git: &Git, link: &TeamLink) -> Result<String, AppError> {
    if let Some(commit) = link
        .synced_commit
        .as_deref()
        .filter(|commit| check_commit(commit).is_ok())
    {
        let spec = format!("{commit}^{{commit}}");
        if git.run(&["cat-file", "-e", &spec]).await.is_ok() {
            return Ok(commit.to_owned());
        }
    }
    if let Some(at) = link.synced_at {
        let before = format!("--before=@{}", at / 1000);
        let found = git.run(&["rev-list", "-1", &before, "HEAD"]).await?;
        if !found.trim().is_empty() {
            return Ok(found.trim().to_owned());
        }
    }
    git.run(&["rev-list", "--max-parents=0", "HEAD"])
        .await?
        .lines()
        .last()
        .map(str::to_owned)
        .ok_or_else(|| AppError::invalid("The repository has no commits yet"))
}

/// What taking in the team's work needs: the vault for values stored here and this device's key
/// for the access groups.
#[derive(Clone, Copy)]
struct Keys<'a> {
    vault: &'a Vault,
    device: &'a DeviceKey,
}

async fn fast_forward(
    db: &Database,
    keys: Keys<'_>,
    git: &Git,
    tracking: &str,
    before: &Snapshot,
) -> Result<Outcome, AppError> {
    let old_head = rev(git, "HEAD").await?;
    let history = group_history(git, &old_head, tracking).await?;
    git.run(&["merge", "--quiet", "--ff-only", tracking])
        .await?;
    let after = files::read(git.dir())?;
    let mut outcome = take_in(db, keys, before, &after, &history)?;
    outcome.authors = authors(git, &old_head, tracking).await?;
    Ok(outcome)
}

/// Both sides have new commits. Each file changed on either side is merged field by field; when
/// every conflict has a decision, a merge commit is written with the result.
async fn merge(
    db: &Database,
    keys: Keys<'_>,
    git: &Git,
    tracking: &str,
    before: &Snapshot,
    decisions: &HashMap<String, Choice>,
    author: &Author,
) -> Result<Merge, AppError> {
    let base = git
        .run(&["merge-base", "HEAD", tracking])
        .await?
        .trim()
        .to_owned();
    let mut paths: BTreeSet<String> = changed_paths(git, &base, "HEAD").await?;
    let theirs_paths = changed_paths(git, &base, tracking).await?;
    paths.extend(theirs_paths.iter().cloned());

    let mut results: BTreeMap<String, Merged> = BTreeMap::new();
    let mut conflicts = Vec::new();
    let mut foreign = Vec::new();
    for path in &paths {
        if !files::is_known_path(path) {
            if theirs_paths.contains(path) {
                foreign.push(path.clone());
            }
            continue;
        }
        let base_value = parsed(git, &base, path).await?;
        let mine = parsed(git, "HEAD", path).await?;
        let theirs = parsed(git, tracking, path).await?;
        let merged = merge_file(
            path,
            base_value.as_ref(),
            mine.as_ref(),
            theirs.as_ref(),
            decisions.get(path).copied(),
        );
        if !merged.conflicts.is_empty() {
            conflicts.push(conflict_view(
                path,
                &merged.conflicts,
                mine.as_ref().or(theirs.as_ref()),
            ));
        }
        results.insert(path.clone(), merged);
    }
    if !conflicts.is_empty() {
        return Ok(Merge::Conflicts(conflicts));
    }

    // Records the remote commit as a parent while keeping this tree; the merged files go on top.
    git.run(&[
        "merge",
        "--quiet",
        "--no-ff",
        "--no-commit",
        "-s",
        "ours",
        tracking,
    ])
    .await?;
    if let Err(error) = write_merge(git, tracking, results, &foreign).await {
        // Back to the state before the merge; the user sees the reason.
        let _ = git.run(&["merge", "--abort"]).await;
        return Err(error);
    }
    git.run(&["add", "--all"]).await?;
    git.commit(
        &[
            "commit",
            "--quiet",
            "--no-edit",
            "-m",
            "Merge changes from collaborators",
        ],
        author,
    )
    .await?;
    let history = group_history(git, &base, tracking).await?;
    let after = files::read(git.dir())?;
    let mut outcome = take_in(db, keys, before, &after, &history)?;
    outcome.authors = authors(git, &base, tracking).await?;
    Ok(Merge::Done(outcome))
}

/// The versions of each access group committed in `from..to`, oldest first, so every new key
/// is checked against the one before it. A version that cannot be read is skipped.
async fn group_history(
    git: &Git,
    from: &str,
    to: &str,
) -> Result<BTreeMap<String, Vec<GroupFile>>, AppError> {
    let range = format!("{from}..{to}");
    let mut history = BTreeMap::new();
    for path in changed_paths(git, from, to).await? {
        let Some(id) = access::group_id_of(&path) else {
            continue;
        };
        let commits = git
            .run(&["log", "--reverse", "--format=%H", &range, "--", &path])
            .await?;
        let mut versions = Vec::new();
        for commit in commits
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            check_commit(commit)?;
            if let Some(value) = parsed(git, commit, &path).await.ok().flatten()
                && let Ok(group) = files::group_from_value(&path, &value)
            {
                versions.push(group);
            }
        }
        history.insert(id.to_owned(), versions);
    }
    Ok(history)
}

async fn write_merge(
    git: &Git,
    tracking: &str,
    mut results: BTreeMap<String, Merged>,
    foreign: &[String],
) -> Result<(), AppError> {
    let mut collection_value = match results.remove(COLLECTION).and_then(|merged| merged.value) {
        Some(value) => value,
        None => parsed(git, "HEAD", COLLECTION)
            .await?
            .ok_or_else(|| AppError::invalid("The repository has no collection.json"))?,
    };
    // "Keep both": the local version becomes a new request right after the original.
    let mut copies = Vec::new();
    for (path, merged) in &results {
        if let (Some(copy), Some(original)) = (&merged.copy, request_id_of(path)) {
            let id = new_id();
            let mut copy = copy.clone();
            let name = copy
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Request")
                .to_owned();
            copy["id"] = Value::String(id.clone());
            copy["name"] = Value::String(copy_name(&name));
            insert_after(&mut collection_value, original, &id);
            copies.push((request_path(&id), copy));
        }
    }
    let collection = files::collection_from_value(&collection_value)?;
    files::write_text(
        git.dir(),
        COLLECTION,
        &files::normalize(COLLECTION, &collection_value, &collection)?,
    )?;
    for (path, value) in results
        .iter()
        .filter_map(|(path, m)| m.value.as_ref().map(|v| (path.clone(), v.clone())))
        .chain(copies)
    {
        files::write_text(
            git.dir(),
            &path,
            &files::normalize(&path, &value, &collection)?,
        )?;
    }
    for (path, _) in results.iter().filter(|(_, merged)| merged.value.is_none()) {
        let target = git.dir().join(path);
        if target.exists() {
            fs::remove_file(target).map_err(|_| AppError::FileAccess)?;
        }
    }
    // Files Codestamp does not manage (a README, say) are taken from the remote as they are.
    for path in foreign {
        if git.show(tracking, path).await?.is_some() {
            git.run(&["checkout", tracking, "--", path]).await?;
        } else {
            git.run(&["rm", "--quiet", "--ignore-unmatch", "--", path])
                .await?;
        }
    }
    Ok(())
}

fn insert_after(collection: &mut Value, original: &str, id: &str) {
    if let Some(order) = collection.get_mut("order").and_then(Value::as_array_mut) {
        let at = order
            .iter()
            .position(|entry| entry.as_str() == Some(original))
            .map_or(order.len(), |index| index + 1);
        order.insert(at, Value::String(id.to_owned()));
    }
}

fn copy_name(name: &str) -> String {
    let room = MAX_NAME_CHARS - COPY_SUFFIX.chars().count();
    let base: String = name.chars().take(room).collect();
    format!("{}{COPY_SUFFIX}", base.trim_end())
}

/// Applies what arrived from the team to the local collection and describes it. Access groups
/// go first, so restricted values open with the keys they bring.
fn take_in(
    db: &Database,
    keys: Keys<'_>,
    before: &Snapshot,
    after: &Snapshot,
    history: &BTreeMap<String, Vec<GroupFile>>,
) -> Result<Outcome, AppError> {
    let changed: BTreeSet<String> = after
        .requests
        .iter()
        .filter(|(id, request)| before.requests.get(*id) != Some(*request))
        .map(|(id, _)| id.clone())
        .collect();
    let removed: Vec<ChangedRequest> = before
        .requests
        .iter()
        .filter(|(id, _)| !after.requests.contains_key(*id))
        .map(|(id, request)| ChangedRequest {
            id: id.clone(),
            name: request.name.clone(),
            change: Change::Removed,
        })
        .collect();
    let collection_changed = before.collection != after.collection;
    let environments_changed = before.environments != after.environments;
    let access_changed =
        before.groups != after.groups || before.access_requests != after.access_requests;
    if changed.is_empty()
        && removed.is_empty()
        && !collection_changed
        && !environments_changed
        && !access_changed
    {
        return Ok(Outcome::default());
    }
    db.with(|conn| {
        apply(conn, after, Some(&changed))?;
        if access_changed {
            groups::absorb(conn, keys.vault, keys.device, after, history)?;
        }
        apply_variables(conn, keys.vault, after)
    })?;
    let mut list: Vec<ChangedRequest> = changed
        .iter()
        .filter_map(|id| {
            let request = after.requests.get(id)?;
            Some(ChangedRequest {
                id: id.clone(),
                name: request.name.clone(),
                change: if before.requests.contains_key(id) {
                    Change::Updated
                } else {
                    Change::Added
                },
            })
        })
        .collect();
    list.extend(removed);
    Ok(Outcome {
        changed: list,
        collection_changed,
        environments_changed,
        access_changed,
        authors: Vec::new(),
    })
}

async fn changed_paths(git: &Git, from: &str, to: &str) -> Result<BTreeSet<String>, AppError> {
    let text = git
        .run(&["diff", "--name-only", "--no-renames", "-z", from, to])
        .await?;
    Ok(text
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

async fn parsed(git: &Git, rev: &str, path: &str) -> Result<Option<Value>, AppError> {
    match git.show(rev, path).await? {
        Some(text) => files::parse_text(path, &text).map(Some),
        None => Ok(None),
    }
}

/// Who made the commits that came in, in the order they were made.
async fn authors(git: &Git, from: &str, to: &str) -> Result<Vec<String>, AppError> {
    let range = format!("{from}..{to}");
    let text = git
        .run(&["log", "--reverse", "--format=%an", &range])
        .await?;
    let mut names: Vec<String> = Vec::new();
    for name in text.lines().map(str::trim).filter(|name| !name.is_empty()) {
        if !names.iter().any(|known| known == name) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

fn conflict_view(path: &str, conflicts: &[FieldConflict], current: Option<&Value>) -> ConflictView {
    let request_id = request_id_of(path).map(str::to_owned);
    let name = current
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or(path);
    let title = if environment_id_of(path).is_some() {
        format!("Environment “{name}”")
    } else if access::group_id_of(path).is_some() {
        format!("Access group “{name}”")
    } else {
        name.to_owned()
    };
    // Who is in a group is what people can judge; its keys and stamp are not.
    if access::group_id_of(path).is_some() {
        return ConflictView {
            file: path.to_owned(),
            allow_both: false,
            request_id,
            title,
            fields: conflicts
                .iter()
                .map(|conflict| ConflictField {
                    label: "Members".to_owned(),
                    mine: members_of(conflict.mine.as_ref()),
                    theirs: members_of(conflict.theirs.as_ref()),
                })
                .collect(),
        };
    }
    ConflictView {
        file: path.to_owned(),
        allow_both: request_id.is_some(),
        request_id,
        title,
        fields: conflicts
            .iter()
            .map(|conflict| ConflictField {
                label: conflict.label.clone(),
                mine: display(conflict.mine.as_ref()),
                theirs: display(conflict.theirs.as_ref()),
            })
            .collect(),
    }
}

/// Records local edits as one commit, described by what changed.
async fn commit_local(git: &Git, snapshot: &Snapshot, author: &Author) -> Result<(), AppError> {
    git.run(&["add", "--all"]).await?;
    let staged = git
        .run(&["diff", "--cached", "--name-status", "--no-renames", "-z"])
        .await?;
    if staged.is_empty() {
        return Ok(());
    }
    let message = commit_message(git, &staged, snapshot).await?;
    git.commit(&["commit", "--quiet", "-m", &message], author)
        .await?;
    Ok(())
}

async fn commit_message(git: &Git, staged: &str, snapshot: &Snapshot) -> Result<String, AppError> {
    let parts: Vec<&str> = staged.split('\0').filter(|part| !part.is_empty()).collect();
    let mut lines = Vec::new();
    for pair in parts.chunks(2) {
        let [status, path] = pair else {
            continue;
        };
        let verb = match status.chars().next() {
            Some('A') => "Add",
            Some('D') => "Delete",
            _ => "Update",
        };
        let what = if let Some(id) = request_id_of(path) {
            let name = match snapshot.requests.get(id) {
                Some(request) => Some(request.name.clone()),
                None => parsed(git, "HEAD", path)
                    .await?
                    .and_then(|value| value.get("name").and_then(Value::as_str).map(str::to_owned)),
            };
            name.map_or_else(|| "a request".to_owned(), |name| format!("“{name}”"))
        } else if *path == COLLECTION {
            "collection settings".to_owned()
        } else if let Some(id) = access::group_id_of(path) {
            let name = snapshot
                .groups
                .get(id)
                .map_or("", |group| group.name.as_str());
            format!("access group “{name}”")
        } else if let Some((group_id, _)) = access::request_of(path) {
            let name = snapshot
                .groups
                .get(group_id)
                .map_or("", |group| group.name.as_str());
            format!("an access request for “{name}”")
        } else if let Some(id) = environment_id_of(path) {
            let name = match snapshot.environments.get(id) {
                Some(environment) => Some(environment.name.clone()),
                None => parsed(git, "HEAD", path)
                    .await?
                    .and_then(|value| value.get("name").and_then(Value::as_str).map(str::to_owned)),
            };
            name.map_or_else(
                || "an environment".to_owned(),
                |name| format!("environment “{name}”"),
            )
        } else {
            continue;
        };
        lines.push(format!("{verb} {what}"));
    }
    let collection = &snapshot.collection.name;
    Ok(match lines.as_slice() {
        [] => format!("Update “{collection}”"),
        [only] => only.clone(),
        many => format!(
            "Update {} items in “{collection}”\n\n{}",
            many.len(),
            many.iter()
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    })
}

fn reset_dir(dir: &Path) -> Result<(), AppError> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|_| AppError::FileAccess)?;
    }
    fs::create_dir_all(dir).map_err(|_| AppError::FileAccess)
}

/// A branch name from the remote, before it is used in commands.
fn check_branch(name: &str) -> Result<String, AppError> {
    let valid = !name.is_empty()
        && name.chars().count() <= MAX_BRANCH_CHARS
        && !name.starts_with(['-', '.'])
        && !name.contains("..")
        && name != "HEAD"
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'));
    if valid {
        Ok(name.to_owned())
    } else {
        Err(AppError::invalid(
            "The repository's main branch has a name Codestamp cannot use",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::models::Auth;
    use crate::environment::models::{VariableInput, VariableView};
    use crate::environment::repo as env_repo;
    use crate::environment::vault::FixedKey;
    use crate::storage::models::{KeyValue, RequestContent, RequestKind};
    use crate::team::{git, history, report};

    /// A device key per name, so each test device is its own member.
    fn seed_of(name: &str) -> u8 {
        let sum: u32 = name.bytes().map(u32::from).sum();
        u8::try_from(sum % 251).unwrap_or(1)
    }

    struct Device {
        db: Database,
        vault: Vault,
        state: TeamState,
        author: Author,
    }

    impl Device {
        fn new(base: &Path, name: &str) -> Self {
            Self {
                db: Database::in_memory().expect("database"),
                vault: Vault::new(Box::new(FixedKey([9; 32]))),
                state: TeamState::new(base.join(name), access::tests::device(seed_of(name))),
                author: Author {
                    name: name.to_owned(),
                    email: format!("{name}@example.com"),
                },
            }
        }

        fn account(&self) -> Account {
            Account {
                author: self.author.clone(),
                username: None,
            }
        }

        async fn sync(&self, id: &str, decisions: &HashMap<String, Choice>) -> SyncReport {
            sync(&self.db, &self.vault, &self.state, id, decisions)
                .await
                .expect("sync")
        }

        fn request(&self, id: &str) -> RequestContent {
            self.db
                .with(|conn| collections::load_request(conn, id))
                .expect("request")
                .content
        }

        fn edit(&self, id: &str, change: impl FnOnce(&mut RequestContent)) {
            let mut content = self.request(id);
            change(&mut content);
            self.db
                .with(|conn| collections::update_request(conn, id, &content).map(|_| ()))
                .expect("edit");
        }

        fn names(&self, collection_id: &str) -> Vec<String> {
            self.db
                .with(|conn| collections::collection_detail(conn, collection_id))
                .expect("collection")
                .requests
                .into_iter()
                .map(|request| request.name)
                .collect()
        }
    }

    fn content(name: &str, url: &str) -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: name.to_owned(),
            method: "GET".to_owned(),
            url: url.to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body_mode: "none".to_owned(),
            body: String::new(),
            form: Vec::new(),
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: String::new(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    #[tokio::test]
    async fn shares_into_a_repository_that_has_content() {
        let base = std::env::temp_dir().join(format!("codestamp-replace-{}", new_id()));
        fs::create_dir_all(&base).expect("temp dir");
        if git::version(&base).await.is_none() {
            fs::remove_dir_all(&base).expect("clean up");
            return;
        }
        replace_scenario(&base).await;
        fs::remove_dir_all(&base).expect("clean up");
    }

    async fn replace_scenario(base: &Path) {
        let tool = Git::new(base.to_path_buf());
        tool.run(&[
            "init",
            "--quiet",
            "--bare",
            "--initial-branch",
            "master",
            "old.git",
        ])
        .await
        .expect("bare repository");
        let remote = base.join("old.git").to_string_lossy().into_owned();
        tool.run(&["clone", "--quiet", &remote, "seed"])
            .await
            .expect("seed clone");
        let seed = Git::new(base.join("seed"));
        fs::write(base.join("seed").join("README.md"), "old\n").expect("readme");
        fs::create_dir_all(base.join("seed").join("postman")).expect("folder");
        fs::write(base.join("seed").join("postman").join("shop.json"), "{}\n").expect("file");
        seed.run(&["add", "--all"]).await.expect("add");
        let seeder = Author {
            name: "Seed".to_owned(),
            email: "seed@example.com".to_owned(),
        };
        seed.commit(&["commit", "--quiet", "-m", "Old files"], &seeder)
            .await
            .expect("commit");
        seed.network(&["push", "--quiet", "origin", "master"])
            .await
            .expect("push");

        let ana = Device::new(base, "ana");
        let collection_id = ana
            .db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Shop")?;
                collections::insert_request(
                    conn,
                    &collection.id,
                    None,
                    content("Policy", "https://api.test/policy"),
                )?;
                Ok(collection.id)
            })
            .expect("collection");

        let empty_remote = base.join("empty.git").to_string_lossy().into_owned();
        tool.run(&["init", "--quiet", "--bare", "empty.git"])
            .await
            .expect("empty repository");
        let empty = preview_share(&ana.state, &empty_remote, None)
            .await
            .expect("empty preview");
        assert!(empty.empty && empty.head.is_none());

        let preview = preview_share(&ana.state, &remote, None)
            .await
            .expect("preview");
        assert!(!preview.empty && !preview.codestamp);
        assert_eq!(preview.branch.as_deref(), Some("master"));
        assert_eq!(preview.files, ["README.md", "postman/shop.json"]);
        assert!(!ana.state.root().read_dir().expect("root").any(|entry| {
            entry.is_ok_and(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(JOINING_PREFIX)
            })
        }));
        let head = preview.head.expect("head");

        assert!(
            share(
                &ana.db,
                &ana.state,
                &collection_id,
                &remote,
                &ana.account(),
                None
            )
            .await
            .is_err(),
            "content is never replaced without a reviewed commit"
        );
        let stale = "0123456789abcdef0123456789abcdef01234567";
        assert!(
            share(
                &ana.db,
                &ana.state,
                &collection_id,
                &remote,
                &ana.account(),
                Some(stale)
            )
            .await
            .is_err(),
            "a moved branch is checked again first"
        );
        let status = share(
            &ana.db,
            &ana.state,
            &collection_id,
            &remote,
            &ana.account(),
            Some(&head),
        )
        .await
        .expect("replace");
        assert_eq!(status.branch, "master");

        let listed = tool
            .run(&[
                "--git-dir",
                &remote,
                "ls-tree",
                "-r",
                "--name-only",
                "master",
            ])
            .await
            .expect("files");
        assert!(listed.lines().any(|path| path == files::MANIFEST));
        assert!(
            !listed
                .lines()
                .any(|path| path == "README.md" || path.starts_with("postman/"))
        );
        let count = tool
            .run(&["--git-dir", &remote, "rev-list", "--count", "master"])
            .await
            .expect("count");
        assert_eq!(count.trim(), "2", "one new commit on top of the old ones");

        let budi = Device::new(base, "budi");
        let joined = join(&budi.db, &budi.vault, &budi.state, &remote, &budi.account())
            .await
            .expect("join");
        assert_eq!(budi.names(&joined.id), ["Policy"]);
        ana.edit(
            &ana.db
                .with(|conn| collections::collection_detail(conn, &collection_id))
                .expect("detail")
                .requests[0]
                .id,
            |c| c.url = "https://api.test/v2/policy".to_owned(),
        );
        assert_eq!(
            ana.sync(&collection_id, &HashMap::new()).await.status,
            SyncState::Synced
        );

        let other = Device::new(base, "other");
        let not_codestamp = base.join("plain.git").to_string_lossy().into_owned();
        tool.run(&[
            "clone",
            "--quiet",
            "--bare",
            &base.join("seed").to_string_lossy(),
            "plain.git",
        ])
        .await
        .expect("plain repository");
        let refused = join(
            &other.db,
            &other.vault,
            &other.state,
            &not_codestamp,
            &other.account(),
        )
        .await
        .expect_err("not a Codestamp repository");
        assert!(
            refused
                .to_string()
                .contains("was not shared from Codestamp")
        );
    }

    #[tokio::test]
    async fn rebuilds_a_clone_that_lost_its_git_folder() {
        let base = std::env::temp_dir().join(format!("codestamp-rebuild-{}", new_id()));
        fs::create_dir_all(&base).expect("temp dir");
        if git::version(&base).await.is_none() {
            fs::remove_dir_all(&base).expect("clean up");
            return;
        }
        rebuild_scenario(&base).await;
        fs::remove_dir_all(&base).expect("clean up");
    }

    async fn clone_head(device: &Device, collection_id: &str) -> String {
        rev(&Git::new(device.state.dir(collection_id)), "HEAD")
            .await
            .expect("head")
    }

    async fn rebuild_scenario(base: &Path) {
        Git::new(base.to_path_buf())
            .run(&[
                "init",
                "--quiet",
                "--bare",
                "--initial-branch",
                BRANCH,
                "team.git",
            ])
            .await
            .expect("bare repository");
        let remote = base.join("team.git").to_string_lossy().into_owned();
        let none = HashMap::new();
        let ana = Device::new(base, "ana");
        let (collection_id, login, users) = ana
            .db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Orders")?;
                let login = collections::insert_request(
                    conn,
                    &collection.id,
                    None,
                    content("Login", "https://api.test/login"),
                )?;
                let users = collections::insert_request(
                    conn,
                    &collection.id,
                    None,
                    content("Users", "https://api.test/users"),
                )?;
                Ok((collection.id, login.id, users.id))
            })
            .expect("collection");
        share(
            &ana.db,
            &ana.state,
            &collection_id,
            &remote,
            &ana.account(),
            None,
        )
        .await
        .expect("share");
        let link = |device: &Device| {
            device
                .db
                .with(|conn| repo::link(conn, &collection_id))
                .expect("link")
                .expect("linked")
        };
        assert_eq!(
            link(&ana).synced_commit,
            Some(clone_head(&ana, &collection_id).await)
        );
        let budi = Device::new(base, "budi");
        join(&budi.db, &budi.vault, &budi.state, &remote, &budi.account())
            .await
            .expect("join");
        assert_eq!(
            link(&budi).synced_commit,
            Some(clone_head(&ana, &collection_id).await)
        );

        // Budi sends a change while Ana edits offline and then loses her clone's .git.
        budi.edit(&users, |c| c.url = "https://api.test/people".to_owned());
        assert_eq!(
            budi.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        ana.edit(&login, |c| c.url = "https://api.test/sign-in".to_owned());
        fs::remove_dir_all(ana.state.dir(&collection_id).join(".git")).expect("lose .git");
        let repaired = ana.sync(&collection_id, &none).await;
        assert_eq!(repaired.status, SyncState::Synced, "{:?}", repaired.message);
        assert_eq!(ana.request(&users).url, "https://api.test/people");
        assert_eq!(ana.request(&login).url, "https://api.test/sign-in");
        assert_eq!(
            link(&ana).synced_commit,
            Some(clone_head(&ana, &collection_id).await)
        );
        assert_eq!(
            budi.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        assert_eq!(budi.request(&login).url, "https://api.test/sign-in");

        // A link from before schema 24 finds its base by the last sync time. Commit times have
        // one-second steps, so Budi's next commit waits past Ana's sync.
        ana.db
            .with(|conn| {
                conn.execute("UPDATE team_links SET synced_commit = NULL", [])?;
                Ok(())
            })
            .expect("forget commit");
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        budi.edit(&users, |c| c.url = "https://api.test/members".to_owned());
        assert_eq!(
            budi.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        ana.edit(&login, |c| c.url = "https://api.test/session".to_owned());
        fs::remove_dir_all(ana.state.dir(&collection_id)).expect("lose the clone");
        let estimated = ana.sync(&collection_id, &none).await;
        assert_eq!(
            estimated.status,
            SyncState::Synced,
            "{:?}",
            estimated.message
        );
        assert_eq!(ana.request(&users).url, "https://api.test/members");
        assert_eq!(ana.request(&login).url, "https://api.test/session");

        // When the repository cannot be reached, the sync reports it and leaves no temp folder.
        fs::remove_dir_all(ana.state.dir(&collection_id).join(".git")).expect("lose .git");
        fs::rename(base.join("team.git"), base.join("moved.git")).expect("move remote");
        let unreachable = ana.sync(&collection_id, &none).await;
        assert_eq!(
            unreachable.status,
            SyncState::SignIn,
            "{:?}",
            unreachable.message
        );
        let leftovers: Vec<_> = fs::read_dir(ana.state.root())
            .expect("clones")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(JOINING_PREFIX)
            })
            .collect();
        assert!(leftovers.is_empty());
        fs::rename(base.join("moved.git"), base.join("team.git")).expect("restore remote");
        assert_eq!(
            ana.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
    }

    #[tokio::test]
    async fn two_devices_share_merge_resolve_and_restore() {
        let base = std::env::temp_dir().join(format!("codestamp-team-{}", new_id()));
        fs::create_dir_all(&base).expect("temp dir");
        if git::version(&base).await.is_none() {
            // The scenario needs Git; machines without it skip it.
            fs::remove_dir_all(&base).expect("clean up");
            return;
        }
        scenario(&base).await;
        fs::remove_dir_all(&base).expect("clean up");
    }

    async fn scenario(base: &Path) {
        Git::new(base.to_path_buf())
            .run(&[
                "init",
                "--quiet",
                "--bare",
                "--initial-branch",
                BRANCH,
                "team.git",
            ])
            .await
            .expect("bare repository");
        let remote = base.join("team.git").to_string_lossy().into_owned();
        let none = HashMap::new();

        let ana = Device::new(base, "ana");
        let (collection_id, login, users) = ana
            .db
            .with(|conn| {
                let collection = collections::insert_collection(conn, "Shop API")?;
                let login = collections::insert_request(
                    conn,
                    &collection.id,
                    None,
                    content("Login", "https://api.test/login"),
                )?;
                let users = collections::insert_request(
                    conn,
                    &collection.id,
                    None,
                    content("Users", "https://api.test/users"),
                )?;
                Ok((collection.id, login.id, users.id))
            })
            .expect("collection");
        let signed_in = Account {
            username: Some("ana".to_owned()),
            ..ana.account()
        };
        assert!(
            share(
                &ana.db,
                &ana.state,
                &collection_id,
                &remote,
                &signed_in,
                None
            )
            .await
            .is_err(),
            "a folder remote does not sign in with a username"
        );
        share(
            &ana.db,
            &ana.state,
            &collection_id,
            &remote,
            &ana.account(),
            None,
        )
        .await
        .expect("share");
        assert!(
            share(
                &ana.db,
                &ana.state,
                &collection_id,
                &remote,
                &ana.account(),
                None
            )
            .await
            .is_err()
        );

        let budi = Device::new(base, "budi");
        let joined = join(&budi.db, &budi.vault, &budi.state, &remote, &budi.account())
            .await
            .expect("join");
        assert_eq!(joined.id, collection_id);
        assert_eq!(budi.names(&collection_id), ["Login", "Users"]);

        // Each collection keeps its own author; the profile only fills the next form.
        let work = Account {
            author: Author {
                name: "Budi at Acme".to_owned(),
                email: "budi@acme.example".to_owned(),
            },
            username: None,
        };
        let changed = set_account(&budi.db, &budi.state, &collection_id, &work).expect("account");
        assert_eq!(changed.author.as_ref(), Some(&work.author));
        assert!(set_account(&budi.db, &budi.state, &collection_id, &signed_in).is_err());
        assert_eq!(
            budi.db.with(repo::author).expect("profile"),
            Some(budi.author.clone())
        );

        // Different fields of one request, and a new request, merge without asking.
        ana.edit(&login, |c| c.url = "https://api.test/v2/login".to_owned());
        budi.edit(&login, |c| {
            c.headers.push(KeyValue {
                enabled: true,
                key: "X-Team".to_owned(),
                value: "1".to_owned(),
            });
        });
        let orders = budi
            .db
            .with(|conn| {
                collections::insert_request(
                    conn,
                    &collection_id,
                    None,
                    content("Orders", "https://api.test/orders"),
                )
            })
            .expect("new request")
            .id;
        assert_eq!(
            ana.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        let merged = budi.sync(&collection_id, &none).await;
        assert_eq!(merged.status, SyncState::Synced);
        assert_eq!(merged.authors, ["ana"]);
        let pulled = ana.sync(&collection_id, &none).await;
        assert_eq!(pulled.authors, ["Budi at Acme"]);
        assert!(
            pulled
                .changed
                .iter()
                .any(|c| c.id == orders && c.change == Change::Added)
        );
        for device in [&ana, &budi] {
            let saved = device.request(&login);
            assert_eq!(saved.url, "https://api.test/v2/login");
            assert_eq!(saved.headers.len(), 1);
        }
        assert_eq!(ana.names(&collection_id), budi.names(&collection_id));

        // Both rename the same request: the one who syncs second decides.
        ana.edit(&users, |c| c.name = "Members".to_owned());
        budi.edit(&users, |c| c.name = "People".to_owned());
        ana.sync(&collection_id, &none).await;
        let conflicted = budi.sync(&collection_id, &none).await;
        assert_eq!(conflicted.status, SyncState::Conflict);
        let file = request_path(&users);
        assert_eq!(conflicted.conflicts.len(), 1);
        assert_eq!(conflicted.conflicts[0].file, file);
        assert_eq!(conflicted.conflicts[0].fields[0].mine, "People");
        assert_eq!(conflicted.conflicts[0].fields[0].theirs, "Members");
        assert_eq!(budi.request(&users).name, "People");
        let waiting = statuses(&budi.db, &budi.state).expect("statuses");
        assert_eq!(waiting[0].conflicts.len(), 1);

        let decisions = HashMap::from([(file, Choice::Both)]);
        assert_eq!(
            budi.sync(&collection_id, &decisions).await.status,
            SyncState::Synced
        );
        assert_eq!(budi.request(&users).name, "Members");
        assert!(
            statuses(&budi.db, &budi.state).expect("statuses")[0]
                .conflicts
                .is_empty()
        );
        ana.sync(&collection_id, &none).await;
        let names = ana.names(&collection_id);
        assert!(names.contains(&"People (my copy)".to_owned()));
        assert_eq!(names, budi.names(&collection_id));

        // The first version of Login can be compared with now and brought back.
        let versions = history::versions(&ana.db, &ana.state, &collection_id, &login)
            .await
            .expect("versions");
        assert!(versions.len() >= 2);
        let first = versions.last().expect("first version").commit.clone();
        let changes = history::changes(&ana.db, &ana.state, &collection_id, &login, &first)
            .await
            .expect("changes");
        assert!(
            changes
                .iter()
                .any(|c| c.label == "URL" && c.then == "https://api.test/login")
        );
        history::restore(&ana.db, &ana.state, &collection_id, &login, &first)
            .await
            .expect("restore");
        assert_eq!(ana.request(&login).url, "https://api.test/login");
        assert!(ana.request(&login).headers.is_empty());
        ana.sync(&collection_id, &none).await;
        budi.sync(&collection_id, &none).await;
        assert_eq!(budi.request(&login).url, "https://api.test/login");

        // A chosen icon travels in collection.json, and so does going back to the automatic one.
        ana.db
            .with(|conn| collections::set_collection_icon(conn, &collection_id, Some("🦊")))
            .expect("icon");
        ana.sync(&collection_id, &none).await;
        assert!(budi.sync(&collection_id, &none).await.collection_changed);
        let detail = budi
            .db
            .with(|conn| collections::collection_detail(conn, &collection_id))
            .expect("detail");
        assert_eq!((detail.icon.as_str(), detail.icon_custom), ("🦊", true));
        budi.db
            .with(|conn| collections::set_collection_icon(conn, &collection_id, None))
            .expect("automatic");
        budi.sync(&collection_id, &none).await;
        ana.sync(&collection_id, &none).await;
        let detail = ana
            .db
            .with(|conn| collections::collection_detail(conn, &collection_id))
            .expect("detail");
        assert!(!detail.icon_custom);

        // An environment that belongs to the collection travels with it; secret values stay local.
        let input = |key: &str, value: &str, secret: bool| VariableInput {
            id: None,
            key: key.to_owned(),
            value: Some(value.to_owned()),
            is_secret: secret,
            enabled: true,
            restricted: None,
        };
        let dev = ana
            .db
            .with(|conn| {
                let dev = env_repo::insert(conn, "DEV")?;
                env_repo::set_owner(conn, &dev.id, Some(&collection_id))?;
                env_repo::replace_variables(
                    conn,
                    &ana.vault,
                    &dev.id,
                    vec![
                        input("baseUrl", "https://dev.test", false),
                        input("token", "ana-secret", true),
                    ],
                )?;
                Ok(dev.id)
            })
            .expect("environment");
        ana.sync(&collection_id, &none).await;
        let arrived = budi.sync(&collection_id, &none).await;
        assert!(arrived.environments_changed);
        let shared = budi
            .db
            .with(|conn| {
                let owned = env_repo::owned(conn, &collection_id)?;
                let variables = env_repo::variables(conn, &dev)?;
                Ok((owned, variables))
            })
            .expect("shared environment");
        assert_eq!(shared.0, [(dev.clone(), "DEV".to_owned())]);
        assert_eq!(shared.1[0].value, "https://dev.test");
        assert!(shared.1[1].is_secret && shared.1[1].value.is_empty());

        // Both edit it: Budi fills in his own secret and adds a variable, Ana renames it.
        budi.db
            .with(|conn| {
                env_repo::replace_variables(
                    conn,
                    &budi.vault,
                    &dev,
                    vec![
                        input("baseUrl", "https://dev.test", false),
                        input("token", "budi-secret", true),
                        input("timeout", "30", false),
                    ],
                )
                .map(|_| ())
            })
            .expect("budi edits");
        ana.db
            .with(|conn| env_repo::rename(conn, &dev, "Development"))
            .expect("ana renames");
        ana.sync(&collection_id, &none).await;
        assert_eq!(
            budi.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        ana.sync(&collection_id, &none).await;
        for device in [&ana, &budi] {
            let (owned, keys) = device
                .db
                .with(|conn| {
                    let keys: Vec<String> = env_repo::variables(conn, &dev)?
                        .into_iter()
                        .map(|variable| variable.key)
                        .collect();
                    Ok((env_repo::owned(conn, &collection_id)?, keys))
                })
                .expect("merged environment");
            assert_eq!(owned, [(dev.clone(), "Development".to_owned())]);
            assert_eq!(keys, ["baseUrl", "token", "timeout"]);
        }
        let budi_token = budi
            .db
            .with(|conn| {
                let token = env_repo::variables(conn, &dev)?
                    .into_iter()
                    .find(|variable| variable.key == "token")
                    .ok_or(AppError::NotFound("Variable"))?;
                env_repo::reveal(conn, &budi.vault, &token.id)
            })
            .expect("secret");
        assert_eq!(
            budi_token, "budi-secret",
            "a merge never sends or overwrites secrets"
        );

        // Removing it from the collection removes it for everyone.
        ana.db
            .with(|conn| env_repo::delete(conn, &dev))
            .expect("delete");
        ana.sync(&collection_id, &none).await;
        assert!(budi.sync(&collection_id, &none).await.environments_changed);
        assert!(
            budi.db
                .with(|conn| env_repo::owned(conn, &collection_id))
                .expect("owned")
                .is_empty()
        );

        // Git changes reads the same history: commits, what the latest changed, who changed what.
        let page = report::commits(&ana.db, &ana.state, &collection_id, 0)
            .await
            .expect("commits");
        assert!(page.commits.len() >= 5 && page.commits.iter().all(|c| c.pushed));
        assert!(page.commits.iter().any(|c| c.merge));
        let latest =
            report::commit_detail(&ana.db, &ana.state, &collection_id, &page.commits[0].commit)
                .await
                .expect("detail");
        assert!(latest.files.iter().any(|f| f.request_id.is_none()));
        let activity = report::request_activity(&ana.db, &ana.state, &collection_id)
            .await
            .expect("activity");
        let login_activity = activity.iter().find(|a| a.id == login).expect("login");
        assert!(login_activity.versions >= 3 && login_activity.last_author.is_some());
        let repos = report::overview(&ana.db, &ana.vault, &ana.state)
            .await
            .expect("overview");
        assert_eq!((repos[0].local_edits, repos[0].to_push), (0, 0));
        ana.edit(&login, |c| c.url = "https://api.test/v3/login".to_owned());
        let edited = report::overview(&ana.db, &ana.vault, &ana.state)
            .await
            .expect("overview");
        assert_eq!(edited[0].local_edits, 1);

        // Leaving keeps the collection on the device and removes only the clone.
        unlink(&budi.db, &budi.state, &collection_id)
            .await
            .expect("unlink");
        assert!(!budi.state.dir(&collection_id).exists());
        assert!(
            budi.db
                .with(|conn| collections::has_collection(conn, &collection_id))
                .expect("collection")
        );
    }

    impl Device {
        fn overview(&self, collection_id: &str) -> groups::AccessOverview {
            self.db
                .with(|conn| {
                    groups::overview(conn, &self.vault, self.state.device(), collection_id)
                })
                .expect("overview")
        }

        fn token(&self, collection_id: &str) -> VariableView {
            let set = self
                .db
                .with(|conn| env_repo::collection_set_id(conn, collection_id))
                .expect("set");
            self.db
                .with(|conn| env_repo::variables(conn, &set))
                .expect("variables")
                .into_iter()
                .find(|variable| variable.key == "token")
                .expect("token")
        }

        fn stored_secret(&self, id: &str) -> String {
            self.db
                .with(|conn| env_repo::reveal(conn, &self.vault, id))
                .expect("secret")
        }

        /// What a request in the collection gets for `{{token}}`.
        fn resolved_token(&self, collection_id: &str) -> Option<String> {
            let names = BTreeSet::from(["token".to_owned()]);
            self.db
                .with(|conn| {
                    env_repo::lookup_values(conn, &self.vault, &names, Some(collection_id), true)
                })
                .expect("lookup")
                .remove("token")
        }

        /// Saves `token` the way the variable editor does.
        fn save_token(
            &self,
            collection_id: &str,
            value: Option<&str>,
            group: Option<&str>,
        ) -> Result<(), AppError> {
            let current = self.token(collection_id);
            let set = self
                .db
                .with(|conn| env_repo::collection_set_id(conn, collection_id))?;
            let inputs = vec![VariableInput {
                id: Some(current.id),
                key: "token".to_owned(),
                value: value.map(str::to_owned),
                is_secret: true,
                enabled: true,
                restricted: group.map(str::to_owned),
            }];
            self.db.with(|conn| {
                groups::check_edit(conn, &set, &inputs)?;
                env_repo::replace_variables(conn, &self.vault, &set, inputs).map(|_| ())
            })
        }
    }

    #[tokio::test]
    async fn restricted_values_reach_group_members_only() {
        let base = std::env::temp_dir().join(format!("codestamp-access-{}", new_id()));
        fs::create_dir_all(&base).expect("temp dir");
        if git::version(&base).await.is_none() {
            fs::remove_dir_all(&base).expect("clean up");
            return;
        }
        access_scenario(&base).await;
        fs::remove_dir_all(&base).expect("clean up");
    }

    async fn access_scenario(base: &Path) {
        Git::new(base.to_path_buf())
            .run(&[
                "init",
                "--quiet",
                "--bare",
                "--initial-branch",
                BRANCH,
                "team.git",
            ])
            .await
            .expect("bare repository");
        let remote = base.join("team.git").to_string_lossy().into_owned();
        let none = HashMap::new();

        let ana = Device::new(base, "ana");
        let collection_id = ana
            .db
            .with(|conn| collections::insert_collection(conn, "Payments").map(|c| c.id))
            .expect("collection");
        let set = ana
            .db
            .with(|conn| env_repo::collection_set_id(conn, &collection_id))
            .expect("set");
        ana.db
            .with(|conn| {
                let token = VariableInput {
                    id: None,
                    key: "token".to_owned(),
                    value: Some("s3cr3t-1".to_owned()),
                    is_secret: true,
                    enabled: true,
                    restricted: None,
                };
                env_repo::replace_variables(conn, &ana.vault, &set, vec![token]).map(|_| ())
            })
            .expect("variables");
        share(
            &ana.db,
            &ana.state,
            &collection_id,
            &remote,
            &ana.account(),
            None,
        )
        .await
        .expect("share");
        assert!(
            ana.save_token(&collection_id, None, Some(&new_id()))
                .is_err(),
            "only a group of the collection can hold a value"
        );
        ana.db
            .with(|conn| {
                groups::create_group(conn, &ana.vault, ana.state.device(), &collection_id, "Prod")
            })
            .expect("group");
        let prod = ana.overview(&collection_id).groups[0].id.clone();
        ana.save_token(&collection_id, None, Some(&prod))
            .expect("restrict");
        assert_eq!(
            ana.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );

        let tree = |path: &str| {
            fs::read_to_string(ana.state.dir(&collection_id).join(path)).expect("repository file")
        };
        assert!(
            !tree(COLLECTION).contains("s3cr3t") && tree(COLLECTION).contains("\"restricted\"")
        );
        assert!(tree(files::MANIFEST).contains("\"format\": 8"));
        assert!(tree(&access::group_path(&prod)).contains("\"keyVersion\": 1"));

        // Budi joins: the value is locked, so requests show it as unresolved.
        let budi = Device::new(base, "budi");
        join(&budi.db, &budi.vault, &budi.state, &remote, &budi.account())
            .await
            .expect("join");
        let locked = budi.token(&collection_id);
        assert!(locked.locked && locked.restricted.as_deref() == Some(prod.as_str()));
        assert_eq!(budi.stored_secret(&locked.id), "");
        assert_eq!(budi.resolved_token(&collection_id), None);
        assert!(
            budi.save_token(&collection_id, Some("guess"), Some(&prod))
                .is_err()
        );
        assert!(budi.save_token(&collection_id, None, None).is_err());

        // Budi asks; Ana compares the fingerprint and grants.
        let budi_print = budi.state.device().fingerprint().expect("fingerprint");
        budi.db
            .with(|conn| {
                groups::request_access(
                    conn,
                    &budi.vault,
                    budi.state.device(),
                    &collection_id,
                    &prod,
                )
            })
            .expect("request");
        assert_eq!(
            budi.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        assert!(ana.sync(&collection_id, &none).await.access_changed);
        let asked = &ana.overview(&collection_id).groups[0].requests;
        assert!(asked.len() == 1 && asked[0].fingerprint == budi_print);
        ana.db
            .with(|conn| groups::grant(conn, &ana.vault, &collection_id, &prod, &budi_print))
            .expect("grant");
        assert_eq!(
            ana.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        assert!(budi.sync(&collection_id, &none).await.access_changed);
        assert!(!budi.token(&collection_id).locked);
        assert_eq!(
            budi.resolved_token(&collection_id).as_deref(),
            Some("s3cr3t-1")
        );

        // A member's edit reaches the other member, and only as ciphertext.
        budi.save_token(&collection_id, Some("s3cr3t-2"), Some(&prod))
            .expect("edit");
        assert_eq!(
            budi.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        ana.sync(&collection_id, &none).await;
        assert_eq!(
            ana.resolved_token(&collection_id).as_deref(),
            Some("s3cr3t-2")
        );
        assert!(!tree(COLLECTION).contains("s3cr3t"));
        let quiet = ana.sync(&collection_id, &none).await;
        assert_eq!(quiet.ahead, 0, "an unchanged value is not encrypted again");

        // Eve can push but is not a member. Her device puts itself in the group with a key of its
        // own; members refuse the change and keep their values locked until it is repaired.
        let eve = Device::new(base, "eve");
        join(&eve.db, &eve.vault, &eve.state, &remote, &eve.account())
            .await
            .expect("join");
        let fake_key = access::new_group_key();
        let (mut stored, _) = eve
            .db
            .with(|conn| groups::stored_files(conn, &collection_id))
            .expect("groups");
        let mut forged = stored.remove(&prod).expect("group");
        forged.members.push(
            eve.state
                .device()
                .member(&eve.author, "Eve", &fake_key, &collection_id, (&prod, 1))
                .expect("member"),
        );
        access::restamp(&mut forged, &collection_id, &fake_key).expect("stamp");
        eve.db
            .with(|conn| {
                conn.execute(
                    "UPDATE team_access_groups SET file = ?3 WHERE collection_id = ?1 AND id = ?2",
                    rusqlite::params![
                        collection_id,
                        prod,
                        serde_json::to_string(&forged).map_err(|_| AppError::Storage)?
                    ],
                )
                .map_err(AppError::from)
            })
            .expect("forge");
        assert_eq!(
            eve.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        ana.sync(&collection_id, &none).await;
        let refused = &ana.overview(&collection_id).groups[0];
        assert!(refused.problem.is_some() && refused.can_repair && !refused.access);
        assert_eq!(ana.resolved_token(&collection_id), None);
        assert_eq!(ana.stored_secret(&ana.token(&collection_id).id), "s3cr3t-2");

        ana.db
            .with(|conn| {
                groups::repair(conn, &ana.vault, ana.state.device(), &collection_id, &prod)
            })
            .expect("repair");
        assert_eq!(
            ana.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        assert_eq!(
            ana.resolved_token(&collection_id).as_deref(),
            Some("s3cr3t-2")
        );
        budi.sync(&collection_id, &none).await;
        let repaired = &budi.overview(&collection_id).groups[0];
        assert!(repaired.access && repaired.problem.is_none() && repaired.key_version == 2);
        assert_eq!(
            budi.resolved_token(&collection_id).as_deref(),
            Some("s3cr3t-2")
        );
        eve.sync(&collection_id, &none).await;
        assert!(eve.token(&collection_id).locked);
        assert_eq!(eve.resolved_token(&collection_id), None);

        // Removing Budi gives the rest a new key and lists what to change at its source.
        let to_change = ana
            .db
            .with(|conn| {
                groups::remove(
                    conn,
                    &ana.vault,
                    ana.state.device(),
                    &collection_id,
                    &prod,
                    &budi_print,
                )
            })
            .expect("remove");
        assert_eq!(to_change.len(), 1);
        assert_eq!(to_change[0].key, "token");
        assert_eq!(
            ana.sync(&collection_id, &none).await.status,
            SyncState::Synced
        );
        assert!(tree(COLLECTION).contains("\"version\": 3"));
        budi.sync(&collection_id, &none).await;
        let removed = budi.token(&collection_id);
        assert!(removed.locked);
        assert_eq!(budi.stored_secret(&removed.id), "");
        assert_eq!(
            ana.resolved_token(&collection_id).as_deref(),
            Some("s3cr3t-2")
        );
    }
}
