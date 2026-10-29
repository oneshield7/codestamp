use tauri::{AppHandle, State, WebviewWindow};

use super::bodies::ResponseBodies;
use super::client::HttpClient;
use super::pipeline::{self, ResolvedRequest};
use super::streams::EventStreams;
use super::types::{HttpRequestInput, HttpResponseOutput, RequestBody};
use super::uploads::{self, PickedFile};
use crate::dialogs::pick_file;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::db::Database;
use crate::storage::validate::check_id;

#[tauri::command]
pub async fn send_http_request(
    app: AppHandle,
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    bodies: State<'_, ResponseBodies>,
    streams: State<'_, EventStreams>,
    request: HttpRequestInput,
) -> Result<HttpResponseOutput, AppError> {
    let mut response = pipeline::send(&client, &db, &vault, request).await?;
    if let Some(whole) = response.whole_body.take() {
        response.body_id = Some(bodies.keep(whole, response.content_type.clone()));
    }
    if let Some(stream) = response.stream.take() {
        match streams.start(app, stream) {
            Ok(id) => response.stream_id = Some(id),
            Err(error) => response.warning = Some(error.to_string()),
        }
    }
    Ok(response)
}

/// Stops reading an event stream; the server sees the connection close.
#[tauri::command]
pub async fn stop_event_stream(
    streams: State<'_, EventStreams>,
    id: String,
) -> Result<(), AppError> {
    check_id(&id)?;
    streams.stop(&id);
    Ok(())
}

/// Asks a GraphQL endpoint for its schema (the introspection query comes from the UI) and returns
/// the whole answer, which may be larger than the preview responses get. Scripts do not run and
/// nothing is added to history.
#[tauri::command]
pub async fn graphql_schema(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    mut request: HttpRequestInput,
) -> Result<String, AppError> {
    if !matches!(request.body, Some(RequestBody::Graphql { .. })) {
        return Err(AppError::invalid(
            "The schema is asked for with a GraphQL query",
        ));
    }
    request.pre_request_script = None;
    request.test_script = None;
    request.record_history = false;
    let response = pipeline::send(&client, &db, &vault, request).await?;
    if !(200..300).contains(&response.status) {
        return Err(AppError::InvalidInput(format!(
            "The server answered {} to the schema query",
            response.status
        )));
    }
    if response.body_size == 0 {
        return Err(AppError::invalid("The server sent an empty answer"));
    }
    let bytes = response
        .whole_body
        .ok_or_else(|| AppError::invalid("The schema is larger than 32 MB"))?;
    String::from_utf8(bytes).map_err(|_| AppError::invalid("The schema answer is not UTF-8 text"))
}

/// The request as it would be sent, for "Copy as code" with variables filled in.
#[tauri::command]
pub async fn resolve_request_for_code(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    request: HttpRequestInput,
) -> Result<ResolvedRequest, AppError> {
    pipeline::resolve_for_code(&db, &vault, request)
}

/// Lets the user pick a file to upload; only files picked here can be sent. `None` if cancelled.
#[tauri::command]
pub async fn pick_upload_file(
    window: WebviewWindow,
    db: State<'_, Database>,
) -> Result<Option<PickedFile>, AppError> {
    let Some(path) = pick_file(&window, "Choose a file to upload", "All files", &["*"]).await
    else {
        return Ok(None);
    };
    db.with(|conn| uploads::grant(conn, &path)).map(Some)
}
