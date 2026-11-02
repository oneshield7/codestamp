use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params};

use super::dynamic::DYNAMIC_VARIABLES;
use super::models::{
    EnvironmentList, EnvironmentSummary, HintSource, VariableDetail, VariableHint, VariableInput,
    VariableScope, VariableView,
};
use super::resolve::is_valid_name;
use super::validate::MAX_VARIABLES;
use super::vault::Vault;
use crate::error::AppError;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::script::models::{ChangeScope, MAX_SCRIPT_CHANGES, VariableChange, VariableSnapshot};
use crate::storage::db::{new_id, now_ms};

pub const SECRET_MASK: &str = "••••••";
const HINT_PREVIEW_CHARS: usize = 60;
const MAX_SKIPPED_MESSAGES: usize = 20;
/// A restricted variable (`v`) whose group this device cannot read. Its value stays out of
/// requests, so the reference shows as unresolved instead of sending an empty value.
const LOCKED: &str = "(v.restricted IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM team_access_groups g WHERE g.id = v.restricted AND g.access = 1))";
/// The sets (`e`) whose variables apply to collection `?1`: the Globals of its workspace (of the
/// open one outside a collection), the collection's own variables, and the environment active in
/// that collection unless another collection owns it.
const APPLIES: &str = "((e.is_global = 1 AND e.workspace_id = COALESCE(
        (SELECT workspace_id FROM collections WHERE id = ?1),
        (SELECT id FROM current_workspace)))
    OR e.collection_id = ?1
    OR (e.collection_id IS NULL
        AND e.id = (SELECT environment_id FROM active_environments WHERE collection_id = ?1)
        AND (e.owner_collection_id IS NULL OR e.owner_collection_id = ?1)))";

struct StoredVariable {
    value: String,
    secret: Option<Vec<u8>>,
}

pub fn list(conn: &Connection) -> Result<EnvironmentList, AppError> {
    let mut statement = conn.prepare(
        "SELECT e.id, e.name, e.is_global, e.owner_collection_id, COUNT(v.id)
         FROM environments e LEFT JOIN variables v ON v.environment_id = e.id
         WHERE e.collection_id IS NULL AND e.workspace_id = (SELECT id FROM current_workspace)
         GROUP BY e.id ORDER BY e.is_global DESC, e.sort_order",
    )?;
    let environments = statement
        .query_map([], summary_from_row)?
        .collect::<Result<_, _>>()?;
    Ok(EnvironmentList {
        environments,
        active_ids: active_ids(conn)?,
    })
}

/// The environment active in a collection, while that collection may still use it.
fn active_for(conn: &Connection, collection_id: Option<&str>) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row(
            "SELECT e.id FROM active_environments a JOIN environments e ON e.id = a.environment_id
             WHERE a.collection_id = ?1
               AND (e.owner_collection_id IS NULL OR e.owner_collection_id = ?1)",
            params![collection_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Each collection's active environment, by collection id.
fn active_ids(conn: &Connection) -> Result<BTreeMap<String, String>, AppError> {
    let mut statement = conn.prepare(
        "SELECT a.collection_id, a.environment_id
         FROM active_environments a JOIN environments e ON e.id = a.environment_id
         WHERE e.owner_collection_id IS NULL OR e.owner_collection_id = a.collection_id",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Chooses the environment a collection uses. One that belongs to another collection is refused.
pub fn set_active(
    conn: &Connection,
    collection_id: &str,
    id: Option<&str>,
) -> Result<(), AppError> {
    ensure_collection(conn, collection_id)?;
    let Some(id) = id else {
        conn.execute(
            "DELETE FROM active_environments WHERE collection_id = ?1",
            params![collection_id],
        )?;
        return Ok(());
    };
    ensure_named_environment(conn, id)?;
    let usable = conn
        .query_row(
            "SELECT 1 FROM environments
             WHERE id = ?1 AND (owner_collection_id IS NULL OR owner_collection_id = ?2)",
            params![id, collection_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !usable {
        return Err(AppError::invalid(
            "This environment belongs to another collection",
        ));
    }
    let same_workspace = conn
        .query_row(
            "SELECT 1 FROM environments e JOIN collections c ON c.id = ?2
             WHERE e.id = ?1 AND e.workspace_id = c.workspace_id",
            params![id, collection_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !same_workspace {
        return Err(AppError::invalid(
            "This environment is in another workspace",
        ));
    }
    conn.execute(
        "INSERT INTO active_environments (collection_id, environment_id) VALUES (?1, ?2)
         ON CONFLICT(collection_id) DO UPDATE SET environment_id = excluded.environment_id",
        params![collection_id, id],
    )?;
    Ok(())
}

pub fn insert(conn: &Connection, name: &str) -> Result<EnvironmentSummary, AppError> {
    let id = new_id();
    conn.execute(
        "INSERT INTO environments (id, name, is_global, sort_order, created_at, workspace_id)
         VALUES (?1, ?2, 0, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM environments), ?3,
                 (SELECT id FROM current_workspace))",
        params![id, name, now_ms()],
    )?;
    Ok(EnvironmentSummary {
        id,
        name: name.to_owned(),
        is_global: false,
        collection_id: None,
        variable_count: 0,
    })
}

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EnvironmentSummary> {
    Ok(EnvironmentSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        is_global: row.get(2)?,
        collection_id: row.get(3)?,
        variable_count: row.get(4)?,
    })
}

/// Gives a named environment to a collection (shared with it) or back to this device only.
pub fn set_owner(conn: &Connection, id: &str, collection_id: Option<&str>) -> Result<(), AppError> {
    ensure_named_environment(conn, id)?;
    if let Some(collection_id) = collection_id {
        ensure_collection(conn, collection_id)?;
    }
    // Given to a collection, it lives in that collection's workspace.
    conn.execute(
        "UPDATE environments SET owner_collection_id = ?2,
             workspace_id = COALESCE((SELECT workspace_id FROM collections WHERE id = ?2),
                                     workspace_id)
         WHERE id = ?1",
        params![id, collection_id],
    )?;
    // Once it belongs to one collection, the others stop using it.
    if let Some(collection_id) = collection_id {
        conn.execute(
            "DELETE FROM active_environments WHERE environment_id = ?1 AND collection_id <> ?2",
            params![id, collection_id],
        )?;
    }
    Ok(())
}

/// The named environments that belong to a collection, by id, with their names.
pub fn owned(conn: &Connection, collection_id: &str) -> Result<Vec<(String, String)>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, name FROM environments
         WHERE owner_collection_id = ?1 AND collection_id IS NULL AND is_global = 0
         ORDER BY sort_order",
    )?;
    let rows = statement.query_map(params![collection_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Creates or renames an environment a shared collection brought in. An id already used by
/// something other than this collection's environments is refused, never taken over.
pub fn put_owned(
    conn: &Connection,
    id: &str,
    name: &str,
    collection_id: &str,
) -> Result<(), AppError> {
    let owner: Option<Option<String>> = conn
        .query_row(
            "SELECT owner_collection_id FROM environments
             WHERE id = ?1 AND collection_id IS NULL AND is_global = 0",
            params![id],
            |row| row.get(0),
        )
        .optional()?;
    let taken = conn
        .query_row(
            "SELECT 1 FROM environments WHERE id = ?1",
            params![id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    match owner {
        Some(Some(own)) if own == collection_id => {
            conn.execute(
                "UPDATE environments SET name = ?2 WHERE id = ?1",
                params![id, name],
            )?;
        }
        None if !taken => {
            conn.execute(
                "INSERT INTO environments
                     (id, name, is_global, owner_collection_id, sort_order, created_at, workspace_id)
                 VALUES (?1, ?2, 0, ?3,
                         (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM environments), ?4,
                         (SELECT workspace_id FROM collections WHERE id = ?3))",
                params![id, name, collection_id, now_ms()],
            )?;
        }
        _ => {
            return Err(AppError::invalid(
                "The shared collection has an environment whose id is used by another one here",
            ));
        }
    }
    Ok(())
}

/// Removes an environment the shared collection no longer has; no collection uses it after.
pub fn delete_owned(conn: &Connection, id: &str, collection_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM environments WHERE id = ?1 AND owner_collection_id = ?2",
        params![id, collection_id],
    )?;
    Ok(())
}

pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE environments SET name = ?2
         WHERE id = ?1 AND is_global = 0 AND collection_id IS NULL",
        params![id, name],
    )?;
    expect_environment_changed(changed)
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "DELETE FROM environments WHERE id = ?1 AND is_global = 0 AND collection_id IS NULL",
        params![id],
    )?;
    expect_environment_changed(changed)
}

pub fn variables(conn: &Connection, environment_id: &str) -> Result<Vec<VariableView>, AppError> {
    ensure_environment(conn, environment_id)?;
    let mut statement = conn.prepare(&format!(
        "SELECT v.id, v.key, v.value, v.secret IS NOT NULL, v.enabled, v.restricted, {LOCKED}
         FROM variables v WHERE v.environment_id = ?1 ORDER BY v.sort_order"
    ))?;
    let rows = statement.query_map(params![environment_id], |row| {
        Ok(VariableView {
            id: row.get(0)?,
            key: row.get(1)?,
            value: row.get(2)?,
            is_secret: row.get(3)?,
            enabled: row.get(4)?,
            restricted: row.get(5)?,
            locked: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Replaces the whole variable list of an environment in one transaction.
pub fn replace_variables(
    conn: &Connection,
    vault: &Vault,
    environment_id: &str,
    inputs: Vec<VariableInput>,
) -> Result<Vec<VariableView>, AppError> {
    ensure_environment(conn, environment_id)?;
    let existing = stored_variables(conn, environment_id)?;
    let mut kept = HashSet::with_capacity(inputs.len());

    let tx = conn.unchecked_transaction()?;
    for (position, input) in inputs.into_iter().enumerate() {
        // An id must belong to this environment; anything else is rejected, not re-homed.
        let previous = match &input.id {
            Some(id) => Some(
                existing
                    .get(id)
                    .ok_or_else(|| AppError::invalid("Unknown variable"))?,
            ),
            None => None,
        };
        let id = input.id.clone().unwrap_or_else(new_id);
        let (value, secret) = if input.is_secret {
            let blob = match (input.value.as_deref(), previous) {
                (Some(plain), _) => vault.encrypt(&id, plain)?,
                (
                    None,
                    Some(StoredVariable {
                        secret: Some(blob), ..
                    }),
                ) => blob.clone(),
                (None, Some(StoredVariable { value, .. })) => vault.encrypt(&id, value)?,
                (None, None) => vault.encrypt(&id, "")?,
            };
            (String::new(), Some(blob))
        } else {
            (input.value.unwrap_or_default(), None)
        };
        // The shared ciphertext only stays while the variable keeps its group.
        tx.execute(
            "INSERT INTO variables (id, environment_id, key, value, secret, enabled, sort_order,
                                    restricted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET key = excluded.key, value = excluded.value,
                 secret = excluded.secret, enabled = excluded.enabled,
                 sort_order = excluded.sort_order, restricted = excluded.restricted,
                 sealed = CASE WHEN excluded.restricted IS variables.restricted
                               THEN variables.sealed END",
            params![
                id,
                environment_id,
                input.key,
                value,
                secret,
                input.enabled,
                i64::try_from(position).unwrap_or(i64::MAX),
                input.restricted,
            ],
        )?;
        kept.insert(id);
    }
    for removed in existing.keys().filter(|id| !kept.contains(*id)) {
        tx.execute("DELETE FROM variables WHERE id = ?1", params![removed])?;
    }
    tx.commit()?;
    variables(conn, environment_id)
}

pub fn reveal(conn: &Connection, vault: &Vault, id: &str) -> Result<String, AppError> {
    let secret: Option<Vec<u8>> = conn
        .query_row(
            "SELECT secret FROM variables WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(AppError::NotFound("Variable"))?;
    let blob = secret.ok_or_else(|| AppError::invalid("This variable is not secret"))?;
    vault.decrypt(id, &blob)
}

/// The Globals a collection's requests use: those of its workspace, or of the open one.
pub fn globals_for(conn: &Connection, collection_id: Option<&str>) -> Result<String, AppError> {
    Ok(conn.query_row(
        &format!("SELECT e.id FROM environments e WHERE e.is_global = 1 AND {APPLIES}"),
        params![collection_id],
        |row| row.get(0),
    )?)
}

/// The variable set that belongs to a collection, created the first time it is needed.
pub fn collection_set_id(conn: &Connection, collection_id: &str) -> Result<String, AppError> {
    if let Some(id) = find_collection_set(conn, collection_id)? {
        return Ok(id);
    }
    conn.query_row(
        "SELECT 1 FROM collections WHERE id = ?1",
        params![collection_id],
        |_| Ok(()),
    )
    .optional()?
    .ok_or(AppError::NotFound("Collection"))?;
    let id = new_id();
    conn.execute(
        "INSERT INTO environments (id, name, is_global, sort_order, created_at, collection_id)
         VALUES (?1, 'Collection variables', 0, 0, ?2, ?3)",
        params![id, now_ms(), collection_id],
    )?;
    Ok(id)
}

/// The collection's variable set if it has one; unlike `collection_set_id`, never creates it.
pub fn find_collection_set(
    conn: &Connection,
    collection_id: &str,
) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row(
            "SELECT id FROM environments WHERE collection_id = ?1",
            params![collection_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Copies a collection's variables onto another collection. Secrets are decrypted and encrypted
/// again, because each one is bound to its own variable id.
pub fn copy_collection_variables(
    conn: &Connection,
    vault: &Vault,
    from_collection: &str,
    to_collection: &str,
) -> Result<(), AppError> {
    let Some(source) = find_collection_set(conn, from_collection)? else {
        return Ok(());
    };
    let mut statement = conn.prepare(
        "SELECT id, key, value, secret, enabled FROM variables
         WHERE environment_id = ?1 ORDER BY sort_order",
    )?;
    let rows = statement
        .query_map(params![source], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<Vec<u8>>>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Ok(());
    }
    let inputs = rows
        .into_iter()
        .map(|(id, key, value, secret, enabled)| {
            let (value, is_secret) = match secret {
                Some(blob) => (vault.decrypt(&id, &blob)?, true),
                None => (value, false),
            };
            Ok(VariableInput {
                id: None,
                key,
                value: Some(value),
                is_secret,
                enabled,
                restricted: None,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let target = collection_set_id(conn, to_collection)?;
    replace_variables(conn, vault, &target, inputs).map(|_| ())
}

/// A named environment or Globals (never a collection's variable set).
pub fn summary(conn: &Connection, id: &str) -> Result<EnvironmentSummary, AppError> {
    conn.query_row(
        "SELECT e.id, e.name, e.is_global, e.owner_collection_id,
                (SELECT COUNT(*) FROM variables v WHERE v.environment_id = e.id)
         FROM environments e WHERE e.id = ?1 AND e.collection_id IS NULL",
        params![id],
        summary_from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("Environment"))
}

struct EffectiveVariable {
    id: String,
    value: String,
    secret: Option<Vec<u8>>,
    scope: VariableScope,
}

/// Enabled variables that apply to a request, keyed by name. Later scopes override earlier
/// ones: Globals, then the request's collection, then the active environment.
fn effective_variables(
    conn: &Connection,
    collection_id: Option<&str>,
) -> Result<HashMap<String, EffectiveVariable>, AppError> {
    Ok(scoped_variables(conn, collection_id)?.into_iter().collect())
}

/// The same variables per scope, decrypted, for `pm.globals` / `pm.collectionVariables` /
/// `pm.environment` in scripts. Only built when a script actually runs.
pub fn script_snapshot(
    conn: &Connection,
    vault: &Vault,
    collection_id: Option<&str>,
) -> Result<VariableSnapshot, AppError> {
    let mut snapshot = VariableSnapshot::default();
    for (key, variable) in scoped_variables(conn, collection_id)? {
        let value = match &variable.secret {
            Some(blob) => vault.decrypt(&variable.id, blob)?,
            None => variable.value,
        };
        let target = match variable.scope {
            VariableScope::Global => &mut snapshot.globals,
            VariableScope::Collection => &mut snapshot.collection,
            VariableScope::Environment => &mut snapshot.environment,
        };
        target.insert(key, value);
    }
    Ok(snapshot)
}

/// Saves `pm.*.set` / `unset` calls from a script. Changes that cannot be applied (no active
/// environment, invalid name, full set) are skipped and returned as messages for the console.
/// Only the first `MAX_SCRIPT_CHANGES` are applied, whatever the script sent.
pub fn apply_script_changes(
    conn: &Connection,
    vault: &Vault,
    collection_id: Option<&str>,
    changes: &[VariableChange],
) -> Result<Vec<String>, AppError> {
    let mut skipped = Vec::new();
    let tx = conn.unchecked_transaction()?;
    for change in changes.iter().take(MAX_SCRIPT_CHANGES) {
        if !is_valid_name(&change.key)
            || change
                .value
                .as_ref()
                .is_some_and(|value| value.len() > MAX_KEY_VALUE_LENGTH)
        {
            skipped.push(format!(
                "Skipped variable “{}”: invalid name or value too long",
                change.key
            ));
            continue;
        }
        let target = match change.scope {
            ChangeScope::Globals => Some(globals_for(&tx, collection_id)?),
            ChangeScope::Collection => collection_id
                .map(|id| collection_set_id(&tx, id))
                .transpose()?,
            ChangeScope::Environment => active_for(&tx, collection_id)?,
        };
        let Some(environment_id) = target else {
            skipped.push(format!(
                "Skipped “{}”: {}",
                change.key,
                match change.scope {
                    ChangeScope::Environment => "no environment of this collection is active",
                    _ => "this request is not in a collection",
                }
            ));
            continue;
        };
        let saved = upsert_variable(
            &tx,
            vault,
            &environment_id,
            &change.key,
            change.value.as_deref(),
        )?;
        if !saved {
            skipped.push(format!(
                "Skipped “{}”: a variable set can hold up to {MAX_VARIABLES} variables",
                change.key
            ));
        }
    }
    tx.commit()?;
    if skipped.len() > MAX_SKIPPED_MESSAGES {
        let more = skipped.len() - MAX_SKIPPED_MESSAGES;
        skipped.truncate(MAX_SKIPPED_MESSAGES);
        skipped.push(format!("…and {more} more skipped variable changes"));
    }
    Ok(skipped)
}

/// Updates a variable by name (keeping it secret if it was), adds it, or removes it.
/// Returns false when a new variable does not fit because the set is full.
fn upsert_variable(
    conn: &Connection,
    vault: &Vault,
    environment_id: &str,
    key: &str,
    value: Option<&str>,
) -> Result<bool, AppError> {
    let existing: Option<(String, bool)> = conn
        .query_row(
            "SELECT id, secret IS NOT NULL FROM variables WHERE environment_id = ?1 AND key = ?2",
            params![environment_id, key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match (existing, value) {
        (Some((id, _)), None) => {
            conn.execute("DELETE FROM variables WHERE id = ?1", params![id])?;
        }
        (Some((id, true)), Some(value)) => {
            conn.execute(
                "UPDATE variables SET secret = ?2 WHERE id = ?1",
                params![id, vault.encrypt(&id, value)?],
            )?;
        }
        (Some((id, false)), Some(value)) => {
            conn.execute(
                "UPDATE variables SET value = ?2 WHERE id = ?1",
                params![id, value],
            )?;
        }
        (None, Some(value)) => {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM variables WHERE environment_id = ?1",
                params![environment_id],
                |row| row.get(0),
            )?;
            if usize::try_from(count).unwrap_or(usize::MAX) >= MAX_VARIABLES {
                return Ok(false);
            }
            conn.execute(
                "INSERT INTO variables (id, environment_id, key, value, secret, enabled, sort_order)
                 VALUES (?1, ?2, ?3, ?4, NULL, 1,
                         (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM variables
                          WHERE environment_id = ?2))",
                params![new_id(), environment_id, key, value],
            )?;
        }
        (None, None) => {}
    }
    Ok(true)
}

fn scoped_variables(
    conn: &Connection,
    collection_id: Option<&str>,
) -> Result<Vec<(String, EffectiveVariable)>, AppError> {
    let mut statement = conn.prepare(&format!(
        "SELECT v.id, v.key, v.value, v.secret,
                CASE WHEN e.is_global = 1 THEN 0
                     WHEN e.collection_id IS NOT NULL THEN 1
                     ELSE 2 END AS rank
         FROM variables v JOIN environments e ON e.id = v.environment_id
         WHERE v.enabled = 1 AND NOT {LOCKED} AND {APPLIES}
         ORDER BY rank, v.sort_order"
    ))?;
    let rows = statement.query_map(params![collection_id], |row| {
        let rank: i64 = row.get(4)?;
        Ok((
            row.get::<_, String>(1)?,
            EffectiveVariable {
                id: row.get(0)?,
                value: row.get(2)?,
                secret: row.get(3)?,
                scope: match rank {
                    0 => VariableScope::Global,
                    1 => VariableScope::Collection,
                    _ => VariableScope::Environment,
                },
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Values for the requested names. Secrets are decrypted only when `reveal_secrets` is set
/// and the name is actually used; otherwise they are shown as a mask.
pub fn lookup_values(
    conn: &Connection,
    vault: &Vault,
    names: &BTreeSet<String>,
    collection_id: Option<&str>,
    reveal_secrets: bool,
) -> Result<HashMap<String, String>, AppError> {
    let mut values = HashMap::new();
    for (key, variable) in effective_variables(conn, collection_id)? {
        if !names.contains(&key) {
            continue;
        }
        let resolved = match (&variable.secret, reveal_secrets) {
            (Some(blob), true) => vault.decrypt(&variable.id, blob)?,
            (Some(_), false) => SECRET_MASK.to_owned(),
            (None, _) => variable.value,
        };
        values.insert(key, resolved);
    }
    Ok(values)
}

/// Names available for autocomplete, with a short preview that never contains a secret.
/// The built-in dynamic variables come last.
pub fn hints(
    conn: &Connection,
    collection_id: Option<&str>,
) -> Result<Vec<VariableHint>, AppError> {
    let mut hints: Vec<VariableHint> = effective_variables(conn, collection_id)?
        .into_iter()
        .map(|(name, variable)| VariableHint {
            name,
            scope: variable.scope.into(),
            is_secret: variable.secret.is_some(),
            preview: if variable.secret.is_some() {
                SECRET_MASK.to_owned()
            } else {
                variable.value.chars().take(HINT_PREVIEW_CHARS).collect()
            },
        })
        .collect();
    hints.sort_by(|a, b| a.name.cmp(&b.name));
    hints.extend(
        DYNAMIC_VARIABLES
            .iter()
            .map(|(name, description)| VariableHint {
                name: (*name).to_owned(),
                scope: HintSource::Dynamic,
                is_secret: false,
                preview: (*description).to_owned(),
            }),
    );
    Ok(hints)
}

/// Where `{{name}}` gets its value in a collection, by the same rules as sending. A restricted
/// variable this device cannot read only shows when nothing else defines the name.
pub fn detail(
    conn: &Connection,
    collection_id: Option<&str>,
    name: &str,
) -> Result<VariableDetail, AppError> {
    if let Some((_, description)) = DYNAMIC_VARIABLES.iter().find(|(known, _)| *known == name) {
        return Ok(VariableDetail {
            source: Some(HintSource::Dynamic),
            value: (*description).to_owned(),
            ..VariableDetail::undefined(name)
        });
    }
    let found = conn
        .query_row(
            &format!(
                "SELECT v.id, v.value, v.secret IS NOT NULL, v.restricted IS NOT NULL, {LOCKED},
                        e.name, COALESCE(e.collection_id, e.owner_collection_id),
                        CASE WHEN e.is_global = 1 THEN 0
                             WHEN e.collection_id IS NOT NULL THEN 1
                             ELSE 2 END AS rank
                 FROM variables v JOIN environments e ON e.id = v.environment_id
                 WHERE v.key = ?2 AND v.enabled = 1 AND {APPLIES}
                 ORDER BY {LOCKED}, rank DESC LIMIT 1"
            ),
            params![collection_id, name],
            |row| {
                let rank: i64 = row.get(7)?;
                let is_secret: bool = row.get(2)?;
                Ok(VariableDetail {
                    source: Some(match rank {
                        0 => HintSource::Global,
                        1 => HintSource::Collection,
                        _ => HintSource::Environment,
                    }),
                    id: Some(row.get(0)?),
                    set_name: if rank == 2 { Some(row.get(5)?) } else { None },
                    value: if is_secret {
                        String::new()
                    } else {
                        row.get(1)?
                    },
                    is_secret,
                    restricted: row.get(3)?,
                    locked: row.get(4)?,
                    shared_by: row.get(6)?,
                    ..VariableDetail::undefined(name)
                })
            },
        )
        .optional()?;
    Ok(found.unwrap_or_else(|| VariableDetail::undefined(name)))
}

/// The inputs that give one variable a new value and keep everything else in its set as it is
/// (other secrets stay encrypted), with the set's id.
pub fn with_new_value(
    conn: &Connection,
    variable_id: &str,
    value: &str,
) -> Result<(String, Vec<VariableInput>), AppError> {
    let set_id: String = conn
        .query_row(
            "SELECT environment_id FROM variables WHERE id = ?1",
            params![variable_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(AppError::NotFound("Variable"))?;
    let inputs = variables(conn, &set_id)?
        .into_iter()
        .map(|variable| VariableInput {
            value: if variable.id == variable_id {
                Some(value.to_owned())
            } else if variable.is_secret {
                None
            } else {
                Some(variable.value)
            },
            id: Some(variable.id),
            key: variable.key,
            is_secret: variable.is_secret,
            enabled: variable.enabled,
            restricted: variable.restricted,
        })
        .collect();
    Ok((set_id, inputs))
}

fn stored_variables(
    conn: &Connection,
    environment_id: &str,
) -> Result<HashMap<String, StoredVariable>, AppError> {
    let mut statement =
        conn.prepare("SELECT id, value, secret FROM variables WHERE environment_id = ?1")?;
    let rows = statement.query_map(params![environment_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            StoredVariable {
                value: row.get(1)?,
                secret: row.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn ensure_environment(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.query_row(
        "SELECT 1 FROM environments WHERE id = ?1",
        params![id],
        |_| Ok(()),
    )
    .optional()?
    .ok_or(AppError::NotFound("Environment"))
}

fn ensure_collection(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.query_row(
        "SELECT 1 FROM collections WHERE id = ?1",
        params![id],
        |_| Ok(()),
    )
    .optional()?
    .ok_or(AppError::NotFound("Collection"))
}

fn ensure_named_environment(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.query_row(
        "SELECT 1 FROM environments WHERE id = ?1 AND is_global = 0 AND collection_id IS NULL",
        params![id],
        |_| Ok(()),
    )
    .optional()?
    .ok_or(AppError::NotFound("Environment"))
}

const fn expect_environment_changed(changed: usize) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::NotFound("Environment"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::vault::FixedKey;
    use crate::storage::db::Database;

    fn setup() -> (Database, Vault) {
        (
            Database::in_memory().expect("database"),
            Vault::new(Box::new(FixedKey([3; 32]))),
        )
    }

    fn var(id: Option<&str>, key: &str, value: Option<&str>, is_secret: bool) -> VariableInput {
        VariableInput {
            id: id.map(str::to_owned),
            key: key.to_owned(),
            value: value.map(str::to_owned),
            is_secret,
            enabled: true,
            restricted: None,
        }
    }

    fn globals_id(conn: &Connection) -> String {
        globals_for(conn, None).expect("globals exist")
    }

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn migration_creates_globals_and_no_active_environment() {
        let (db, _) = setup();
        let list = db.with(list).expect("list");
        assert_eq!(list.environments.len(), 1);
        assert!(list.environments.first().is_some_and(|e| e.is_global));
        assert!(list.active_ids.is_empty());
    }

    #[test]
    fn secrets_are_encrypted_at_rest_and_hidden_from_views() {
        let (db, vault) = setup();
        db.with(|conn| {
            let env = insert(conn, "Development")?;
            let views = replace_variables(
                conn,
                &vault,
                &env.id,
                vec![var(None, "token", Some("super-secret-token"), true)],
            )?;
            let view = views.first().ok_or(AppError::Storage)?;
            assert!(view.is_secret);
            assert_eq!(view.value, "");

            let raw: Vec<u8> = conn.query_row("SELECT secret FROM variables", [], |r| r.get(0))?;
            assert!(!raw.windows(6).any(|w| w == b"secret"));
            assert_eq!(reveal(conn, &vault, &view.id)?, "super-secret-token");
            Ok(())
        })
        .expect("secret flow");
    }

    #[test]
    fn keeps_an_unchanged_secret_when_value_is_omitted() {
        let (db, vault) = setup();
        db.with(|conn| {
            let env = insert(conn, "Development")?;
            let first = replace_variables(
                conn,
                &vault,
                &env.id,
                vec![var(None, "token", Some("abc"), true)],
            )?;
            let id = first
                .first()
                .map(|v| v.id.clone())
                .ok_or(AppError::Storage)?;
            replace_variables(
                conn,
                &vault,
                &env.id,
                vec![var(Some(&id), "token", None, true)],
            )?;
            assert_eq!(reveal(conn, &vault, &id)?, "abc");
            Ok(())
        })
        .expect("unchanged secret");
    }

    #[test]
    fn rejects_variable_ids_from_another_environment() {
        let (db, vault) = setup();
        let result = db.with(|conn| {
            let first = insert(conn, "First")?;
            let second = insert(conn, "Second")?;
            let created = replace_variables(
                conn,
                &vault,
                &first.id,
                vec![var(None, "a", Some("1"), false)],
            )?;
            let foreign = created
                .first()
                .map(|v| v.id.clone())
                .ok_or(AppError::Storage)?;
            replace_variables(
                conn,
                &vault,
                &second.id,
                vec![var(Some(&foreign), "a", Some("2"), false)],
            )
        });
        assert!(matches!(result, Err(AppError::InvalidInput(_))));
    }

    #[test]
    fn script_changes_update_secrets_in_place_and_skip_missing_targets() {
        let (db, vault) = setup();
        db.with(|conn| {
            let dev = insert(conn, "Development")?;
            let saved = replace_variables(
                conn,
                &vault,
                &dev.id,
                vec![
                    var(None, "token", Some("old"), true),
                    var(None, "gone", Some("x"), false),
                ],
            )?;
            let token_id = saved
                .first()
                .map(|v| v.id.clone())
                .ok_or(AppError::Storage)?;
            let change = |scope, key: &str, value: Option<&str>| VariableChange {
                scope,
                key: key.to_owned(),
                value: value.map(str::to_owned),
            };

            let users = crate::storage::repo::insert_collection(conn, "Users")?;
            let skipped = apply_script_changes(
                conn,
                &vault,
                Some(&users.id),
                &[change(ChangeScope::Environment, "token", Some("new"))],
            )?;
            assert_eq!(skipped.len(), 1, "no active environment yet");
            let skipped = apply_script_changes(
                conn,
                &vault,
                None,
                &[change(ChangeScope::Collection, "nope", Some("1"))],
            )?;
            assert_eq!(skipped.len(), 1, "not in a collection");

            set_active(conn, &users.id, Some(&dev.id))?;
            let skipped = apply_script_changes(
                conn,
                &vault,
                Some(&users.id),
                &[
                    change(ChangeScope::Environment, "token", Some("new")),
                    change(ChangeScope::Environment, "gone", None),
                    change(ChangeScope::Environment, "added", Some("1")),
                    change(ChangeScope::Globals, "bad name", Some("1")),
                ],
            )?;
            assert_eq!(skipped.len(), 1);
            assert_eq!(reveal(conn, &vault, &token_id)?, "new");
            let keys: Vec<String> = variables(conn, &dev.id)?
                .into_iter()
                .map(|v| v.key)
                .collect();
            assert_eq!(keys, ["token", "added"]);

            let snapshot = script_snapshot(conn, &vault, Some(&users.id))?;
            assert_eq!(
                snapshot.environment.get("token").map(String::as_str),
                Some("new")
            );
            Ok(())
        })
        .expect("script changes");
    }

    #[test]
    fn collection_variables_sit_between_globals_and_the_active_environment() {
        let (db, vault) = setup();
        let (values, masked, hints, listed) = db
            .with(|conn| {
                let collection = crate::storage::repo::insert_collection(conn, "Users API")?;
                let globals = globals_id(conn);
                replace_variables(
                    conn,
                    &vault,
                    &globals,
                    vec![
                        var(None, "baseUrl", Some("https://global"), false),
                        var(None, "region", Some("id"), false),
                    ],
                )?;
                let set = collection_set_id(conn, &collection.id)?;
                assert_eq!(collection_set_id(conn, &collection.id)?, set);
                replace_variables(
                    conn,
                    &vault,
                    &set,
                    vec![
                        var(None, "baseUrl", Some("https://collection"), false),
                        var(None, "region", Some("sg"), false),
                        var(None, "apiKey", Some("k3y"), true),
                    ],
                )?;
                let dev = insert(conn, "Development")?;
                replace_variables(
                    conn,
                    &vault,
                    &dev.id,
                    vec![var(None, "region", Some("jp"), false)],
                )?;
                set_active(conn, &collection.id, Some(&dev.id))?;
                let wanted = names(&["baseUrl", "region", "apiKey"]);
                Ok((
                    lookup_values(conn, &vault, &wanted, Some(&collection.id), true)?,
                    lookup_values(conn, &vault, &wanted, Some(&collection.id), false)?,
                    hints(conn, Some(&collection.id))?,
                    list(conn)?,
                ))
            })
            .expect("scopes");

        assert_eq!(
            values.get("baseUrl").map(String::as_str),
            Some("https://collection")
        );
        assert_eq!(values.get("region").map(String::as_str), Some("jp"));
        assert_eq!(values.get("apiKey").map(String::as_str), Some("k3y"));
        assert_eq!(masked.get("apiKey").map(String::as_str), Some(SECRET_MASK));
        assert!(
            hints
                .iter()
                .any(|h| h.name == "apiKey" && h.preview == SECRET_MASK && h.is_secret)
        );
        assert!(
            hints
                .iter()
                .any(|h| h.name == "region" && h.scope == HintSource::Environment)
        );
        assert!(
            hints
                .iter()
                .any(|h| h.name == "$guid" && h.scope == HintSource::Dynamic)
        );
        assert_eq!(
            listed.environments.len(),
            2,
            "collection sets stay out of the list"
        );
    }

    #[test]
    fn active_environment_overrides_globals_and_disabled_values_are_ignored() {
        let (db, vault) = setup();
        let values = db
            .with(|conn| {
                let globals = globals_id(conn);
                replace_variables(
                    conn,
                    &vault,
                    &globals,
                    vec![
                        var(None, "baseUrl", Some("https://global"), false),
                        var(None, "region", Some("id"), false),
                    ],
                )?;
                let dev = insert(conn, "Development")?;
                let mut disabled = var(None, "region", Some("sg"), false);
                disabled.enabled = false;
                replace_variables(
                    conn,
                    &vault,
                    &dev.id,
                    vec![
                        var(None, "baseUrl", Some("https://dev"), false),
                        disabled,
                        var(None, "token", Some("t0k"), true),
                    ],
                )?;
                let users = crate::storage::repo::insert_collection(conn, "Users")?;
                set_active(conn, &users.id, Some(&dev.id))?;
                lookup_values(
                    conn,
                    &vault,
                    &names(&["baseUrl", "region", "token"]),
                    Some(&users.id),
                    true,
                )
            })
            .expect("lookup");
        assert_eq!(
            values.get("baseUrl").map(String::as_str),
            Some("https://dev")
        );
        assert_eq!(values.get("region").map(String::as_str), Some("id"));
        assert_eq!(values.get("token").map(String::as_str), Some("t0k"));
    }

    #[test]
    fn deleting_the_active_environment_clears_the_selection_and_globals_cannot_be_deleted() {
        let (db, _) = setup();
        db.with(|conn| {
            let users = crate::storage::repo::insert_collection(conn, "Users")?;
            let dev = insert(conn, "Development")?;
            set_active(conn, &users.id, Some(&dev.id))?;
            assert_eq!(
                list(conn)?.active_ids.get(&users.id).map(String::as_str),
                Some(dev.id.as_str())
            );
            delete(conn, &dev.id)?;
            assert!(list(conn)?.active_ids.is_empty());
            let globals = globals_id(conn);
            assert!(matches!(delete(conn, &globals), Err(AppError::NotFound(_))));
            assert!(matches!(
                set_active(conn, &users.id, Some(&globals)),
                Err(AppError::NotFound(_))
            ));
            Ok(())
        })
        .expect("delete flow");
    }

    #[test]
    fn another_collections_environment_never_fills_in_values() {
        let (db, vault) = setup();
        db.with(|conn| {
            let orders = crate::storage::repo::insert_collection(conn, "Orders")?;
            let users = crate::storage::repo::insert_collection(conn, "Users")?;
            let staging = insert(conn, "Orders staging")?;
            set_owner(conn, &staging.id, Some(&orders.id))?;
            replace_variables(
                conn,
                &vault,
                &staging.id,
                vec![var(None, "region", Some("sg"), false)],
            )?;
            set_active(conn, &orders.id, Some(&staging.id))?;
            assert!(matches!(
                set_active(conn, &users.id, Some(&staging.id)),
                Err(AppError::InvalidInput(_))
            ));
            let wanted = names(&["region"]);

            let own = lookup_values(conn, &vault, &wanted, Some(&orders.id), true)?;
            assert_eq!(own.get("region").map(String::as_str), Some("sg"));
            assert!(lookup_values(conn, &vault, &wanted, Some(&users.id), true)?.is_empty());
            assert!(lookup_values(conn, &vault, &wanted, None, true)?.is_empty());
            assert!(
                hints(conn, Some(&users.id))?
                    .iter()
                    .all(|h| h.name != "region")
            );
            assert_eq!(detail(conn, Some(&users.id), "region")?.source, None);
            let here = detail(conn, Some(&orders.id), "region")?;
            assert_eq!(here.source, Some(HintSource::Environment));
            assert_eq!(here.set_name.as_deref(), Some("Orders staging"));
            assert_eq!(here.shared_by.as_deref(), Some(orders.id.as_str()));

            let change = VariableChange {
                scope: ChangeScope::Environment,
                key: "token".to_owned(),
                value: Some("t".to_owned()),
            };
            let skipped = apply_script_changes(conn, &vault, Some(&users.id), &[change])?;
            assert!(
                skipped
                    .iter()
                    .any(|m| m.contains("no environment of this collection"))
            );

            // Kept on this device, it can be chosen in any collection, and each one keeps its own
            // choice; giving it to a collection again ends its use everywhere else.
            set_owner(conn, &staging.id, None)?;
            assert!(lookup_values(conn, &vault, &wanted, Some(&users.id), true)?.is_empty());
            set_active(conn, &users.id, Some(&staging.id))?;
            let shared = lookup_values(conn, &vault, &wanted, Some(&users.id), true)?;
            assert_eq!(shared.get("region").map(String::as_str), Some("sg"));
            set_owner(conn, &staging.id, Some(&orders.id))?;
            let actives = list(conn)?.active_ids;
            assert_eq!(actives.len(), 1);
            assert_eq!(actives.get(&orders.id), Some(&staging.id));
            Ok(())
        })
        .expect("environment owners");
    }

    #[test]
    fn a_new_value_from_the_popover_keeps_the_rest_of_the_set() {
        let (db, vault) = setup();
        db.with(|conn| {
            let collection = crate::storage::repo::insert_collection(conn, "Users")?;
            let set = collection_set_id(conn, &collection.id)?;
            replace_variables(
                conn,
                &vault,
                &set,
                vec![
                    var(None, "baseUrl", Some("https://old.test"), false),
                    var(None, "token", Some("s3cr3t"), true),
                ],
            )?;
            let base = detail(conn, Some(&collection.id), "baseUrl")?;
            assert_eq!(base.source, Some(HintSource::Collection));
            assert_eq!(base.value, "https://old.test");
            assert_eq!(base.shared_by.as_deref(), Some(collection.id.as_str()));
            let token = detail(conn, Some(&collection.id), "token")?;
            assert!(token.is_secret);
            assert_eq!(token.value, "");

            let id = base.id.ok_or(AppError::Storage)?;
            let (set_id, inputs) = with_new_value(conn, &id, "https://new.test")?;
            assert_eq!(set_id, set);
            replace_variables(conn, &vault, &set_id, inputs)?;
            let wanted = names(&["baseUrl", "token"]);
            let values = lookup_values(conn, &vault, &wanted, Some(&collection.id), true)?;
            assert_eq!(
                values.get("baseUrl").map(String::as_str),
                Some("https://new.test")
            );
            assert_eq!(values.get("token").map(String::as_str), Some("s3cr3t"));

            assert_eq!(
                detail(conn, Some(&collection.id), "$guid")?.source,
                Some(HintSource::Dynamic)
            );
            assert_eq!(detail(conn, Some(&collection.id), "missing")?.source, None);
            assert!(matches!(
                with_new_value(conn, "no-such-variable", "x"),
                Err(AppError::NotFound(_))
            ));
            Ok(())
        })
        .expect("popover edit");
    }
}
