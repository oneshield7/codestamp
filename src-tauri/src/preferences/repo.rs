use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::models::{CertificateInfo, CertificateKind, Preferences, ProxySettings};
use crate::error::AppError;
use crate::storage::db::now_ms;

const PREFERENCES_KEY: &str = "preferences";
const PROXY_KEY: &str = "proxy";
pub const PROXY_PASSWORD_KEY: &str = "proxy_password";

/// Saved settings, or the defaults when nothing was saved yet or it cannot be read anymore.
pub fn load_preferences(conn: &Connection) -> Result<Preferences, AppError> {
    Ok(load_setting(conn, PREFERENCES_KEY)?.unwrap_or_default())
}

pub fn save_preferences(conn: &Connection, preferences: &Preferences) -> Result<(), AppError> {
    save_setting(conn, PREFERENCES_KEY, preferences)
}

pub fn load_proxy(conn: &Connection) -> Result<ProxySettings, AppError> {
    Ok(load_setting(conn, PROXY_KEY)?.unwrap_or_default())
}

pub fn save_proxy(conn: &Connection, proxy: &ProxySettings) -> Result<(), AppError> {
    save_setting(conn, PROXY_KEY, proxy)
}

fn load_setting<T: DeserializeOwned>(conn: &Connection, key: &str) -> Result<Option<T>, AppError> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?;
    Ok(stored.and_then(|json| serde_json::from_str(&json).ok()))
}

fn save_setting<T: Serialize>(conn: &Connection, key: &str, value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string(value).map_err(|_| AppError::Storage)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, json],
    )?;
    Ok(())
}

/// An encrypted value such as the proxy password.
pub fn load_secret(conn: &Connection, key: &str) -> Result<Option<Vec<u8>>, AppError> {
    Ok(conn
        .query_row(
            "SELECT data FROM app_secrets WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn save_secret(conn: &Connection, key: &str, data: &[u8]) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO app_secrets (key, data) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET data = excluded.data",
        params![key, data],
    )?;
    Ok(())
}

pub fn delete_secret(conn: &Connection, key: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM app_secrets WHERE key = ?1", params![key])?;
    Ok(())
}

pub struct NewCertificate<'a> {
    pub id: &'a str,
    pub kind: CertificateKind,
    pub name: &'a str,
    pub host: Option<&'a str>,
    pub cert_pem: &'a str,
    pub key_secret: Option<&'a [u8]>,
}

pub fn insert_certificate(conn: &Connection, certificate: &NewCertificate) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO certificates (id, kind, name, host, cert_pem, key_secret, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            certificate.id,
            certificate.kind.as_str(),
            certificate.name,
            certificate.host,
            certificate.cert_pem,
            certificate.key_secret,
            now_ms()
        ],
    )?;
    Ok(())
}

pub fn list_certificates(conn: &Connection) -> Result<Vec<CertificateInfo>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, kind, name, host, created_at FROM certificates ORDER BY kind, created_at",
    )?;
    let rows = statement
        .query_map([], |row| {
            let kind: String = row.get(1)?;
            Ok(CertificateInfo {
                id: row.get(0)?,
                kind: if kind == CertificateKind::Client.as_str() {
                    CertificateKind::Client
                } else {
                    CertificateKind::Ca
                },
                name: row.get(2)?,
                host: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub fn count_certificates(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row("SELECT COUNT(*) FROM certificates", [], |row| row.get(0))?)
}

/// Every CA certificate (PEM).
pub fn ca_pems(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut statement =
        conn.prepare("SELECT cert_pem FROM certificates WHERE kind = ?1 ORDER BY created_at")?;
    let rows = statement
        .query_map(params![CertificateKind::Ca.as_str()], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub struct StoredClientCertificate {
    pub id: String,
    pub host: String,
    pub cert_pem: String,
    pub key_secret: Vec<u8>,
}

pub fn client_certificates(conn: &Connection) -> Result<Vec<StoredClientCertificate>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, host, cert_pem, key_secret FROM certificates
         WHERE kind = ?1 AND host IS NOT NULL AND key_secret IS NOT NULL ORDER BY created_at",
    )?;
    let rows = statement
        .query_map(params![CertificateKind::Client.as_str()], |row| {
            Ok(StoredClientCertificate {
                id: row.get(0)?,
                host: row.get(1)?,
                cert_pem: row.get(2)?,
                key_secret: row.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub fn delete_certificate(conn: &Connection, id: &str) -> Result<(), AppError> {
    if conn.execute("DELETE FROM certificates WHERE id = ?1", params![id])? == 0 {
        return Err(AppError::NotFound("The certificate"));
    }
    Ok(())
}
