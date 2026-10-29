use std::time::Duration;

use rusqlite::Connection;

use super::models::{ProxyMode, ProxyScheme, ProxySettings, RequestPreferences};
use super::repo::{self, PROXY_PASSWORD_KEY};
use super::validate::parse_host_pattern;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::request::network::{DEFAULT_USER_AGENT, HostIdentity, NetworkConfig, ProxyConfig};
use crate::storage::db::Database;

const BYTES_PER_MB: u64 = 1024 * 1024;

/// The vault owner of a client certificate's private key.
pub fn key_owner(certificate_id: &str) -> String {
    format!("certificate:{certificate_id}")
}

/// Everything saved that decides how requests leave this device.
pub fn load_config(conn: &Connection, vault: &Vault) -> Result<NetworkConfig, AppError> {
    let requests = repo::load_preferences(conn)?.requests;
    let proxy = repo::load_proxy(conn)?;
    let password = proxy_password(conn, vault)?;
    config_with(conn, vault, &requests, &proxy, password)
}

/// The configuration with these request and proxy settings and the saved certificates.
pub fn config_with(
    conn: &Connection,
    vault: &Vault,
    requests: &RequestPreferences,
    proxy: &ProxySettings,
    password: String,
) -> Result<NetworkConfig, AppError> {
    let mut config = config_from(requests, proxy, password);
    config.ca_pems = repo::ca_pems(conn)?;
    config.identities = repo::client_certificates(conn)?
        .into_iter()
        .map(|stored| {
            let key = vault.decrypt(&key_owner(&stored.id), &stored.key_secret)?;
            Ok(HostIdentity {
                host: parse_host_pattern(&stored.host)?,
                pem: format!("{}\n{key}", stored.cert_pem),
            })
        })
        .collect::<Result<_, AppError>>()?;
    Ok(config)
}

pub fn proxy_password(conn: &Connection, vault: &Vault) -> Result<String, AppError> {
    repo::load_secret(conn, PROXY_PASSWORD_KEY)?.map_or(Ok(String::new()), |blob| {
        vault.decrypt(PROXY_PASSWORD_KEY, &blob)
    })
}

fn config_from(
    requests: &RequestPreferences,
    proxy: &ProxySettings,
    password: String,
) -> NetworkConfig {
    NetworkConfig {
        user_agent: if requests.user_agent.is_empty() {
            DEFAULT_USER_AGENT.to_owned()
        } else {
            requests.user_agent.clone()
        },
        follow_redirects: requests.follow_redirects,
        cookies: requests.cookies,
        timeout: Duration::from_secs(u64::from(requests.timeout_seconds)),
        max_response_bytes: (requests.max_response_mb > 0)
            .then(|| u64::from(requests.max_response_mb) * BYTES_PER_MB),
        no_cache: requests.no_cache,
        proxy: match proxy.mode {
            ProxyMode::System => ProxyConfig::System,
            ProxyMode::Off => ProxyConfig::Off,
            ProxyMode::Custom => ProxyConfig::Custom {
                url: format!("{}://{}:{}", scheme(proxy.scheme), proxy.host, proxy.port),
                username: proxy.username.clone(),
                password,
                bypass: proxy.bypass.clone(),
            },
        },
        ca_pems: Vec::new(),
        identities: Vec::new(),
        min_tls: requests.min_tls_version,
    }
}

const fn scheme(scheme: ProxyScheme) -> &'static str {
    match scheme {
        ProxyScheme::Http => "http",
        ProxyScheme::Https => "https",
        ProxyScheme::Socks5 => "socks5",
    }
}

/// Switches the shared HTTP client to what is saved now.
pub fn apply(db: &Database, vault: &Vault, client: &HttpClient) -> Result<(), AppError> {
    let config = db.with(|conn| load_config(conn, vault))?;
    client.configure(&config)
}

/// Why the saved settings cannot be used, if they cannot; requests then go out with defaults.
pub fn problem(db: &Database, vault: &Vault, client: &HttpClient) -> Option<String> {
    db.with(|conn| load_config(conn, vault))
        .and_then(|config| client.check(&config))
        .err()
        .map(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::models::TlsVersion;

    #[test]
    fn turns_settings_into_a_client_configuration() {
        let requests = RequestPreferences {
            timeout_seconds: 5,
            max_response_mb: 2,
            min_tls_version: TlsVersion::Tls13,
            ..RequestPreferences::default()
        };
        let proxy = ProxySettings {
            mode: ProxyMode::Custom,
            scheme: ProxyScheme::Socks5,
            host: "proxy.test".to_owned(),
            port: 1080,
            username: "me".to_owned(),
            bypass: "localhost".to_owned(),
        };
        let config = config_from(&requests, &proxy, "pw".to_owned());
        assert_eq!(config.user_agent, DEFAULT_USER_AGENT);
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.max_response_bytes, Some(2 * BYTES_PER_MB));
        assert_eq!(config.min_tls, TlsVersion::Tls13);
        assert!(matches!(
            config.proxy,
            ProxyConfig::Custom { ref url, ref password, .. } if url == "socks5://proxy.test:1080" && password == "pw"
        ));
        let unlimited = config_from(
            &RequestPreferences::default(),
            &ProxySettings::default(),
            String::new(),
        );
        assert_eq!(unlimited.max_response_bytes, None);
        assert!(matches!(unlimited.proxy, ProxyConfig::System));
        assert_eq!(unlimited.min_tls, TlsVersion::Tls12);
    }

    #[test]
    fn settings_saved_before_the_tls_choice_still_load() {
        let older: RequestPreferences =
            serde_json::from_str(r#"{"timeoutSeconds":30,"userAgent":""}"#).expect("older");
        assert_eq!(older.min_tls_version, TlsVersion::Tls12);
        let newer: RequestPreferences =
            serde_json::from_str(r#"{"minTlsVersion":"1.3"}"#).expect("newer");
        assert_eq!(newer.min_tls_version, TlsVersion::Tls13);
        assert!(serde_json::from_str::<RequestPreferences>(r#"{"minTlsVersion":"1.0"}"#).is_err());
    }
}
