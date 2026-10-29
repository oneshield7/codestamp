use std::collections::BTreeSet;

use rusqlite::Connection;

use super::access::ValuePlace;
use super::files::{RequestFile, Snapshot, VariableEntry};
use super::groups::{self, COLLECTION_SET, Keyring};
use crate::environment::models::VariableInput;
use crate::environment::repo as environments;
use crate::environment::validate::check_variables;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::descriptions::{self, Described};
use crate::storage::examples;
use crate::storage::models::RequestGroup;
use crate::storage::repo as collections;

/// Brings the local collection in line with a snapshot from the repository, in one transaction.
/// Only requests in `changed` are rewritten (plus new ones), so an edit made on this device while
/// the sync ran is not overwritten by an identical copy. `None` rewrites every request.
pub fn apply(
    conn: &Connection,
    snapshot: &Snapshot,
    changed: Option<&BTreeSet<String>>,
) -> Result<(), AppError> {
    let collection = &snapshot.collection;
    let tx = conn.unchecked_transaction()?;
    let (existing_requests, current_icon): (BTreeSet<String>, Option<String>) =
        if collections::has_collection(&tx, &collection.id)? {
            let detail = collections::collection_detail(&tx, &collection.id)?;
            if detail.name != collection.name {
                collections::rename_collection(&tx, &collection.id, &collection.name)?;
            }
            for section in detail.sections.iter().filter(|section| {
                !collection
                    .sections
                    .iter()
                    .any(|entry| entry.id == section.id)
            }) {
                collections::delete_section(&tx, &section.id)?;
            }
            let icon = detail.icon_custom.then_some(detail.icon);
            let requests = detail
                .requests
                .into_iter()
                .map(|request| request.id)
                .collect();
            (requests, icon)
        } else {
            collections::insert_collection_with_id(&tx, &collection.id, &collection.name)?;
            (BTreeSet::new(), None)
        };
    if current_icon != collection.icon {
        collections::set_collection_icon(&tx, &collection.id, collection.icon.as_deref())?;
    }
    collections::set_collection_auth(&tx, &collection.id, &collection.auth)?;
    descriptions::set(
        &tx,
        Described::Collection,
        &collection.id,
        &collection.description,
    )?;
    for section in &collection.sections {
        collections::upsert_section(&tx, &collection.id, &section.id, &section.name)?;
        descriptions::set(&tx, Described::Section, &section.id, &section.description)?;
    }

    for (id, request) in &snapshot.requests {
        let wanted = changed.is_none_or(|ids| ids.contains(id)) || !existing_requests.contains(id);
        if wanted {
            write_request(&tx, &collection.id, request)?;
        }
    }
    for id in existing_requests
        .iter()
        .filter(|id| !snapshot.requests.contains_key(*id))
    {
        collections::delete_request(&tx, id)?;
    }

    let sections: Vec<String> = collection.sections.iter().map(|s| s.id.clone()).collect();
    collections::write_order(&tx, &collection.id, &sections, &groups(snapshot))?;
    tx.commit()?;
    Ok(())
}

fn write_request(
    conn: &Connection,
    collection_id: &str,
    request: &RequestFile,
) -> Result<(), AppError> {
    let local = match collections::load_request(conn, &request.id) {
        Ok(saved) if saved.collection_id == collection_id => Some(saved.content),
        Ok(_) | Err(AppError::NotFound(_)) => None,
        Err(error) => return Err(error),
    };
    collections::upsert_request(
        conn,
        collection_id,
        &request.id,
        request.section_id.as_deref(),
        request.to_content(local.as_ref()),
    )?;
    descriptions::set(conn, Described::Request, &request.id, &request.description)?;
    let shared: Vec<_> = request
        .examples
        .iter()
        .map(|example| example.to_example(&request.id))
        .collect();
    examples::replace(conn, &request.id, &shared)
}

/// Requests outside sections first, then each section's, all in the snapshot's order. Requests
/// the order list does not mention go last, so nothing can be dropped.
pub(crate) fn groups(snapshot: &Snapshot) -> Vec<RequestGroup> {
    let mut ordered: Vec<&String> = snapshot
        .collection
        .order
        .iter()
        .filter(|id| snapshot.requests.contains_key(*id))
        .collect();
    let mut seen: BTreeSet<&String> = ordered.iter().copied().collect();
    for id in snapshot.requests.keys() {
        if seen.insert(id) {
            ordered.push(id);
        }
    }
    ordered.dedup();
    let in_section = |section: Option<&str>| -> Vec<String> {
        ordered
            .iter()
            .filter(|id| {
                snapshot
                    .requests
                    .get(**id)
                    .is_some_and(|request| request.section_id.as_deref() == section)
            })
            .map(|id| (*id).clone())
            .collect()
    };
    let mut groups = vec![RequestGroup {
        section_id: None,
        request_ids: in_section(None),
    }];
    for section in &snapshot.collection.sections {
        groups.push(RequestGroup {
            section_id: Some(section.id.clone()),
            request_ids: in_section(Some(&section.id)),
        });
    }
    groups
}

/// The collection's variables and the environments that belong to it, as the repository has
/// them. Secret values are kept as each device stored them, matched by variable name (each
/// member fills in their own; new secrets start empty). Restricted values are opened with the
/// group keys this device holds, so access groups must be taken in first.
pub fn apply_variables(
    conn: &Connection,
    vault: &Vault,
    snapshot: &Snapshot,
) -> Result<(), AppError> {
    let collection_id = &snapshot.collection.id;
    let keyring = Keyring::load(conn, vault, collection_id)?;
    let target = Target {
        conn,
        vault,
        keyring: &keyring,
        collection_id,
    };
    let current = environments::find_collection_set(conn, collection_id)?;
    let set = match current {
        Some(set) => set,
        None if snapshot.collection.variables.is_empty() => String::new(),
        None => environments::collection_set_id(conn, collection_id)?,
    };
    if !set.is_empty() {
        target.put(&set, COLLECTION_SET, &snapshot.collection.variables)?;
    }
    for (id, _) in environments::owned(conn, collection_id)? {
        if !snapshot.environments.contains_key(&id) {
            environments::delete_owned(conn, &id, collection_id)?;
        }
    }
    for environment in snapshot.environments.values() {
        environments::put_owned(conn, &environment.id, &environment.name, collection_id)?;
        target.put(&environment.id, &environment.id, &environment.variables)?;
    }
    Ok(())
}

struct Target<'a> {
    conn: &'a Connection,
    vault: &'a Vault,
    keyring: &'a Keyring,
    collection_id: &'a str,
}

impl Target<'_> {
    /// Replaces a set's variables with the shared ones, unless they already match. Restricted
    /// values are always written again: whether this device can open them may have changed.
    fn put(&self, set_id: &str, set: &str, shared: &[VariableEntry]) -> Result<(), AppError> {
        let current = environments::variables(self.conn, set_id)?;
        let same = current.len() == shared.len()
            && current.iter().zip(shared).all(|(own, shared)| {
                own.key == shared.key
                    && own.is_secret == shared.secret
                    && own.enabled == shared.enabled
                    && shared.restricted.is_none()
                    && own.restricted.is_none()
                    && (own.is_secret || own.value == shared.value)
            });
        if same {
            return Ok(());
        }
        let inputs = shared
            .iter()
            .map(|shared| {
                let own = current.iter().find(|variable| variable.key == shared.key);
                let value = match (&shared.restricted, shared.secret, own) {
                    (Some(group_id), _, _) => self.keyring.value_for(
                        shared,
                        ValuePlace {
                            collection_id: self.collection_id,
                            set,
                            key: &shared.key,
                            group_id,
                        },
                        own.is_some_and(|own| own.restricted.as_ref() == Some(group_id)),
                    ),
                    (None, true, Some(_)) => None,
                    (None, true, None) => Some(String::new()),
                    (None, false, _) => Some(shared.value.clone()),
                };
                VariableInput {
                    id: own.map(|variable| variable.id.clone()),
                    key: shared.key.clone(),
                    value,
                    is_secret: shared.secret,
                    enabled: shared.enabled,
                    restricted: shared.restricted.clone(),
                }
            })
            .collect();
        environments::replace_variables(self.conn, self.vault, set_id, check_variables(inputs)?)?;
        groups::store_sealed(self.conn, set_id, shared)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::auth::models::Auth;
    use crate::environment::vault::FixedKey;
    use crate::storage::db::{Database, new_id};
    use crate::storage::models::{FormField, FormFieldKind, RequestContent, RequestKind};
    use crate::team::files::{CollectionFile, SectionEntry, VariableEntry, from_db};

    fn request(id: &str, name: &str, section: Option<&str>) -> RequestFile {
        RequestFile {
            id: id.to_owned(),
            name: name.to_owned(),
            kind: RequestKind::Http,
            method: "GET".to_owned(),
            url: "{{baseUrl}}/x".to_owned(),
            section_id: section.map(str::to_owned),
            params: Vec::new(),
            headers: Vec::new(),
            body_mode: "formdata".to_owned(),
            body: String::new(),
            form: vec![FormField {
                enabled: true,
                key: "doc".to_owned(),
                value: String::new(),
                kind: FormFieldKind::File,
            }],
            pre_request_script: String::new(),
            test_script: String::new(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
            examples: Vec::new(),
            description: format!("About {name}"),
        }
    }

    #[test]
    fn creates_updates_and_removes_to_match_the_snapshot() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([4; 32])));
        let (collection_id, section_id, first, second) = (new_id(), new_id(), new_id(), new_id());
        let mut snapshot = Snapshot {
            collection: CollectionFile {
                id: collection_id.clone(),
                name: "Shared".to_owned(),
                auth: Auth::None,
                icon: None,
                description: "# Shared API".to_owned(),
                sections: vec![SectionEntry {
                    id: section_id.clone(),
                    name: "Users".to_owned(),
                    description: "User endpoints".to_owned(),
                }],
                order: vec![second.clone(), first.clone()],
                variables: vec![
                    VariableEntry {
                        key: "baseUrl".to_owned(),
                        value: "https://api.test".to_owned(),
                        secret: false,
                        enabled: true,
                        restricted: None,
                        sealed: None,
                    },
                    VariableEntry {
                        key: "token".to_owned(),
                        value: String::new(),
                        secret: true,
                        enabled: true,
                        restricted: None,
                        sealed: None,
                    },
                ],
            },
            requests: [
                (first.clone(), request(&first, "First", None)),
                (
                    second.clone(),
                    request(&second, "Second", Some(&section_id)),
                ),
            ]
            .into_iter()
            .collect(),
            environments: BTreeMap::new(),
            groups: BTreeMap::new(),
            access_requests: BTreeMap::new(),
        };

        db.with(|conn| {
            apply(conn, &snapshot, None)?;
            apply_variables(conn, &vault, &snapshot)
        })
        .expect("join");
        // This device picks a file and fills in its own secret.
        db.with(|conn| {
            let saved = collections::load_request(conn, &first)?;
            let mut content: RequestContent = saved.content;
            if let Some(field) = content.form.first_mut() {
                field.value = "C:\\mine\\doc.pdf".to_owned();
            }
            collections::update_request(conn, &first, &content)?;
            let set = environments::collection_set_id(conn, &collection_id)?;
            let token = environments::variables(conn, &set)?
                .into_iter()
                .find(|v| v.key == "token")
                .map(|v| v.id);
            environments::replace_variables(
                conn,
                &vault,
                &set,
                vec![
                    VariableInput {
                        id: None,
                        key: "baseUrl".to_owned(),
                        value: Some("https://api.test".to_owned()),
                        is_secret: false,
                        enabled: true,
                        restricted: None,
                    },
                    VariableInput {
                        id: token,
                        key: "token".to_owned(),
                        value: Some("my-own-token".to_owned()),
                        is_secret: true,
                        enabled: true,
                        restricted: None,
                    },
                ],
            )
            .map(|_| ())
        })
        .expect("local edits");

        // A teammate renames the first request and removes the second.
        snapshot.requests.remove(&second);
        if let Some(changed) = snapshot.requests.get_mut(&first) {
            changed.name = "First renamed".to_owned();
            changed.description = "Now with docs".to_owned();
        }
        snapshot.collection.variables[0].value = "https://staging.test".to_owned();
        let changed: BTreeSet<String> = [first.clone()].into_iter().collect();
        let after = db
            .with(|conn| {
                apply(conn, &snapshot, Some(&changed))?;
                apply_variables(conn, &vault, &snapshot)?;
                from_db(conn, &collection_id)
            })
            .expect("update");

        assert_eq!(after.requests.len(), 1);
        assert_eq!(
            after.requests.get(&first).map(|r| r.name.as_str()),
            Some("First renamed")
        );
        let kept_file = db
            .with(|conn| collections::load_request(conn, &first))
            .expect("load")
            .content
            .form
            .first()
            .map(|f| f.value.clone());
        assert_eq!(kept_file.as_deref(), Some("C:\\mine\\doc.pdf"));
        let secret = db
            .with(|conn| {
                let set = environments::collection_set_id(conn, &collection_id)?;
                let id = environments::variables(conn, &set)?
                    .into_iter()
                    .find(|v| v.key == "token")
                    .map(|v| v.id)
                    .ok_or(AppError::Storage)?;
                environments::reveal(conn, &vault, &id)
            })
            .expect("secret");
        assert_eq!(secret, "my-own-token");
        assert_eq!(after.collection.variables[0].value, "https://staging.test");
        assert_eq!(after.collection.description, "# Shared API");
        assert_eq!(after.collection.sections[0].description, "User endpoints");
        assert_eq!(
            after.requests.get(&first).map(|r| r.description.as_str()),
            Some("Now with docs")
        );
    }

    #[test]
    fn refuses_ids_that_belong_to_another_collection() {
        let db = Database::in_memory().expect("database");
        let result = db.with(|conn| {
            let other = collections::insert_collection(conn, "Mine")?;
            let mine = collections::insert_request(
                conn,
                &other.id,
                None,
                request(&new_id(), "Mine", None).to_content(None),
            )?;
            let snapshot = Snapshot {
                collection: CollectionFile {
                    id: new_id(),
                    name: "Hostile".to_owned(),
                    auth: Auth::None,
                    icon: None,
                    description: String::new(),
                    sections: Vec::new(),
                    order: Vec::new(),
                    variables: Vec::new(),
                },
                requests: [(mine.id.clone(), request(&mine.id, "Taken over", None))]
                    .into_iter()
                    .collect(),
                environments: BTreeMap::new(),
                groups: BTreeMap::new(),
                access_requests: BTreeMap::new(),
            };
            apply(conn, &snapshot, None)
        });
        assert!(result.is_err());
        let names = db
            .with(collections::list_collections)
            .expect("list")
            .into_iter()
            .map(|c| c.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["Mine"], "the failed apply left nothing behind");
    }
}
