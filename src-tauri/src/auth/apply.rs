use super::models::{ApiKeyLocation, Auth};
use crate::encoding::base64_encode;
use crate::error::AppError;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::request::types::{HttpRequestInput, NameValue};
use crate::storage::db::Database;
use crate::storage::repo;

const AUTHORIZATION: &str = "Authorization";
const MAX_PREFIX_CHARS: usize = 40;

/// Checked when saving. Incomplete settings are fine here, since the user may still be typing.
pub fn check_auth(auth: &Auth, allow_inherit: bool) -> Result<(), AppError> {
    if !allow_inherit && *auth == Auth::Inherit {
        return Err(AppError::invalid("A collection cannot inherit auth"));
    }
    if auth
        .templates()
        .iter()
        .any(|text| text.len() > MAX_KEY_VALUE_LENGTH)
    {
        return Err(AppError::invalid("An auth value is too long"));
    }
    let prefix = match auth {
        Auth::Bearer { prefix, .. } => Some(prefix),
        Auth::OAuth2(settings) => Some(&settings.header_prefix),
        _ => None,
    };
    if prefix.is_some_and(|prefix| {
        prefix.chars().count() > MAX_PREFIX_CHARS || prefix.chars().any(char::is_control)
    }) {
        return Err(AppError::invalid(
            "The header prefix can be up to 40 plain characters",
        ));
    }
    Ok(())
}

/// The auth a request really uses: its own, or its collection's when it inherits.
/// `None` means the caller does not use auth at all (for example a replayed history entry).
pub fn effective_auth(
    db: &Database,
    auth: Option<Auth>,
    collection_id: Option<&str>,
) -> Result<Auth, AppError> {
    match (auth, collection_id) {
        (None, _) | (Some(Auth::Inherit), None) => Ok(Auth::None),
        (Some(Auth::Inherit), Some(id)) => db.with(|conn| repo::collection_auth(conn, id)),
        (Some(own), _) => Ok(own),
    }
}

/// Adds the auth header or query parameter to a request whose variables are already filled in.
/// A header or parameter that the user typed themselves wins over the generated one. Digest and
/// AWS Signature are added while sending (see `auth::flow`), and an OAuth 2.0 token arrives here
/// as Bearer auth.
pub fn apply_auth(request: &mut HttpRequestInput, auth: &Auth) -> Result<(), AppError> {
    apply_auth_to(&mut request.headers, &mut request.params, auth)
}

/// The same for any list of headers (such as gRPC metadata) and query parameters.
pub fn apply_auth_to(
    headers: &mut Vec<NameValue>,
    params: &mut Vec<NameValue>,
    auth: &Auth,
) -> Result<(), AppError> {
    match auth {
        Auth::Inherit | Auth::None | Auth::Digest { .. } | Auth::AwsSigV4 { .. } => {}
        Auth::OAuth2(_) => {
            return Err(AppError::invalid(
                "Get an access token first: Auth tab › Get new access token",
            ));
        }
        Auth::Bearer { token, prefix } => {
            if token.trim().is_empty() {
                return Err(AppError::invalid("Bearer auth needs a token"));
            }
            let value = match prefix.trim() {
                "" => token.trim().to_owned(),
                prefix => format!("{prefix} {}", token.trim()),
            };
            add_header(headers, AUTHORIZATION, value);
        }
        Auth::Basic { username, password } => {
            if username.is_empty() {
                return Err(AppError::invalid("Basic auth needs a username"));
            }
            // RFC 7617: the first colon separates the username from the password.
            if username.contains(':') {
                return Err(AppError::invalid(
                    "A Basic auth username cannot contain a colon",
                ));
            }
            let credentials = base64_encode(format!("{username}:{password}").as_bytes());
            add_header(headers, AUTHORIZATION, format!("Basic {credentials}"));
        }
        Auth::ApiKey {
            key,
            value,
            location,
        } => {
            let key = key.trim();
            if key.is_empty() {
                return Err(AppError::invalid("API key auth needs a key name"));
            }
            let pairs = match location {
                ApiKeyLocation::Header => &mut *headers,
                ApiKeyLocation::Query => &mut *params,
            };
            let header = *location == ApiKeyLocation::Header;
            if !pairs.iter().any(|pair| same_name(&pair.name, key, header)) {
                pairs.push(NameValue {
                    name: key.to_owned(),
                    value: value.clone(),
                });
            }
        }
    }
    Ok(())
}

fn add_header(headers: &mut Vec<NameValue>, name: &str, value: String) {
    if !headers
        .iter()
        .any(|header| same_name(&header.name, name, true))
    {
        headers.push(NameValue {
            name: name.to_owned(),
            value,
        });
    }
}

/// Header names are case-insensitive; query parameter names are not.
fn same_name(existing: &str, name: &str, case_insensitive: bool) -> bool {
    let existing = existing.trim();
    if case_insensitive {
        existing.eq_ignore_ascii_case(name)
    } else {
        existing == name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> HttpRequestInput {
        HttpRequestInput {
            method: "GET".to_owned(),
            url: "https://api.example.com".to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body: None,
            timeout_ms: None,
            collection_id: None,
            request_id: None,
            name: None,
            pre_request_script: None,
            test_script: None,
            auth: None,
            record_history: true,
            path_variables: Vec::new(),
            iteration_data: Vec::new(),
            iteration: Default::default(),
        }
    }

    fn header(request: &HttpRequestInput, name: &str) -> Option<String> {
        request
            .headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.clone())
    }

    #[test]
    fn adds_bearer_and_basic_headers() {
        let mut bearer = request();
        let auth = Auth::Bearer {
            token: " abc ".to_owned(),
            prefix: "Bearer".to_owned(),
        };
        assert!(apply_auth(&mut bearer, &auth).is_ok());
        assert_eq!(
            header(&bearer, "authorization").as_deref(),
            Some("Bearer abc")
        );

        let mut basic = request();
        let auth = Auth::Basic {
            username: "user".to_owned(),
            password: "p@ss".to_owned(),
        };
        assert!(apply_auth(&mut basic, &auth).is_ok());
        assert_eq!(
            header(&basic, "Authorization").as_deref(),
            Some("Basic dXNlcjpwQHNz")
        );
    }

    #[test]
    fn adds_api_keys_to_headers_or_query() {
        let mut in_header = request();
        let header_key = Auth::ApiKey {
            key: "X-API-Key".to_owned(),
            value: "k1".to_owned(),
            location: ApiKeyLocation::Header,
        };
        assert!(apply_auth(&mut in_header, &header_key).is_ok());
        assert_eq!(header(&in_header, "x-api-key").as_deref(), Some("k1"));

        let mut in_query = request();
        let query_key = Auth::ApiKey {
            key: "api_key".to_owned(),
            value: "k2".to_owned(),
            location: ApiKeyLocation::Query,
        };
        assert!(apply_auth(&mut in_query, &query_key).is_ok());
        assert_eq!(in_query.params.len(), 1);
        assert!(in_query.headers.is_empty());
    }

    #[test]
    fn a_header_typed_by_the_user_wins() {
        let mut typed = request();
        typed.headers.push(NameValue {
            name: "authorization".to_owned(),
            value: "Custom 1".to_owned(),
        });
        let auth = Auth::Bearer {
            token: "abc".to_owned(),
            prefix: "Bearer".to_owned(),
        };
        assert!(apply_auth(&mut typed, &auth).is_ok());
        assert_eq!(typed.headers.len(), 1);
        assert_eq!(header(&typed, "Authorization").as_deref(), Some("Custom 1"));
    }

    #[test]
    fn rejects_incomplete_or_invalid_settings_when_sending() {
        let empty_token = Auth::Bearer {
            token: "  ".to_owned(),
            prefix: "Bearer".to_owned(),
        };
        let colon_user = Auth::Basic {
            username: "a:b".to_owned(),
            password: String::new(),
        };
        let no_key = Auth::ApiKey {
            key: String::new(),
            value: "v".to_owned(),
            location: ApiKeyLocation::Query,
        };
        for auth in [empty_token, colon_user, no_key] {
            assert!(apply_auth(&mut request(), &auth).is_err(), "{auth:?}");
        }
    }

    #[test]
    fn checks_saved_settings() {
        assert!(check_auth(&Auth::Inherit, true).is_ok());
        assert!(check_auth(&Auth::Inherit, false).is_err());
        let bad_prefix = Auth::Bearer {
            token: String::new(),
            prefix: "Bearer\r\nX-Evil: 1".to_owned(),
        };
        assert!(check_auth(&bad_prefix, true).is_err());
        let long = Auth::Basic {
            username: "u".repeat(MAX_KEY_VALUE_LENGTH + 1),
            password: String::new(),
        };
        assert!(check_auth(&long, true).is_err());
    }

    #[test]
    fn inherits_the_collection_auth() {
        let db = Database::in_memory().expect("database");
        let collection = db
            .with(|conn| repo::insert_collection(conn, "Users API"))
            .expect("collection");
        let shared = Auth::Bearer {
            token: "{{token}}".to_owned(),
            prefix: "Bearer".to_owned(),
        };
        db.with(|conn| repo::set_collection_auth(conn, &collection.id, &shared))
            .expect("set auth");

        let inherited = effective_auth(&db, Some(Auth::Inherit), Some(&collection.id));
        assert_eq!(inherited.ok(), Some(shared));
        let own = effective_auth(&db, Some(Auth::None), Some(&collection.id));
        assert_eq!(own.ok(), Some(Auth::None));
        let outside = effective_auth(&db, Some(Auth::Inherit), None);
        assert_eq!(outside.ok(), Some(Auth::None));
    }

    #[test]
    fn stored_auth_round_trips_and_falls_back() {
        let auth = Auth::ApiKey {
            key: "k".to_owned(),
            value: "{{v}}".to_owned(),
            location: ApiKeyLocation::Query,
        };
        let stored = auth.to_stored().expect("json");
        assert_eq!(Auth::from_stored(&stored, Auth::None), auth);
        assert_eq!(Auth::from_stored("not json", Auth::None), Auth::None);
        assert_eq!(
            Auth::from_stored(
                r#"{"type":"bearer","token":"t","prefix":"B","extra":1}"#,
                Auth::None
            ),
            Auth::None
        );
    }
}
