use rusqlite::Connection;

use super::models::{
    ImportKind, ImportPlan, ImportResult, PlannedEnvironment, PlannedRequest, PlannedSection,
};
use crate::auth::models::Auth;
use crate::environment::models::VariableInput;
use crate::environment::repo as environments;
use crate::environment::validate::check_variables;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::db::new_id;
use crate::storage::descriptions::{self, Described};
use crate::storage::examples;
use crate::storage::repo as collections;

/// Writes a checked plan. Collections, sections and requests go in one transaction; variables
/// are saved afterwards, and if that fails the new collection or environment is removed again.
pub fn apply(
    conn: &Connection,
    vault: &Vault,
    plan: ImportPlan,
    name: &str,
) -> Result<ImportResult, AppError> {
    let variables = check_variables(plan.variables)?;
    let owned = check_environments(plan.environments)?;
    match plan.kind {
        ImportKind::Collection => {
            let tx = conn.unchecked_transaction()?;
            let collection = collections::insert_collection(&tx, name)?;
            if plan.auth != Auth::None {
                collections::set_collection_auth(&tx, &collection.id, &plan.auth)?;
            }
            if plan.icon.is_some() {
                collections::set_collection_icon(&tx, &collection.id, plan.icon.as_deref())?;
            }
            fill(
                &tx,
                &collection.id,
                &plan.description,
                &plan.sections,
                plan.requests,
            )?;
            tx.commit()?;

            let saved = if variables.is_empty() {
                Ok(())
            } else {
                environments::collection_set_id(conn, &collection.id)
                    .and_then(|set| environments::replace_variables(conn, vault, &set, variables))
                    .map(drop)
            }
            .and_then(|()| add_environments(conn, vault, &collection.id, owned));
            if let Err(error) = saved {
                // The original error is what the user needs to see; if the cleanup fails too,
                // an empty imported collection is left behind, which they can delete.
                let _ = collections::delete_collection(conn, &collection.id);
                return Err(error);
            }
            Ok(ImportResult {
                kind: ImportKind::Collection,
                id: collection.id,
            })
        }
        ImportKind::Environment => {
            let environment = environments::insert(conn, name)?;
            if let Err(error) =
                environments::replace_variables(conn, vault, &environment.id, variables)
            {
                // As above: report the original error even if the cleanup fails.
                let _ = environments::delete(conn, &environment.id);
                return Err(error);
            }
            Ok(ImportResult {
                kind: ImportKind::Environment,
                id: environment.id,
            })
        }
    }
}

/// Puts a collection file's content into an existing collection in place of what it holds:
/// sections, requests, auth, docs and variables. The collection keeps its id, name and icon, so a
/// shared collection sends the replacement as an ordinary change on its next sync.
pub fn replace(
    conn: &Connection,
    vault: &Vault,
    plan: ImportPlan,
    collection_id: &str,
) -> Result<ImportResult, AppError> {
    if plan.kind != ImportKind::Collection {
        return Err(AppError::invalid(
            "Only a collection file can replace a collection",
        ));
    }
    let variables = check_variables(plan.variables)?;
    let owned = check_environments(plan.environments)?;
    let tx = conn.unchecked_transaction()?;
    let current = collections::collection_detail(&tx, collection_id)?;
    for request in &current.requests {
        collections::delete_request(&tx, &request.id)?;
    }
    for section in &current.sections {
        collections::delete_section(&tx, &section.id)?;
    }
    collections::set_collection_auth(&tx, collection_id, &plan.auth)?;
    fill(
        &tx,
        collection_id,
        &plan.description,
        &plan.sections,
        plan.requests,
    )?;
    tx.commit()?;
    // Saved in a transaction of its own; checked above, so only a vault failure can stop it.
    let set = environments::collection_set_id(conn, collection_id)?;
    environments::replace_variables(conn, vault, &set, variables)?;
    // A file that carries the collection's own environments replaces those too.
    if !owned.is_empty() {
        for (id, _) in environments::owned(conn, collection_id)? {
            environments::delete_owned(conn, &id, collection_id)?;
        }
        add_environments(conn, vault, collection_id, owned)?;
    }
    Ok(ImportResult {
        kind: ImportKind::Collection,
        id: collection_id.to_owned(),
    })
}

/// The collection's docs, sections and requests (with their examples) from a plan.
fn check_environments(
    planned: Vec<PlannedEnvironment>,
) -> Result<Vec<(String, Vec<VariableInput>)>, AppError> {
    planned
        .into_iter()
        .map(|environment| Ok((environment.name, check_variables(environment.variables)?)))
        .collect()
}

/// Environments that belong to the collection, each with a new id.
fn add_environments(
    conn: &Connection,
    vault: &Vault,
    collection_id: &str,
    owned: Vec<(String, Vec<VariableInput>)>,
) -> Result<(), AppError> {
    for (name, variables) in owned {
        let id = new_id();
        environments::put_owned(conn, &id, &name, collection_id)?;
        environments::replace_variables(conn, vault, &id, variables)?;
    }
    Ok(())
}

fn fill(
    conn: &Connection,
    collection_id: &str,
    description: &str,
    sections: &[PlannedSection],
    requests: Vec<PlannedRequest>,
) -> Result<(), AppError> {
    descriptions::set(conn, Described::Collection, collection_id, description)?;
    let mut section_ids = Vec::with_capacity(sections.len());
    for section in sections {
        let id = collections::insert_section(conn, collection_id, &section.name)?.id;
        descriptions::set(conn, Described::Section, &id, &section.description)?;
        section_ids.push(id);
    }
    for request in requests {
        let section_id = request
            .section
            .and_then(|index| section_ids.get(index))
            .map(String::as_str);
        let saved = collections::insert_request(conn, collection_id, section_id, request.content)?;
        descriptions::set(conn, Described::Request, &saved.id, &request.description)?;
        for example in request.examples {
            examples::insert(conn, &saved.id, &example)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::vault::FixedKey;
    use crate::import::file::plan;
    use crate::storage::db::Database;

    #[test]
    fn imports_collection_sections_requests_and_secret_variables() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([9; 32])));
        let json = r#"{ "info": { "name": "Shop" },
            "variable": [{ "key": "baseUrl", "value": "https://shop.test" }, { "key": "key", "value": "k", "type": "secret" }],
            "item": [
              { "name": "Health", "request": { "method": "GET", "url": "{{baseUrl}}/health" } },
              { "name": "Orders", "item": [
                { "name": "List", "request": { "method": "GET", "url": "{{baseUrl}}/orders" },
                  "event": [{ "listen": "test", "script": { "exec": ["pm.test('ok', () => {});"] } }] }
              ] }
            ] }"#;

        let detail = db
            .with(|conn| {
                let result = apply(conn, &vault, plan(json)?, "Shop (imported)")?;
                assert_eq!(result.kind, ImportKind::Collection);
                let set = environments::collection_set_id(conn, &result.id)?;
                let variables = environments::variables(conn, &set)?;
                assert_eq!(variables.len(), 2);
                assert!(
                    variables
                        .iter()
                        .any(|v| v.key == "key" && v.is_secret && v.value.is_empty())
                );
                collections::collection_detail(conn, &result.id)
            })
            .expect("import");

        assert_eq!(detail.name, "Shop (imported)");
        assert_eq!(detail.sections.len(), 1);
        let names: Vec<_> = detail.requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Health", "List"]);
        assert!(detail.requests.iter().any(|r| r.section_id.is_some()));
    }

    #[test]
    fn imports_a_codestamp_file_next_to_the_collection_it_came_from() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([9; 32])));
        let json = r#"{ "info": { "name": "Shop" },
            "variable": [{ "key": "baseUrl", "value": "https://shop.test" }, { "key": "key", "value": "k", "type": "secret" }],
            "item": [
              { "name": "Health", "request": { "method": "GET", "url": "{{baseUrl}}/health" } },
              { "name": "Orders", "item": [
                { "name": "List", "request": { "method": "GET", "url": "{{baseUrl}}/orders" } }
              ] }
            ] }"#;
        db.with(|conn| {
            let original = apply(conn, &vault, plan(json)?, "Shop")?;
            collections::set_collection_icon(conn, &original.id, Some("🦊"))?;
            let staging = new_id();
            environments::put_owned(conn, &staging, "Staging", &original.id)?;
            let exported =
                crate::team::files::bundle(crate::team::files::from_db(conn, &original.id)?)?;
            let text = serde_json::to_string(&exported).map_err(|_| AppError::Storage)?;
            let checked = plan(&text)?;
            assert_eq!(checked.source, "Codestamp collection");
            assert_eq!(checked.warnings.len(), 1);

            let copy = apply(conn, &vault, checked, "Shop")?;
            assert_ne!(copy.id, original.id);
            let before = collections::collection_detail(conn, &original.id)?;
            let after = collections::collection_detail(conn, &copy.id)?;
            let names = |detail: &crate::storage::models::CollectionDetail| {
                detail
                    .requests
                    .iter()
                    .map(|request| (request.name.clone(), request.section_id.is_some()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(names(&after), names(&before));
            assert!(
                after
                    .requests
                    .iter()
                    .all(|request| before.requests.iter().all(|old| old.id != request.id))
            );
            assert_eq!(after.icon, "🦊");
            let owned = environments::owned(conn, &copy.id)?;
            assert_eq!(owned.len(), 1);
            assert!(
                owned
                    .iter()
                    .all(|(id, name)| id != &staging && name == "Staging")
            );
            let set = environments::collection_set_id(conn, &copy.id)?;
            let variables = environments::variables(conn, &set)?;
            assert!(
                variables
                    .iter()
                    .any(|v| v.key == "key" && v.is_secret && v.value.is_empty())
            );
            Ok(())
        })
        .expect("round trip");
    }

    #[test]
    fn replaces_a_collection_but_keeps_its_id_and_name() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([9; 32])));
        let first = r#"{ "info": { "name": "Old" },
            "variable": [{ "key": "old", "value": "1" }],
            "item": [{ "name": "Old folder", "item": [
              { "name": "Gone", "request": { "method": "GET", "url": "https://old.test/a" } }
            ] }] }"#;
        let second = r#"{ "info": { "name": "New", "description": "Fresh docs" },
            "variable": [{ "key": "baseUrl", "value": "https://new.test" }],
            "item": [
              { "name": "Health", "request": { "method": "GET", "url": "{{baseUrl}}/health" } },
              { "name": "Orders", "item": [
                { "name": "List", "request": { "method": "GET", "url": "{{baseUrl}}/orders" } }
              ] }
            ] }"#;
        let environment = r#"{ "name": "Staging", "values": [] }"#;

        db.with(|conn| {
            let created = apply(conn, &vault, plan(first)?, "Shop")?;
            let replaced = replace(conn, &vault, plan(second)?, &created.id)?;
            assert_eq!(replaced.id, created.id);
            assert!(replace(conn, &vault, plan(environment)?, &created.id).is_err());

            let detail = collections::collection_detail(conn, &created.id)?;
            assert_eq!(detail.name, "Shop");
            let names: Vec<_> = detail.requests.iter().map(|r| r.name.as_str()).collect();
            assert_eq!(names, ["Health", "List"]);
            let sections: Vec<_> = detail.sections.iter().map(|s| s.name.as_str()).collect();
            assert_eq!(sections, ["Orders"]);
            let docs = descriptions::get(conn, Described::Collection, &created.id)?;
            assert_eq!(docs, "Fresh docs");
            let set = environments::collection_set_id(conn, &created.id)?;
            let keys: Vec<_> = environments::variables(conn, &set)?
                .into_iter()
                .map(|v| v.key)
                .collect();
            assert_eq!(keys, ["baseUrl"]);
            Ok(())
        })
        .expect("replace");
    }

    #[test]
    fn imports_an_environment() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([9; 32])));
        let json = r#"{ "name": "Staging", "values": [{ "key": "baseUrl", "value": "https://staging.test", "enabled": true }] }"#;
        let list = db
            .with(|conn| {
                apply(conn, &vault, plan(json)?, "Staging")?;
                environments::list(conn)
            })
            .expect("import");
        assert!(
            list.environments
                .iter()
                .any(|e| e.name == "Staging" && e.variable_count == 1)
        );
    }
}
