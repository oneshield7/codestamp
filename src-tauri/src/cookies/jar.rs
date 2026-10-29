use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use cookie_store::{CookieDomain, CookieExpiration, CookieStore, RawCookie};
use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
use url::Url;

use super::models::{CookieKey, ResponseCookie, StoredCookie};
use crate::error::AppError;

/// A runaway server cannot fill the jar without limit; past this, new cookies are not kept.
const MAX_COOKIES: usize = 1_000;

/// The cookie jar shared by every request, like a browser's: cookies set by a response are sent
/// back to matching hosts (RFC 6265 domain, path, Secure and expiry rules). A `Cookie` header
/// typed by the user replaces the jar's for that request.
#[derive(Default)]
pub struct CookieJar {
    store: RwLock<CookieStore>,
    changed: AtomicBool,
}

impl reqwest::cookie::CookieStore for CookieJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let cookies: Vec<RawCookie<'static>> =
            cookie_headers.filter_map(parse_set_cookie).collect();
        if cookies.is_empty() {
            return;
        }
        // A poisoned lock means a panic happened mid-update; these cookies are then not kept.
        let Ok(mut store) = self.store.write() else {
            return;
        };
        if store.iter_any().count() >= MAX_COOKIES {
            return;
        }
        store.store_response_cookies(cookies.into_iter(), url);
        self.changed.store(true, Ordering::Release);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let store = self.store.read().ok()?;
        let header = store
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        if header.is_empty() {
            return None;
        }
        HeaderValue::from_str(&header).ok()
    }
}

impl CookieJar {
    /// Unexpired cookies, without their values, sorted by domain, path and name.
    pub fn list(&self) -> Result<Vec<StoredCookie>, AppError> {
        let store = self.read()?;
        let mut cookies: Vec<StoredCookie> = store
            .iter_unexpired()
            .filter_map(|cookie| {
                Some(StoredCookie {
                    domain: cookie.domain.as_cow()?.into_owned(),
                    path: cookie.path.as_ref().to_owned(),
                    name: cookie.name().to_owned(),
                    expires: match &cookie.expires {
                        CookieExpiration::AtUtc(at) => Some(at.unix_timestamp()),
                        CookieExpiration::SessionEnd => None,
                    },
                    host_only: matches!(cookie.domain, CookieDomain::HostOnly(_)),
                    secure: cookie.secure().unwrap_or(false),
                    http_only: cookie.http_only().unwrap_or(false),
                })
            })
            .collect();
        cookies.sort_by(|a, b| (&a.domain, &a.path, &a.name).cmp(&(&b.domain, &b.path, &b.name)));
        Ok(cookies)
    }

    pub fn value(&self, key: &CookieKey) -> Result<String, AppError> {
        self.read()?
            .get(&key.domain, &key.path, &key.name)
            .map(|cookie| cookie.value().to_owned())
            .ok_or(AppError::NotFound("Cookie"))
    }

    pub fn remove(&self, key: &CookieKey) -> Result<(), AppError> {
        self.write()?
            .remove(&key.domain, &key.path, &key.name)
            .map(|_| ())
            .ok_or(AppError::NotFound("Cookie"))
    }

    /// Removes every cookie, or only those of one domain.
    pub fn clear(&self, domain: Option<&str>) -> Result<(), AppError> {
        let mut store = self.write()?;
        let Some(domain) = domain else {
            store.clear();
            return Ok(());
        };
        let keys: Vec<(String, String)> = store
            .iter_any()
            .filter(|cookie| cookie.domain.as_cow().is_some_and(|own| own == domain))
            .map(|cookie| (cookie.path.as_ref().to_owned(), cookie.name().to_owned()))
            .collect();
        for (path, name) in keys {
            store.remove(domain, &path, &name);
        }
        Ok(())
    }

    /// Every cookie as JSON, session cookies included, so a login survives a restart.
    pub fn to_json(&self) -> Result<String, AppError> {
        let store = self.read()?;
        let mut buffer = Vec::new();
        cookie_store::serde::json::save_incl_expired_and_nonpersistent(&store, &mut buffer)
            .map_err(|_| AppError::Storage)?;
        String::from_utf8(buffer).map_err(|_| AppError::Storage)
    }

    /// Replaces the jar with saved cookies; expired ones are dropped on the way in.
    pub fn restore_json(&self, json: &str) -> Result<(), AppError> {
        let loaded =
            cookie_store::serde::json::load(json.as_bytes()).map_err(|_| AppError::Storage)?;
        *self.write()? = loaded;
        Ok(())
    }

    /// Whether responses changed the jar since the last call.
    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::AcqRel)
    }

    pub fn mark_changed(&self) {
        self.changed.store(true, Ordering::Release);
    }

    fn read(&self) -> Result<RwLockReadGuard<'_, CookieStore>, AppError> {
        self.store.read().map_err(|_| AppError::Storage)
    }

    fn write(&self) -> Result<RwLockWriteGuard<'_, CookieStore>, AppError> {
        self.store.write().map_err(|_| AppError::Storage)
    }
}

/// The cookies a response sets, for its Cookies tab.
pub fn response_cookies(headers: &HeaderMap) -> Vec<ResponseCookie> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(parse_set_cookie)
        .map(|cookie| ResponseCookie {
            name: cookie.name().to_owned(),
            value: cookie.value().to_owned(),
            domain: cookie.domain().map(str::to_owned),
            path: cookie.path().map(str::to_owned),
            expires: cookie.expires_datetime().map(|at| at.unix_timestamp()),
            max_age: cookie.max_age().map(|age| age.whole_seconds()),
            http_only: cookie.http_only().unwrap_or(false),
            secure: cookie.secure().unwrap_or(false),
            same_site: cookie.same_site().map(|same_site| same_site.to_string()),
        })
        .collect()
}

/// Malformed `Set-Cookie` headers are ignored, as browsers do.
fn parse_set_cookie(value: &HeaderValue) -> Option<RawCookie<'static>> {
    let text = value.to_str().ok()?;
    RawCookie::parse(text.to_owned()).ok()
}

#[cfg(test)]
mod tests {
    use reqwest::cookie::CookieStore as _;

    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).expect("url")
    }

    fn set(jar: &CookieJar, url: &Url, headers: &[&'static str]) {
        let values: Vec<HeaderValue> = headers
            .iter()
            .map(|h| HeaderValue::from_static(h))
            .collect();
        jar.set_cookies(&mut values.iter(), url);
    }

    #[test]
    fn sends_cookies_back_only_where_they_belong() {
        let jar = CookieJar::default();
        let site = url("https://api.example.com/v1/login");
        set(
            &jar,
            &site,
            &[
                "sid=abc; Path=/; HttpOnly",
                "theme=dark; Path=/v1",
                "secret=x; Secure",
            ],
        );

        let header = jar.cookies(&url("https://api.example.com/v1/users"));
        let sent = header
            .as_ref()
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();
        assert!(
            sent.contains("sid=abc") && sent.contains("theme=dark") && sent.contains("secret=x")
        );
        let other_path = jar.cookies(&url("https://api.example.com/v2"));
        assert!(other_path.is_some_and(|h| !h.to_str().unwrap_or_default().contains("theme")));
        assert!(jar.cookies(&url("https://evil.example.net/")).is_none());
        let plain_http = jar.cookies(&url("http://api.example.com/v1/users"));
        assert!(plain_http.is_some_and(|h| !h.to_str().unwrap_or_default().contains("secret")));
        assert!(jar.take_changed());
        assert!(!jar.take_changed());
    }

    #[test]
    fn lists_reveals_removes_and_survives_a_round_trip() {
        let jar = CookieJar::default();
        set(
            &jar,
            &url("https://a.test/"),
            &["one=1; Path=/", "two=2; Path=/; Max-Age=3600"],
        );
        set(&jar, &url("https://b.test/"), &["three=3"]);

        let listed = jar.list().expect("list");
        assert_eq!(listed.len(), 3);
        assert!(
            listed
                .iter()
                .any(|c| c.name == "two" && c.expires.is_some())
        );
        let key = CookieKey {
            domain: "a.test".to_owned(),
            path: "/".to_owned(),
            name: "one".to_owned(),
        };
        assert_eq!(jar.value(&key).expect("value"), "1");

        let restored = CookieJar::default();
        restored
            .restore_json(&jar.to_json().expect("json"))
            .expect("restore");
        assert_eq!(restored.list().expect("list").len(), 3);

        jar.remove(&key).expect("remove");
        assert!(jar.value(&key).is_err());
        jar.clear(Some("b.test")).expect("clear domain");
        assert_eq!(jar.list().expect("list").len(), 1);
        jar.clear(None).expect("clear all");
        assert!(jar.list().expect("list").is_empty());
    }

    #[test]
    fn reads_the_cookies_a_response_sets() {
        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("sid=abc; Path=/; Max-Age=60; HttpOnly; Secure; SameSite=Lax"),
        );
        headers.append(SET_COOKIE, HeaderValue::from_static("not a cookie"));
        let cookies = response_cookies(&headers);
        assert_eq!(cookies.len(), 1);
        let sid = cookies.first().expect("cookie");
        assert_eq!((sid.name.as_str(), sid.value.as_str()), ("sid", "abc"));
        assert!(sid.http_only && sid.secure);
        assert_eq!(sid.max_age, Some(60));
        assert_eq!(sid.same_site.as_deref(), Some("Lax"));
    }
}
