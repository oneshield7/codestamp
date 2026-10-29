use serde::Serialize;

use crate::auth::models::Auth;
use crate::storage::descriptions::Described;
use crate::storage::models::{RequestKind, SavedRequest};

/// One collection as its documentation shows it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDocs {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub auth: Auth,
    /// Enabled collection variables; a secret's value is never included.
    pub variables: Vec<DocVariable>,
    pub sections: Vec<SectionDocs>,
    /// In notebook order: requests outside sections, then each section's.
    pub requests: Vec<RequestDocs>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocVariable {
    pub key: String,
    pub value: String,
    pub secret: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionDocs {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestDocs {
    #[serde(flatten)]
    pub request: SavedRequest,
    pub description: String,
    pub examples: Vec<DocExample>,
}

/// A collection, section or request found by a docs search.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocHit {
    pub item: Described,
    pub id: String,
    pub collection_id: String,
    pub collection_name: String,
    pub collection_icon: String,
    /// The section a request sits in.
    pub section_name: Option<String>,
    pub name: String,
    /// Requests only: what their stamp shows.
    pub kind: Option<RequestKind>,
    pub method: Option<String>,
    /// Part of the description around the first word found there; none for a name match.
    pub snippet: Option<String>,
}

/// A saved example, with its body cut short when it is long.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocExample {
    pub name: String,
    pub status: u16,
    pub content_type: Option<String>,
    pub body: String,
    pub cut: bool,
}
