//! Earlier versions of one request, read from the commits of its file.

use serde_json::Value;

use super::files::{self, CollectionFile, RequestFile, request_path};
use super::git::Git;
use super::models::{FieldChange, Version};
use super::present::{display, request_fields};
use super::repo;
use super::sync::TeamState;
use crate::error::AppError;
use crate::storage::db::Database;
use crate::storage::descriptions::{self, Described};
use crate::storage::models::SavedRequest;
use crate::storage::repo as collections;
use crate::storage::validate::{check_id, check_request};

const MAX_VERSIONS: &str = "50";
const FIELD_SEPARATOR: char = '\u{1f}';
const MIN_COMMIT_CHARS: usize = 7;
const MAX_COMMIT_CHARS: usize = 40;

pub async fn versions(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    request_id: &str,
) -> Result<Vec<Version>, AppError> {
    let git = linked_repo(db, state, collection_id, request_id)?;
    let path = request_path(request_id);
    let text = git
        .run(&[
            "log",
            "--format=%H%x1f%an%x1f%at%x1f%s",
            "-n",
            MAX_VERSIONS,
            "--",
            &path,
        ])
        .await?;
    Ok(text.lines().filter_map(parse_version).collect())
}

/// The fields that differ between a version and the request as it is saved now.
pub async fn changes(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    request_id: &str,
    commit: &str,
) -> Result<Vec<FieldChange>, AppError> {
    let git = linked_repo(db, state, collection_id, request_id)?;
    let snapshot = db.with(|conn| files::from_db(conn, collection_id))?;
    let then = version_at(&git, commit, request_id, &snapshot.collection).await?;
    let now = snapshot
        .requests
        .get(request_id)
        .ok_or(AppError::NotFound("Request"))?;
    let then = to_value(&then)?;
    let now = to_value(now)?;
    Ok(request_fields()
        .filter(|(key, _)| then.get(key) != now.get(key))
        .map(|(key, label)| FieldChange {
            label: label.to_owned(),
            then: shown(key, then.get(key), &snapshot.collection),
            now: shown(key, now.get(key), &snapshot.collection),
        })
        .collect())
}

/// Puts a version's content back into the request. The next sync shares it as a new change,
/// so nothing in the history is rewritten.
pub async fn restore(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    request_id: &str,
    commit: &str,
) -> Result<SavedRequest, AppError> {
    let git = linked_repo(db, state, collection_id, request_id)?;
    let collection = db
        .with(|conn| files::from_db(conn, collection_id))?
        .collection;
    let version = version_at(&git, commit, request_id, &collection).await?;
    db.with(|conn| {
        let current = collections::load_request(conn, request_id)?;
        let content = check_request(version.to_content(Some(&current.content)))?;
        collections::update_request(conn, request_id, &content)?;
        descriptions::set(conn, Described::Request, request_id, &version.description)?;
        collections::load_request(conn, request_id)
    })
}

fn linked_repo(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    request_id: &str,
) -> Result<Git, AppError> {
    check_id(collection_id)?;
    check_id(request_id)?;
    db.with(|conn| {
        repo::link(conn, collection_id)?.ok_or(AppError::NotFound("Shared collection"))?;
        let request = collections::load_request(conn, request_id)?;
        if request.collection_id == collection_id {
            Ok(())
        } else {
            Err(AppError::NotFound("Request"))
        }
    })?;
    Ok(Git::new(state.root().join(collection_id)))
}

async fn version_at(
    git: &Git,
    commit: &str,
    request_id: &str,
    collection: &CollectionFile,
) -> Result<RequestFile, AppError> {
    check_commit(commit)?;
    let path = request_path(request_id);
    let text = git
        .show(commit, &path)
        .await?
        .ok_or(AppError::NotFound("Version"))?;
    files::request_from_text(&path, &text, collection)
}

pub(super) fn check_commit(commit: &str) -> Result<(), AppError> {
    let valid = (MIN_COMMIT_CHARS..=MAX_COMMIT_CHARS).contains(&commit.len())
        && commit.chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(AppError::invalid("The version is not valid"))
    }
}

fn parse_version(line: &str) -> Option<Version> {
    let mut parts = line.split(FIELD_SEPARATOR);
    let commit = parts.next()?.trim().to_owned();
    let author = parts.next()?.to_owned();
    let time = parts.next()?.trim().parse().ok()?;
    let message = parts.next().unwrap_or_default().to_owned();
    check_commit(&commit).ok()?;
    Some(Version {
        commit,
        author,
        time,
        message,
    })
}

fn to_value(request: &RequestFile) -> Result<Value, AppError> {
    serde_json::to_value(request).map_err(|_| AppError::Storage)
}

/// Sections are stored by id; people know them by name.
fn shown(key: &str, value: Option<&Value>, collection: &CollectionFile) -> String {
    if key != "sectionId" {
        return display(value);
    }
    let name = value.and_then(Value::as_str).and_then(|id| {
        collection
            .sections
            .iter()
            .find(|section| section.id == id)
            .map(|section| section.name.clone())
    });
    name.unwrap_or_else(|| "(no section)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_log_lines_and_refuses_bad_commits() {
        let line = "0123456789abcdef0123456789abcdef01234567\u{1f}Rina\u{1f}1760000000\u{1f}Update “Login”";
        let version = parse_version(line).expect("a version");
        assert_eq!(version.author, "Rina");
        assert_eq!(version.time, 1_760_000_000);
        assert_eq!(version.message, "Update “Login”");

        assert!(parse_version("not a log line").is_none());
        assert!(check_commit("abc").is_err());
        assert!(check_commit("HEAD~1").is_err());
        assert!(check_commit("--all").is_err());
        assert!(check_commit("0123abc").is_ok());
    }
}
