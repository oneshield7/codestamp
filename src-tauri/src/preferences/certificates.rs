use std::path::Path;

use reqwest::tls::Identity;

use super::models::{CertificateInfo, CertificateKind};
use super::network::{apply, key_owner};
use super::repo::{self, NewCertificate};
use super::validate::{host_pattern_text, parse_host_pattern};
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::files::{display_name, read_limited};
use crate::request::client::HttpClient;
use crate::storage::db::{Database, new_id};

pub const CERTIFICATE_EXTENSIONS: [&str; 5] = ["pem", "crt", "cer", "key", "txt"];
const MAX_CERTIFICATE_BYTES: u64 = 256 * 1024;
const MAX_CERTIFICATES: i64 = 50;
const TOO_BIG: &str = "Certificate files can be up to 256 KB";

/// The PEM blocks of a file, split into certificates and private keys.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PemBlocks {
    pub certificates: Vec<String>,
    pub keys: Vec<String>,
}

pub fn split_pem(text: &str) -> PemBlocks {
    let mut blocks = PemBlocks::default();
    let mut rest = text;
    while let Some(start) = rest.find("-----BEGIN ") {
        let after = &rest[start..];
        let Some(label_end) = after[11..].find("-----") else {
            break;
        };
        let label = &after[11..11 + label_end];
        let end_marker = format!("-----END {label}-----");
        let Some(end) = after.find(&end_marker) else {
            break;
        };
        let block = after[..end + end_marker.len()].to_owned();
        if label == "CERTIFICATE" {
            blocks.certificates.push(block);
        } else if label.ends_with("PRIVATE KEY") {
            blocks.keys.push(block);
        }
        rest = &after[end + end_marker.len()..];
    }
    blocks
}

fn read_pem(path: &Path) -> Result<PemBlocks, AppError> {
    let bytes = read_limited(path, MAX_CERTIFICATE_BYTES, TOO_BIG)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| AppError::invalid("Choose a PEM file (.pem, .crt or .key)"))?;
    Ok(split_pem(&text))
}

fn check_room(db: &Database) -> Result<(), AppError> {
    if db.with(repo::count_certificates)? >= MAX_CERTIFICATES {
        return Err(AppError::invalid("Up to 50 certificates can be added"));
    }
    Ok(())
}

/// Adds a CA certificate that requests trust on top of the ones Windows trusts.
pub fn add_ca(
    db: &Database,
    vault: &Vault,
    client: &HttpClient,
    path: &Path,
) -> Result<CertificateInfo, AppError> {
    check_room(db)?;
    let blocks = read_pem(path)?;
    // A CA file is stored as it is, so a private key must never ride along in plain text.
    if !blocks.keys.is_empty() {
        return Err(AppError::invalid(
            "This file contains a private key. Add it as a client certificate instead",
        ));
    }
    if blocks.certificates.is_empty() {
        return Err(AppError::invalid("No certificate was found in this file"));
    }
    let id = new_id();
    let name = display_name(path, "CA certificate");
    let pem = blocks.certificates.join("\n");
    db.with(|conn| {
        repo::insert_certificate(
            conn,
            &NewCertificate {
                id: &id,
                kind: CertificateKind::Ca,
                name: &name,
                host: None,
                cert_pem: &pem,
                key_secret: None,
            },
        )
    })?;
    finish(db, vault, client, &id)
}

/// Adds a client certificate for one host. The key may be in the certificate file or in its
/// own file; it is stored encrypted, like secret variables.
pub fn add_client(
    db: &Database,
    vault: &Vault,
    client: &HttpClient,
    host: &str,
    certificate_path: &Path,
    key_path: Option<&Path>,
) -> Result<CertificateInfo, AppError> {
    check_room(db)?;
    let host = host_pattern_text(&parse_host_pattern(host)?);
    let mut blocks = read_pem(certificate_path)?;
    if let Some(key_path) = key_path {
        blocks.keys.extend(read_pem(key_path)?.keys);
    }
    let (Some(key), false) = (blocks.keys.first(), blocks.certificates.is_empty()) else {
        return Err(AppError::invalid(
            "Choose a certificate and its private key in PEM format",
        ));
    };
    let pem = blocks.certificates.join("\n");
    Identity::from_pem(format!("{pem}\n{key}").as_bytes()).map_err(|_| {
        AppError::invalid(
            "The certificate and key could not be read. An encrypted key is not supported",
        )
    })?;
    let id = new_id();
    let secret = vault.encrypt(&key_owner(&id), key)?;
    let name = display_name(certificate_path, "Client certificate");
    db.with(|conn| {
        repo::insert_certificate(
            conn,
            &NewCertificate {
                id: &id,
                kind: CertificateKind::Client,
                name: &name,
                host: Some(&host),
                cert_pem: &pem,
                key_secret: Some(&secret),
            },
        )
    })?;
    finish(db, vault, client, &id)
}

/// Puts the new certificate to use, or takes it out again when the client refuses it.
fn finish(
    db: &Database,
    vault: &Vault,
    client: &HttpClient,
    id: &str,
) -> Result<CertificateInfo, AppError> {
    if let Err(error) = apply(db, vault, client) {
        db.with(|conn| repo::delete_certificate(conn, id))?;
        return Err(error);
    }
    db.with(repo::list_certificates)?
        .into_iter()
        .find(|certificate| certificate.id == id)
        .ok_or(AppError::NotFound("The certificate"))
}

pub fn remove(db: &Database, vault: &Vault, client: &HttpClient, id: &str) -> Result<(), AppError> {
    db.with(|conn| repo::delete_certificate(conn, id))?;
    apply(db, vault, client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_certificates_from_keys() {
        let text = "junk\n-----BEGIN CERTIFICATE-----\nAAA\n-----END CERTIFICATE-----\n\
                    -----BEGIN EC PRIVATE KEY-----\nBBB\n-----END EC PRIVATE KEY-----\n\
                    -----BEGIN PUBLIC KEY-----\nCCC\n-----END PUBLIC KEY-----\n\
                    -----BEGIN CERTIFICATE-----\nDDD";
        let blocks = split_pem(text);
        assert_eq!(
            blocks.certificates,
            ["-----BEGIN CERTIFICATE-----\nAAA\n-----END CERTIFICATE-----"]
        );
        assert_eq!(
            blocks.keys,
            ["-----BEGIN EC PRIVATE KEY-----\nBBB\n-----END EC PRIVATE KEY-----"]
        );
    }
}
