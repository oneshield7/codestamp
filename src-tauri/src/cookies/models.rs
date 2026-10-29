use serde::{Deserialize, Serialize};

/// A cookie set by one response, as shown in its Cookies tab.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    /// Unix time in seconds; `None` for a session cookie.
    pub expires: Option<i64>,
    pub max_age: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

/// A cookie in the jar. The value is left out; it is read on request with `reveal_cookie`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCookie {
    pub domain: String,
    pub path: String,
    pub name: String,
    /// Unix time in seconds; `None` until the end of the session.
    pub expires: Option<i64>,
    /// Sent to this exact host only (no Domain attribute), not to its subdomains.
    pub host_only: bool,
    pub secure: bool,
    pub http_only: bool,
}

/// Identifies one cookie in the jar.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CookieKey {
    pub domain: String,
    pub path: String,
    pub name: String,
}
