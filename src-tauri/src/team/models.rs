use serde::{Deserialize, Serialize};

use crate::storage::models::RequestKind;

/// The name and email written on commits; the access rights come from the Git host.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Author {
    pub name: String,
    pub email: String,
}

/// Who works on one shared collection. `username` picks the account Git signs in with on https
/// hosts, so one host can serve collections from different accounts; `None` leaves it to Git.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub author: Author,
    pub username: Option<String>,
}

/// An account added in Settings › Git accounts.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAccount {
    pub host: String,
    pub username: String,
    pub label: String,
}

/// One row of Settings › Git accounts: a saved account, one a collection uses, or Git's default
/// account for a host (`username` is `None`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountView {
    /// Set for accounts saved in the list; they can be removed when no collection uses them.
    pub id: Option<String>,
    pub host: String,
    pub username: Option<String>,
    pub label: String,
    pub collections: Vec<AccountUse>,
    /// What the credential manager reports for GitHub accounts; `None` when it cannot tell.
    pub signed_in: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    SignedIn,
    SignInNeeded,
    Offline,
    Failed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCheck {
    pub status: CheckStatus,
    pub message: Option<String>,
}

/// What a repository holds before a collection is shared into it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharePreview {
    /// No commits yet: sharing simply fills it.
    pub empty: bool,
    /// It holds a collection shared from Codestamp, which people may have joined.
    pub codestamp: bool,
    pub branch: Option<String>,
    /// The commit that replacing would build on; sharing checks it has not moved meanwhile.
    pub head: Option<String>,
    /// Files that replacing removes, up to a limit.
    pub files: Vec<String>,
    /// Files past the limit.
    pub more: u32,
}

/// What the Team panel needs before sharing or joining.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSetup {
    /// `None` when Git is not installed.
    pub git_version: Option<String>,
    pub author: Option<Author>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStatus {
    pub collection_id: String,
    pub remote: String,
    pub branch: String,
    pub synced_at: Option<i64>,
    /// Written on this collection's commits; `None` until a profile exists.
    pub author: Option<Author>,
    pub username: Option<String>,
    /// Conflicts waiting for a decision from the last sync.
    pub conflicts: Vec<ConflictView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncState {
    Synced,
    Conflict,
    Offline,
    SignIn,
    ReadOnly,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Change {
    Added,
    Updated,
    Removed,
}

/// A request that a teammate's changes added, updated or removed on this device.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedRequest {
    pub id: String,
    pub name: String,
    pub change: Change,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub status: SyncState,
    pub message: Option<String>,
    /// Local commits not on the remote yet (offline, read-only, or after an error).
    pub ahead: u32,
    pub changed: Vec<ChangedRequest>,
    /// The collection's name, auth, sections, order or variables changed.
    pub collection_changed: bool,
    /// An environment that belongs to the collection was added, changed or removed.
    pub environments_changed: bool,
    /// An access group or request for access changed.
    pub access_changed: bool,
    pub authors: Vec<String>,
    pub conflicts: Vec<ConflictView>,
    pub synced_at: Option<i64>,
}

/// One file that both sides changed in the same places. The user picks a side for it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictView {
    pub file: String,
    pub request_id: Option<String>,
    pub title: String,
    pub fields: Vec<ConflictField>,
    /// Requests can keep both versions (yours becomes a copy).
    pub allow_both: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictField {
    pub label: String,
    pub mine: String,
    pub theirs: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Choice {
    Mine,
    Theirs,
    Both,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub file: String,
    pub choice: Choice,
}

/// One commit that changed a request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub commit: String,
    pub author: String,
    /// Unix time in seconds.
    pub time: i64,
    pub message: String,
}

/// One shared collection's repository in Tools › Git changes, read from its clone offline.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSummary {
    pub collection_id: String,
    pub name: String,
    pub remote: String,
    pub branch: String,
    pub synced_at: Option<i64>,
    /// Files whose saved content differs from the clone; the next sync commits them.
    pub local_edits: u32,
    /// Commits made here that the remote does not have yet.
    pub to_push: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub commit: String,
    pub author: String,
    pub email: String,
    /// Unix time in seconds.
    pub time: i64,
    pub message: String,
    pub merge: bool,
    pub pushed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitPage {
    pub commits: Vec<CommitSummary>,
    pub more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
    pub commit: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub message: String,
    pub files: Vec<FileChange>,
    /// Changed files past the shown limit.
    pub hidden: u32,
}

/// A request, or the collection itself (`request_id` is `None`), as one commit changed it.
/// For an update, `fields` holds each changed field with `then` before and `now` after.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub kind: FileKind,
    pub request_id: Option<String>,
    pub name: String,
    pub change: Change,
    pub fields: Vec<FieldChange>,
}

/// What a changed file of the repository holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Request,
    Collection,
    Environment,
    AccessGroup,
    AccessRequest,
}

/// A request of the collection with who changed it last in the repository.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestActivity {
    pub id: String,
    pub name: String,
    pub method: String,
    pub kind: RequestKind,
    pub versions: u32,
    pub last_author: Option<String>,
    pub last_time: Option<i64>,
}

/// How a field differed in an older version compared with now.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldChange {
    pub label: String,
    pub then: String,
    pub now: String,
}
