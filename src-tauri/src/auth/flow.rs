//! Auth that cannot be a header made up front: Digest answers the server's challenge and AWS
//! Signature signs the final request.

use ::http::header::AUTHORIZATION;
use ::http::{HeaderMap, HeaderName, HeaderValue};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use url::Url;
use uuid::Uuid;

use super::aws::{self, AwsCredentials};
use super::digest::{self, DigestInput};
use super::models::Auth;
use crate::encoding::hex_lower;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::request::types::{HttpResponseOutput, NameValue, RequestBody, encode_form};
use crate::request::validate::{ValidatedBody, ValidatedRequest, graphql_json};
use crate::storage::db::now_ms;

const UNSIGNABLE_BODY: &str = "AWS Signature works with raw, URL-encoded and file bodies";

/// Sends the request, answering a Digest challenge or signing it for AWS when that is its auth.
pub async fn send(
    client: &HttpClient,
    mut request: ValidatedRequest,
    auth: Option<&Auth>,
) -> Result<HttpResponseOutput, AppError> {
    match auth {
        Some(Auth::Digest { username, password }) => {
            send_with_digest(client, request, username, password).await
        }
        Some(Auth::AwsSigV4 { .. }) => {
            let payload = body_hash(request.body.as_ref()).await?;
            sign_aws(
                &mut request.headers,
                request.method.as_str(),
                &request.url,
                &payload,
                auth,
            )?;
            client.send(request).await
        }
        _ => client.send(request).await,
    }
}

/// The first attempt goes without credentials; a 401 with a Digest challenge is answered once.
async fn send_with_digest(
    client: &HttpClient,
    request: ValidatedRequest,
    username: &str,
    password: &str,
) -> Result<HttpResponseOutput, AppError> {
    if username.is_empty() {
        return Err(AppError::invalid("Digest auth needs a username"));
    }
    if request.headers.contains_key(AUTHORIZATION) {
        return client.send(request).await;
    }
    let mut retry = request.clone();
    let first = client.send(request).await?;
    if first.status != 401 {
        return Ok(first);
    }
    let Some(challenge) = digest::find_challenge(&first.headers)? else {
        return Ok(first);
    };
    let uri = match retry.url.query() {
        Some(query) => format!("{}?{query}", retry.url.path()),
        None => retry.url.path().to_owned(),
    };
    let cnonce = Uuid::new_v4().simple().to_string();
    let header = digest::authorization(
        &challenge,
        &DigestInput {
            username,
            password,
            method: retry.method.as_str(),
            uri: &uri,
            cnonce: &cnonce,
        },
    );
    let value = HeaderValue::from_str(&header)
        .map_err(|_| AppError::invalid("The Digest credentials cannot be sent in a header"))?;
    retry.headers.insert(AUTHORIZATION, value);
    client.send(retry).await
}

fn sign_aws(
    headers: &mut HeaderMap,
    method: &str,
    url: &Url,
    payload: &str,
    auth: Option<&Auth>,
) -> Result<(), AppError> {
    let Some(Auth::AwsSigV4 {
        access_key,
        secret_key,
        session_token,
        region,
        service,
    }) = auth
    else {
        return Ok(());
    };
    let credentials = AwsCredentials {
        access_key,
        secret_key,
        session_token,
        region,
        service,
    };
    aws::sign(method, url, headers, payload, &credentials, now_ms())
}

async fn body_hash(body: Option<&ValidatedBody>) -> Result<String, AppError> {
    match body {
        None => Ok(aws::payload_hash(b"")),
        Some(ValidatedBody::Text(text)) => Ok(aws::payload_hash(text.as_bytes())),
        Some(ValidatedBody::File(path)) => {
            let mut file = tokio::fs::File::open(path)
                .await
                .map_err(|_| AppError::FileAccess)?;
            let mut hasher = Sha256::new();
            let mut buffer = vec![0; 64 * 1024];
            loop {
                let read = file
                    .read(&mut buffer)
                    .await
                    .map_err(|_| AppError::FileAccess)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok(hex_lower(&hasher.finalize()))
        }
        Some(ValidatedBody::Multipart(_)) => Err(AppError::invalid(UNSIGNABLE_BODY)),
    }
}

/// Adds the AWS signature headers to a request shown as code, signed for the current time.
pub fn sign_for_code(
    method: &str,
    url: &Url,
    headers: &mut Vec<NameValue>,
    body: Option<&RequestBody>,
    auth: &Auth,
) -> Result<(), AppError> {
    let payload = match body {
        None => aws::payload_hash(b""),
        Some(RequestBody::Raw { text }) => aws::payload_hash(text.as_bytes()),
        Some(RequestBody::Urlencoded { fields }) => {
            aws::payload_hash(encode_form(fields).as_bytes())
        }
        Some(RequestBody::Graphql { query, variables }) => {
            aws::payload_hash(graphql_json(query.clone(), variables)?.as_bytes())
        }
        Some(RequestBody::FormData { .. } | RequestBody::Binary { .. }) => {
            return Err(AppError::invalid(
                "Copy as code can sign raw and URL-encoded bodies only",
            ));
        }
    };
    let mut map = HeaderMap::new();
    for header in headers.iter() {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(header.name.trim().as_bytes()),
            HeaderValue::from_str(&header.value),
        ) {
            map.append(name, value);
        }
    }
    let before: Vec<HeaderName> = map.keys().cloned().collect();
    sign_aws(&mut map, method, url, &payload, Some(auth))?;
    for (name, value) in &map {
        if !before.contains(name) {
            headers.push(NameValue {
                name: name.as_str().to_owned(),
                value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
            });
        }
    }
    Ok(())
}
