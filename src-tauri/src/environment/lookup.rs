use std::collections::{BTreeSet, HashMap};

use url::Url;

use super::models::UrlPreview;
use super::repo;
use super::resolve::{Dynamic, collect_names, replace_known, substitute};
use super::vault::Vault;
use crate::auth::models::Auth;
use crate::error::AppError;
use crate::request::path_vars;
use crate::request::types::{HttpRequestInput, NameValue, RequestBody};
use crate::script::models::{VariableChange, VariableSnapshot};
use crate::storage::db::Database;

/// Fills in `{{variables}}` in the URL, query parameters, headers, path variables and body
/// before sending. A run's data row wins over the saved scopes, and `local` (`pm.variables.set`
/// values from the pre-request script) wins over everything.
pub fn resolve_request(
    db: &Database,
    vault: &Vault,
    input: HttpRequestInput,
    local: &HashMap<String, String>,
) -> Result<HttpRequestInput, AppError> {
    let names = referenced_names(&input);
    if names.is_empty() {
        return Ok(input);
    }

    let collection_id = input.collection_id.as_deref();
    let mut values =
        db.with(|conn| repo::lookup_values(conn, vault, &names, collection_id, true))?;
    for pair in &input.iteration_data {
        if names.contains(&pair.name) {
            values.insert(pair.name.clone(), pair.value.clone());
        }
    }
    for (key, value) in local {
        if names.contains(key) {
            values.insert(key.clone(), value.clone());
        }
    }
    let auth = match input.auth {
        Some(mut auth) => {
            for text in auth.templates_mut() {
                *text = substitute(text, &values)?;
            }
            Some(auth)
        }
        None => None,
    };
    let body = match input.body {
        Some(mut body) => {
            for text in body.templates_mut() {
                *text = substitute(text, &values)?;
            }
            Some(body)
        }
        None => None,
    };
    Ok(HttpRequestInput {
        url: substitute(&input.url, &values)?,
        params: substitute_pairs(&input.params, &values)?,
        headers: substitute_pairs(&input.headers, &values)?,
        path_variables: substitute_pairs(&input.path_variables, &values)?,
        body,
        auth,
        ..input
    })
}

/// Every `{{name}}` in what the request sends: URL, query parameters, headers, path variables,
/// body and auth.
pub fn referenced_names(input: &HttpRequestInput) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_names(&input.url, &mut names);
    for pair in input.params.iter().chain(&input.headers) {
        collect_names(&pair.name, &mut names);
        collect_names(&pair.value, &mut names);
    }
    for pair in &input.path_variables {
        collect_names(&pair.value, &mut names);
    }
    for text in input.body.iter().flat_map(RequestBody::templates) {
        collect_names(text, &mut names);
    }
    for text in input.auth.iter().flat_map(Auth::templates) {
        collect_names(text, &mut names);
    }
    names
}

/// Fills in `{{variables}}` in auth settings, for getting an OAuth 2.0 token from the Auth tab.
pub fn resolve_auth(
    db: &Database,
    vault: &Vault,
    mut auth: Auth,
    collection_id: Option<&str>,
) -> Result<Auth, AppError> {
    let mut names = BTreeSet::new();
    for text in auth.templates() {
        collect_names(text, &mut names);
    }
    if names.is_empty() {
        return Ok(auth);
    }
    let values = db.with(|conn| repo::lookup_values(conn, vault, &names, collection_id, true))?;
    for text in auth.templates_mut() {
        *text = substitute(text, &values)?;
    }
    Ok(auth)
}

/// Fills in `{{variables}}` in a single text, such as a WebSocket message.
pub fn resolve_text(
    db: &Database,
    vault: &Vault,
    text: String,
    collection_id: Option<&str>,
) -> Result<String, AppError> {
    let mut names = BTreeSet::new();
    collect_names(&text, &mut names);
    if names.is_empty() {
        return Ok(text);
    }
    let values = db.with(|conn| repo::lookup_values(conn, vault, &names, collection_id, true))?;
    substitute(&text, &values)
}

/// Every variable a script may read, per scope and decrypted.
pub fn script_variables(
    db: &Database,
    vault: &Vault,
    collection_id: Option<&str>,
) -> Result<VariableSnapshot, AppError> {
    db.with(|conn| repo::script_snapshot(conn, vault, collection_id))
}

/// Persists a script's variable changes; returns messages for changes that were skipped.
pub fn save_script_changes(
    db: &Database,
    vault: &Vault,
    collection_id: Option<&str>,
    changes: &[VariableChange],
) -> Result<Vec<String>, AppError> {
    if changes.is_empty() {
        return Ok(Vec::new());
    }
    db.with(|conn| repo::apply_script_changes(conn, vault, collection_id, changes))
}

/// The URL as it would be sent, with secrets masked and unknown variables listed. Path variables
/// still empty keep their `:name` here; sending asks for them.
pub fn preview_url(
    db: &Database,
    vault: &Vault,
    url: &str,
    params: &[NameValue],
    path_variables: &[NameValue],
    collection_id: Option<&str>,
) -> Result<UrlPreview, AppError> {
    let mut names = BTreeSet::new();
    collect_names(url, &mut names);
    for pair in params {
        collect_names(&pair.name, &mut names);
        collect_names(&pair.value, &mut names);
    }
    for pair in path_variables {
        collect_names(&pair.value, &mut names);
    }
    let values = db.with(|conn| repo::lookup_values(conn, vault, &names, collection_id, false))?;

    let (base, mut missing) = replace_known(url.trim(), &values, Dynamic::Keep);
    let mut filled = Vec::with_capacity(path_variables.len());
    for pair in path_variables.iter().filter(|pair| !pair.value.is_empty()) {
        let (value, missing_in_value) = replace_known(&pair.value, &values, Dynamic::Keep);
        missing.extend(missing_in_value);
        filled.push(NameValue {
            name: pair.name.clone(),
            value,
        });
    }
    let base = path_vars::fill(&base, &filled)?;
    let mut pairs = Vec::with_capacity(params.len());
    for pair in params {
        let (name, missing_in_name) = replace_known(pair.name.trim(), &values, Dynamic::Keep);
        let (value, missing_in_value) = replace_known(&pair.value, &values, Dynamic::Keep);
        missing.extend(missing_in_name);
        missing.extend(missing_in_value);
        pairs.push((name, value));
    }
    missing.sort();
    missing.dedup();

    let url = match Url::parse(&base) {
        Ok(mut parsed) if !pairs.is_empty() => {
            parsed.query_pairs_mut().extend_pairs(&pairs);
            parsed.to_string()
        }
        Ok(parsed) => parsed.to_string(),
        Err(_) => base,
    };
    Ok(UrlPreview { url, missing })
}

fn substitute_pairs(
    pairs: &[NameValue],
    values: &HashMap<String, String>,
) -> Result<Vec<NameValue>, AppError> {
    pairs
        .iter()
        .map(|pair| {
            Ok(NameValue {
                name: substitute(&pair.name, values)?,
                value: substitute(&pair.value, values)?,
            })
        })
        .collect()
}
