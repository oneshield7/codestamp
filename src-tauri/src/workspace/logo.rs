//! Finds a logo for a workspace through Google's favicon service: from the websites its name
//! suggests, or from a website the user typed. Only that service is contacted, and only when the
//! user asks for a logo; what comes back must be a real image of a bounded size.

use std::collections::HashSet;
use std::time::Duration;

use futures_util::future::join_all;
use reqwest::redirect::Policy;
use sha2::{Digest, Sha256};
use url::Url;

use super::icon::{MAX_IMAGE_BYTES, data_url, image_type};
use super::models::LogoCandidate;
use crate::error::AppError;

const SERVICE: &str = "https://www.google.com/s2/favicons";
/// Test builds point the lookup at a local server; anything but loopback http is ignored.
const TEST_SERVICE_VAR: &str = "CODESTAMP_ICON_SERVICE";
const TIMEOUT: Duration = Duration::from_secs(8);
const MAX_REDIRECTS: usize = 3;
const ICON_SIZE: &str = "128";
/// Endings tried for a name, most common first.
const ENDINGS: [&str; 6] = ["com", "co.id", "id", "net", "org", "io"];
const MAX_GUESSES: usize = 8;
/// Words that are part of a company's legal name but never of its website.
const LEGAL_WORDS: [&str; 13] = [
    "pt", "tbk", "persero", "cv", "inc", "ltd", "llc", "corp", "co", "company", "group", "the",
    "and",
];

/// Websites a name may belong to: "PT Telkom Indonesia Tbk" gives telkomindonesia.com, …, then
/// telkom.com and telkom.co.id.
pub fn guesses(name: &str) -> Vec<String> {
    let lower = name.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty() && !LEGAL_WORDS.contains(word))
        .collect();
    let mut bases = Vec::new();
    if let Some(first) = words.first() {
        bases.push(words.concat());
        bases.push((*first).to_owned());
    }
    let mut seen = HashSet::new();
    bases
        .into_iter()
        .filter(|base| (1..=63).contains(&base.len()) && seen.insert(base.clone()))
        .flat_map(|base| ENDINGS.iter().map(move |ending| format!("{base}.{ending}")))
        .take(MAX_GUESSES)
        .collect()
}

/// The logos found for `websites`, in their order, without repeats (parked domains often share
/// one icon). An error only when the service could not be reached at all.
pub async fn find(websites: &[String]) -> Result<Vec<LogoCandidate>, AppError> {
    let service = service()?;
    let client = client(&service)?;
    let answers = join_all(
        websites
            .iter()
            .map(|website| logo_of(&client, &service, website)),
    )
    .await;
    if !answers.is_empty() && answers.iter().all(Result::is_err) {
        return Err(AppError::Connect(
            "the logo service could not be reached. Check the connection".to_owned(),
        ));
    }
    let mut seen = HashSet::new();
    Ok(websites
        .iter()
        .zip(answers)
        .filter_map(|(website, answer)| {
            let (bytes, mime) = answer.ok().flatten()?;
            seen.insert(Sha256::digest(&bytes)).then(|| LogoCandidate {
                website: website.clone(),
                data_url: data_url(mime, &bytes),
            })
        })
        .collect())
}

/// Tries the site as written, then its `www.` host, which is often the only one Google knows.
async fn logo_of(
    client: &reqwest::Client,
    service: &Url,
    website: &str,
) -> Result<Option<(Vec<u8>, &'static str)>, ()> {
    let found = fetch(client, service, website).await?;
    if found.is_some() || website.starts_with("www.") {
        return Ok(found);
    }
    fetch(client, service, &format!("www.{website}")).await
}

/// `Ok(None)` when the service has no icon for the site (it answers 404 with a stock globe).
async fn fetch(
    client: &reqwest::Client,
    service: &Url,
    host: &str,
) -> Result<Option<(Vec<u8>, &'static str)>, ()> {
    let mut url = service.clone();
    url.query_pairs_mut()
        .append_pair("domain", host)
        .append_pair("sz", ICON_SIZE);
    let mut response = client.get(url).send().await.map_err(|_| ())?;
    if !response.status().is_success() {
        return Ok(None);
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
    {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
        if bytes.len() + chunk.len() > MAX_IMAGE_BYTES {
            return Ok(None);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(image_type(&bytes).map(|mime| (bytes, mime)))
}

fn service() -> Result<Url, AppError> {
    let local = std::env::var(TEST_SERVICE_VAR)
        .ok()
        .and_then(|value| Url::parse(&value).ok())
        .filter(|url| url.scheme() == "http" && url.host_str() == Some("127.0.0.1"));
    match local {
        Some(url) => Ok(url),
        None => Url::parse(SERVICE).map_err(|_| AppError::ClientInit),
    }
}

/// Google answers from www.google.com through a redirect to one of its tN.gstatic.com hosts; a
/// redirect anywhere else is not followed.
fn client(service: &Url) -> Result<reqwest::Client, AppError> {
    let local = service.scheme() == "http";
    let local_port = service.port();
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(Policy::custom(move |attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                return attempt.stop();
            }
            let to = attempt.url();
            let allowed = if local {
                to.host_str() == Some("127.0.0.1") && to.port() == local_port
            } else {
                to.scheme() == "https" && is_google_icon_host(to.host_str().unwrap_or_default())
            };
            if allowed {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| AppError::ClientInit)
}

fn is_google_icon_host(host: &str) -> bool {
    host == "www.google.com"
        || host
            .strip_prefix('t')
            .and_then(|rest| rest.strip_suffix(".gstatic.com"))
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_suggests_its_likely_websites() {
        assert_eq!(
            guesses("Telkom"),
            [
                "telkom.com",
                "telkom.co.id",
                "telkom.id",
                "telkom.net",
                "telkom.org",
                "telkom.io"
            ]
        );
        assert_eq!(
            guesses("PT Telkom Indonesia (Persero) Tbk")[..3],
            [
                "telkomindonesia.com",
                "telkomindonesia.co.id",
                "telkomindonesia.id"
            ]
        );
        assert_eq!(guesses("PT Telkom Indonesia").len(), MAX_GUESSES);
        assert!(guesses("PT Tbk").is_empty());
        assert!(guesses("Ünïcödé ★").iter().all(|site| site.is_ascii()));
    }

    #[test]
    fn only_googles_icon_hosts_are_followed() {
        for host in ["www.google.com", "t0.gstatic.com", "t12.gstatic.com"] {
            assert!(is_google_icon_host(host), "{host}");
        }
        for host in [
            "t.gstatic.com",
            "tx.gstatic.com",
            "evil.com",
            "t1.gstatic.com.evil.com",
        ] {
            assert!(!is_google_icon_host(host), "{host}");
        }
    }
}
