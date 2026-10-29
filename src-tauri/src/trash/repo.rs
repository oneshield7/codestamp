use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::rows::{LinkKind, Saved, Stored, Table};
use crate::auth::oauth::KEY_PREFIX as TOKEN_KEY_PREFIX;
use crate::error::AppError;
use crate::storage::db::{new_id, now_ms};
use crate::storage::repo as collections;

/// Deleted collections and requests can be restored for this long.
pub const KEEP_DAYS: i64 = 30;
const KEEP_MS: i64 = KEEP_DAYS * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeletedKind {
    Collection,
    Request,
}

impl DeletedKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Collection => "collection",
            Self::Request => "request",
        }
    }

    fn parse(text: &str) -> Result<Self, AppError> {
        match text {
            "collection" => Ok(Self::Collection),
            "request" => Ok(Self::Request),
            _ => Err(AppError::Storage),
        }
    }
}

/// A deleted item as Recently deleted lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedItem {
    pub id: String,
    pub kind: DeletedKind,
    pub name: String,
    /// The collection itself, or the one the request was in.
    pub collection_id: String,
    pub collection_name: String,
    pub request_count: i64,
    pub deleted_at: i64,
    /// For a request: its collection is still here, so it goes back into it.
    pub collection_exists: bool,
}

/// What a restore put back, so the app can open it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    pub kind: DeletedKind,
    pub collection_id: String,
    pub request_id: Option<String>,
}

const IN_COLLECTION_REQUESTS: &str =
    "request_id IN (SELECT id FROM requests WHERE collection_id = ?1)";
const COLLECTION_ENVIRONMENTS: &str = "collection_id = ?1 OR owner_collection_id = ?1";

/// Deletes the collection and everything that belongs to it, keeping a copy to restore.
pub fn delete_collection(conn: &Connection, id: &str) -> Result<DeletedItem, AppError> {
    let tx = conn.unchecked_transaction()?;
    let (name, request_count): (String, i64) = tx
        .query_row(
            "SELECT name, (SELECT COUNT(*) FROM requests WHERE collection_id = ?1)
             FROM collections WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound("Collection"))?;
    let mut saved = Saved::default();
    for (table, filter) in [
        (Table::Collections, "id = ?1"),
        (Table::Sections, "collection_id = ?1"),
        (Table::Requests, "collection_id = ?1"),
        (Table::RequestExamples, IN_COLLECTION_REQUESTS),
        (Table::Environments, COLLECTION_ENVIRONMENTS),
        (
            Table::Variables,
            "environment_id IN (SELECT id FROM environments
             WHERE collection_id = ?1 OR owner_collection_id = ?1)",
        ),
        (Table::CatalogInfo, "collection_id = ?1"),
        (Table::CollectionRuns, "collection_id = ?1"),
        (Table::TeamLinks, "collection_id = ?1"),
        (Table::TeamAccessGroups, "collection_id = ?1"),
        (Table::TeamAccessRequests, "collection_id = ?1"),
    ] {
        saved.keep(&tx, table, filter, &[&id])?;
    }
    saved.keep(
        &tx,
        Table::AppSecrets,
        "key = ?2 || ?1 OR key IN (SELECT ?2 || id FROM requests WHERE collection_id = ?1)",
        &[&id, &TOKEN_KEY_PREFIX],
    )?;
    saved.keep_links(
        &tx,
        LinkKind::FlowStepRequest,
        &format!("SELECT id, request_id FROM flow_steps WHERE {IN_COLLECTION_REQUESTS}"),
        &[&id],
    )?;
    saved.keep_links(
        &tx,
        LinkKind::HistoryRequest,
        &format!("SELECT id, request_id FROM history WHERE {IN_COLLECTION_REQUESTS}"),
        &[&id],
    )?;
    saved.keep_links(
        &tx,
        LinkKind::HistoryCollection,
        "SELECT id, collection_id FROM history WHERE collection_id = ?1",
        &[&id],
    )?;
    let item = DeletedItem {
        id: new_id(),
        kind: DeletedKind::Collection,
        name: name.clone(),
        collection_id: id.to_owned(),
        collection_name: name,
        request_count,
        deleted_at: now_ms(),
        collection_exists: false,
    };
    keep_item(&tx, &item, &saved)?;
    collections::delete_collection(&tx, id)?;
    tx.commit()?;
    Ok(item)
}

/// Deletes one request, keeping a copy to restore.
pub fn delete_request(conn: &Connection, id: &str) -> Result<DeletedItem, AppError> {
    let tx = conn.unchecked_transaction()?;
    let (name, collection_id, collection_name): (String, String, String) = tx
        .query_row(
            "SELECT r.name, c.id, c.name FROM requests r
             JOIN collections c ON c.id = r.collection_id WHERE r.id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound("Request"))?;
    let mut saved = Saved::default();
    saved.keep(&tx, Table::Requests, "id = ?1", &[&id])?;
    saved.keep(&tx, Table::RequestExamples, "request_id = ?1", &[&id])?;
    saved.keep(
        &tx,
        Table::AppSecrets,
        "key = ?2 || ?1",
        &[&id, &TOKEN_KEY_PREFIX],
    )?;
    saved.keep_links(
        &tx,
        LinkKind::FlowStepRequest,
        "SELECT id, request_id FROM flow_steps WHERE request_id = ?1",
        &[&id],
    )?;
    saved.keep_links(
        &tx,
        LinkKind::HistoryRequest,
        "SELECT id, request_id FROM history WHERE request_id = ?1",
        &[&id],
    )?;
    let item = DeletedItem {
        id: new_id(),
        kind: DeletedKind::Request,
        name,
        collection_id,
        collection_name,
        request_count: 1,
        deleted_at: now_ms(),
        collection_exists: true,
    };
    keep_item(&tx, &item, &saved)?;
    collections::delete_request(&tx, id)?;
    tx.commit()?;
    Ok(item)
}

fn keep_item(conn: &Connection, item: &DeletedItem, saved: &Saved) -> Result<(), AppError> {
    let rows = serde_json::to_string(saved).map_err(|_| AppError::Storage)?;
    conn.execute(
        "INSERT INTO deleted_items
             (id, kind, name, collection_id, collection_name, request_count, rows, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            item.id,
            item.kind.as_str(),
            item.name,
            item.collection_id,
            item.collection_name,
            item.request_count,
            rows,
            item.deleted_at
        ],
    )?;
    forget_expired(conn)
}

/// Copies older than the keep period are deleted for good.
pub fn forget_expired(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM deleted_items WHERE deleted_at < ?1",
        params![now_ms() - KEEP_MS],
    )?;
    Ok(())
}

/// Newest first.
pub fn list(conn: &Connection) -> Result<Vec<DeletedItem>, AppError> {
    forget_expired(conn)?;
    let mut statement = conn.prepare(
        "SELECT d.id, d.kind, d.name, d.collection_id, d.collection_name, d.request_count,
                d.deleted_at, EXISTS (SELECT 1 FROM collections c WHERE c.id = d.collection_id)
         FROM deleted_items d ORDER BY d.deleted_at DESC",
    )?;
    let items = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                DeletedItem {
                    id: row.get(0)?,
                    kind: DeletedKind::Collection,
                    name: row.get(2)?,
                    collection_id: row.get(3)?,
                    collection_name: row.get(4)?,
                    request_count: row.get(5)?,
                    deleted_at: row.get(6)?,
                    collection_exists: row.get(7)?,
                },
            ))
        })?
        .map(|row| {
            let (kind, item) = row?;
            Ok(DeletedItem {
                kind: DeletedKind::parse(&kind)?,
                ..item
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(items)
}

/// Puts a deleted item back as it was. A request whose collection is gone goes into a new
/// collection with the old name; one whose section is gone goes outside sections.
pub fn restore(conn: &Connection, id: &str) -> Result<Restored, AppError> {
    let tx = conn.unchecked_transaction()?;
    let (kind, name, collection_id, collection_name, rows): (String, String, String, String, String) = tx
        .query_row(
            "SELECT kind, name, collection_id, collection_name, rows FROM deleted_items WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound("Deleted item"))?;
    let mut saved: Saved = serde_json::from_str(&rows).map_err(|_| AppError::Storage)?;
    let restored = match DeletedKind::parse(&kind)? {
        DeletedKind::Collection => {
            if collections::has_collection(&tx, &collection_id)? {
                return Err(AppError::InvalidInput(format!(
                    "“{name}” is already in Codestamp"
                )));
            }
            saved.restore(&tx)?;
            Restored {
                kind: DeletedKind::Collection,
                collection_id,
                request_id: None,
            }
        }
        DeletedKind::Request => {
            let request_id = saved_id(&saved, Table::Requests).ok_or(AppError::Storage)?;
            if request_exists(&tx, &request_id)? {
                return Err(AppError::InvalidInput(format!(
                    "“{name}” is already in Codestamp"
                )));
            }
            let target = if collections::has_collection(&tx, &collection_id)? {
                collection_id
            } else {
                let created = collections::insert_collection(&tx, &collection_name)?;
                saved.set(
                    Table::Requests,
                    "collection_id",
                    &Stored::Text(created.id.clone()),
                );
                created.id
            };
            if !section_is_in(&tx, &saved, &target)? {
                saved.set(Table::Requests, "section_id", &Stored::Null);
            }
            saved.restore(&tx)?;
            Restored {
                kind: DeletedKind::Request,
                collection_id: target,
                request_id: Some(request_id),
            }
        }
    };
    tx.execute("DELETE FROM deleted_items WHERE id = ?1", params![id])?;
    tx.commit()?;
    Ok(restored)
}

/// Deletes the kept copy for good.
pub fn forget(conn: &Connection, id: &str) -> Result<(), AppError> {
    let changed = conn.execute("DELETE FROM deleted_items WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::NotFound("Deleted item"));
    }
    Ok(())
}

pub fn forget_all(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM deleted_items", [])?;
    Ok(())
}

/// The id of the first saved row of `table`.
fn saved_id(saved: &Saved, table: Table) -> Option<String> {
    saved_text(saved, table, "id")
}

fn saved_text(saved: &Saved, table: Table, column: &str) -> Option<String> {
    let rows = saved.tables.iter().find(|rows| rows.table == table)?;
    let index = rows.columns.iter().position(|name| name == column)?;
    match rows.rows.first()?.get(index)? {
        Stored::Text(text) => Some(text.clone()),
        _ => None,
    }
}

fn request_exists(conn: &Connection, id: &str) -> Result<bool, AppError> {
    Ok(conn
        .query_row("SELECT 1 FROM requests WHERE id = ?1", params![id], |_| {
            Ok(())
        })
        .optional()?
        .is_some())
}

/// Whether the request's section still exists in the collection it goes back to.
fn section_is_in(conn: &Connection, saved: &Saved, collection_id: &str) -> Result<bool, AppError> {
    let Some(section) = saved_text(saved, Table::Requests, "section_id") else {
        return Ok(false);
    };
    Ok(conn
        .query_row(
            "SELECT 1 FROM sections WHERE id = ?1 AND collection_id = ?2",
            params![section, collection_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::models::Auth;
    use crate::environment::models::VariableInput;
    use crate::environment::repo as environments;
    use crate::environment::vault::{FixedKey, Vault};
    use crate::storage::db::Database;
    use crate::storage::examples::{self, ExampleInput};
    use crate::storage::models::{RequestContent, RequestKind};

    fn content(name: &str) -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: name.to_owned(),
            method: "GET".to_owned(),
            url: "{{baseUrl}}/users".to_owned(),
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

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |row| row.get(0)).expect("count")
    }

    #[test]
    fn restores_a_deleted_collection_with_everything_that_belonged_to_it() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([4; 32])));
        db.with(|conn| {
            let collection = collections::insert_collection(conn, "Users API")?;
            let section = collections::insert_section(conn, &collection.id, "Admin")?;
            let request = collections::insert_request(
                conn,
                &collection.id,
                Some(&section.id),
                content("List"),
            )?;
            examples::insert(
                conn,
                &request.id,
                &ExampleInput {
                    name: "OK".to_owned(),
                    status: 200,
                    headers: Vec::new(),
                    body: "[]".to_owned(),
                },
            )?;
            let set = environments::collection_set_id(conn, &collection.id)?;
            environments::replace_variables(
                conn,
                &vault,
                &set,
                vec![VariableInput {
                    id: None,
                    key: "token".to_owned(),
                    value: Some("s3cret".to_owned()),
                    is_secret: true,
                    enabled: true,
                    restricted: None,
                }],
            )?;
            environments::put_owned(conn, &new_id(), "Staging", &collection.id)?;
            conn.execute(
                "INSERT INTO team_links (collection_id, remote, branch, created_at)
                 VALUES (?1, 'https://git.example.com/users.git', 'main', 0)",
                params![collection.id],
            )?;
            conn.execute(
                "INSERT INTO flows (id, name, sort_order, created_at, updated_at)
                 VALUES ('flow', 'Login', 0, 0, 0)",
                [],
            )?;
            conn.execute(
                "INSERT INTO flow_steps (id, flow_id, request_id, sort_order)
                 VALUES ('step', 'flow', ?1, 0)",
                params![request.id],
            )?;
            let secret_of = |conn: &Connection| -> Result<Vec<u8>, AppError> {
                Ok(conn.query_row(
                    "SELECT secret FROM variables WHERE key = 'token'",
                    [],
                    |row| row.get(0),
                )?)
            };
            let secret_before = secret_of(conn)?;
            let detail = |conn: &Connection| -> Result<serde_json::Value, AppError> {
                serde_json::to_value(collections::collection_detail(conn, &collection.id)?)
                    .map_err(|_| AppError::Storage)
            };
            let before = detail(conn)?;

            let deleted = delete_collection(conn, &collection.id)?;
            assert_eq!(deleted.request_count, 1);
            assert!(!collections::has_collection(conn, &collection.id)?);
            assert_eq!(
                count(
                    conn,
                    "SELECT COUNT(*) FROM flow_steps WHERE request_id IS NULL"
                ),
                1
            );
            assert_eq!(list(conn)?.len(), 1);

            let restored = restore(conn, &deleted.id)?;
            assert_eq!(restored.collection_id, collection.id);
            assert_eq!(detail(conn)?, before);
            assert_eq!(examples::all_for(conn, &request.id)?.len(), 1);
            assert_eq!(secret_of(conn)?, secret_before);
            assert_eq!(environments::owned(conn, &collection.id)?.len(), 1);
            assert_eq!(count(conn, "SELECT COUNT(*) FROM team_links"), 1);
            assert_eq!(
                count(
                    conn,
                    "SELECT COUNT(*) FROM flow_steps WHERE request_id IS NOT NULL"
                ),
                1
            );
            assert!(list(conn)?.is_empty());
            Ok(())
        })
        .expect("restore collection");
    }

    #[test]
    fn puts_a_request_back_where_it_can() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let collection = collections::insert_collection(conn, "Shop")?;
            let section = collections::insert_section(conn, &collection.id, "Orders")?;
            let first =
                collections::insert_request(conn, &collection.id, Some(&section.id), content("A"))?;
            let second =
                collections::insert_request(conn, &collection.id, Some(&section.id), content("B"))?;

            let deleted = delete_request(conn, &first.id)?;
            assert!(list(conn)?[0].collection_exists);
            restore(conn, &deleted.id)?;
            let back = collections::load_request(conn, &first.id)?;
            assert_eq!(back.section_id.as_deref(), Some(section.id.as_str()));

            // Its section is gone: it comes back outside sections.
            let deleted = delete_request(conn, &first.id)?;
            collections::delete_section(conn, &section.id)?;
            restore(conn, &deleted.id)?;
            assert_eq!(collections::load_request(conn, &first.id)?.section_id, None);

            // Its collection is gone: it comes back in a new collection with the old name.
            let deleted = delete_request(conn, &second.id)?;
            let gone = delete_collection(conn, &collection.id)?;
            let items = list(conn)?;
            assert_eq!(items.len(), 2);
            assert!(items.iter().any(|item| item.id == gone.id));
            assert!(
                items
                    .iter()
                    .any(|item| item.id == deleted.id && !item.collection_exists)
            );
            let restored = restore(conn, &deleted.id)?;
            assert_ne!(restored.collection_id, collection.id);
            let detail = collections::collection_detail(conn, &restored.collection_id)?;
            assert_eq!(detail.name, "Shop");
            assert_eq!(detail.requests.len(), 1);
            Ok(())
        })
        .expect("restore requests");
    }

    #[test]
    fn forgets_old_copies_and_refuses_changed_ones() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let collection = collections::insert_collection(conn, "Old")?;
            let deleted = delete_collection(conn, &collection.id)?;
            conn.execute(
                "UPDATE deleted_items SET deleted_at = ?1",
                params![now_ms() - KEEP_MS - 1],
            )?;
            assert!(list(conn)?.is_empty());
            assert!(restore(conn, &deleted.id).is_err());

            let collection = collections::insert_collection(conn, "Changed")?;
            collections::insert_request(conn, &collection.id, None, content("Kept"))?;
            let deleted = delete_collection(conn, &collection.id)?;
            let rows: String = conn.query_row(
                "SELECT rows FROM deleted_items WHERE id = ?1",
                params![deleted.id],
                |row| row.get(0),
            )?;
            let injected =
                rows.replacen("\"name\"", "\"name\\\" TEXT); DROP TABLE requests; --\"", 1);
            for tampered in [
                rows.replace("\"collections\"", "\"sqlite_master\""),
                injected,
            ] {
                conn.execute(
                    "UPDATE deleted_items SET rows = ?1 WHERE id = ?2",
                    params![tampered, deleted.id],
                )?;
                assert!(restore(conn, &deleted.id).is_err());
            }
            assert_eq!(
                count(
                    conn,
                    "SELECT COUNT(*) FROM sqlite_master WHERE name = 'requests'"
                ),
                1
            );
            forget(conn, &deleted.id)?;
            assert!(list(conn)?.is_empty());
            Ok(())
        })
        .expect("expiry and checks");
    }
}
