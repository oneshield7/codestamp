//! OAuth 2.0 tokens (RFC 6749): asking the token endpoint, refreshing, and keeping tokens
//! encrypted on this device under the request or collection that uses them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use super::models::{Auth, ClientAuth, OAuth2Settings, OAuthGrant};
use crate::encoding::base64_url_encode;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::preferences::repo::{delete_secret, load_secret, save_secret};
use crate::request::client::{HttpClient, map_error};
use crate::request::validate::secure_url as endpoint;
use crate::storage::db::{Database, now_ms};

pub const KEY_PREFIX: &str = "oauth:";
/// A token this close to expiring is treated as expired, so it does not lapse in flight.
const EXPIRY_MARGIN_MS: i64 = 30_000;
const MAX_TOKEN_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredToken {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Milliseconds since 1970; `None` when the server gave no lifetime.
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub scope: Option<String>,
    pub obtained_at: i64,
}

/// What the Auth tab shows about a token; the token itself never reaches the UI.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStatus {
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
    pub refreshable: bool,
    pub obtained_at: i64,
}

impl StoredToken {
    pub fn status(&self) -> TokenStatus {
        TokenStatus {
            expires_at: self.expires_at,
            scope: self.scope.clone(),
            refreshable: self.refresh_token.is_some(),
            obtained_at: self.obtained_at,
        }
    }

    fn is_fresh(&self, now: i64) -> bool {
        self.expires_at
            .is_none_or(|expires| now < expires - EXPIRY_MARGIN_MS)
    }
}

/// A PKCE pair (RFC 7636, S256) and the `state` that ties the answer to this sign-in.
pub struct Authorization {
    pub verifier: String,
    pub state: String,
}

impl Authorization {
    pub fn new() -> Self {
        // Each v4 UUID carries 122 random bits from the operating system's secure source.
        let verifier = (0..3)
            .map(|_| Uuid::new_v4().simple().to_string())
            .collect();
        Self {
            verifier,
            state: Uuid::new_v4().simple().to_string(),
        }
    }

    fn challenge(&self) -> String {
        base64_url_encode(&Sha256::digest(self.verifier.as_bytes()))
    }
}

pub fn load_token(
    db: &Database,
    vault: &Vault,
    owner: &str,
) -> Result<Option<StoredToken>, AppError> {
    let key = secret_key(owner);
    let Some(blob) = db.with(|conn| load_secret(conn, &key))? else {
        return Ok(None);
    };
    // A token that no longer decrypts (the key changed, say) is simply gone.
    Ok(vault
        .decrypt(&key, &blob)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok()))
}

pub fn save_token(
    db: &Database,
    vault: &Vault,
    owner: &str,
    token: &StoredToken,
) -> Result<(), AppError> {
    let key = secret_key(owner);
    let json = serde_json::to_string(token).map_err(|_| AppError::Storage)?;
    let blob = vault.encrypt(&key, &json)?;
    db.with(|conn| save_secret(conn, &key, &blob))
}

pub fn clear_token(db: &Database, owner: &str) -> Result<(), AppError> {
    db.with(|conn| delete_secret(conn, &secret_key(owner)))
}

pub fn secret_key(owner: &str) -> String {
    format!("{KEY_PREFIX}{owner}")
}

/// Whose token a request uses: the collection's when it inherits auth, otherwise its own.
pub fn token_owner(
    auth: Option<&Auth>,
    request_id: Option<&str>,
    collection_id: Option<&str>,
) -> Option<String> {
    match auth {
        Some(Auth::Inherit) => collection_id,
        _ => request_id,
    }
    .map(str::to_owned)
}

/// The header an OAuth 2.0 token is sent in.
pub fn bearer(settings: &OAuth2Settings, token: String) -> Auth {
    let prefix = settings.header_prefix.trim();
    Auth::Bearer {
        token,
        prefix: if prefix.is_empty() { "Bearer" } else { prefix }.to_owned(),
    }
}

/// The access token for a request, refreshed or renewed when that needs no sign-in.
pub async fn access_token(
    client: &HttpClient,
    db: &Database,
    vault: &Vault,
    owner: Option<&str>,
    settings: &OAuth2Settings,
) -> Result<String, AppError> {
    let owner = owner.ok_or_else(|| {
        AppError::invalid("Save the request in a collection before using OAuth 2.0")
    })?;
    let stored = load_token(db, vault, owner)?;
    if let Some(token) = stored.as_ref().filter(|token| token.is_fresh(now_ms())) {
        return Ok(token.access_token.clone());
    }
    let renewed = match stored.and_then(|token| token.refresh_token) {
        Some(refresh_token) => refresh(client, settings, &refresh_token).await.map_err(|error| {
            AppError::InvalidInput(format!(
                "The access token expired and could not be refreshed. Get a new one in the Auth tab ({error})"
            ))
        })?,
        None if settings.grant == OAuthGrant::ClientCredentials => {
            request_without_sign_in(client, settings).await?
        }
        None => {
            return Err(AppError::invalid(
                "Get an access token first: Auth tab › Get new access token",
            ));
        }
    };
    save_token(db, vault, owner, &renewed)?;
    Ok(renewed.access_token)
}

/// The access token for "Copy as code", without asking the network.
pub fn current_token(
    db: &Database,
    vault: &Vault,
    owner: Option<&str>,
) -> Result<String, AppError> {
    owner
        .map(|owner| load_token(db, vault, owner))
        .transpose()?
        .flatten()
        .filter(|token| token.is_fresh(now_ms()))
        .map(|token| token.access_token)
        .ok_or_else(|| {
            AppError::invalid("Get an access token first: Auth tab › Get new access token")
        })
}

/// Client credentials and password grants talk to the token endpoint directly.
pub async fn request_without_sign_in(
    client: &HttpClient,
    settings: &OAuth2Settings,
) -> Result<StoredToken, AppError> {
    let mut form = match settings.grant {
        OAuthGrant::ClientCredentials => vec![("grant_type", "client_credentials".to_owned())],
        OAuthGrant::Password => vec![
            ("grant_type", "password".to_owned()),
            ("username", settings.username.clone()),
            ("password", settings.password.clone()),
        ],
        OAuthGrant::AuthorizationCode => {
            return Err(AppError::invalid("This grant needs a sign-in window"));
        }
    };
    if !settings.scope.trim().is_empty() {
        form.push(("scope", settings.scope.trim().to_owned()));
    }
    token_request(client, settings, form).await
}

/// The sign-in page to open, with PKCE when it is on.
pub fn authorize_url(
    settings: &OAuth2Settings,
    authorization: &Authorization,
) -> Result<Url, AppError> {
    let mut url = endpoint(&settings.auth_url, "authorization URL")?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("response_type", "code")
            .append_pair("client_id", settings.client_id.trim())
            .append_pair("redirect_uri", redirect_uri(settings)?.as_str())
            .append_pair("state", &authorization.state);
        if !settings.scope.trim().is_empty() {
            query.append_pair("scope", settings.scope.trim());
        }
        if settings.use_pkce {
            query
                .append_pair("code_challenge", &authorization.challenge())
                .append_pair("code_challenge_method", "S256");
        }
    }
    Ok(url)
}

pub fn redirect_uri(settings: &OAuth2Settings) -> Result<Url, AppError> {
    Url::parse(settings.redirect_uri.trim())
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .ok_or_else(|| AppError::invalid("Enter the callback URL registered for this client"))
}

/// Whether a page the sign-in window is about to open is the callback.
pub fn is_callback(url: &Url, redirect: &Url) -> bool {
    url.scheme() == redirect.scheme()
        && url.host_str() == redirect.host_str()
        && url.port_or_known_default() == redirect.port_or_known_default()
        && url.path() == redirect.path()
}

/// The code from the callback, after checking it answers this sign-in.
pub fn code_from_callback(url: &Url, state: &str) -> Result<String, AppError> {
    let param = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    if let Some(error) = param("error") {
        let detail = param("error_description").unwrap_or_default();
        return Err(AppError::InvalidInput(format!(
            "Sign-in failed: {error}{}",
            if detail.is_empty() {
                String::new()
            } else {
                format!(" ({detail})")
            }
        )));
    }
    if param("state").as_deref() != Some(state) {
        return Err(AppError::invalid(
            "The sign-in answer does not match this request; try again",
        ));
    }
    param("code").ok_or_else(|| AppError::invalid("The sign-in answer has no code"))
}

pub async fn exchange_code(
    client: &HttpClient,
    settings: &OAuth2Settings,
    code: String,
    authorization: &Authorization,
) -> Result<StoredToken, AppError> {
    let mut form = vec![
        ("grant_type", "authorization_code".to_owned()),
        ("code", code),
        ("redirect_uri", redirect_uri(settings)?.to_string()),
    ];
    if settings.use_pkce {
        form.push(("code_verifier", authorization.verifier.clone()));
    }
    token_request(client, settings, form).await
}

async fn refresh(
    client: &HttpClient,
    settings: &OAuth2Settings,
    refresh_token: &str,
) -> Result<StoredToken, AppError> {
    let form = vec![
        ("grant_type", "refresh_token".to_owned()),
        ("refresh_token", refresh_token.to_owned()),
    ];
    let mut token = token_request(client, settings, form).await?;
    // Servers may keep the refresh token unchanged and leave it out of the answer.
    if token.refresh_token.is_none() {
        token.refresh_token = Some(refresh_token.to_owned());
    }
    Ok(token)
}

async fn token_request(
    client: &HttpClient,
    settings: &OAuth2Settings,
    mut form: Vec<(&str, String)>,
) -> Result<StoredToken, AppError> {
    let url = endpoint(&settings.token_url, "token URL")?;
    let client_id = settings.client_id.trim();
    let secret = settings.client_secret.as_str();
    let mut builder = client
        .client_for(&url)?
        .post(url.clone())
        .timeout(client.default_timeout()?)
        .header(reqwest::header::ACCEPT, "application/json");
    if secret.is_empty() {
        form.push(("client_id", client_id.to_owned()));
    } else if settings.client_auth == ClientAuth::Header {
        // RFC 6749 §2.3.1: both parts are form-encoded before they are joined.
        let encode =
            |text: &str| url::form_urlencoded::byte_serialize(text.as_bytes()).collect::<String>();
        builder = builder.basic_auth(encode(client_id), Some(encode(secret)));
    } else {
        form.push(("client_id", client_id.to_owned()));
        form.push(("client_secret", secret.to_owned()));
    }
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(&form)
        .finish();
    let response = builder
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body)
        .send()
        .await
        .map_err(map_error)?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|_| AppError::invalid("The token server's answer could not be read"))?;
    if bytes.len() > MAX_TOKEN_RESPONSE_BYTES {
        return Err(AppError::invalid("The token server's answer is too large"));
    }
    parse_token(status.as_u16(), &bytes, now_ms())
}

fn parse_token(status: u16, body: &[u8], now: i64) -> Result<StoredToken, AppError> {
    let json: Option<Value> = serde_json::from_slice(body).ok();
    let text = |key: &str| {
        json.as_ref()
            .and_then(|json| json.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    if let Some(error) = text("error") {
        let detail = text("error_description").unwrap_or_default();
        return Err(AppError::InvalidInput(format!(
            "The token server refused: {error}{}",
            if detail.is_empty() {
                String::new()
            } else {
                format!(" ({detail})")
            }
        )));
    }
    if !(200..300).contains(&status) {
        return Err(AppError::InvalidInput(format!(
            "The token server answered {status}"
        )));
    }
    let access_token = text("access_token")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| AppError::invalid("The token server's answer has no access_token"))?;
    let lifetime = json
        .as_ref()
        .and_then(|json| json.get("expires_in"))
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        });
    Ok(StoredToken {
        access_token,
        refresh_token: text("refresh_token").filter(|token| !token.is_empty()),
        expires_at: lifetime
            .filter(|seconds| *seconds > 0)
            .map(|seconds| now + seconds * 1000),
        scope: text("scope"),
        obtained_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> OAuth2Settings {
        OAuth2Settings {
            grant: OAuthGrant::AuthorizationCode,
            auth_url: "https://login.example.test/authorize?prompt=login".to_owned(),
            token_url: "https://login.example.test/token".to_owned(),
            client_id: "codestamp app".to_owned(),
            client_secret: String::new(),
            scope: " read write ".to_owned(),
            redirect_uri: "https://app.example.test/callback".to_owned(),
            use_pkce: true,
            username: String::new(),
            password: String::new(),
            client_auth: ClientAuth::Header,
            header_prefix: String::new(),
        }
    }

    #[test]
    fn builds_the_sign_in_url_with_pkce() {
        let authorization = Authorization::new();
        assert_eq!(authorization.verifier.len(), 96);
        let url = authorize_url(&settings(), &authorization).expect("url");
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let get = |key: &str| {
            pairs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("prompt"), Some("login"));
        assert_eq!(get("client_id"), Some("codestamp app"));
        assert_eq!(get("scope"), Some("read write"));
        assert_eq!(get("code_challenge_method"), Some("S256"));
        assert_eq!(get("code_challenge").map(str::len), Some(43));
        assert_eq!(get("state"), Some(authorization.state.as_str()));
    }

    #[test]
    fn reads_the_callback() {
        let redirect = Url::parse("https://app.example.test/callback").expect("url");
        let ok = Url::parse("https://app.example.test/callback?code=abc&state=s1").expect("url");
        assert!(is_callback(&ok, &redirect));
        assert_eq!(code_from_callback(&ok, "s1").ok().as_deref(), Some("abc"));
        assert!(code_from_callback(&ok, "other").is_err());
        let denied = Url::parse(
            "https://app.example.test/callback?error=access_denied&error_description=No",
        )
        .expect("url");
        assert!(
            code_from_callback(&denied, "s1")
                .is_err_and(|e| e.to_string() == "Sign-in failed: access_denied (No)")
        );
        let elsewhere = Url::parse("https://app.example.test/other?code=abc").expect("url");
        assert!(!is_callback(&elsewhere, &redirect));
    }

    #[test]
    fn parses_token_answers() {
        let token = parse_token(
            200,
            br#"{"access_token":"t1","expires_in":"3600","refresh_token":"r1","scope":"read"}"#,
            1_000,
        )
        .expect("token");
        assert_eq!(token.access_token, "t1");
        assert_eq!(token.expires_at, Some(3_601_000));
        assert!(token.is_fresh(3_500_000) && !token.is_fresh(3_590_000));
        assert!(
            parse_token(
                400,
                br#"{"error":"invalid_client","error_description":"Bad secret"}"#,
                0
            )
            .is_err_and(
                |e| e.to_string() == "The token server refused: invalid_client (Bad secret)"
            )
        );
        assert!(
            parse_token(500, b"oops", 0)
                .is_err_and(|e| e.to_string() == "The token server answered 500")
        );
        assert!(parse_token(200, br#"{"token_type":"bearer"}"#, 0).is_err());
    }

    #[test]
    fn requires_https_except_on_this_machine() {
        assert!(endpoint("http://localhost:8080/token", "token URL").is_ok());
        assert!(endpoint("http://login.example.test/token", "token URL").is_err());
        assert!(endpoint("ftp://login.example.test", "token URL").is_err());
    }
}
