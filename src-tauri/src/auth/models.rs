use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OAuthGrant {
    #[default]
    AuthorizationCode,
    ClientCredentials,
    Password,
}

/// Where the client id and secret go when asking for a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientAuth {
    /// An `Authorization: Basic` header (RFC 6749 §2.3.1, the recommended way).
    #[default]
    Header,
    Body,
}

/// OAuth 2.0 settings. The token itself is not part of them: it is kept encrypted on this
/// device and never shared with the collection.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OAuth2Settings {
    pub grant: OAuthGrant,
    #[serde(default)]
    pub auth_url: String,
    pub token_url: String,
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default = "pkce_by_default")]
    pub use_pkce: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub client_auth: ClientAuth,
    /// Empty means `Bearer`.
    #[serde(default)]
    pub header_prefix: String,
}

const fn pkce_by_default() -> bool {
    true
}

impl OAuth2Settings {
    fn templates(&self) -> Vec<&String> {
        vec![
            &self.auth_url,
            &self.token_url,
            &self.client_id,
            &self.client_secret,
            &self.scope,
            &self.redirect_uri,
            &self.username,
            &self.password,
            &self.header_prefix,
        ]
    }

    fn templates_mut(&mut self) -> Vec<&mut String> {
        vec![
            &mut self.auth_url,
            &mut self.token_url,
            &mut self.client_id,
            &mut self.client_secret,
            &mut self.scope,
            &mut self.redirect_uri,
            &mut self.username,
            &mut self.password,
            &mut self.header_prefix,
        ]
    }
}

/// How a request authenticates. Values may contain `{{variables}}`, filled in when sending.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Auth {
    /// Uses the collection's auth; only requests can inherit.
    #[default]
    Inherit,
    None,
    Bearer {
        token: String,
        prefix: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        location: ApiKeyLocation,
    },
    /// RFC 7616: answered after the server's 401 challenge.
    Digest {
        username: String,
        password: String,
    },
    /// AWS Signature Version 4, computed from the final request just before it goes out.
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        #[serde(default)]
        session_token: String,
        region: String,
        service: String,
    },
    #[serde(rename = "oauth2")]
    OAuth2(Box<OAuth2Settings>),
}

impl Auth {
    pub fn to_stored(&self) -> Result<String, AppError> {
        serde_json::to_string(self).map_err(|_| AppError::Storage)
    }

    /// Rows written before auth existed, or edited by hand, fall back to `fallback`.
    pub fn from_stored(text: &str, fallback: Self) -> Self {
        serde_json::from_str(text).unwrap_or(fallback)
    }

    /// The values that may contain `{{variables}}`.
    pub fn templates(&self) -> Vec<&String> {
        match self {
            Self::Inherit | Self::None => Vec::new(),
            Self::Bearer { token, prefix } => vec![token, prefix],
            Self::Basic { username, password } | Self::Digest { username, password } => {
                vec![username, password]
            }
            Self::ApiKey { key, value, .. } => vec![key, value],
            Self::AwsSigV4 {
                access_key,
                secret_key,
                session_token,
                region,
                service,
            } => vec![access_key, secret_key, session_token, region, service],
            Self::OAuth2(settings) => settings.templates(),
        }
    }

    pub fn templates_mut(&mut self) -> Vec<&mut String> {
        match self {
            Self::Inherit | Self::None => Vec::new(),
            Self::Bearer { token, prefix } => vec![token, prefix],
            Self::Basic { username, password } | Self::Digest { username, password } => {
                vec![username, password]
            }
            Self::ApiKey { key, value, .. } => vec![key, value],
            Self::AwsSigV4 {
                access_key,
                secret_key,
                session_token,
                region,
                service,
            } => vec![access_key, secret_key, session_token, region, service],
            Self::OAuth2(settings) => settings.templates_mut(),
        }
    }
}
