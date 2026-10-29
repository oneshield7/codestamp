use std::sync::Arc;
use std::time::Duration;

use reqwest::redirect::Policy;
use reqwest::tls::{Certificate, Identity, Version};
use reqwest::{Client, ClientBuilder, NoProxy, Proxy, Url};

use crate::cookies::jar::CookieJar;
use crate::error::AppError;
use crate::preferences::models::TlsVersion;

pub const DEFAULT_USER_AGENT: &str = concat!("Codestamp/", env!("CARGO_PKG_VERSION"));
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How requests leave this device, from Settings › Requests, Proxy and Certificates.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub user_agent: String,
    pub follow_redirects: bool,
    pub cookies: bool,
    pub timeout: Duration,
    /// Downloads stop after this many bytes; `None` reads the whole response.
    pub max_response_bytes: Option<u64>,
    pub no_cache: bool,
    pub proxy: ProxyConfig,
    /// Extra trusted CA certificates (PEM), on top of the ones the operating system trusts.
    pub ca_pems: Vec<String>,
    pub identities: Vec<HostIdentity>,
    pub min_tls: TlsVersion,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            user_agent: DEFAULT_USER_AGENT.to_owned(),
            follow_redirects: true,
            cookies: true,
            timeout: DEFAULT_TIMEOUT,
            max_response_bytes: None,
            no_cache: false,
            proxy: ProxyConfig::System,
            ca_pems: Vec::new(),
            identities: Vec::new(),
            min_tls: TlsVersion::Tls12,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProxyConfig {
    /// The operating system's proxy settings (and `HTTPS_PROXY` style variables).
    System,
    Off,
    Custom {
        url: String,
        username: String,
        password: String,
        bypass: String,
    },
}

/// A client certificate with its private key (PEM), sent only to matching hosts.
#[derive(Debug, Clone)]
pub struct HostIdentity {
    pub host: HostPattern,
    pub pem: String,
}

/// `api.example.com`, `*.example.com`, optionally with `:port`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPattern {
    pub host: String,
    pub port: Option<u16>,
}

impl HostPattern {
    pub fn matches(&self, url: &Url) -> bool {
        let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
            return false;
        };
        let host_matches = match self.host.strip_prefix("*.") {
            Some(domain) => host
                .strip_suffix(domain)
                .is_some_and(|sub| sub.len() > 1 && sub.ends_with('.')),
            None => host == self.host,
        };
        host_matches
            && self
                .port
                .is_none_or(|port| url.port_or_known_default() == Some(port))
    }
}

/// The clients for one configuration: the shared one, and one per client certificate.
pub struct Clients {
    pub default: Client,
    pub by_host: Vec<(HostPattern, Client)>,
    pub timeout: Duration,
    pub max_response_bytes: Option<u64>,
    pub no_cache: bool,
}

impl Clients {
    pub fn build(jar: &Arc<CookieJar>, config: &NetworkConfig) -> Result<Self, AppError> {
        let roots = ca_certificates(&config.ca_pems)?;
        let proxy = proxy(&config.proxy)?;
        let by_host = config
            .identities
            .iter()
            .map(|identity| {
                let key = Identity::from_pem(identity.pem.as_bytes())
                    .map_err(|_| AppError::invalid("A client certificate could not be read"))?;
                let client = builder(jar, config, &roots, proxy.as_ref())
                    .identity(key)
                    .build()
                    .map_err(|_| AppError::ClientInit)?;
                Ok((identity.host.clone(), client))
            })
            .collect::<Result<_, AppError>>()?;
        Ok(Self {
            default: builder(jar, config, &roots, proxy.as_ref())
                .build()
                .map_err(|_| AppError::ClientInit)?,
            by_host,
            timeout: config.timeout,
            max_response_bytes: config.max_response_bytes,
            no_cache: config.no_cache,
        })
    }

    pub fn for_url(&self, url: &Url) -> &Client {
        self.by_host
            .iter()
            .find(|(pattern, _)| pattern.matches(url))
            .map_or(&self.default, |(_, client)| client)
    }
}

fn builder(
    jar: &Arc<CookieJar>,
    config: &NetworkConfig,
    roots: &[Certificate],
    proxy: Option<&Proxy>,
) -> ClientBuilder {
    let mut builder = Client::builder()
        .user_agent(config.user_agent.as_str())
        .redirect(if config.follow_redirects {
            redirect_policy()
        } else {
            Policy::none()
        })
        .tls_certs_merge(roots.iter().cloned())
        .tls_version_min(match config.min_tls {
            TlsVersion::Tls12 => Version::TLS_1_2,
            TlsVersion::Tls13 => Version::TLS_1_3,
        });
    if config.cookies {
        builder = builder.cookie_provider(Arc::clone(jar));
    }
    match (&config.proxy, proxy) {
        (ProxyConfig::Off, _) => builder.no_proxy(),
        (_, Some(proxy)) => builder.proxy(proxy.clone()),
        (_, None) => builder,
    }
}

fn ca_certificates(pems: &[String]) -> Result<Vec<Certificate>, AppError> {
    let mut roots = Vec::new();
    for pem in pems {
        let certificates = Certificate::from_pem_bundle(pem.as_bytes())
            .map_err(|_| AppError::invalid("A CA certificate could not be read"))?;
        roots.extend(certificates);
    }
    Ok(roots)
}

fn proxy(config: &ProxyConfig) -> Result<Option<Proxy>, AppError> {
    let ProxyConfig::Custom {
        url,
        username,
        password,
        bypass,
    } = config
    else {
        return Ok(None);
    };
    let mut proxy = Proxy::all(url.as_str())
        .map_err(|_| AppError::invalid("The proxy address is not valid"))?;
    if !username.is_empty() {
        proxy = proxy.basic_auth(username, password);
    }
    Ok(Some(proxy.no_proxy(NoProxy::from_string(bypass))))
}

/// Headers such as API keys or tokens filled in from variables must not follow a redirect to
/// another host, so those redirects are returned to the user instead of being followed.
fn redirect_policy() -> Policy {
    Policy::custom(|attempt| {
        if attempt.previous().len() > MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        let same_site = attempt
            .previous()
            .first()
            .is_none_or(|first| same_site(first, attempt.url()));
        if same_site {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

const MAX_REDIRECTS: usize = 10;

/// Same host and port, or the same host moving from http to https.
pub fn same_site(from: &Url, to: &Url) -> bool {
    from.host_str() == to.host_str()
        && (from.port_or_known_default() == to.port_or_known_default()
            || (from.scheme() == "http" && to.scheme() == "https"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).expect("url")
    }

    #[test]
    fn follows_redirects_within_the_same_site() {
        let parse = url;
        assert!(same_site(
            &parse("http://api.example.com/a"),
            &parse("https://api.example.com/b")
        ));
        assert!(same_site(
            &parse("https://api.example.com/a"),
            &parse("https://api.example.com:443/b")
        ));
        assert!(!same_site(
            &parse("https://api.example.com/a"),
            &parse("http://api.example.com/b")
        ));
        assert!(!same_site(
            &parse("https://api.example.com/a"),
            &parse("https://evil.example.net/b")
        ));
    }

    #[test]
    fn host_patterns_match_hosts_ports_and_subdomains() {
        let exact = HostPattern {
            host: "api.example.com".to_owned(),
            port: None,
        };
        assert!(exact.matches(&url("https://API.example.com/users")));
        assert!(!exact.matches(&url("https://example.com/")));

        let wildcard = HostPattern {
            host: "*.example.com".to_owned(),
            port: Some(8443),
        };
        assert!(wildcard.matches(&url("https://a.b.example.com:8443/")));
        assert!(!wildcard.matches(&url("https://a.example.com/")));
        assert!(!wildcard.matches(&url("https://badexample.com:8443/")));
        assert!(!wildcard.matches(&url("https://example.com:8443/")));
    }

    #[test]
    fn refuses_proxies_and_certificates_it_cannot_read() {
        let jar = Arc::new(CookieJar::default());
        let bad_proxy = NetworkConfig {
            proxy: ProxyConfig::Custom {
                url: "not a proxy".to_owned(),
                username: String::new(),
                password: String::new(),
                bypass: String::new(),
            },
            ..NetworkConfig::default()
        };
        assert!(matches!(
            Clients::build(&jar, &bad_proxy),
            Err(AppError::InvalidInput(message)) if message.contains("proxy")
        ));
        let bad_ca = NetworkConfig {
            ca_pems: vec![
                "-----BEGIN CERTIFICATE-----\nnope\n-----END CERTIFICATE-----".to_owned(),
            ],
            ..NetworkConfig::default()
        };
        assert!(Clients::build(&jar, &bad_ca).is_err());
        assert!(Clients::build(&jar, &NetworkConfig::default()).is_ok());
    }
}
