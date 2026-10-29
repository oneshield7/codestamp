//! Downloads of files to import (collections, specs, WSDLs), through the same network settings
//! as requests.

use futures_util::StreamExt;
use url::Url;

use super::postman::MAX_IMPORT_BYTES;
use crate::error::AppError;
use crate::request::client::{HttpClient, map_error};
use crate::request::rules::MAX_URL_LENGTH;
use crate::request::validate::secure_url;

/// A URL Codestamp downloads an import from: HTTPS, or plain HTTP on this machine.
pub fn checked_url(raw: &str, what: &str) -> Result<Url, AppError> {
    if raw.len() > MAX_URL_LENGTH {
        return Err(AppError::invalid("The URL is too long"));
    }
    secure_url(raw, what)
}

/// Downloads a UTF-8 text file of at most 20 MB, so the proxy and certificate settings from
/// Settings apply.
pub async fn fetch_text(client: &HttpClient, url: &Url, accept: &str) -> Result<String, AppError> {
    let response = client
        .client_for(url)?
        .get(url.clone())
        .timeout(client.default_timeout()?)
        .header(reqwest::header::ACCEPT, accept)
        .send()
        .await
        .map_err(map_error)?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::InvalidInput(format!(
            "The server answered {}",
            status.as_u16()
        )));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        body.extend_from_slice(&chunk.map_err(map_error)?);
        if body.len() > MAX_IMPORT_BYTES {
            return Err(AppError::invalid("The file is larger than 20 MB"));
        }
    }
    String::from_utf8(body).map_err(|_| AppError::invalid("The file is not UTF-8 text"))
}
