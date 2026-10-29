use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use std::collections::{HashMap, HashSet};

use super::db::{new_id, now_ms};
use super::descriptions::{self, Described};
use super::examples;
use super::icons;
use super::models::{
    CollectionDetail, CollectionSummary, RequestContent, RequestGroup, RequestKind, RequestSummary,
    SavedRequest, SearchEntry, SectionInfo,
};
use crate::auth::models::Auth;
use crate::auth::oauth::KEY_PREFIX as TOKEN_KEY_PREFIX;
use crate::error::AppError;

const MAX_SEARCH_ENTRIES: i64 = 20_000;

pub fn list_collections(conn: &Connection) -> Result<Vec<CollectionSummary>, AppError> {
    let mut statement = conn.prepare(
        "SELECT c.id, c.name, COUNT(r.id), c.icon
         FROM collections c LEFT JOIN requests r ON r.collection_id = c.id
         GROUP BY c.id ORDER BY c.sort_order",
    )?;
    let rows = statement.query_map([], summary_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn collection_summary(conn: &Connection, id: &str) -> Result<CollectionSummary, AppError> {
    conn.query_row(
        "SELECT c.id, c.name, (SELECT COUNT(*) FROM requests r WHERE r.collection_id = c.id), c.icon
         FROM collections c WHERE c.id = ?1",
        params![id],
        summary_from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("Collection"))
}

fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<CollectionSummary> {
    let name: String = row.get(1)?;
    let icon: Option<String> = row.get(3)?;
    Ok(CollectionSummary {
        id: row.get(0)?,
        icon: icons::resolve(&name, icon.as_deref()),
        name,
        request_count: row.get(2)?,
    })
}

pub fn insert_collection(conn: &Connection, name: &str) -> Result<CollectionSummary, AppError> {
    insert_collection_with_id(conn, &new_id(), name)
}

/// A team collection keeps the id it has in its repository on every member's device.
pub fn insert_collection_with_id(
    conn: &Connection,
    id: &str,
    name: &str,
) -> Result<CollectionSummary, AppError> {
    let id = id.to_owned();
    let now = now_ms();
    conn.execute(
        "INSERT INTO collections (id, name, sort_order, created_at, updated_at)
         VALUES (?1, ?2, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM collections), ?3, ?3)",
        params![id, name, now],
    )?;
    Ok(CollectionSummary {
        id,
        name: name.to_owned(),
        request_count: 0,
        icon: icons::suggest(name).emoji.to_owned(),
    })
}

/// `None` goes back to the icon picked from the name.
pub fn set_collection_icon(
    conn: &Connection,
    id: &str,
    icon: Option<&str>,
) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE collections SET icon = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, icon, now_ms()],
    )?;
    expect_changed(changed, "Collection")
}

pub fn rename_collection(conn: &Connection, id: &str, name: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE collections SET name = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, name, now_ms()],
    )?;
    expect_changed(changed, "Collection")
}

pub fn delete_collection(conn: &Connection, id: &str) -> Result<(), AppError> {
    // OAuth tokens of the collection and its requests go with them.
    conn.execute(
        "DELETE FROM app_secrets
         WHERE key = ?1 || ?2 OR key IN (SELECT ?1 || id FROM requests WHERE collection_id = ?2)",
        params![TOKEN_KEY_PREFIX, id],
    )?;
    let changed = conn.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
    expect_changed(changed, "Collection")
}

pub fn collection_detail(conn: &Connection, id: &str) -> Result<CollectionDetail, AppError> {
    let (name, auth, icon): (String, String, Option<String>) = conn
        .query_row(
            "SELECT name, auth, icon FROM collections WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound("Collection"))?;
    let chosen = icon
        .as_deref()
        .and_then(|icon| icons::check_icon(icon).ok());

    let mut section_query =
        conn.prepare("SELECT id, name FROM sections WHERE collection_id = ?1 ORDER BY sort_order")?;
    let sections = section_query
        .query_map(params![id], |row| {
            Ok(SectionInfo {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut request_query = conn.prepare(
        "SELECT id, section_id, name, method, url, kind FROM requests
         WHERE collection_id = ?1 ORDER BY sort_order",
    )?;
    let requests = request_query
        .query_map(params![id], |row| {
            Ok(RequestSummary {
                id: row.get(0)?,
                section_id: row.get(1)?,
                kind: RequestKind::from_stored(&row.get::<_, String>(5)?),
                name: row.get(2)?,
                method: row.get(3)?,
                url: row.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    Ok(CollectionDetail {
        id: id.to_owned(),
        icon: icons::resolve(&name, chosen),
        icon_custom: chosen.is_some(),
        name,
        auth: Auth::from_stored(&auth, Auth::None),
        sections,
        requests,
    })
}

/// Every request in notebook order, for quick search. Capped so a huge database cannot flood
/// the UI.
pub fn search_entries(conn: &Connection) -> Result<Vec<SearchEntry>, AppError> {
    let mut statement = conn.prepare(
        "SELECT r.id, r.collection_id, c.name, c.icon, s.name, r.kind, r.name, r.method, r.url
         FROM requests r
         JOIN collections c ON c.id = r.collection_id
         LEFT JOIN sections s ON s.id = r.section_id
         ORDER BY c.sort_order, COALESCE(s.sort_order, -1), r.sort_order
         LIMIT ?1",
    )?;
    let rows = statement.query_map(params![MAX_SEARCH_ENTRIES], |row| {
        let collection_name: String = row.get(2)?;
        let icon: Option<String> = row.get(3)?;
        Ok(SearchEntry {
            request_id: row.get(0)?,
            collection_id: row.get(1)?,
            collection_icon: icons::resolve(&collection_name, icon.as_deref()),
            collection_name,
            section_name: row.get(4)?,
            kind: RequestKind::from_stored(&row.get::<_, String>(5)?),
            name: row.get(6)?,
            method: row.get(7)?,
            url: row.get(8)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The auth that requests inheriting from this collection use; none for a missing collection.
pub fn collection_auth(conn: &Connection, id: &str) -> Result<Auth, AppError> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT auth FROM collections WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(stored.map_or(Auth::None, |text| Auth::from_stored(&text, Auth::None)))
}

pub fn set_collection_auth(conn: &Connection, id: &str, auth: &Auth) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE collections SET auth = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, auth.to_stored()?, now_ms()],
    )?;
    expect_changed(changed, "Collection")
}

pub fn insert_section(
    conn: &Connection,
    collection_id: &str,
    name: &str,
) -> Result<SectionInfo, AppError> {
    insert_section_with_id(conn, collection_id, &new_id(), name)
}

pub fn insert_section_with_id(
    conn: &Connection,
    collection_id: &str,
    id: &str,
    name: &str,
) -> Result<SectionInfo, AppError> {
    ensure_collection(conn, collection_id)?;
    let id = id.to_owned();
    conn.execute(
        "INSERT INTO sections (id, collection_id, name, sort_order)
         VALUES (?1, ?2, ?3,
                 (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM sections WHERE collection_id = ?2))",
        params![id, collection_id, name],
    )?;
    Ok(SectionInfo {
        id,
        name: name.to_owned(),
    })
}

pub fn rename_section(conn: &Connection, id: &str, name: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE sections SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    expect_changed(changed, "Section")
}

/// Requests in a deleted section stay in the collection, just without a section.
pub fn delete_section(conn: &Connection, id: &str) -> Result<(), AppError> {
    let changed = conn.execute("DELETE FROM sections WHERE id = ?1", params![id])?;
    expect_changed(changed, "Section")
}

/// Adds a request at the end of the collection (and so at the end of its section).
pub fn insert_request(
    conn: &Connection,
    collection_id: &str,
    section_id: Option<&str>,
    content: RequestContent,
) -> Result<SavedRequest, AppError> {
    ensure_collection(conn, collection_id)?;
    if let Some(section) = section_id {
        ensure_section_in_collection(conn, section, collection_id)?;
    }
    let position = next_request_position(conn, collection_id)?;
    write_request(conn, collection_id, new_id(), section_id, content, position)
}

fn next_request_position(conn: &Connection, collection_id: &str) -> Result<i64, AppError> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM requests WHERE collection_id = ?1",
        params![collection_id],
        |row| row.get(0),
    )?)
}

/// Writes a request with a known id (from a team repository): inserts it, or replaces the row
/// with that id. A row with that id in another collection is refused, never taken over.
pub fn upsert_request(
    conn: &Connection,
    collection_id: &str,
    id: &str,
    section_id: Option<&str>,
    content: RequestContent,
) -> Result<(), AppError> {
    if let Some(section) = section_id {
        ensure_section_in_collection(conn, section, collection_id)?;
    }
    match owner_of(conn, Children::Requests, id)? {
        Some(owner) if owner != collection_id => Err(AppError::invalid(
            "The repository has a request whose id belongs to another collection",
        )),
        Some(_) => {
            update_request(conn, id, &content)?;
            conn.execute(
                "UPDATE requests SET section_id = ?2 WHERE id = ?1",
                params![id, section_id],
            )?;
            Ok(())
        }
        None => {
            let position = next_request_position(conn, collection_id)?;
            write_request(
                conn,
                collection_id,
                id.to_owned(),
                section_id,
                content,
                position,
            )
            .map(|_| ())
        }
    }
}

/// Like `upsert_request`, for sections.
pub fn upsert_section(
    conn: &Connection,
    collection_id: &str,
    id: &str,
    name: &str,
) -> Result<(), AppError> {
    match owner_of(conn, Children::Sections, id)? {
        Some(owner) if owner != collection_id => Err(AppError::invalid(
            "The repository has a section whose id belongs to another collection",
        )),
        Some(_) => rename_section(conn, id, name),
        None => insert_section_with_id(conn, collection_id, id, name).map(|_| ()),
    }
}

fn owner_of(conn: &Connection, children: Children, id: &str) -> Result<Option<String>, AppError> {
    let sql = match children {
        Children::Sections => "SELECT collection_id FROM sections WHERE id = ?1",
        Children::Requests => "SELECT collection_id FROM requests WHERE id = ?1",
    };
    Ok(conn
        .query_row(sql, params![id], |row| row.get(0))
        .optional()?)
}

fn write_request(
    conn: &Connection,
    collection_id: &str,
    id: String,
    section_id: Option<&str>,
    content: RequestContent,
    position: i64,
) -> Result<SavedRequest, AppError> {
    conn.execute(
        "INSERT INTO requests
           (id, collection_id, section_id, name, method, url, params, headers, body_mode, body,
            form, body_file, pre_request_script, test_script, kind, auth, sort_order, updated_at,
            mcp_operation, path_vars, graphql_variables, grpc, soap)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                 ?19, ?20, ?21, ?22, ?23)",
        params![
            id,
            collection_id,
            section_id,
            content.name,
            content.method,
            content.url,
            to_json(&content.params)?,
            to_json(&content.headers)?,
            content.body_mode,
            content.body,
            to_json(&content.form)?,
            content.body_file,
            content.pre_request_script,
            content.test_script,
            content.kind.as_str(),
            content.auth.to_stored()?,
            position,
            now_ms(),
            optional_json(content.mcp_operation.as_ref())?,
            to_json(&content.path_variables)?,
            content.graphql_variables,
            optional_json(content.grpc.as_ref())?,
            optional_json(content.soap.as_ref())?,
        ],
    )?;
    Ok(SavedRequest {
        id,
        collection_id: collection_id.to_owned(),
        section_id: section_id.map(str::to_owned),
        content,
    })
}

pub fn update_request(
    conn: &Connection,
    id: &str,
    content: &RequestContent,
) -> Result<RequestSummary, AppError> {
    let changed = conn.execute(
        "UPDATE requests
         SET name = ?2, method = ?3, url = ?4, params = ?5, headers = ?6, body_mode = ?7,
             body = ?8, pre_request_script = ?9, test_script = ?10, updated_at = ?11, auth = ?12,
             form = ?13, body_file = ?14, mcp_operation = ?15, path_vars = ?16,
             graphql_variables = ?17, grpc = ?18, soap = ?19
         WHERE id = ?1",
        params![
            id,
            content.name,
            content.method,
            content.url,
            to_json(&content.params)?,
            to_json(&content.headers)?,
            content.body_mode,
            content.body,
            content.pre_request_script,
            content.test_script,
            now_ms(),
            content.auth.to_stored()?,
            to_json(&content.form)?,
            content.body_file,
            optional_json(content.mcp_operation.as_ref())?,
            to_json(&content.path_variables)?,
            content.graphql_variables,
            optional_json(content.grpc.as_ref())?,
            optional_json(content.soap.as_ref())?,
        ],
    )?;
    expect_changed(changed, "Request")?;
    let section_id = conn.query_row(
        "SELECT section_id FROM requests WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    // The kind is fixed when a request is created, so the stored one is reported back.
    let kind: String = conn.query_row(
        "SELECT kind FROM requests WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    Ok(RequestSummary {
        id: id.to_owned(),
        section_id,
        kind: RequestKind::from_stored(&kind),
        name: content.name.clone(),
        method: content.method.clone(),
        url: content.url.clone(),
    })
}

pub fn load_request(conn: &Connection, id: &str) -> Result<SavedRequest, AppError> {
    let stored = conn
        .query_row(
            "SELECT id, collection_id, section_id, name, method, url, params, headers, body_mode,
                    body, pre_request_script, test_script, kind, auth, form, body_file,
                    mcp_operation, path_vars, graphql_variables, grpc, soap
             FROM requests WHERE id = ?1",
            params![id],
            StoredRequest::from_row,
        )
        .optional()?
        .ok_or(AppError::NotFound("Request"))?;
    stored.into_saved()
}

pub fn delete_request(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM app_secrets WHERE key = ?1 || ?2",
        params![TOKEN_KEY_PREFIX, id],
    )?;
    let changed = conn.execute("DELETE FROM requests WHERE id = ?1", params![id])?;
    expect_changed(changed, "Request")
}

/// Places a copy of the request, named `name`, right after it in the same section.
pub fn duplicate_request(
    conn: &Connection,
    id: &str,
    name: &str,
) -> Result<SavedRequest, AppError> {
    let original = load_request(conn, id)?;
    let position: i64 = conn.query_row(
        "SELECT sort_order FROM requests WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE requests SET sort_order = sort_order + 1
         WHERE collection_id = ?1 AND sort_order > ?2",
        params![original.collection_id, position],
    )?;
    let content = RequestContent {
        name: name.to_owned(),
        ..original.content
    };
    let copy = write_request(
        &tx,
        &original.collection_id,
        new_id(),
        original.section_id.as_deref(),
        content,
        position + 1,
    )?;
    examples::copy(&tx, id, &copy.id)?;
    descriptions::copy(&tx, Described::Request, id, &copy.id)?;
    tx.commit()?;
    Ok(copy)
}

/// Places a copy of the section, named `name` and holding copies of its requests, right after it.
pub fn duplicate_section(
    conn: &Connection,
    id: &str,
    name: &str,
) -> Result<CollectionDetail, AppError> {
    let (collection_id, position): (String, i64) = conn
        .query_row(
            "SELECT collection_id, sort_order FROM sections WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound("Section"))?;
    let request_ids = {
        let mut statement =
            conn.prepare("SELECT id FROM requests WHERE section_id = ?1 ORDER BY sort_order")?;
        let rows = statement.query_map(params![id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE sections SET sort_order = sort_order + 1
         WHERE collection_id = ?1 AND sort_order > ?2",
        params![collection_id, position],
    )?;
    let section_id = new_id();
    tx.execute(
        "INSERT INTO sections (id, collection_id, name, sort_order) VALUES (?1, ?2, ?3, ?4)",
        params![section_id, collection_id, name, position + 1],
    )?;
    descriptions::copy(&tx, Described::Section, id, &section_id)?;
    for request_id in request_ids {
        let saved = load_request(&tx, &request_id)?;
        let copy = insert_request(&tx, &collection_id, Some(&section_id), saved.content)?;
        examples::copy(&tx, &request_id, &copy.id)?;
        descriptions::copy(&tx, Described::Request, &request_id, &copy.id)?;
    }
    tx.commit()?;
    collection_detail(conn, &collection_id)
}

/// Copies a collection, as `name`, with its auth, sections and requests. Variables are copied
/// separately, because secrets need the vault.
pub fn duplicate_collection(
    conn: &Connection,
    id: &str,
    name: &str,
) -> Result<CollectionSummary, AppError> {
    let detail = collection_detail(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    let copy = insert_collection(&tx, name)?;
    set_collection_auth(&tx, &copy.id, &detail.auth)?;
    if detail.icon_custom {
        set_collection_icon(&tx, &copy.id, Some(&detail.icon))?;
    }
    descriptions::copy(&tx, Described::Collection, id, &copy.id)?;
    let mut sections = HashMap::with_capacity(detail.sections.len());
    for section in &detail.sections {
        let created = insert_section(&tx, &copy.id, &section.name)?;
        descriptions::copy(&tx, Described::Section, &section.id, &created.id)?;
        sections.insert(section.id.clone(), created.id);
    }
    for summary in &detail.requests {
        let saved = load_request(&tx, &summary.id)?;
        let section_id = saved
            .section_id
            .as_ref()
            .and_then(|old| sections.get(old))
            .map(String::as_str);
        let request = insert_request(&tx, &copy.id, section_id, saved.content)?;
        examples::copy(&tx, &summary.id, &request.id)?;
        descriptions::copy(&tx, Described::Request, &summary.id, &request.id)?;
    }
    tx.commit()?;
    Ok(CollectionSummary {
        request_count: i64::try_from(detail.requests.len()).unwrap_or(i64::MAX),
        icon: if detail.icon_custom {
            detail.icon
        } else {
            copy.icon.clone()
        },
        ..copy
    })
}

/// Saves a new order for a whole collection in one go. The new layout must list exactly the
/// collection's own sections and requests, so nothing can be dropped or pulled in from elsewhere.
pub fn reorder(
    conn: &Connection,
    collection_id: &str,
    sections: &[String],
    groups: &[RequestGroup],
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    write_order(&tx, collection_id, sections, groups)?;
    tx.commit()?;
    Ok(())
}

/// `reorder` for callers that already hold a transaction.
pub fn write_order(
    conn: &Connection,
    collection_id: &str,
    sections: &[String],
    groups: &[RequestGroup],
) -> Result<(), AppError> {
    let mismatch = || AppError::invalid("The new order does not match this collection");
    ensure_collection(conn, collection_id)?;

    let existing_sections = ids_in(conn, Children::Sections, collection_id)?;
    if !same_ids(sections.iter(), &existing_sections) {
        return Err(mismatch());
    }
    let existing_requests = ids_in(conn, Children::Requests, collection_id)?;
    if !same_ids(
        groups.iter().flat_map(|g| &g.request_ids),
        &existing_requests,
    ) {
        return Err(mismatch());
    }
    let unknown_section = groups
        .iter()
        .filter_map(|group| group.section_id.as_ref())
        .any(|id| !existing_sections.contains(id));
    if unknown_section {
        return Err(mismatch());
    }

    for (position, id) in (1_i64..).zip(sections) {
        conn.execute(
            "UPDATE sections SET sort_order = ?2 WHERE id = ?1 AND collection_id = ?3",
            params![id, position, collection_id],
        )?;
    }
    let ordered = groups.iter().flat_map(|group| {
        group
            .request_ids
            .iter()
            .map(move |id| (id, group.section_id.as_deref()))
    });
    for (position, (id, section_id)) in (1_i64..).zip(ordered) {
        conn.execute(
            "UPDATE requests SET section_id = ?2, sort_order = ?3
             WHERE id = ?1 AND collection_id = ?4",
            params![id, section_id, position, collection_id],
        )?;
    }
    Ok(())
}

enum Children {
    Sections,
    Requests,
}

fn ids_in(
    conn: &Connection,
    children: Children,
    collection_id: &str,
) -> Result<HashSet<String>, AppError> {
    let sql = match children {
        Children::Sections => "SELECT id FROM sections WHERE collection_id = ?1",
        Children::Requests => "SELECT id FROM requests WHERE collection_id = ?1",
    };
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params![collection_id], |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn same_ids<'a>(given: impl Iterator<Item = &'a String>, expected: &HashSet<String>) -> bool {
    let mut seen = HashSet::with_capacity(expected.len());
    for id in given {
        if !expected.contains(id) || !seen.insert(id) {
            return false;
        }
    }
    seen.len() == expected.len()
}

struct StoredRequest {
    id: String,
    collection_id: String,
    section_id: Option<String>,
    name: String,
    method: String,
    url: String,
    params: String,
    headers: String,
    body_mode: String,
    body: String,
    pre_request_script: String,
    test_script: String,
    kind: String,
    auth: String,
    form: String,
    body_file: String,
    mcp_operation: Option<String>,
    path_vars: String,
    graphql_variables: String,
    grpc: Option<String>,
    soap: Option<String>,
}

impl StoredRequest {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            collection_id: row.get(1)?,
            section_id: row.get(2)?,
            name: row.get(3)?,
            method: row.get(4)?,
            url: row.get(5)?,
            params: row.get(6)?,
            headers: row.get(7)?,
            body_mode: row.get(8)?,
            body: row.get(9)?,
            pre_request_script: row.get(10)?,
            test_script: row.get(11)?,
            kind: row.get(12)?,
            auth: row.get(13)?,
            form: row.get(14)?,
            body_file: row.get(15)?,
            mcp_operation: row.get(16)?,
            path_vars: row.get(17)?,
            graphql_variables: row.get(18)?,
            grpc: row.get(19)?,
            soap: row.get(20)?,
        })
    }

    fn into_saved(self) -> Result<SavedRequest, AppError> {
        Ok(SavedRequest {
            id: self.id,
            collection_id: self.collection_id,
            section_id: self.section_id,
            content: RequestContent {
                kind: RequestKind::from_stored(&self.kind),
                name: self.name,
                method: self.method,
                url: self.url,
                params: from_json(&self.params)?,
                headers: from_json(&self.headers)?,
                body_mode: self.body_mode,
                body: self.body,
                form: from_json(&self.form)?,
                body_file: self.body_file,
                pre_request_script: self.pre_request_script,
                test_script: self.test_script,
                auth: Auth::from_stored(&self.auth, Auth::Inherit),
                // An operation that no longer reads (edited by hand, say) is simply unset.
                mcp_operation: self
                    .mcp_operation
                    .as_deref()
                    .and_then(|json| serde_json::from_str(json).ok()),
                path_variables: from_json(&self.path_vars)?,
                graphql_variables: self.graphql_variables,
                grpc: self
                    .grpc
                    .as_deref()
                    .and_then(|json| serde_json::from_str(json).ok()),
                soap: self
                    .soap
                    .as_deref()
                    .and_then(|json| serde_json::from_str(json).ok()),
            },
        })
    }
}

pub fn has_collection(conn: &Connection, id: &str) -> Result<bool, AppError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM collections WHERE id = ?1",
            params![id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn ensure_collection(conn: &Connection, id: &str) -> Result<(), AppError> {
    if has_collection(conn, id)? {
        Ok(())
    } else {
        Err(AppError::NotFound("Collection"))
    }
}

fn ensure_section_in_collection(
    conn: &Connection,
    section_id: &str,
    collection_id: &str,
) -> Result<(), AppError> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sections WHERE id = ?1 AND collection_id = ?2",
            params![section_id, collection_id],
            |_| Ok(()),
        )
        .optional()?;
    exists.ok_or(AppError::NotFound("Section"))
}

const fn expect_changed(changed: usize, entity: &'static str) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::NotFound(entity))
    } else {
        Ok(())
    }
}

fn optional_json<T: Serialize>(value: Option<&T>) -> Result<Option<String>, AppError> {
    value
        .map(|value| serde_json::to_string(value).map_err(|_| AppError::Storage))
        .transpose()
}

pub fn to_json<T: Serialize>(rows: &[T]) -> Result<String, AppError> {
    serde_json::to_string(rows).map_err(|_| AppError::Storage)
}

pub fn from_json<T: DeserializeOwned>(text: &str) -> Result<Vec<T>, AppError> {
    serde_json::from_str(text).map_err(|_| AppError::Storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use crate::storage::models::{FormField, FormFieldKind, KeyValue};

    fn content(name: &str) -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: name.to_owned(),
            method: "POST".to_owned(),
            url: "https://api.example.com/v1/users".to_owned(),
            params: vec![KeyValue {
                enabled: true,
                key: "page".to_owned(),
                value: "1".to_owned(),
            }],
            headers: Vec::new(),
            body_mode: "json".to_owned(),
            body: r#"{"name":"Budi"}"#.to_owned(),
            form: vec![FormField {
                enabled: true,
                key: "avatar".to_owned(),
                value: "C:\\files\\avatar.png".to_owned(),
                kind: FormFieldKind::File,
            }],
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: "pm.test('ok', () => {});".to_owned(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    fn database() -> Database {
        Database::in_memory().expect("in-memory database")
    }

    #[test]
    fn stores_and_reloads_a_full_request() {
        let db = database();
        let saved = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                let section = insert_section(conn, &collection.id, "Users")?;
                insert_request(
                    conn,
                    &collection.id,
                    Some(&section.id),
                    content("Create user"),
                )
            })
            .expect("insert");

        let loaded = db.with(|conn| load_request(conn, &saved.id)).expect("load");

        assert_eq!(loaded.content.name, "Create user");
        assert_eq!(loaded.content.params.len(), 1);
        assert_eq!(loaded.content.body, r#"{"name":"Budi"}"#);
        assert!(
            loaded
                .content
                .form
                .first()
                .is_some_and(|f| f.kind == FormFieldKind::File && f.key == "avatar")
        );
        assert_eq!(loaded.content.test_script, "pm.test('ok', () => {});");
        assert_eq!(loaded.section_id, saved.section_id);
    }

    #[test]
    fn keeps_what_an_mcp_request_calls_and_copies_it() {
        use crate::storage::models::{McpOperation, McpOperationKind};
        let db = database();
        let operation = McpOperation {
            kind: McpOperationKind::Tool,
            name: "search_docs".to_owned(),
        };
        let (original, copy) = db
            .with(|conn| {
                let collection = insert_collection(conn, "Docs MCP")?;
                let saved = insert_request(
                    conn,
                    &collection.id,
                    None,
                    RequestContent {
                        kind: RequestKind::Mcp,
                        method: "STDIO".to_owned(),
                        url: "npx -y docs-server".to_owned(),
                        mcp_operation: Some(operation.clone()),
                        ..content("Search docs")
                    },
                )?;
                let copy = duplicate_request(conn, &saved.id, "Search docs copy")?;
                Ok((
                    load_request(conn, &saved.id)?,
                    load_request(conn, &copy.id)?,
                ))
            })
            .expect("mcp request");
        assert_eq!(original.content.kind, RequestKind::Mcp);
        assert_eq!(original.content.mcp_operation.as_ref(), Some(&operation));
        assert_eq!(copy.content.mcp_operation, Some(operation));
        assert_eq!(copy.content.method, "STDIO");
    }

    #[test]
    fn collection_detail_lists_sections_and_request_summaries_in_order() {
        let db = database();
        let detail = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                insert_section(conn, &collection.id, "Auth")?;
                insert_section(conn, &collection.id, "Users")?;
                insert_request(conn, &collection.id, None, content("First"))?;
                insert_request(conn, &collection.id, None, content("Second"))?;
                collection_detail(conn, &collection.id)
            })
            .expect("detail");

        let sections: Vec<_> = detail.sections.iter().map(|s| s.name.as_str()).collect();
        let requests: Vec<_> = detail.requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(sections, ["Auth", "Users"]);
        assert_eq!(requests, ["First", "Second"]);
    }

    #[test]
    fn updates_a_request_and_reports_missing_ones() {
        let db = database();
        let summary = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                let saved = insert_request(conn, &collection.id, None, content("Old"))?;
                update_request(conn, &saved.id, &content("New"))
            })
            .expect("update");
        assert_eq!(summary.name, "New");

        let missing = db.with(|conn| update_request(conn, &new_id(), &content("X")));
        assert!(matches!(missing, Err(AppError::NotFound("Request"))));
    }

    #[test]
    fn rejects_a_section_from_another_collection() {
        let db = database();
        let result = db.with(|conn| {
            let first = insert_collection(conn, "First")?;
            let second = insert_collection(conn, "Second")?;
            let foreign = insert_section(conn, &first.id, "Theirs")?;
            insert_request(conn, &second.id, Some(&foreign.id), content("Sneaky"))
        });
        assert!(matches!(result, Err(AppError::NotFound("Section"))));
    }

    #[test]
    fn deleting_a_section_keeps_its_requests() {
        let db = database();
        let detail = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                let section = insert_section(conn, &collection.id, "Users")?;
                insert_request(conn, &collection.id, Some(&section.id), content("Kept"))?;
                delete_section(conn, &section.id)?;
                collection_detail(conn, &collection.id)
            })
            .expect("detail");

        assert!(detail.sections.is_empty());
        assert_eq!(detail.requests.len(), 1);
        assert_eq!(
            detail.requests.first().and_then(|r| r.section_id.clone()),
            None
        );
    }

    #[test]
    fn deleting_a_collection_removes_everything_inside() {
        let db = database();
        let collections = db
            .with(|conn| {
                let collection = insert_collection(conn, "Temp")?;
                insert_section(conn, &collection.id, "Section")?;
                insert_request(conn, &collection.id, None, content("Request"))?;
                delete_collection(conn, &collection.id)?;
                let orphans: i64 =
                    conn.query_row("SELECT COUNT(*) FROM requests", [], |row| row.get(0))?;
                assert_eq!(orphans, 0);
                list_collections(conn)
            })
            .expect("list");
        assert!(collections.is_empty());
    }

    #[test]
    fn reorder_moves_requests_between_sections_and_reorders_sections() {
        let db = database();
        let detail = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                let auth = insert_section(conn, &collection.id, "Auth")?;
                let users = insert_section(conn, &collection.id, "Users")?;
                let a = insert_request(conn, &collection.id, Some(&auth.id), content("A"))?;
                let b = insert_request(conn, &collection.id, None, content("B"))?;
                reorder(
                    conn,
                    &collection.id,
                    &[users.id.clone(), auth.id.clone()],
                    &[
                        RequestGroup {
                            section_id: None,
                            request_ids: Vec::new(),
                        },
                        RequestGroup {
                            section_id: Some(users.id.clone()),
                            request_ids: vec![b.id.clone(), a.id.clone()],
                        },
                    ],
                )?;
                collection_detail(conn, &collection.id)
            })
            .expect("reorder");

        let sections: Vec<_> = detail.sections.iter().map(|s| s.name.as_str()).collect();
        let requests: Vec<_> = detail.requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(sections, ["Users", "Auth"]);
        assert_eq!(requests, ["B", "A"]);
        assert!(detail.requests.iter().all(|r| r.section_id.is_some()));
    }

    #[test]
    fn duplicates_a_request_right_after_the_original() {
        let db = database();
        let (copy, detail) = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                let first = insert_request(conn, &collection.id, None, content("First"))?;
                insert_request(conn, &collection.id, None, content("Second"))?;
                let copy = duplicate_request(conn, &first.id, "First copy")?;
                Ok((copy, collection_detail(conn, &collection.id)?))
            })
            .expect("duplicate");

        let names: Vec<_> = detail.requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["First", "First copy", "Second"]);
        assert_eq!(copy.content.body, r#"{"name":"Budi"}"#);
    }

    #[test]
    fn duplicates_a_section_with_its_requests() {
        let db = database();
        let detail = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                let auth = insert_section(conn, &collection.id, "Auth")?;
                insert_section(conn, &collection.id, "Users")?;
                insert_request(conn, &collection.id, Some(&auth.id), content("Login"))?;
                insert_request(conn, &collection.id, Some(&auth.id), content("Logout"))?;
                duplicate_section(conn, &auth.id, "Auth copy")
            })
            .expect("duplicate");

        let sections: Vec<_> = detail.sections.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(sections, ["Auth", "Auth copy", "Users"]);
        let copied = detail.sections.get(1).map(|s| s.id.clone());
        let in_copy: Vec<_> = detail
            .requests
            .iter()
            .filter(|r| r.section_id == copied)
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(in_copy, ["Login", "Logout"]);
    }

    #[test]
    fn duplicates_a_collection_with_auth_sections_and_requests() {
        let db = database();
        let (copy, detail) = db
            .with(|conn| {
                let collection = insert_collection(conn, "Users API")?;
                set_collection_auth(
                    conn,
                    &collection.id,
                    &Auth::Bearer {
                        token: "{{token}}".to_owned(),
                        prefix: "Bearer".to_owned(),
                    },
                )?;
                let section = insert_section(conn, &collection.id, "Users")?;
                insert_request(conn, &collection.id, None, content("Health"))?;
                insert_request(conn, &collection.id, Some(&section.id), content("List"))?;
                let copy = duplicate_collection(conn, &collection.id, "Users API copy")?;
                Ok((copy.clone(), collection_detail(conn, &copy.id)?))
            })
            .expect("duplicate");

        assert_eq!(copy.name, "Users API copy");
        assert_eq!(copy.request_count, 2);
        assert!(matches!(detail.auth, Auth::Bearer { .. }));
        assert_eq!(detail.sections.len(), 1);
        let in_section = detail.requests.iter().find(|r| r.name == "List");
        assert_eq!(
            in_section.and_then(|r| r.section_id.clone()),
            detail.sections.first().map(|s| s.id.clone())
        );
    }

    #[test]
    fn reorder_rejects_missing_duplicate_or_foreign_ids() {
        let db = database();
        db.with(|conn| {
            let collection = insert_collection(conn, "Mine")?;
            let other = insert_collection(conn, "Other")?;
            let mine = insert_request(conn, &collection.id, None, content("Mine"))?;
            let theirs = insert_request(conn, &other.id, None, content("Theirs"))?;
            let group = |ids: Vec<String>| RequestGroup {
                section_id: None,
                request_ids: ids,
            };

            assert!(reorder(conn, &collection.id, &[], &[group(Vec::new())]).is_err());
            assert!(
                reorder(
                    conn,
                    &collection.id,
                    &[],
                    &[group(vec![mine.id.clone(), mine.id.clone()])]
                )
                .is_err()
            );
            assert!(
                reorder(
                    conn,
                    &collection.id,
                    &[],
                    &[group(vec![mine.id.clone(), theirs.id.clone()])]
                )
                .is_err()
            );
            Ok(())
        })
        .expect("reorder checks");
    }

    #[test]
    fn icons_follow_the_name_until_one_is_chosen() {
        let db = database();
        db.with(|conn| {
            let created = insert_collection(conn, "Payments API")?;
            assert_eq!(created.icon, "💳");
            rename_collection(conn, &created.id, "Shop API")?;
            assert_eq!(collection_summary(conn, &created.id)?.icon, "🛒");

            set_collection_icon(conn, &created.id, Some("🦊"))?;
            let detail = collection_detail(conn, &created.id)?;
            assert_eq!((detail.icon.as_str(), detail.icon_custom), ("🦊", true));
            let copy = duplicate_collection(conn, &created.id, "Shop API copy")?;
            assert_eq!(copy.icon, "🦊");
            assert_eq!(list_collections(conn)?.len(), 2);

            set_collection_icon(conn, &created.id, None)?;
            let detail = collection_detail(conn, &created.id)?;
            assert_eq!((detail.icon.as_str(), detail.icon_custom), ("🛒", false));
            assert!(set_collection_icon(conn, "missing", Some("🦊")).is_err());
            Ok(())
        })
        .expect("icons");
    }
}
