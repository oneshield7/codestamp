use rusqlite::{Connection, OptionalExtension, params};

use super::models::{HistoryEntry, HistoryFilter, SentRequest, StatusFilter};
use crate::auth::models::Auth;
use crate::error::AppError;
use crate::preferences::models::HistoryPreferences;
use crate::preferences::repo::load_preferences;
use crate::request::types::{HttpResponseOutput, NameValue};
use crate::storage::db::{new_id, now_ms};
use crate::storage::models::{KeyValue, PathVariable, RequestContent, RequestKind};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const LIST_LIMIT: i64 = 300;
const MAX_QUERY_CHARS: usize = 200;
const MAX_NAME_CHARS: usize = 120;

/// Adds one sent request to history, then trims it to the limits in Settings › Data.
pub fn record(
    conn: &Connection,
    sent: &SentRequest,
    outcome: Result<&HttpResponseOutput, &AppError>,
) -> Result<(), AppError> {
    // Links are taken from the database, not from the UI, so they always point at real rows.
    let linked: Option<(String, String)> = match &sent.request_id {
        Some(id) => conn
            .query_row(
                "SELECT name, collection_id FROM requests WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?,
        None => None,
    };
    let (request_id, collection_id, name) = match linked {
        Some((name, collection_id)) => (sent.request_id.clone(), Some(collection_id), name),
        None => (
            None,
            existing_collection(conn, sent.collection_id.as_deref())?,
            format!("{} {}", sent.method, sent.url)
                .chars()
                .take(MAX_NAME_CHARS)
                .collect(),
        ),
    };
    let (status, error, elapsed_ms, body_size) = match outcome {
        Ok(response) => (
            Some(response.status),
            None,
            i64::try_from(response.elapsed_ms).ok(),
            i64::try_from(response.body_size).ok(),
        ),
        Err(error) => (None, Some(error.to_string()), None, None),
    };

    conn.execute(
        "INSERT INTO history
           (id, collection_id, request_id, name, method, url, params, headers, body_mode, body,
            body_omitted, status, error, elapsed_ms, body_size, created_at, auth, path_vars)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            new_id(),
            collection_id,
            request_id,
            name,
            sent.method,
            sent.url,
            rows_json(&sent.params)?,
            rows_json(&sent.headers)?,
            sent.body_mode,
            sent.body.as_deref().unwrap_or_default(),
            sent.body_omitted,
            status,
            error,
            elapsed_ms,
            body_size,
            now_ms(),
            sent.auth.to_stored()?,
            path_json(&sent.path_variables)?,
        ],
    )?;
    prune(conn, &load_preferences(conn)?.history)
}

pub fn list(conn: &Connection, filter: &HistoryFilter) -> Result<Vec<HistoryEntry>, AppError> {
    let pattern = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(|query| {
            let limited: String = query.chars().take(MAX_QUERY_CHARS).collect();
            format!("%{}%", escape_like(&limited))
        });
    let (kind, low, high) = match filter.status {
        StatusFilter::All => ("all", 0, 0),
        StatusFilter::Success => ("range", 200, 300),
        StatusFilter::Redirect => ("range", 300, 400),
        StatusFilter::ClientError => ("range", 400, 500),
        StatusFilter::ServerError => ("range", 500, 600),
        StatusFilter::Failed => ("failed", 0, 0),
    };
    let mut statement = conn.prepare(
        r"SELECT h.id, h.collection_id, c.name, h.request_id, h.name, h.method, h.url, h.status,
                 h.error, h.elapsed_ms, h.body_size, h.created_at
          FROM history h LEFT JOIN collections c ON c.id = h.collection_id
          WHERE (?1 IS NULL OR h.url LIKE ?1 ESCAPE '\' OR h.name LIKE ?1 ESCAPE '\')
            AND (?2 = 'all'
                 OR (?2 = 'failed' AND h.status IS NULL)
                 OR (?2 = 'range' AND h.status >= ?3 AND h.status < ?4))
            AND (?6 IS NULL OR h.collection_id = ?6)
          ORDER BY h.created_at DESC, h.rowid DESC
          LIMIT ?5",
    )?;
    let params = params![pattern, kind, low, high, LIST_LIMIT, filter.collection_id];
    let rows = statement.query_map(params, |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            collection_id: row.get(1)?,
            collection_name: row.get(2)?,
            request_id: row.get(3)?,
            name: row.get(4)?,
            method: row.get(5)?,
            url: row.get(6)?,
            status: row.get(7)?,
            error: row.get(8)?,
            elapsed_ms: row.get(9)?,
            body_size: row.get(10)?,
            created_at: row.get(11)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    let changed = conn.execute("DELETE FROM history WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::NotFound("History entry"));
    }
    Ok(())
}

/// Every entry, or only those sent from one collection.
pub fn clear(conn: &Connection, collection_id: Option<&str>) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM history WHERE ?1 IS NULL OR collection_id = ?1",
        params![collection_id],
    )?;
    Ok(())
}

/// The stored request, ready to be saved into a collection.
pub fn request_content(conn: &Connection, id: &str) -> Result<RequestContent, AppError> {
    let row = conn
        .query_row(
            "SELECT name, method, url, params, headers, body_mode, body, auth, path_vars
             FROM history WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()?
        .ok_or(AppError::NotFound("History entry"))?;
    let (name, method, url, params, headers, body_mode, body, auth, path_vars) = row;
    // History keeps form rows as JSON and a binary body's file path in the body column.
    let (body, form, body_file) = match body_mode.as_str() {
        "urlencoded" | "formdata" if !body.is_empty() => (
            String::new(),
            serde_json::from_str(&body).map_err(|_| AppError::Storage)?,
            String::new(),
        ),
        "binary" => (String::new(), Vec::new(), body),
        _ => (body, Vec::new(), String::new()),
    };
    Ok(RequestContent {
        kind: RequestKind::Http,
        name,
        method,
        url,
        params: serde_json::from_str(&params).map_err(|_| AppError::Storage)?,
        headers: serde_json::from_str(&headers).map_err(|_| AppError::Storage)?,
        body_mode,
        body,
        form,
        body_file,
        pre_request_script: String::new(),
        test_script: String::new(),
        auth: Auth::from_stored(&auth, Auth::None),
        mcp_operation: None,
        path_variables: serde_json::from_str(&path_vars).map_err(|_| AppError::Storage)?,
        graphql_variables: String::new(),
        grpc: None,
        soap: None,
    })
}

fn existing_collection(conn: &Connection, id: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(id) = id else {
        return Ok(None);
    };
    Ok(conn
        .query_row(
            "SELECT id FROM collections WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?)
}

fn rows_json(pairs: &[NameValue]) -> Result<String, AppError> {
    let rows: Vec<KeyValue> = pairs
        .iter()
        .map(|pair| KeyValue {
            enabled: true,
            key: pair.name.clone(),
            value: pair.value.clone(),
        })
        .collect();
    serde_json::to_string(&rows).map_err(|_| AppError::Storage)
}

fn path_json(pairs: &[NameValue]) -> Result<String, AppError> {
    let variables: Vec<PathVariable> = pairs
        .iter()
        .map(|pair| PathVariable {
            key: pair.name.clone(),
            value: pair.value.clone(),
        })
        .collect();
    serde_json::to_string(&variables).map_err(|_| AppError::Storage)
}

/// `%` and `_` typed by the user are matched literally, not as LIKE wildcards.
fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Removes entries older than the kept days and beyond the kept number.
pub fn prune(conn: &Connection, limits: &HistoryPreferences) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM history WHERE created_at < ?1",
        params![now_ms() - i64::from(limits.keep_days) * DAY_MS],
    )?;
    conn.execute(
        "DELETE FROM history WHERE id IN (
             SELECT id FROM history ORDER BY created_at DESC, rowid DESC LIMIT -1 OFFSET ?1)",
        params![limits.max_entries],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    fn pair(name: &str, value: &str) -> NameValue {
        NameValue {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    fn sent(url: &str) -> SentRequest {
        SentRequest {
            collection_id: None,
            request_id: None,
            method: "POST".to_owned(),
            url: url.to_owned(),
            params: vec![pair("page", "1")],
            headers: vec![
                pair("Authorization", "Bearer {{token}}"),
                pair("Content-Type", "application/json"),
            ],
            body_mode: "json",
            body: Some(r#"{"a":1}"#.to_owned()),
            body_omitted: false,
            auth: Auth::Basic {
                username: "{{user}}".to_owned(),
                password: "{{password}}".to_owned(),
            },
            path_variables: Vec::new(),
        }
    }

    fn response(status: u16) -> HttpResponseOutput {
        HttpResponseOutput {
            status,
            status_text: String::new(),
            http_version: "HTTP/1.1".to_owned(),
            headers: Vec::new(),
            content_type: None,
            body_preview: String::new(),
            body_size: 42,
            body_truncated: false,
            body_id: None,
            whole_body: None,
            stream_id: None,
            stream: None,
            elapsed_ms: 15,
            cookies: Vec::new(),
            warning: None,
            script: None,
        }
    }

    fn filter(query: Option<&str>, status: StatusFilter) -> HistoryFilter {
        HistoryFilter {
            query: query.map(str::to_owned),
            status,
            collection_id: None,
        }
    }

    #[test]
    fn lists_and_clears_one_collection_at_a_time() {
        use crate::storage::models::{RequestContent, RequestKind};
        use crate::storage::repo as collections;

        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let content = |name: &str| RequestContent {
                kind: RequestKind::Http,
                name: name.to_owned(),
                method: "GET".to_owned(),
                url: "https://api.test".to_owned(),
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
            };
            let mut ids = Vec::new();
            for name in ["Shop", "Users"] {
                let collection = collections::insert_collection(conn, name)?;
                let request =
                    collections::insert_request(conn, &collection.id, None, content(name))?;
                let from = SentRequest {
                    collection_id: Some(collection.id.clone()),
                    request_id: Some(request.id),
                    ..sent("https://api.test")
                };
                record(conn, &from, Ok(&response(200)))?;
                record(conn, &from, Ok(&response(500)))?;
                ids.push(collection.id);
            }
            let only = |id: &str| HistoryFilter {
                collection_id: Some(id.to_owned()),
                ..filter(None, StatusFilter::All)
            };
            let shop = list(conn, &only(&ids[0]))?;
            assert_eq!(shop.len(), 2);
            assert!(
                shop.iter()
                    .all(|entry| entry.collection_name.as_deref() == Some("Shop"))
            );
            assert_eq!(list(conn, &filter(None, StatusFilter::All))?.len(), 4);

            clear(conn, Some(&ids[0]))?;
            assert!(list(conn, &only(&ids[0]))?.is_empty());
            assert_eq!(list(conn, &only(&ids[1]))?.len(), 2);
            clear(conn, None)?;
            assert!(list(conn, &filter(None, StatusFilter::All))?.is_empty());
            Ok(())
        })
        .expect("per collection");
    }

    #[test]
    fn records_the_template_not_resolved_values() {
        let db = Database::in_memory().expect("database");
        let content = db
            .with(|conn| {
                record(conn, &sent("{{baseUrl}}/users"), Ok(&response(201)))?;
                let entry = list(conn, &filter(None, StatusFilter::All))?
                    .into_iter()
                    .next()
                    .ok_or(AppError::Storage)?;
                assert_eq!(entry.status, Some(201));
                request_content(conn, &entry.id)
            })
            .expect("history");
        assert_eq!(content.url, "{{baseUrl}}/users");
        assert_eq!(content.body_mode, "json");
        assert_eq!(content.auth, sent("").auth);
        assert!(
            content
                .headers
                .iter()
                .any(|h| h.value == "Bearer {{token}}")
        );
    }

    #[test]
    fn reopens_form_and_file_bodies() {
        use crate::request::types::{FormPart, HttpRequestInput, RequestBody};
        use crate::storage::models::FormFieldKind;

        let input = |body: RequestBody| HttpRequestInput {
            method: "POST".to_owned(),
            url: "https://upload.test".to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body: Some(body),
            timeout_ms: None,
            collection_id: None,
            request_id: None,
            name: None,
            pre_request_script: None,
            test_script: None,
            auth: None,
            record_history: true,
            path_variables: Vec::new(),
            iteration_data: Vec::new(),
            iteration: Default::default(),
        };
        let form = SentRequest::capture(&input(RequestBody::FormData {
            parts: vec![
                FormPart {
                    name: "title".to_owned(),
                    value: "{{title}}".to_owned(),
                    file: false,
                },
                FormPart {
                    name: "photo".to_owned(),
                    value: "C:\\pics\\a.png".to_owned(),
                    file: true,
                },
            ],
        }));
        let binary = SentRequest::capture(&input(RequestBody::Binary {
            path: "C:\\pics\\b.bin".to_owned(),
        }));

        let db = Database::in_memory().expect("database");
        let contents = db
            .with(|conn| {
                record(conn, &form, Ok(&response(200)))?;
                record(conn, &binary, Ok(&response(200)))?;
                list(conn, &filter(None, StatusFilter::All))?
                    .iter()
                    .map(|entry| request_content(conn, &entry.id))
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("history");

        let reopened_binary = contents.iter().find(|c| c.body_mode == "binary");
        assert_eq!(
            reopened_binary.map(|c| c.body_file.as_str()),
            Some("C:\\pics\\b.bin")
        );
        let reopened_form = contents.iter().find(|c| c.body_mode == "formdata");
        let fields = reopened_form.map(|c| c.form.clone()).unwrap_or_default();
        assert_eq!(fields.len(), 2);
        assert!(
            fields
                .iter()
                .any(|f| f.kind == FormFieldKind::File && f.value == "C:\\pics\\a.png")
        );
        assert!(fields.iter().any(|f| f.value == "{{title}}"));
    }

    #[test]
    fn filters_by_status_and_literal_text() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            record(conn, &sent("https://a.test/ok"), Ok(&response(200)))?;
            record(conn, &sent("https://a.test/missing"), Ok(&response(404)))?;
            record(
                conn,
                &sent("https://a.test/100%_done"),
                Err(&AppError::Timeout),
            )?;

            assert_eq!(list(conn, &filter(None, StatusFilter::Success))?.len(), 1);
            assert_eq!(
                list(conn, &filter(None, StatusFilter::ClientError))?.len(),
                1
            );
            let failed = list(conn, &filter(None, StatusFilter::Failed))?;
            assert_eq!(failed.len(), 1);
            assert!(failed.first().is_some_and(|e| e.error.is_some()));
            assert_eq!(
                list(conn, &filter(Some("missing"), StatusFilter::All))?.len(),
                1
            );
            assert_eq!(list(conn, &filter(Some("%_"), StatusFilter::All))?.len(), 1);
            Ok(())
        })
        .expect("filters");
    }

    #[test]
    fn keeps_at_most_the_newest_entries() {
        let db = Database::in_memory().expect("database");
        let count = db
            .with(|conn| {
                for _ in 0..(HistoryPreferences::default().max_entries + 5) {
                    record(conn, &sent("https://a.test"), Ok(&response(200)))?;
                }
                Ok(conn.query_row("SELECT COUNT(*) FROM history", [], |row| {
                    row.get::<_, i64>(0)
                })?)
            })
            .expect("prune");
        assert_eq!(count, i64::from(HistoryPreferences::default().max_entries));
    }

    #[test]
    fn ignores_links_to_rows_that_do_not_exist() {
        let db = Database::in_memory().expect("database");
        let entry = db
            .with(|conn| {
                let mut request = sent("https://a.test");
                request.collection_id = Some(new_id());
                request.request_id = Some(new_id());
                record(conn, &request, Ok(&response(200)))?;
                list(conn, &filter(None, StatusFilter::All))?
                    .into_iter()
                    .next()
                    .ok_or(AppError::Storage)
            })
            .expect("record");
        assert_eq!(entry.collection_id, None);
        assert_eq!(entry.request_id, None);
    }
}
