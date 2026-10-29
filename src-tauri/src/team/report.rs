//! Tools › Git changes: commits, what each one changed, and who last changed each request, read
//! from the shared collections' clones. Nothing here talks to the remote.

use std::collections::{HashMap, HashSet};
use std::fs;

use serde_json::Value;

use super::access;
use super::files::{self, COLLECTION, MANIFEST, environment_id_of, request_id_of};
use super::git::Git;
use super::groups;
use super::history::check_commit;
use super::models::{
    Change, CommitDetail, CommitPage, CommitSummary, FieldChange, FileChange, FileKind,
    RepoSummary, RequestActivity,
};
use super::present::{display, members_of, request_fields};
use super::repo::{self, TeamLink};
use super::sync::TeamState;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::db::Database;
use crate::storage::repo as collections;
use crate::storage::validate::check_id;

const PAGE: usize = 50;
const MAX_SKIP: u32 = 100_000;
/// Changed files read field by field for one commit; a first share can hold thousands.
const MAX_DETAILED_FILES: usize = 200;
const MAX_LOGGED_COMMITS: &str = "2000";
const FIELD: char = '\u{1f}';
const RECORD: char = '\u{1e}';
const COLLECTION_FIELDS: [(&str, &str); 7] = [
    ("name", "Name"),
    ("description", "Docs description"),
    ("icon", "Icon"),
    ("auth", "Auth"),
    ("sections", "Sections"),
    ("order", "Request order"),
    ("variables", "Variables"),
];
const ENVIRONMENT_FIELDS: [(&str, &str); 2] = [("name", "Name"), ("variables", "Variables")];

pub async fn overview(
    db: &Database,
    vault: &Vault,
    state: &TeamState,
) -> Result<Vec<RepoSummary>, AppError> {
    let links = db.with(repo::links)?;
    let mut summaries = Vec::with_capacity(links.len());
    for link in links {
        let dir = state.dir(&link.collection_id);
        let (name, rendered) = db.with(|conn| {
            // Restricted values edited here count as waiting once they are encrypted.
            groups::seal_values(conn, vault, &link.collection_id)?;
            let snapshot = files::from_db(conn, &link.collection_id)?;
            Ok((snapshot.collection.name.clone(), files::render(&snapshot)?))
        })?;
        let local_edits = if dir.exists() {
            let differs = rendered
                .iter()
                .filter(|(path, text)| {
                    fs::read_to_string(dir.join(path)).ok().as_deref() != Some(text.as_str())
                })
                .count();
            let removed = files::request_files(&dir)?
                .iter()
                .filter(|path| !rendered.contains_key(*path))
                .count();
            differs + removed
        } else {
            0
        };
        let to_push = unpushed(&Git::new(dir), &link.branch).await.len();
        summaries.push(RepoSummary {
            collection_id: link.collection_id,
            name,
            remote: link.remote,
            branch: link.branch,
            synced_at: link.synced_at,
            local_edits: count(local_edits),
            to_push: count(to_push),
        });
    }
    Ok(summaries)
}

pub async fn commits(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    skip: u32,
) -> Result<CommitPage, AppError> {
    let (git, link) = linked(db, state, collection_id)?;
    if skip > MAX_SKIP {
        return Err(AppError::invalid("The history does not go back that far"));
    }
    let skip = format!("--skip={skip}");
    let limit = (PAGE + 1).to_string();
    let format = format!("--format=%H{FIELD}%P{FIELD}%an{FIELD}%ae{FIELD}%at{FIELD}%s");
    // An empty repository has no HEAD yet, so no commits.
    let text = git
        .run(&["log", &format, &skip, "-n", &limit, "HEAD"])
        .await
        .unwrap_or_default();
    let unpushed = unpushed(&git, &link.branch).await;
    let mut commits: Vec<CommitSummary> = text
        .lines()
        .filter_map(|line| parse_summary(line, &unpushed))
        .collect();
    let more = commits.len() > PAGE;
    commits.truncate(PAGE);
    Ok(CommitPage { commits, more })
}

pub async fn commit_detail(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
    commit: &str,
) -> Result<CommitDetail, AppError> {
    let (git, _) = linked(db, state, collection_id)?;
    check_commit(commit)?;
    let format = format!("--format=%H{FIELD}%P{FIELD}%an{FIELD}%ae{FIELD}%at{FIELD}%B");
    let meta = git
        .run(&["show", "-s", &format, commit])
        .await
        .map_err(|_| AppError::NotFound("Commit"))?;
    let mut parts = meta.splitn(6, FIELD);
    let (Some(hash), Some(parents), Some(author), Some(email), Some(time), Some(message)) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
        return Err(AppError::NotFound("Commit"));
    };
    let hash = hash.trim().to_owned();
    let parents: Vec<String> = parents.split_whitespace().map(str::to_owned).collect();
    // A merge is shown against its first parent: what it brought into this line of work.
    let listed = git
        .run(&[
            "diff-tree",
            "-r",
            "-z",
            "--no-commit-id",
            "--no-renames",
            "--name-status",
            "--root",
            "--diff-merges=first-parent",
            &hash,
        ])
        .await?;
    let changed = parse_name_status(&listed);
    let total = changed.len();
    let parent = parents.first().map(String::as_str);
    let before_collection = match parent {
        Some(parent) => read_value(&git, parent, COLLECTION).await?,
        None => None,
    };
    let after_collection = read_value(&git, &hash, COLLECTION).await?;
    let mut shown = Vec::new();
    for (change, path) in changed.into_iter().take(MAX_DETAILED_FILES) {
        let before = match (parent, change) {
            (Some(parent), Change::Updated | Change::Removed) => {
                read_value(&git, parent, &path).await?
            }
            _ => None,
        };
        let after = match change {
            Change::Added | Change::Updated => read_value(&git, &hash, &path).await?,
            Change::Removed => None,
        };
        shown.push(file_change(
            &path,
            change,
            before.as_ref(),
            after.as_ref(),
            (before_collection.as_ref(), after_collection.as_ref()),
        ));
    }
    Ok(CommitDetail {
        commit: hash,
        parents,
        author: author.to_owned(),
        email: email.to_owned(),
        time: time.trim().parse().unwrap_or_default(),
        message: message.trim().to_owned(),
        hidden: count(total.saturating_sub(shown.len())),
        files: shown,
    })
}

/// Every request of the collection with how many commits changed it and who changed it last.
pub async fn request_activity(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
) -> Result<Vec<RequestActivity>, AppError> {
    let (git, _) = linked(db, state, collection_id)?;
    let format = format!("--format={RECORD}%an{FIELD}%at");
    let text = git
        .run(&[
            "log",
            &format,
            "--name-only",
            "--no-renames",
            "-n",
            MAX_LOGGED_COMMITS,
            "HEAD",
            "--",
            "requests/",
        ])
        .await
        .unwrap_or_default();
    let activity = parse_activity(&text);
    let detail = db.with(|conn| collections::collection_detail(conn, collection_id))?;
    Ok(detail
        .requests
        .into_iter()
        .map(|request| {
            let seen = activity.get(&request.id);
            RequestActivity {
                versions: seen.map_or(0, |seen| seen.versions),
                last_author: seen.map(|seen| seen.author.clone()),
                last_time: seen.map(|seen| seen.time),
                id: request.id,
                name: request.name,
                method: request.method,
                kind: request.kind,
            }
        })
        .collect())
}

fn linked(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
) -> Result<(Git, TeamLink), AppError> {
    check_id(collection_id)?;
    let link = db
        .with(|conn| repo::link(conn, collection_id))?
        .ok_or(AppError::NotFound("Shared collection"))?;
    Ok((Git::new(state.dir(collection_id)), link))
}

/// Commits here that the remote's branch does not have, as of the last fetch.
async fn unpushed(git: &Git, branch: &str) -> HashSet<String> {
    let range = format!("origin/{branch}..HEAD");
    git.run(&["rev-list", &range])
        .await
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn parse_summary(line: &str, unpushed: &HashSet<String>) -> Option<CommitSummary> {
    let mut parts = line.split(FIELD);
    let commit = parts.next()?.trim().to_owned();
    check_commit(&commit).ok()?;
    let parents = parts.next()?.split_whitespace().count();
    let author = parts.next()?.to_owned();
    let email = parts.next()?.to_owned();
    let time = parts.next()?.trim().parse().ok()?;
    let message = parts.next().unwrap_or_default().to_owned();
    Some(CommitSummary {
        pushed: !unpushed.contains(&commit),
        commit,
        author,
        email,
        time,
        message,
        merge: parents > 1,
    })
}

/// `--name-status -z` output: a status letter, then the path, each ended by NUL. Paths
/// Codestamp does not write (a README, say) are left out.
fn parse_name_status(text: &str) -> Vec<(Change, String)> {
    let mut parts = text.split('\0').filter(|part| !part.is_empty());
    let mut changed = Vec::new();
    while let (Some(status), Some(path)) = (parts.next(), parts.next()) {
        let change = match status.chars().next() {
            Some('A') => Change::Added,
            Some('D') => Change::Removed,
            _ => Change::Updated,
        };
        if files::is_known_path(path) && path != MANIFEST {
            changed.push((change, path.to_owned()));
        }
    }
    changed
}

async fn read_value(git: &Git, rev: &str, path: &str) -> Result<Option<Value>, AppError> {
    match git.show(rev, path).await? {
        Some(text) => files::parse_text(path, &text).map(Some),
        None => Ok(None),
    }
}

fn file_change(
    path: &str,
    change: Change,
    before: Option<&Value>,
    after: Option<&Value>,
    collections: (Option<&Value>, Option<&Value>),
) -> FileChange {
    let request_id = request_id_of(path).map(str::to_owned);
    let kind = if request_id.is_some() {
        FileKind::Request
    } else if environment_id_of(path).is_some() {
        FileKind::Environment
    } else if access::group_id_of(path).is_some() {
        FileKind::AccessGroup
    } else if access::request_of(path).is_some() {
        FileKind::AccessRequest
    } else {
        FileKind::Collection
    };
    let name = after
        .or(before)
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let fields = match (change, before, after) {
        (Change::Updated, Some(before), Some(after)) if kind == FileKind::AccessGroup => {
            let members = (members_of(Some(before)), members_of(Some(after)));
            let mut fields = Vec::new();
            if members.0 != members.1 {
                fields.push(FieldChange {
                    label: "Members".to_owned(),
                    then: members.0,
                    now: members.1,
                });
            }
            if before.get("keyVersion") != after.get("keyVersion") {
                fields.push(FieldChange {
                    label: "Key".to_owned(),
                    then: format!("Version {}", display(before.get("keyVersion"))),
                    now: format!("Version {}", display(after.get("keyVersion"))),
                });
            }
            fields
        }
        (Change::Updated, Some(before), Some(after)) if kind == FileKind::Request => {
            request_fields()
                .filter(|(key, _)| before.get(key) != after.get(key))
                .map(|(key, label)| FieldChange {
                    label: label.to_owned(),
                    then: shown(key, before.get(key), collections.0),
                    now: shown(key, after.get(key), collections.1),
                })
                .collect()
        }
        (Change::Updated, Some(before), Some(after)) => (if kind == FileKind::Environment {
            &ENVIRONMENT_FIELDS[..]
        } else {
            &COLLECTION_FIELDS[..]
        })
        .iter()
        .filter(|(key, _)| before.get(key) != after.get(key))
        .map(|(key, label)| FieldChange {
            label: (*label).to_owned(),
            then: collection_value(key, before.get(key)),
            now: collection_value(key, after.get(key)),
        })
        .collect(),
        _ => Vec::new(),
    };
    FileChange {
        kind,
        request_id,
        name,
        change,
        fields,
    }
}

/// A request field's value, with its section shown by name as the collection had it then.
fn shown(key: &str, value: Option<&Value>, collection: Option<&Value>) -> String {
    if key != "sectionId" {
        return display(value);
    }
    let id = value.and_then(Value::as_str);
    let name = collection
        .and_then(|collection| collection.get("sections"))
        .and_then(Value::as_array)
        .and_then(|sections| {
            sections
                .iter()
                .find(|section| section.get("id").and_then(Value::as_str) == id)
        })
        .and_then(|section| section.get("name"))
        .and_then(Value::as_str);
    name.unwrap_or("(no section)").to_owned()
}

/// Sections by name and the request order by size; their ids mean nothing to people.
fn collection_value(key: &str, value: Option<&Value>) -> String {
    let rows = value.and_then(Value::as_array);
    match (key, rows) {
        ("sections", Some(sections)) => {
            let names: Vec<&str> = sections
                .iter()
                .filter_map(|section| section.get("name").and_then(Value::as_str))
                .collect();
            if names.is_empty() {
                "(none)".to_owned()
            } else {
                names.join("\n")
            }
        }
        ("order", Some(order)) if order.len() == 1 => "1 request".to_owned(),
        ("order", Some(order)) => format!("{} requests", order.len()),
        _ => display(value),
    }
}

struct Seen {
    versions: u32,
    author: String,
    time: i64,
}

/// `git log` records (newest first) of an author, a time and the request files they touched.
fn parse_activity(text: &str) -> HashMap<String, Seen> {
    let mut seen: HashMap<String, Seen> = HashMap::new();
    for record in text
        .split(RECORD)
        .filter(|record| !record.trim().is_empty())
    {
        let mut lines = record.lines();
        let Some((author, time)) = lines.next().and_then(|head| head.split_once(FIELD)) else {
            continue;
        };
        let time: i64 = time.trim().parse().unwrap_or_default();
        for id in lines.filter_map(|line| request_id_of(line.trim())) {
            seen.entry(id.to_owned())
                .and_modify(|entry| entry.versions += 1)
                .or_insert_with(|| Seen {
                    versions: 1,
                    author: author.to_owned(),
                    time,
                });
        }
    }
    seen
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn reads_name_status_and_skips_unknown_files() {
        let id = "0b5c2c1e-0000-4000-8000-000000000001";
        let text =
            format!("M\0collection.json\0A\0requests/{id}.json\0M\0codestamp.json\0D\0README.md\0");
        let changed = parse_name_status(&text);
        assert_eq!(changed.len(), 2);
        assert_eq!(changed[0], (Change::Updated, COLLECTION.to_owned()));
        assert_eq!(changed[1].0, Change::Added);
    }

    #[test]
    fn counts_versions_and_keeps_the_latest_author() {
        let a = "0b5c2c1e-0000-4000-8000-00000000000a";
        let b = "0b5c2c1e-0000-4000-8000-00000000000b";
        let text = format!(
            "\u{1e}Budi\u{1f}200\n\nrequests/{a}.json\n\u{1e}Ana\u{1f}100\n\nrequests/{a}.json\nrequests/{b}.json\n"
        );
        let seen = parse_activity(&text);
        assert_eq!(seen[a].versions, 2);
        assert_eq!(seen[a].author, "Budi");
        assert_eq!(seen[a].time, 200);
        assert_eq!(seen[b].author, "Ana");
    }

    #[test]
    fn shows_what_changed_in_a_request_and_in_the_collection() {
        let before_collection = json!({ "sections": [{ "id": "s1", "name": "Auth" }] });
        let after_collection = json!({ "sections": [{ "id": "s1", "name": "Auth" }, { "id": "s2", "name": "Admin" }] });
        let before =
            json!({ "name": "Login", "url": "/v1/login", "sectionId": null, "method": "POST" });
        let after =
            json!({ "name": "Login", "url": "/v2/login", "sectionId": "s2", "method": "POST" });
        let change = file_change(
            "requests/0b5c2c1e-0000-4000-8000-000000000001.json",
            Change::Updated,
            Some(&before),
            Some(&after),
            (Some(&before_collection), Some(&after_collection)),
        );
        assert_eq!(change.name, "Login");
        let fields: Vec<_> = change
            .fields
            .iter()
            .map(|f| (f.label.as_str(), f.then.as_str(), f.now.as_str()))
            .collect();
        assert_eq!(
            fields,
            [
                ("URL", "/v1/login", "/v2/login"),
                ("Section", "(no section)", "Admin")
            ]
        );

        let collection = file_change(
            COLLECTION,
            Change::Updated,
            Some(&json!({ "name": "Shop", "order": ["a"], "sections": [] })),
            Some(
                &json!({ "name": "Shop", "order": ["a", "b"], "sections": [{ "id": "s", "name": "Admin" }] }),
            ),
            (None, None),
        );
        assert!(collection.request_id.is_none());
        let fields: Vec<_> = collection
            .fields
            .iter()
            .map(|f| (f.label.as_str(), f.then.as_str(), f.now.as_str()))
            .collect();
        assert_eq!(
            fields,
            [
                ("Sections", "(none)", "Admin"),
                ("Request order", "1 request", "2 requests")
            ]
        );
    }
}
