//! AWS Signature Version 4: the request is signed with the secret key just before it goes out,
//! over its final method, URL, signed headers and a hash of its body.

use ::http::{HeaderMap, HeaderName, HeaderValue};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};
use url::Url;

use crate::encoding::hex_lower;
use crate::environment::dynamic::iso_timestamp;
use crate::error::AppError;

const ALGORITHM: &str = "AWS4-HMAC-SHA256";
/// S3 signs the body hash in its own header and does not encode paths twice.
const S3: &str = "s3";

pub struct AwsCredentials<'a> {
    pub access_key: &'a str,
    pub secret_key: &'a str,
    pub session_token: &'a str,
    pub region: &'a str,
    pub service: &'a str,
}

/// The hex SHA-256 of a body, as the signature and S3's `x-amz-content-sha256` expect.
pub fn payload_hash(body: &[u8]) -> String {
    hex_lower(&Sha256::digest(body))
}

/// Adds `x-amz-date` (and the session token or S3 body hash when needed) and the
/// `Authorization` header. A typed `Authorization` header wins, so nothing is signed then.
pub fn sign(
    method: &str,
    url: &Url,
    headers: &mut HeaderMap,
    payload_hash: &str,
    credentials: &AwsCredentials<'_>,
    now_ms: i64,
) -> Result<(), AppError> {
    if headers.contains_key(::http::header::AUTHORIZATION) {
        return Ok(());
    }
    for (name, value) in [
        ("access key", credentials.access_key),
        ("secret key", credentials.secret_key),
        ("region", credentials.region),
        ("service", credentials.service),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::InvalidInput(format!(
                "AWS Signature needs the {name}"
            )));
        }
    }
    let amz_date = amz_date(now_ms);
    let date = &amz_date[..8];
    let region = credentials.region.trim();
    let service = credentials.service.trim();
    insert(headers, "x-amz-date", &amz_date)?;
    if !credentials.session_token.is_empty() {
        insert(headers, "x-amz-security-token", credentials.session_token)?;
    }
    if service == S3 {
        insert(headers, "x-amz-content-sha256", payload_hash)?;
    }

    let (canonical_headers, signed_headers) = canonical_headers(url, headers);
    let canonical_request = [
        method,
        &canonical_path(url, service == S3),
        &canonical_query(url),
        &canonical_headers,
        &signed_headers,
        payload_hash,
    ]
    .join("\n");
    let scope = format!("{date}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "{ALGORITHM}\n{amz_date}\n{scope}\n{}",
        payload_hash_of(&canonical_request)
    );
    let mut key = hmac(
        format!("AWS4{}", credentials.secret_key).as_bytes(),
        date.as_bytes(),
    )?;
    for part in [region, service, "aws4_request"] {
        key = hmac(&key, part.as_bytes())?;
    }
    let signature = hex_lower(&hmac(&key, string_to_sign.as_bytes())?);
    insert(
        headers,
        "authorization",
        &format!(
            "{ALGORITHM} Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            credentials.access_key.trim()
        ),
    )
}

fn payload_hash_of(text: &str) -> String {
    payload_hash(text.as_bytes())
}

/// `20150830T123600Z`, from the same UTC conversion as `{{$isoTimestamp}}`.
fn amz_date(now_ms: i64) -> String {
    let iso = iso_timestamp(now_ms);
    format!(
        "{}{}{}T{}{}{}Z",
        &iso[0..4],
        &iso[5..7],
        &iso[8..10],
        &iso[11..13],
        &iso[14..16],
        &iso[17..19]
    )
}

fn hmac(key: &[u8], data: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key)
        .map_err(|_| AppError::invalid("The AWS secret key cannot be used"))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn insert(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<(), AppError> {
    let value = HeaderValue::from_str(value)
        .map_err(|_| AppError::invalid("An AWS Signature value cannot be used in a header"))?;
    headers.insert(HeaderName::from_static(name), value);
    Ok(())
}

/// Host, Content-Type and every `x-amz-*` header, lowercased and sorted.
fn canonical_headers(url: &Url, headers: &HeaderMap) -> (String, String) {
    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
        None => url.host_str().unwrap_or_default().to_owned(),
    };
    let mut signed: Vec<(String, String)> = vec![("host".to_owned(), host)];
    for (name, value) in headers {
        let name = name.as_str();
        if name == "content-type" || name.starts_with("x-amz-") {
            let value = String::from_utf8_lossy(value.as_bytes());
            let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
            signed.push((name.to_owned(), collapsed));
        }
    }
    signed.sort();
    let canonical = signed
        .iter()
        .map(|(name, value)| format!("{name}:{value}\n"))
        .collect();
    let names = signed
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(";");
    (canonical, names)
}

/// The URL keeps its path encoded once; services other than S3 expect it encoded again.
fn canonical_path(url: &Url, s3: bool) -> String {
    let path = url.path();
    let path = if path.is_empty() { "/" } else { path };
    if s3 {
        return path.to_owned();
    }
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte == b'/' {
            encoded.push('/');
        } else {
            encode_byte(byte, &mut encoded);
        }
    }
    encoded
}

fn canonical_query(url: &Url) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| (encode(&key), encode(&value)))
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        encode_byte(byte, &mut encoded);
    }
    encoded
}

/// RFC 3986 unreserved characters stay; everything else becomes `%XX` (uppercase).
fn encode_byte(byte: u8, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
        out.push(char::from(byte));
    } else {
        out.push('%');
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0F)]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(service: &'static str) -> AwsCredentials<'static> {
        AwsCredentials {
            access_key: "AKIDEXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            session_token: "",
            region: "us-east-1",
            service,
        }
    }

    // The worked example from the AWS Signature Version 4 documentation (IAM ListUsers).
    #[test]
    fn matches_the_aws_example() {
        let url = Url::parse("https://iam.amazonaws.com/?Action=ListUsers&Version=2010-05-08")
            .expect("url");
        let mut headers = HeaderMap::new();
        headers.insert(
            ::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=utf-8"),
        );
        let when = 1_440_938_160_000; // 2015-08-30T12:36:00Z
        sign(
            "GET",
            &url,
            &mut headers,
            &payload_hash(b""),
            &credentials("iam"),
            when,
        )
        .expect("signed");
        assert_eq!(headers["x-amz-date"], "20150830T123600Z");
        assert_eq!(
            headers["authorization"],
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/iam/aws4_request, SignedHeaders=content-type;host;x-amz-date, Signature=5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
        );
    }

    #[test]
    fn s3_signs_the_body_hash_and_typed_auth_wins() {
        let url = Url::parse("https://bucket.s3.amazonaws.com/a%20b.txt").expect("url");
        let mut headers = HeaderMap::new();
        sign(
            "PUT",
            &url,
            &mut headers,
            &payload_hash(b"hi"),
            &credentials("s3"),
            0,
        )
        .expect("signed");
        assert_eq!(headers["x-amz-content-sha256"], payload_hash(b"hi"));
        assert!(headers["authorization"].to_str().is_ok_and(|value| {
            value.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date")
        }));
        assert_eq!(canonical_path(&url, false), "/a%2520b.txt");

        let mut typed = HeaderMap::new();
        typed.insert(
            ::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer x"),
        );
        sign("GET", &url, &mut typed, "", &credentials("s3"), 0).expect("left alone");
        assert!(!typed.contains_key("x-amz-date"));
        let mut missing = HeaderMap::new();
        let error = sign(
            "GET",
            &url,
            &mut missing,
            "",
            &AwsCredentials {
                region: " ",
                ..credentials("s3")
            },
            0,
        );
        assert!(error.is_err_and(|e| e.to_string() == "AWS Signature needs the region"));
    }
}
