use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::oneshot;
use tokio::time::timeout;

use super::apply::check_auth;
use super::models::{Auth, OAuth2Settings, OAuthGrant};
use super::oauth::{
    self, Authorization, TokenStatus, authorize_url, code_from_callback, is_callback, redirect_uri,
};
use crate::environment::lookup::resolve_auth;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::storage::db::Database;
use crate::storage::validate::{check_id, check_optional_id};

const SIGN_IN_WINDOW: &str = "oauth-sign-in";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(600);
const WINDOW_CLOSED: &str = "The sign-in window was closed";

type Answer = Arc<Mutex<Option<oneshot::Sender<Result<String, AppError>>>>>;

/// Gets an OAuth 2.0 token for a request (or a collection, whose requests inherit it) and keeps
/// it encrypted on this device. Authorization Code opens a sign-in window first.
#[tauri::command]
pub async fn oauth_get_token(
    app: AppHandle,
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    owner_id: String,
    collection_id: Option<String>,
    auth: Auth,
) -> Result<TokenStatus, AppError> {
    check_id(&owner_id)?;
    check_optional_id(collection_id.as_deref())?;
    check_auth(&auth, false)?;
    let Auth::OAuth2(settings) = resolve_auth(&db, &vault, auth, collection_id.as_deref())? else {
        return Err(AppError::invalid("Choose OAuth 2.0 as the auth type first"));
    };
    let token = if settings.grant == OAuthGrant::AuthorizationCode {
        let authorization = Authorization::new();
        let code = sign_in(&app, &settings, &authorization).await?;
        oauth::exchange_code(&client, &settings, code, &authorization).await?
    } else {
        oauth::request_without_sign_in(&client, &settings).await?
    };
    oauth::save_token(&db, &vault, &owner_id, &token)?;
    Ok(token.status())
}

#[tauri::command]
pub async fn oauth_token_status(
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    owner_id: String,
) -> Result<Option<TokenStatus>, AppError> {
    check_id(&owner_id)?;
    Ok(oauth::load_token(&db, &vault, &owner_id)?.map(|token| token.status()))
}

#[tauri::command]
pub async fn oauth_clear_token(db: State<'_, Database>, owner_id: String) -> Result<(), AppError> {
    check_id(&owner_id)?;
    oauth::clear_token(&db, &owner_id)
}

/// Opens the provider's sign-in page in its own window and waits for the callback. The window
/// never loads the callback page itself, and it gets no access to the app.
async fn sign_in(
    app: &AppHandle,
    settings: &OAuth2Settings,
    authorization: &Authorization,
) -> Result<String, AppError> {
    let url = authorize_url(settings, authorization)?;
    let redirect = redirect_uri(settings)?;
    close_sign_in(app);
    let (sender, receiver) = oneshot::channel();
    let answer: Answer = Arc::new(Mutex::new(Some(sender)));
    let state = authorization.state.clone();
    let navigation = Arc::clone(&answer);
    let mut builder = WebviewWindowBuilder::new(app, SIGN_IN_WINDOW, WebviewUrl::External(url));
    // WebView2 shares one browser per data folder, so every window needs the same arguments.
    if let Some(arguments) = app
        .config()
        .app
        .windows
        .first()
        .and_then(|window| window.additional_browser_args.as_deref())
    {
        builder = builder.additional_browser_args(arguments);
    }
    let window = builder
        .title("Sign in")
        .inner_size(520.0, 720.0)
        .center()
        .on_navigation(move |target| {
            if !is_callback(target, &redirect) {
                return true;
            }
            reply(&navigation, code_from_callback(target, &state));
            false
        })
        .build()
        .map_err(|_| AppError::Window)?;
    let closed = Arc::clone(&answer);
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            reply(&closed, Err(AppError::invalid(WINDOW_CLOSED)));
        }
    });
    let result = timeout(SIGN_IN_TIMEOUT, receiver).await;
    close_sign_in(app);
    match result {
        Ok(Ok(answer)) => answer,
        Ok(Err(_)) => Err(AppError::invalid(WINDOW_CLOSED)),
        Err(_) => Err(AppError::invalid(
            "Sign-in took longer than 10 minutes; try again",
        )),
    }
}

/// The first answer wins: the callback, or the window closing before it.
fn reply(answer: &Answer, result: Result<String, AppError>) {
    if let Some(sender) = answer.lock().unwrap_or_else(PoisonError::into_inner).take() {
        // The command may have stopped waiting (timed out); then nobody needs the answer.
        let _ = sender.send(result);
    }
}

fn close_sign_in(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SIGN_IN_WINDOW) {
        // A window that is already closing cannot be destroyed again, which is fine here.
        let _ = window.destroy();
    }
}
