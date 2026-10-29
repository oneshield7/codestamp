use std::collections::HashMap;

use rusqlite::{Connection, params};

use super::models::{CollectionDocs, DocExample, DocVariable, RequestDocs, SectionDocs};
use crate::environment::repo as environments;
use crate::error::AppError;
use crate::storage::descriptions::{self, Described};
use crate::storage::examples;
use crate::storage::repo as collections;
use crate::text::clip;

/// Example bodies longer than this are cut in Docs; the full body stays with the request.
const MAX_EXAMPLE_BODY_BYTES: usize = 20 * 1024;

pub fn load(conn: &Connection, collection_id: &str) -> Result<CollectionDocs, AppError> {
    let detail = collections::collection_detail(conn, collection_id)?;
    let section_texts = section_descriptions(conn, collection_id)?;
    let sections: Vec<SectionDocs> = detail
        .sections
        .into_iter()
        .map(|section| SectionDocs {
            description: section_texts.get(&section.id).cloned().unwrap_or_default(),
            id: section.id,
            name: section.name,
        })
        .collect();

    // Notebook order: requests outside sections first, then each section's in turn.
    let mut ordered: Vec<&str> = detail
        .requests
        .iter()
        .filter(|request| request.section_id.is_none())
        .map(|request| request.id.as_str())
        .collect();
    for section in &sections {
        ordered.extend(
            detail
                .requests
                .iter()
                .filter(|request| request.section_id.as_deref() == Some(section.id.as_str()))
                .map(|request| request.id.as_str()),
        );
    }
    let requests = ordered
        .into_iter()
        .map(|id| {
            Ok(RequestDocs {
                request: collections::load_request(conn, id)?,
                description: descriptions::get(conn, Described::Request, id)?,
                examples: examples::all_for(conn, id)?
                    .into_iter()
                    .map(|example| {
                        let (body, cut) = clip(&example.body, MAX_EXAMPLE_BODY_BYTES);
                        DocExample {
                            content_type: example
                                .headers
                                .iter()
                                .find(|header| header.name.eq_ignore_ascii_case("content-type"))
                                .map(|header| header.value.clone()),
                            body: body.to_owned(),
                            cut,
                            name: example.name,
                            status: example.status,
                        }
                    })
                    .collect(),
            })
        })
        .collect::<Result<_, AppError>>()?;

    Ok(CollectionDocs {
        description: descriptions::get(conn, Described::Collection, collection_id)?,
        variables: variables(conn, collection_id)?,
        id: detail.id,
        name: detail.name,
        icon: detail.icon,
        auth: detail.auth,
        sections,
        requests,
    })
}

fn section_descriptions(
    conn: &Connection,
    collection_id: &str,
) -> Result<HashMap<String, String>, AppError> {
    let mut statement =
        conn.prepare("SELECT id, description FROM sections WHERE collection_id = ?1")?;
    let rows = statement.query_map(params![collection_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn variables(conn: &Connection, collection_id: &str) -> Result<Vec<DocVariable>, AppError> {
    let Some(set) = environments::find_collection_set(conn, collection_id)? else {
        return Ok(Vec::new());
    };
    Ok(environments::variables(conn, &set)?
        .into_iter()
        .filter(|variable| variable.enabled)
        .map(|variable| DocVariable {
            key: variable.key,
            value: if variable.is_secret {
                String::new()
            } else {
                variable.value
            },
            secret: variable.is_secret,
        })
        .collect())
}
