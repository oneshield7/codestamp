use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::ipc::{InvokeBody, Request};
use tauri::{State, WebviewWindow};

use super::files::{ExportKind, checked_path, file_name, slug};
use super::png::check_png;
use super::postman::{self, Exported};
use crate::dialogs::pick_save_path;
use crate::docs;
use crate::environment::repo as environments;
use crate::error::AppError;
use crate::request::bodies::{BodyFormat, ResponseBodies};
use crate::storage::db::Database;
use crate::storage::descriptions::{self, Described};
use crate::storage::examples;
use crate::storage::repo as collections;
use crate::storage::validate::check_id;
use crate::team::files::{self as collection_files, Snapshot};

/// Where an export was saved, and what the user should know about its content.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub notes: Vec<String>,
}

/// Who a collection export is for.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CollectionFormat {
    /// Another Codestamp: every request type, examples, Docs and the collection's environments.
    Codestamp,
    /// Postman Collection v2.1, which other API tools read too.
    Postman,
}

/// Carries the suggested file name next to the raw PNG body.
const NAME_HEADER: &str = "x-capture-name";
const MAX_REPORT_BYTES: usize = 10 * 1024 * 1024;

/// Saves the image made by the Capture dialog where the user chooses; `None` if they cancel.
#[tauri::command]
pub async fn save_capture(
    window: WebviewWindow,
    request: Request<'_>,
) -> Result<Option<String>, AppError> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::invalid("The image is missing"));
    };
    check_png(bytes)?;
    let stem = request
        .headers()
        .get(NAME_HEADER)
        .and_then(|value| value.to_str().ok());
    let Some(path) = choose_path(&window, "Save capture", stem, ExportKind::Png).await? else {
        return Ok(None);
    };
    fs::write(&path, bytes).map_err(|_| AppError::FileAccess)?;
    Ok(Some(path.display().to_string()))
}

/// Saves a collection run report as JSON where the user chooses; `None` if they cancel.
#[tauri::command]
pub async fn save_report(
    window: WebviewWindow,
    report: String,
    name: String,
) -> Result<Option<String>, AppError> {
    if report.len() > MAX_REPORT_BYTES {
        return Err(AppError::invalid("The report is larger than 10 MB"));
    }
    serde_json::from_str::<serde_json::Value>(&report)
        .map_err(|_| AppError::invalid("The report is not valid JSON"))?;
    let Some(path) = choose_path(&window, "Save run report", Some(&name), ExportKind::Json).await?
    else {
        return Ok(None);
    };
    fs::write(&path, report).map_err(|_| AppError::FileAccess)?;
    Ok(Some(path.display().to_string()))
}

/// Writes the collection where the user chooses, as a Codestamp file or a Postman Collection
/// v2.1 file. Secret values are left empty and local file paths never leave. `None` if they
/// cancel.
#[tauri::command]
pub async fn export_collection(
    window: WebviewWindow,
    db: State<'_, Database>,
    id: String,
    format: CollectionFormat,
) -> Result<Option<ExportResult>, AppError> {
    check_id(&id)?;
    if let CollectionFormat::Codestamp = format {
        let snapshot = db.with(|conn| collection_files::from_db(conn, &id))?;
        let name = snapshot.collection.name.clone();
        let exported = Exported {
            notes: codestamp_notes(&snapshot),
            json: serde_json::to_string_pretty(&collection_files::bundle(snapshot)?)
                .map_err(|_| AppError::Storage)?,
        };
        return save_export(
            &window,
            "Export collection",
            &name,
            ExportKind::CodestampCollection,
            exported,
        )
        .await;
    }
    let (name, exported) = db.with(|conn| {
        let detail = collections::collection_detail(conn, &id)?;
        let requests = detail
            .requests
            .iter()
            .map(|summary| collections::load_request(conn, &summary.id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut kept = HashMap::new();
        for request in &requests {
            kept.insert(request.id.clone(), examples::all_for(conn, &request.id)?);
        }
        let variables = match environments::find_collection_set(conn, &id)? {
            Some(set) => environments::variables(conn, &set)?,
            None => Vec::new(),
        };
        let mut docs = postman::Docs {
            collection: descriptions::get(conn, Described::Collection, &id)?,
            by_id: HashMap::new(),
        };
        for section in &detail.sections {
            let text = descriptions::get(conn, Described::Section, &section.id)?;
            docs.by_id.insert(section.id.clone(), text);
        }
        for request in &requests {
            let text = descriptions::get(conn, Described::Request, &request.id)?;
            docs.by_id.insert(request.id.clone(), text);
        }
        Ok((
            detail.name.clone(),
            postman::collection(&detail, &requests, &kept, &variables, &docs)?,
        ))
    })?;
    save_export(
        &window,
        "Export collection",
        &name,
        ExportKind::Collection,
        exported,
    )
    .await
}

/// Writes a collection's Docs as one Markdown file where the user chooses; `None` if they cancel.
#[tauri::command]
pub async fn export_docs(
    window: WebviewWindow,
    db: State<'_, Database>,
    collection_id: String,
) -> Result<Option<String>, AppError> {
    check_id(&collection_id)?;
    let (name, text) = db.with(|conn| {
        let loaded = docs::repo::load(conn, &collection_id)?;
        Ok((loaded.name.clone(), docs::markdown::render(&loaded)))
    })?;
    let stem = slug(&name);
    let Some(path) = choose_path(&window, "Save docs", Some(&stem), ExportKind::Docs).await? else {
        return Ok(None);
    };
    fs::write(&path, text).map_err(|_| AppError::FileAccess)?;
    Ok(Some(path.display().to_string()))
}

/// Writes an environment (or Globals) as a Postman environment file; secret values stay empty.
#[tauri::command]
pub async fn export_environment(
    window: WebviewWindow,
    db: State<'_, Database>,
    id: String,
) -> Result<Option<ExportResult>, AppError> {
    check_id(&id)?;
    let (summary, exported) = db.with(|conn| {
        let summary = environments::summary(conn, &id)?;
        let variables = environments::variables(conn, &id)?;
        let exported = postman::environment(&summary, &variables)?;
        Ok((summary, exported))
    })?;
    let kind = if summary.is_global {
        ExportKind::Globals
    } else {
        ExportKind::Environment
    };
    save_export(&window, "Export environment", &summary.name, kind, exported).await
}

/// Saves a kept response body, byte for byte, where the user chooses; `None` if they cancel.
/// The file type follows the response's Content-Type.
#[tauri::command]
pub async fn save_response_body(
    window: WebviewWindow,
    bodies: State<'_, ResponseBodies>,
    body_id: String,
    name: Option<String>,
) -> Result<Option<String>, AppError> {
    check_id(&body_id)?;
    let (bytes, content_type) = bodies.get(&body_id)?;
    let kind = ExportKind::ResponseBody(BodyFormat::from_content_type(content_type.as_deref()));
    let stem = name.as_deref().map(slug);
    let Some(path) = choose_path(&window, "Save response body", stem.as_deref(), kind).await?
    else {
        return Ok(None);
    };
    fs::write(&path, bytes.as_slice()).map_err(|_| AppError::FileAccess)?;
    Ok(Some(path.display().to_string()))
}

/// What another Codestamp will not get from the file.
fn codestamp_notes(snapshot: &Snapshot) -> Vec<String> {
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
    let mut notes = Vec::new();
    if secrets > 0 {
        notes.push(format!(
            "{secrets} secret value(s) were left empty. Fill them in after importing."
        ));
    }
    if !snapshot.groups.is_empty() {
        notes.push(
            "Access groups stay with the shared collection, so restricted values were left empty."
                .to_owned(),
        );
    }
    notes
}

async fn save_export(
    window: &WebviewWindow,
    title: &str,
    name: &str,
    kind: ExportKind,
    exported: Exported,
) -> Result<Option<ExportResult>, AppError> {
    let Some(path) = choose_path(window, title, Some(&slug(name)), kind).await? else {
        return Ok(None);
    };
    fs::write(&path, exported.json).map_err(|_| AppError::FileAccess)?;
    Ok(Some(ExportResult {
        path: path.display().to_string(),
        notes: exported.notes,
    }))
}

async fn choose_path(
    window: &WebviewWindow,
    title: &str,
    stem: Option<&str>,
    kind: ExportKind,
) -> Result<Option<PathBuf>, AppError> {
    let suggested = file_name(stem, kind);
    let (filter, extensions) = kind.filter();
    let picked = pick_save_path(window, title, &suggested, filter, extensions).await;
    picked.map(|path| checked_path(path, kind)).transpose()
}
