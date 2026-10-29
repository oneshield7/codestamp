use serde::Serialize;

use crate::auth::models::Auth;
use crate::environment::models::VariableInput;
use crate::storage::examples::ExampleInput;
use crate::storage::models::RequestContent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportKind {
    Collection,
    Environment,
}

/// Everything an import will create, built and checked before anything is written.
#[derive(Debug)]
pub struct ImportPlan {
    pub kind: ImportKind,
    /// What was read, such as "Collection v2.1" or "OpenAPI 3.0.3".
    pub source: String,
    pub name: String,
    /// Docs for the collection, in Markdown.
    pub description: String,
    /// The collection's own auth; requests that inherit use it.
    pub auth: Auth,
    pub sections: Vec<PlannedSection>,
    pub requests: Vec<PlannedRequest>,
    pub variables: Vec<VariableInput>,
    /// Environments that belong to the collection; only a Codestamp file carries them.
    pub environments: Vec<PlannedEnvironment>,
    /// An icon someone chose, as a Codestamp file keeps it.
    pub icon: Option<String>,
    pub script_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct PlannedEnvironment {
    pub name: String,
    pub variables: Vec<VariableInput>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlannedSection {
    pub name: String,
    pub description: String,
}

impl PlannedSection {
    pub fn named(name: String) -> Self {
        Self {
            name,
            description: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct PlannedRequest {
    /// Index into `ImportPlan::sections`, or `None` for requests outside any folder.
    pub section: Option<usize>,
    pub content: RequestContent,
    pub description: String,
    /// Saved responses that come with the request (Postman responses, OpenAPI examples).
    pub examples: Vec<ExampleInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub kind: ImportKind,
    pub source: String,
    pub name: String,
    pub request_count: usize,
    pub section_count: usize,
    pub variable_count: usize,
    pub environment_count: usize,
    pub script_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub kind: ImportKind,
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurlImport {
    pub request: RequestContent,
    pub warnings: Vec<String>,
}

impl ImportPlan {
    #[cfg(test)]
    pub fn section_names(&self) -> Vec<&str> {
        self.sections
            .iter()
            .map(|section| section.name.as_str())
            .collect()
    }

    pub fn preview(&self) -> ImportPreview {
        ImportPreview {
            kind: self.kind,
            source: self.source.clone(),
            name: self.name.clone(),
            request_count: self.requests.len(),
            section_count: self.sections.len(),
            variable_count: self.variables.len(),
            environment_count: self.environments.len(),
            script_count: self.script_count,
            warnings: self.warnings.clone(),
        }
    }
}
