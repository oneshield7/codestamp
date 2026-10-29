//! Loads what a run needs into a database that lives only as long as the run, so nothing on
//! this machine (the app's own database included) is read or changed.

use std::fs;
use std::path::Path;

use super::args::RunOptions;
use crate::environment::models::VariableInput;
use crate::environment::repo as environments;
use crate::environment::validate::check_variables;
use crate::environment::vault::{EphemeralKey, Vault};
use crate::error::AppError;
use crate::import::file::plan;
use crate::import::models::{ImportKind, ImportPlan};
use crate::import::postman::MAX_IMPORT_BYTES;
use crate::import::run::apply;
use crate::storage::db::Database;
use crate::team::apply as team;
use crate::team::files;

/// Variables given with `--var` go into this environment when no environment file is given.
const COMMAND_LINE_ENVIRONMENT: &str = "Command line";

pub struct Loaded {
    pub db: Database,
    pub vault: Vault,
    pub collection_id: String,
    pub collection_name: String,
    pub environment_name: Option<String>,
    /// Things the import could not carry over, worth a look but not fatal.
    pub warnings: Vec<String>,
}

pub fn load(options: &RunOptions) -> Result<Loaded, String> {
    let db = Database::in_memory().map_err(|error| error.to_string())?;
    let vault = Vault::new(Box::new(EphemeralKey::generate()));
    let mut warnings = Vec::new();
    let path = &options.collection;
    let (collection_id, collection_name) = if path.is_dir() {
        let snapshot = files::read(path).map_err(|error| located(path, &error))?;
        db.with(|conn| {
            team::apply(conn, &snapshot, None)?;
            team::apply_variables(conn, &vault, &snapshot)
        })
        .map_err(|error| located(path, &error))?;
        (snapshot.collection.id, snapshot.collection.name)
    } else {
        let planned = read_plan(path, ImportKind::Collection, &mut warnings)?;
        let name = planned.name.clone();
        let imported = db
            .with(|conn| apply(conn, &vault, planned, &name))
            .map_err(|error| located(path, &error))?;
        (imported.id, name)
    };

    let mut environment = None;
    if let Some(path) = &options.environment {
        let planned = read_plan(path, ImportKind::Environment, &mut warnings)?;
        let name = planned.name.clone();
        let imported = db
            .with(|conn| apply(conn, &vault, planned, &name))
            .map_err(|error| located(path, &error))?;
        environment = Some((imported.id, name));
    }
    if !options.variables.is_empty() {
        let (id, _) = match &environment {
            Some(found) => found,
            None => environment.insert(
                db.with(|conn| environments::insert(conn, COMMAND_LINE_ENVIRONMENT))
                    .map(|created| (created.id, COMMAND_LINE_ENVIRONMENT.to_owned()))
                    .map_err(|error| error.to_string())?,
            ),
        };
        db.with(|conn| set_variables(conn, &vault, id, &options.variables))
            .map_err(|error| format!("--var: {error}"))?;
    }
    if let Some((id, _)) = &environment {
        db.with(|conn| environments::set_active(conn, Some(id)))
            .map_err(|error| error.to_string())?;
    }
    Ok(Loaded {
        db,
        vault,
        collection_id,
        collection_name,
        environment_name: environment.map(|(_, name)| name),
        warnings,
    })
}

fn read_plan(
    path: &Path,
    wanted: ImportKind,
    warnings: &mut Vec<String>,
) -> Result<ImportPlan, String> {
    let size = fs::metadata(path)
        .map_err(|_| format!("{}: the file cannot be read", path.display()))?
        .len();
    if usize::try_from(size).map_or(true, |size| size > MAX_IMPORT_BYTES) {
        return Err(format!("{}: files can be up to 20 MB", path.display()));
    }
    let text = fs::read_to_string(path)
        .map_err(|_| format!("{}: the file cannot be read as UTF-8 text", path.display()))?;
    let planned = plan(&text).map_err(|error| located(path, &error))?;
    if planned.kind != wanted {
        return Err(match wanted {
            ImportKind::Collection => format!(
                "{}: this is an environment file; pass it with --environment",
                path.display()
            ),
            ImportKind::Environment => {
                format!("{}: this is not an environment file", path.display())
            }
        });
    }
    warnings.extend(planned.warnings.iter().cloned());
    Ok(planned)
}

/// `--var` values win over the environment file's; new names are added as secrets, since
/// they are often tokens.
fn set_variables(
    conn: &rusqlite::Connection,
    vault: &Vault,
    environment_id: &str,
    overrides: &[(String, String)],
) -> Result<(), AppError> {
    let wanted = |key: &str| {
        overrides
            .iter()
            .rev()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    };
    let current = environments::variables(conn, environment_id)?;
    let mut inputs: Vec<VariableInput> = current
        .iter()
        .map(|variable| {
            let given = wanted(&variable.key);
            VariableInput {
                id: Some(variable.id.clone()),
                key: variable.key.clone(),
                enabled: variable.enabled || given.is_some(),
                restricted: None,
                // A secret's stored value is kept unless a new one is given.
                value: given.or_else(|| (!variable.is_secret).then(|| variable.value.clone())),
                is_secret: variable.is_secret,
            }
        })
        .collect();
    for (name, _) in overrides {
        if !inputs.iter().any(|input| &input.key == name) {
            inputs.push(VariableInput {
                id: None,
                key: name.clone(),
                value: wanted(name),
                is_secret: true,
                enabled: true,
                restricted: None,
            });
        }
    }
    environments::replace_variables(conn, vault, environment_id, check_variables(inputs)?)
        .map(|_| ())
}

fn located(path: &Path, error: &AppError) -> String {
    format!("{}: {error}", path.display())
}
