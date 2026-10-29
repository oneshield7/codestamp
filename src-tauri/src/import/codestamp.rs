use serde_json::Value;

use super::models::{ImportKind, ImportPlan, PlannedEnvironment, PlannedRequest, PlannedSection};
use crate::environment::models::VariableInput;
use crate::error::AppError;
use crate::team::apply;
use crate::team::files::{self, BUNDLE_KIND, VariableEntry};

/// A collection another Codestamp exported: `{"codestamp": "collection", "files": {..}}`.
pub fn is_bundle(value: &Value) -> bool {
    value.get("codestamp").and_then(Value::as_str) == Some(BUNDLE_KIND)
}

/// Everything in the file, in its own order. Each part gets a new id when it is written, so the
/// same file can be imported next to the collection it came from.
pub fn plan(value: &Value) -> Result<ImportPlan, AppError> {
    let files = value
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::invalid("The Codestamp file has no collection in it"))?;
    let snapshot = files::read_bundle(files)?;
    let secrets = snapshot
        .collection
        .variables
        .iter()
        .chain(
            snapshot
                .environments
                .values()
                .flat_map(|environment| environment.variables.iter()),
        )
        .filter(|variable| variable.secret)
        .count();
    let mut requests = Vec::with_capacity(snapshot.requests.len());
    // The first group holds the requests outside sections, then one group per section.
    for (index, group) in apply::groups(&snapshot).into_iter().enumerate() {
        for id in group.request_ids {
            let Some(request) = snapshot.requests.get(&id) else {
                continue;
            };
            requests.push(PlannedRequest {
                section: index.checked_sub(1),
                content: request.to_content(None),
                description: request.description.clone(),
                examples: request
                    .examples
                    .iter()
                    .map(|example| example.to_example(&request.id).into_input())
                    .collect(),
            });
        }
    }
    let script_count = requests
        .iter()
        .flat_map(|request| {
            [
                &request.content.pre_request_script,
                &request.content.test_script,
            ]
        })
        .filter(|script| !script.trim().is_empty())
        .count();
    let collection = snapshot.collection;
    let environments: Vec<PlannedEnvironment> = snapshot
        .environments
        .into_values()
        .map(|environment| PlannedEnvironment {
            name: environment.name,
            variables: inputs(&environment.variables),
        })
        .collect();
    let warnings = match secrets {
        0 => Vec::new(),
        count => vec![format!(
            "{count} secret value(s) arrive empty; fill them in after importing"
        )],
    };
    Ok(ImportPlan {
        kind: ImportKind::Collection,
        source: "Codestamp collection".to_owned(),
        name: collection.name,
        description: collection.description,
        auth: collection.auth,
        sections: collection
            .sections
            .into_iter()
            .map(|section| PlannedSection {
                name: section.name,
                description: section.description,
            })
            .collect(),
        requests,
        variables: inputs(&collection.variables),
        environments,
        icon: collection.icon,
        script_count,
        warnings,
    })
}

fn inputs(variables: &[VariableEntry]) -> Vec<VariableInput> {
    variables
        .iter()
        .map(|variable| VariableInput {
            id: None,
            key: variable.key.clone(),
            value: Some(if variable.secret {
                String::new()
            } else {
                variable.value.clone()
            }),
            is_secret: variable.secret,
            enabled: variable.enabled,
            restricted: None,
        })
        .collect()
}
