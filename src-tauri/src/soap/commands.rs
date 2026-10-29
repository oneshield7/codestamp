use tauri::{State, WebviewWindow};

use super::{WsdlDefinition, WsdlSource, load_definition};
use crate::dialogs::pick_file;
use crate::error::AppError;
use crate::import::postman::MAX_IMPORT_BYTES;
use crate::request::client::HttpClient;
use crate::request::uploads::{self, PickedFile, file_label};
use crate::storage::db::Database;

/// Lets the user pick a WSDL file; only picked files (and the imports beside them) are read.
#[tauri::command]
pub async fn pick_wsdl_file(
    window: WebviewWindow,
    db: State<'_, Database>,
) -> Result<Option<PickedFile>, AppError> {
    let Some(path) = pick_file(&window, "Choose a WSDL file", "WSDL", &["wsdl", "xml"]).await
    else {
        return Ok(None);
    };
    db.with(|conn| uploads::grant(conn, &path)).map(Some)
}

#[tauri::command]
pub async fn soap_load_wsdl(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    source: WsdlSource,
) -> Result<WsdlDefinition, AppError> {
    read_definition(&client, &db, &source).await
}

/// Reads a WSDL from a URL, a file the user picked on this device, or a dropped file's text.
pub async fn read_definition(
    client: &HttpClient,
    db: &Database,
    source: &WsdlSource,
) -> Result<WsdlDefinition, AppError> {
    match source {
        WsdlSource::File { path } => {
            if !db.with(|conn| uploads::is_granted(conn, path))? {
                return Err(AppError::InvalidInput(format!(
                    "Choose “{}” again: only files picked on this device can be read",
                    file_label(path)
                )));
            }
        }
        WsdlSource::Content { content } if content.len() > MAX_IMPORT_BYTES => {
            return Err(AppError::invalid("The file is larger than 20 MB"));
        }
        WsdlSource::Url { .. } | WsdlSource::Content { .. } => {}
    }
    load_definition(client, source).await
}
