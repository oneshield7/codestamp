//! HTTP Digest auth (RFC 7616). The server answers the first attempt with 401 and a challenge;
//! the request is then sent again with the credentials hashed against that challenge.

use md5::Md5;
use sha2::{Digest, Sha256};

use crate::encoding::hex_lower;
use crate::error::AppError;
use crate::request::types::NameValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Algorithm {
    Md5,
    Sha256,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Challenge {
    realm: String,
    nonce: String,
    opaque: Option<String>,
    algorithm: Algorithm,
    /// `-sess` algorithms hash the client nonce into the key.
    session: bool,
    /// The server asked for `qop=auth`; older servers send no qop at all.
    qop: bool,
}

/// Credentials and the request line the response is computed for.
pub struct DigestInput<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub method: &'a str,
    /// The path and query, exactly as sent.
    pub uri: &'a str,
    pub cnonce: &'a str,
}

/// The Digest challenge among the response's `WWW-Authenticate` headers, if there is one.
pub fn find_challenge(headers: &[NameValue]) -> Result<Option<Challenge>, AppError> {
    let Some(value) = headers.iter().find_map(|header| {
        let value = header.value.trim();
        let is_digest = header.name.eq_ignore_ascii_case("www-authenticate")
            && value.len() > 7
            && value[..7].eq_ignore_ascii_case("digest ");
        is_digest.then(|| &value[7..])
    }) else {
        return Ok(None);
    };
    let params = parse_params(value);
    let param = |name: &str| {
        params
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
    };
    let (algorithm, session) = match param("algorithm")
        .unwrap_or_else(|| "MD5".to_owned())
        .to_ascii_uppercase()
        .as_str()
    {
        "MD5" => (Algorithm::Md5, false),
        "MD5-SESS" => (Algorithm::Md5, true),
        "SHA-256" => (Algorithm::Sha256, false),
        "SHA-256-SESS" => (Algorithm::Sha256, true),
        _ => {
            return Err(AppError::invalid(
                "The server asks for a Digest algorithm Codestamp does not support",
            ));
        }
    };
    let qop = match param("qop") {
        None => false,
        Some(offered)
            if offered
                .split(',')
                .any(|q| q.trim().eq_ignore_ascii_case("auth")) =>
        {
            true
        }
        Some(_) => {
            return Err(AppError::invalid(
                "The server only offers Digest auth-int, which Codestamp does not support",
            ));
        }
    };
    let nonce =
        param("nonce").ok_or_else(|| AppError::invalid("The Digest challenge has no nonce"))?;
    Ok(Some(Challenge {
        realm: param("realm").unwrap_or_default(),
        nonce,
        opaque: param("opaque"),
        algorithm,
        session,
        qop,
    }))
}

/// The `Authorization` header value answering the challenge (nonce count 1).
pub fn authorization(challenge: &Challenge, input: &DigestInput<'_>) -> String {
    const NONCE_COUNT: &str = "00000001";
    let hash = |text: &str| match challenge.algorithm {
        Algorithm::Md5 => hex_lower(&Md5::digest(text.as_bytes())),
        Algorithm::Sha256 => hex_lower(&Sha256::digest(text.as_bytes())),
    };
    let mut key = hash(&format!(
        "{}:{}:{}",
        input.username, challenge.realm, input.password
    ));
    if challenge.session {
        key = hash(&format!("{key}:{}:{}", challenge.nonce, input.cnonce));
    }
    let request = hash(&format!("{}:{}", input.method, input.uri));
    let response = if challenge.qop {
        hash(&format!(
            "{key}:{}:{NONCE_COUNT}:{}:auth:{request}",
            challenge.nonce, input.cnonce
        ))
    } else {
        hash(&format!("{key}:{}:{request}", challenge.nonce))
    };
    let algorithm = match (challenge.algorithm, challenge.session) {
        (Algorithm::Md5, false) => "MD5",
        (Algorithm::Md5, true) => "MD5-sess",
        (Algorithm::Sha256, false) => "SHA-256",
        (Algorithm::Sha256, true) => "SHA-256-sess",
    };
    let mut header = format!(
        "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", algorithm={algorithm}, response=\"{response}\"",
        quote(input.username),
        quote(&challenge.realm),
        quote(&challenge.nonce),
        quote(input.uri),
    );
    if challenge.qop {
        header.push_str(&format!(
            ", qop=auth, nc={NONCE_COUNT}, cnonce=\"{}\"",
            quote(input.cnonce)
        ));
    }
    if let Some(opaque) = &challenge.opaque {
        header.push_str(&format!(", opaque=\"{}\"", quote(opaque)));
    }
    header
}

/// `key=value` and `key="quoted, value"` pairs separated by commas.
fn parse_params(text: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut rest = text.trim();
    while let Some((key, after)) = rest.split_once('=') {
        let key = key.trim().trim_start_matches(',').trim().to_owned();
        let after = after.trim_start();
        let (value, remaining) = if let Some(quoted) = after.strip_prefix('"') {
            let mut value = String::new();
            let mut chars = quoted.char_indices();
            let mut end = quoted.len();
            while let Some((index, c)) = chars.next() {
                match c {
                    '\\' => {
                        if let Some((_, escaped)) = chars.next() {
                            value.push(escaped);
                        }
                    }
                    '"' => {
                        end = index + 1;
                        break;
                    }
                    _ => value.push(c),
                }
            }
            (value, &quoted[end..])
        } else {
            let end = after.find(',').unwrap_or(after.len());
            (after[..end].trim().to_owned(), &after[end..])
        };
        params.push((key, value));
        rest = remaining.trim_start().trim_start_matches(',');
    }
    params
}

fn quote(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn challenge_header(algorithm: &str) -> Vec<NameValue> {
        vec![
            NameValue {
                name: "WWW-Authenticate".to_owned(),
                value: "Basic realm=\"other\"".to_owned(),
            },
            NameValue {
                name: "www-authenticate".to_owned(),
                value: format!(
                    "Digest realm=\"http-auth@example.org\", qop=\"auth, auth-int\", algorithm={algorithm}, nonce=\"7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v\", opaque=\"FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS\""
                ),
            },
        ]
    }

    fn input() -> DigestInput<'static> {
        DigestInput {
            username: "Mufasa",
            password: "Circle of Life",
            method: "GET",
            uri: "/dir/index.html",
            cnonce: "f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ",
        }
    }

    // The worked examples of RFC 7616 §3.9.1.
    #[test]
    fn answers_the_rfc_examples() {
        let md5 = find_challenge(&challenge_header("MD5"))
            .ok()
            .flatten()
            .expect("challenge");
        let header = authorization(&md5, &input());
        assert!(
            header.contains("response=\"8ca523f5e9506fed4657c9700eebdbec\""),
            "{header}"
        );
        assert!(header.contains("qop=auth, nc=00000001"));
        assert!(header.contains("opaque=\"FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS\""));

        let sha = find_challenge(&challenge_header("SHA-256"))
            .ok()
            .flatten()
            .expect("challenge");
        let header = authorization(&sha, &input());
        assert!(
            header.contains(
                "response=\"753927fa0e85d155564e2e272a28d1802ca10daf4496794697cf8db5856cb6c1\""
            ),
            "{header}"
        );
    }

    #[test]
    fn handles_missing_and_unsupported_challenges() {
        assert!(matches!(find_challenge(&[]), Ok(None)));
        let only_int = vec![NameValue {
            name: "WWW-Authenticate".to_owned(),
            value: "Digest realm=\"r\", nonce=\"n\", qop=\"auth-int\"".to_owned(),
        }];
        assert!(find_challenge(&only_int).is_err());
        let legacy = vec![NameValue {
            name: "WWW-Authenticate".to_owned(),
            value: "Digest realm=\"r\", nonce=\"n\"".to_owned(),
        }];
        let challenge = find_challenge(&legacy).ok().flatten().expect("challenge");
        assert!(!authorization(&challenge, &input()).contains("qop"));
    }
}
