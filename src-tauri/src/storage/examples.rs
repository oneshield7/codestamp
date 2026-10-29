//! Saved examples: responses kept with a request, for reference and for the mock server.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::db::{new_id, now_ms};
use super::validate::clean_name;
use crate::error::AppError;
use crate::request::rules::{MAX_KEY_VALUE_LENGTH, MAX_KEY_VALUE_ROWS};
use crate::request::types::NameValue;

/// The same limit as the response preview, so any shown response can be kept.
pub const MAX_EXAMPLE_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_EXAMPLES_PER_REQUEST: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExampleInput {
    pub name: String,
    pub status: u16,
    pub headers: Vec<NameValue>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub id: String,
    pub request_id: String,
    pub name: String,
    pub status: u16,
    pub headers: Vec<NameValue>,
    pub body: String,
}

/// What a list shows; bodies load when an example is opened.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleSummary {
    pub id: String,
    pub name: String,
    pub status: u16,
    pub size: usize,
}

/// A request of a collection that has examples, for the mock server to match against.
#[derive(Debug, Clone)]
pub struct ExampleOwner {
    pub request_id: String,
    pub method: String,
    pub url: String,
}

pub fn owners(conn: &Connection, collection_id: &str) -> Result<Vec<ExampleOwner>, AppError> {
    let mut statement = conn.prepare(
        "SELECT r.id, r.method, r.url FROM requests r
         WHERE r.collection_id = ?1
           AND EXISTS (SELECT 1 FROM request_examples e WHERE e.request_id = r.id)
         ORDER BY r.sort_order",
    )?;
    let rows = statement.query_map(params![collection_id], |row| {
        Ok(ExampleOwner {
            request_id: row.get(0)?,
            method: row.get(1)?,
            url: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn check_example(input: ExampleInput) -> Result<ExampleInput, AppError> {
    let name = clean_name(&input.name)?;
    if !(100..=599).contains(&input.status) {
        return Err(AppError::invalid(
            "An example status is between 100 and 599",
        ));
    }
    if input.headers.len() > MAX_KEY_VALUE_ROWS
        || input.headers.iter().any(|header| {
            header.name.len() > MAX_KEY_VALUE_LENGTH || header.value.len() > MAX_KEY_VALUE_LENGTH
        })
    {
        return Err(AppError::invalid(
            "The example has too many or too long headers",
        ));
    }
    if input.body.len() > MAX_EXAMPLE_BODY_BYTES {
        return Err(AppError::invalid("Examples can hold bodies up to 2 MB"));
    }
    Ok(ExampleInput { name, ..input })
}

pub fn list(conn: &Connection, request_id: &str) -> Result<Vec<ExampleSummary>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, name, status, length(CAST(body AS BLOB)) FROM request_examples
         WHERE request_id = ?1 ORDER BY sort_order",
    )?;
    let rows = statement.query_map(params![request_id], |row| {
        let size: i64 = row.get(3)?;
        Ok(ExampleSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row.get(2)?,
            size: usize::try_from(size).unwrap_or(0),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every example of a request, bodies included, in their order.
pub fn all_for(conn: &Connection, request_id: &str) -> Result<Vec<Example>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, request_id, name, status, headers, body FROM request_examples
         WHERE request_id = ?1 ORDER BY sort_order",
    )?;
    let rows = statement.query_map(params![request_id], stored)?;
    rows.collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(StoredExample::into_example)
        .collect()
}

pub fn get(conn: &Connection, id: &str) -> Result<Example, AppError> {
    conn.query_row(
        "SELECT id, request_id, name, status, headers, body FROM request_examples WHERE id = ?1",
        params![id],
        stored,
    )
    .optional()?
    .ok_or(AppError::NotFound("Example"))?
    .into_example()
}

pub fn insert(
    conn: &Connection,
    request_id: &str,
    input: &ExampleInput,
) -> Result<Example, AppError> {
    insert_with_id(conn, &new_id(), request_id, input)
}

fn insert_with_id(
    conn: &Connection,
    id: &str,
    request_id: &str,
    input: &ExampleInput,
) -> Result<Example, AppError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM request_examples WHERE request_id = ?1",
        params![request_id],
        |row| row.get(0),
    )?;
    if usize::try_from(count).unwrap_or(usize::MAX) >= MAX_EXAMPLES_PER_REQUEST {
        return Err(AppError::invalid("A request can keep up to 50 examples"));
    }
    conn.execute(
        "INSERT INTO request_examples (id, request_id, name, status, headers, body, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6,
                 (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM request_examples WHERE request_id = ?2), ?7)",
        params![
            id,
            request_id,
            input.name,
            input.status,
            headers_json(&input.headers)?,
            input.body,
            now_ms()
        ],
    )?;
    Ok(Example {
        id: id.to_owned(),
        request_id: request_id.to_owned(),
        name: input.name.clone(),
        status: input.status,
        headers: input.headers.clone(),
        body: input.body.clone(),
    })
}

pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE request_examples SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound("Example"));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    let changed = conn.execute("DELETE FROM request_examples WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::NotFound("Example"));
    }
    Ok(())
}

/// Copies a request's examples to its duplicate.
pub fn copy(conn: &Connection, from_request: &str, to_request: &str) -> Result<(), AppError> {
    for example in all_for(conn, from_request)? {
        insert(conn, to_request, &example.into_input())?;
    }
    Ok(())
}

/// Makes a request's examples exactly these (from a shared repository), keeping their ids.
pub fn replace(conn: &Connection, request_id: &str, examples: &[Example]) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM request_examples WHERE request_id = ?1",
        params![request_id],
    )?;
    for example in examples {
        insert_with_id(conn, &example.id, request_id, &example.clone().into_input())?;
    }
    Ok(())
}

impl Example {
    pub fn into_input(self) -> ExampleInput {
        ExampleInput {
            name: self.name,
            status: self.status,
            headers: self.headers,
            body: self.body,
        }
    }
}

/// A row as stored; headers are JSON until they are read.
struct StoredExample {
    id: String,
    request_id: String,
    name: String,
    status: u16,
    headers: String,
    body: String,
}

impl StoredExample {
    fn into_example(self) -> Result<Example, AppError> {
        Ok(Example {
            id: self.id,
            request_id: self.request_id,
            name: self.name,
            status: self.status,
            headers: serde_json::from_str(&self.headers).map_err(|_| AppError::Storage)?,
            body: self.body,
        })
    }
}

fn stored(row: &Row<'_>) -> rusqlite::Result<StoredExample> {
    Ok(StoredExample {
        id: row.get(0)?,
        request_id: row.get(1)?,
        name: row.get(2)?,
        status: row.get(3)?,
        headers: row.get(4)?,
        body: row.get(5)?,
    })
}

fn headers_json(headers: &[NameValue]) -> Result<String, AppError> {
    serde_json::to_string(headers).map_err(|_| AppError::Storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use crate::storage::models::RequestContent;
    use crate::storage::repo;

    fn example(name: &str) -> ExampleInput {
        ExampleInput {
            name: name.to_owned(),
            status: 200,
            headers: vec![NameValue {
                name: "Content-Type".to_owned(),
                value: "application/json".to_owned(),
            }],
            body: "{\"ok\":true}".to_owned(),
        }
    }

    #[test]
    fn keeps_copies_and_replaces_examples() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let collection = repo::insert_collection(conn, "Shop")?;
            let content: RequestContent = serde_json::from_str(
                r#"{"name":"Get","method":"GET","url":"/x","params":[],"headers":[],"bodyMode":"none","body":""}"#,
            )
            .map_err(|_| AppError::Storage)?;
            let saved = repo::insert_request(conn, &collection.id, None, content)?;
            let first = insert(conn, &saved.id, &check_example(example("  OK  "))?)?;
            assert_eq!(first.name, "OK");
            insert(conn, &saved.id, &example("Not found"))?;
            let listed = list(conn, &saved.id)?;
            assert_eq!(listed.len(), 2);
            assert_eq!(listed[0].size, 11);

            let copy = repo::duplicate_request(conn, &saved.id, "Get copy")?;
            assert_eq!(list(conn, &copy.id)?.len(), 2);

            replace(conn, &saved.id, &[get(conn, &first.id)?])?;
            assert_eq!(all_for(conn, &saved.id)?.len(), 1);
            rename(conn, &first.id, "Fine")?;
            assert_eq!(get(conn, &first.id)?.name, "Fine");
            repo::delete_request(conn, &saved.id)?;
            assert!(get(conn, &first.id).is_err());
            Ok(())
        })
        .expect("examples");
        assert!(
            check_example(ExampleInput {
                status: 42,
                ..example("x")
            })
            .is_err()
        );
    }
}
