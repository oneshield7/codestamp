use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Indent {
    /// New lines start at the left edge, and Beautify puts each element on its own line
    /// without indenting it.
    #[default]
    None,
    Two,
    Four,
    Tab,
}

/// Where a request card shows the response: below the request or next to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CardLayout {
    #[default]
    Stacked,
    Side,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct EditorPreferences {
    pub indent: Indent,
    pub close_brackets: bool,
    pub wrap_lines: bool,
    pub layout: CardLayout,
    pub variable_hints: bool,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            indent: Indent::None,
            close_brackets: true,
            wrap_lines: false,
            layout: CardLayout::Stacked,
            variable_hints: true,
        }
    }
}

/// The oldest TLS version a server may answer with. Certificates are checked either way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum TlsVersion {
    #[default]
    #[serde(rename = "1.2")]
    Tls12,
    #[serde(rename = "1.3")]
    Tls13,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct RequestPreferences {
    /// Used when a request does not set its own timeout.
    pub timeout_seconds: u32,
    /// `0` downloads responses of any size.
    pub max_response_mb: u32,
    pub follow_redirects: bool,
    pub cookies: bool,
    pub no_cache: bool,
    /// Empty sends `Codestamp/<version>`.
    pub user_agent: String,
    /// Settings › SSL/TLS.
    pub min_tls_version: TlsVersion,
}

impl Default for RequestPreferences {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_response_mb: 0,
            follow_redirects: true,
            cookies: true,
            no_cache: false,
            user_agent: String::new(),
            min_tls_version: TlsVersion::Tls12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct HistoryPreferences {
    pub max_entries: u32,
    pub keep_days: u32,
}

impl Default for HistoryPreferences {
    fn default() -> Self {
        Self {
            max_entries: 1_000,
            keep_days: 30,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct Preferences {
    pub editor: EditorPreferences,
    pub requests: RequestPreferences,
    pub history: HistoryPreferences,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// The operating system's proxy, as before this setting existed.
    #[default]
    System,
    Off,
    Custom,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyScheme {
    #[default]
    Http,
    Https,
    Socks5,
}

/// Saved without the password, which is kept encrypted on its own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct ProxySettings {
    pub mode: ProxyMode,
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// Hosts that skip the proxy, separated by commas (`localhost, .internal.test`).
    pub bypass: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyInput {
    pub settings: ProxySettings,
    /// `None` keeps the saved password; an empty text removes it.
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyView {
    pub settings: ProxySettings,
    pub has_password: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CertificateKind {
    Ca,
    Client,
}

impl CertificateKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ca => "ca",
            Self::Client => "client",
        }
    }
}

/// What the UI shows of a certificate; the PEM and the private key stay in the Rust core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    pub id: String,
    pub kind: CertificateKind,
    pub name: String,
    pub host: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    pub version: String,
    pub data_dir: String,
    pub database_bytes: u64,
}
