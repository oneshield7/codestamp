//! Settings › Git accounts: the accounts shared collections sign in with. Codestamp keeps the host
//! and username only; signing in, and the password or token, stay with Git's credential manager.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension, params};

use super::git::{self, Git, GitError, check_account_name, check_host, host_of};
use super::models::{AccountCheck, AccountUse, CheckStatus, GitAccountView, NewAccount};
use super::repo::{self, TeamLink};
use super::sync::TeamState;
use crate::error::AppError;
use crate::storage::db::{Database, new_id, now_ms};
use crate::storage::repo as collections;
use crate::storage::validate::check_id;

/// The host whose accounts the credential manager can list, sign in and sign out by name.
const GITHUB: &str = "github.com";
const MAX_LABEL_CHARS: usize = 60;

/// Host, then username; `None` is Git's default account for the host and sorts first.
type Key = (String, Option<String>);

struct Saved {
    id: String,
    host: String,
    username: String,
    label: String,
}

fn saved(conn: &Connection) -> Result<Vec<Saved>, AppError> {
    let mut statement =
        conn.prepare("SELECT id, host, username, label FROM git_accounts ORDER BY created_at")?;
    let rows = statement.query_map([], |row| {
        Ok(Saved {
            id: row.get(0)?,
            host: row.get(1)?,
            username: row.get(2)?,
            label: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The account of every collection shared over https, with the collection's name.
fn uses(conn: &Connection) -> Result<Vec<(Key, AccountUse)>, AppError> {
    let mut found = Vec::new();
    for link in repo::links(conn)? {
        let Some(host) = host_of(&link.remote) else {
            continue;
        };
        let name = collections::collection_summary(conn, &link.collection_id)?.name;
        found.push((
            (host, link.username),
            AccountUse {
                id: link.collection_id,
                name,
            },
        ));
    }
    Ok(found)
}

/// Adds the account a collection now signs in with, so it can be picked for the next one.
pub fn remember(conn: &Connection, remote: &str, username: Option<&str>) -> Result<(), AppError> {
    let (Some(host), Some(username)) = (host_of(remote), username) else {
        return Ok(());
    };
    conn.execute(
        "INSERT OR IGNORE INTO git_accounts (id, host, username, label, created_at)
         VALUES (?1, ?2, ?3, '', ?4)",
        params![new_id(), host, username, now_ms()],
    )?;
    Ok(())
}

pub async fn list(db: &Database, state: &TeamState) -> Result<Vec<GitAccountView>, AppError> {
    let (saved, uses) = db.with(|conn| Ok((saved(conn)?, uses(conn)?)))?;
    let mut rows: BTreeMap<Key, GitAccountView> = BTreeMap::new();
    for account in saved {
        let key = (account.host, Some(account.username));
        rows.insert(key.clone(), row(key, Some(account.id), account.label));
    }
    for (key, used) in uses {
        rows.entry(key.clone())
            .or_insert_with(|| row(key, None, String::new()))
            .collections
            .push(used);
    }
    let asks_github = rows
        .keys()
        .any(|(host, name)| host == GITHUB && name.is_some());
    if asks_github && let Some(signed_in) = git::github_accounts(&root(state)?).await {
        for row in rows.values_mut() {
            if let (GITHUB, Some(name)) = (row.host.as_str(), &row.username) {
                row.signed_in = Some(signed_in.iter().any(|s| s.eq_ignore_ascii_case(name)));
            }
        }
    }
    Ok(rows.into_values().collect())
}

fn row((host, username): Key, id: Option<String>, label: String) -> GitAccountView {
    GitAccountView {
        id,
        host,
        username,
        label,
        collections: Vec::new(),
        signed_in: None,
    }
}

pub fn add(db: &Database, account: &NewAccount) -> Result<(), AppError> {
    let host = check_host(&account.host)?;
    let username = check_account_name(&account.username)?;
    let label = check_label(&account.label)?;
    db.with(|conn| {
        let exists = conn
            .query_row(
                "SELECT 1 FROM git_accounts WHERE host = ?1 AND username = ?2",
                params![host, username],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if exists {
            return Err(AppError::invalid("This account is already in the list"));
        }
        conn.execute(
            "INSERT INTO git_accounts (id, host, username, label, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![new_id(), host, username, label, now_ms()],
        )?;
        Ok(())
    })
}

/// Takes an account off the list; one a collection still signs in with stays.
pub fn remove(db: &Database, id: &str) -> Result<(), AppError> {
    check_id(id)?;
    db.with(|conn| {
        let account = saved(conn)?
            .into_iter()
            .find(|account| account.id == id)
            .ok_or(AppError::NotFound("Git account"))?;
        let used = uses(conn)?
            .iter()
            .filter(|((host, name), _)| {
                *host == account.host && name.as_deref() == Some(account.username.as_str())
            })
            .count();
        if used > 0 {
            return Err(AppError::invalid(
                "Shared collections still sign in with this account. Give them another one first",
            ));
        }
        conn.execute("DELETE FROM git_accounts WHERE id = ?1", params![id])?;
        Ok(())
    })
}

/// Asks the host whether the account can reach a repository it is used for. With no stored
/// sign-in, Git's credential manager opens its sign-in window.
pub async fn check(
    db: &Database,
    state: &TeamState,
    host: &str,
    username: Option<&str>,
) -> Result<AccountCheck, AppError> {
    let host = check_host(host)?;
    let username = username.map(check_account_name).transpose()?;
    let link = db.with(|conn| {
        Ok(repo::links(conn)?.into_iter().find(|link: &TeamLink| {
            host_of(&link.remote).as_deref() == Some(host.as_str()) && link.username == username
        }))
    })?;
    if let Some(link) = link {
        return Ok(outcome(reach(state, &link).await?));
    }
    match username {
        Some(name) if host == GITHUB => github_status(state, &name).await,
        _ => Err(AppError::invalid(
            "No shared collection uses this account yet. Git asks you to sign in when you share or join one with it",
        )),
    }
}

/// GitHub accounts sign in through the credential manager itself; other hosts sign in when a
/// repository asks, which is what `check` does.
pub async fn sign_in(
    db: &Database,
    state: &TeamState,
    host: &str,
    username: Option<&str>,
) -> Result<AccountCheck, AppError> {
    let host = check_host(host)?;
    let username = username.map(check_account_name).transpose()?;
    match username {
        Some(name) if host == GITHUB => {
            if let Err(error) = git::github_login(&root(state)?, &name).await {
                return Ok(outcome(Err(error)));
            }
            github_status(state, &name).await
        }
        name => check(db, state, &host, name.as_deref()).await,
    }
}

/// Signs in for one shared collection with the account it uses. Asking the repository opens the
/// credential manager's window when no sign-in is stored.
pub async fn sign_in_collection(
    db: &Database,
    state: &TeamState,
    collection_id: &str,
) -> Result<AccountCheck, AppError> {
    check_id(collection_id)?;
    let link = db
        .with(|conn| repo::link(conn, collection_id))?
        .ok_or(AppError::NotFound("Shared collection"))?;
    let Some(host) = host_of(&link.remote) else {
        return Err(AppError::invalid(
            "This repository signs in with your SSH key, so there is nothing to sign in to here",
        ));
    };
    if let (GITHUB, Some(name)) = (host.as_str(), link.username.as_deref()) {
        if let Err(error) = git::github_login(&root(state)?, name).await {
            return Ok(outcome(Err(error)));
        }
        return github_status(state, name).await;
    }
    Ok(outcome(reach(state, &link).await?))
}

/// Asks the repository for its branches from the clones' folder, so a broken clone does not
/// matter. The outer error is local, the inner one Git's.
async fn reach(state: &TeamState, link: &TeamLink) -> Result<Result<String, GitError>, AppError> {
    Ok(Git::signed_in_as(root(state)?, link.username.as_deref())
        .network(&["ls-remote", "--heads", "--", &link.remote])
        .await)
}

/// Removes the account's stored sign-in from Git's credential manager. Collections that use it
/// ask to sign in again on their next sync.
pub async fn sign_out(state: &TeamState, host: &str, username: &str) -> Result<(), AppError> {
    let host = check_host(host)?;
    let username = check_account_name(username)?;
    let dir = root(state)?;
    let result = if host == GITHUB {
        git::github_logout(&dir, &username).await
    } else {
        git::forget_credential(&dir, &host, &username).await
    };
    result.map_err(AppError::from)
}

async fn github_status(state: &TeamState, username: &str) -> Result<AccountCheck, AppError> {
    let Some(signed_in) = git::github_accounts(&root(state)?).await else {
        return Ok(AccountCheck {
            status: CheckStatus::Failed,
            message: Some("Git's credential manager did not list its GitHub accounts".to_owned()),
        });
    };
    let found = signed_in
        .iter()
        .any(|name| name.eq_ignore_ascii_case(username));
    Ok(AccountCheck {
        status: if found {
            CheckStatus::SignedIn
        } else {
            CheckStatus::SignInNeeded
        },
        message: None,
    })
}

fn outcome(result: Result<String, GitError>) -> AccountCheck {
    let status = match &result {
        Ok(_) | Err(GitError::ReadOnly) => CheckStatus::SignedIn,
        Err(GitError::SignIn | GitError::NotFound) => CheckStatus::SignInNeeded,
        Err(GitError::Offline | GitError::Timeout) => CheckStatus::Offline,
        Err(GitError::Missing | GitError::Rejected | GitError::Failed(_)) => CheckStatus::Failed,
    };
    AccountCheck {
        status,
        message: result.err().map(|error| AppError::from(error).to_string()),
    }
}

/// Git commands for accounts run in the clones' folder, which may not exist yet.
fn root(state: &TeamState) -> Result<PathBuf, AppError> {
    fs::create_dir_all(state.root()).map_err(|_| AppError::FileAccess)?;
    Ok(state.root().to_path_buf())
}

fn check_label(raw: &str) -> Result<String, AppError> {
    let label = raw.trim();
    if label.chars().count() > MAX_LABEL_CHARS || label.chars().any(char::is_control) {
        return Err(AppError::invalid("Keep the label under 60 characters"));
    }
    Ok(label.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repo::insert_collection;

    fn link(id: &str, remote: &str, username: Option<&str>) -> TeamLink {
        TeamLink {
            synced_commit: None,
            collection_id: id.to_owned(),
            remote: remote.to_owned(),
            branch: "main".to_owned(),
            synced_at: None,
            username: username.map(str::to_owned),
            author: None,
        }
    }

    #[tokio::test]
    async fn lists_saved_and_used_accounts_and_keeps_used_ones() {
        let db = Database::in_memory().expect("database");
        let state = TeamState::new(
            std::env::temp_dir().join(format!("cs-accounts-{}", new_id())),
            crate::team::access::tests::device(1),
        );
        let remote = "https://gitlab.acme.test/team/shop.git";
        let shop = db
            .with(|conn| {
                let shop = insert_collection(conn, "Shop API")?;
                let users = insert_collection(conn, "Users API")?;
                repo::insert_link(conn, &link(&shop.id, remote, Some("ana-work")))?;
                remember(conn, remote, Some("ana-work"))?;
                repo::insert_link(conn, &link(&users.id, remote, None))?;
                remember(conn, "git@gitlab.acme.test:team/x.git", Some("ignored"))?;
                Ok(shop)
            })
            .expect("links");
        add(
            &db,
            &NewAccount {
                host: "GitLab.acme.test".to_owned(),
                username: "ana-home".to_owned(),
                label: " Home ".to_owned(),
            },
        )
        .expect("add");
        let duplicate = NewAccount {
            host: "gitlab.acme.test".to_owned(),
            username: "ana-home".to_owned(),
            label: String::new(),
        };
        assert!(add(&db, &duplicate).is_err());
        let bad_label = NewAccount {
            label: "x".repeat(61),
            ..duplicate
        };
        assert!(add(&db, &bad_label).is_err());

        let rows = list(&db, &state).await.expect("list");
        let shown: Vec<_> = rows
            .iter()
            .map(|row| {
                (
                    row.username.clone(),
                    row.label.clone(),
                    row.collections.len(),
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                (None, String::new(), 1),
                (Some("ana-home".to_owned()), "Home".to_owned(), 0),
                (Some("ana-work".to_owned()), String::new(), 1),
            ]
        );
        assert_eq!(rows[2].collections[0].id, shop.id);

        let used = rows[2].id.clone().expect("remembered");
        assert!(remove(&db, &used).is_err());
        let unused = rows[1].id.clone().expect("saved");
        remove(&db, &unused).expect("remove");
        assert_eq!(list(&db, &state).await.expect("list").len(), 2);
        assert!(
            check(&db, &state, "gitlab.acme.test", Some("nobody"))
                .await
                .is_err()
        );
    }

    #[test]
    fn maps_git_failures_to_a_check() {
        assert_eq!(outcome(Ok(String::new())).status, CheckStatus::SignedIn);
        assert_eq!(
            outcome(Err(GitError::SignIn)).status,
            CheckStatus::SignInNeeded
        );
        assert_eq!(outcome(Err(GitError::Timeout)).status, CheckStatus::Offline);
        assert!(
            outcome(Err(GitError::Failed("x".to_owned())))
                .message
                .is_some()
        );
    }
}
