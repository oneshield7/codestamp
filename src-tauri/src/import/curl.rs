use std::iter::Peekable;
use std::str::Chars;

use url::form_urlencoded::byte_serialize;

use super::models::CurlImport;
use super::url_parts::{parse_query, split_url};
use crate::auth::models::Auth;
use crate::encoding::base64_encode;
use crate::error::AppError;
use crate::request::body_mode::raw_body_mode;
use crate::storage::models::{FormField, FormFieldKind, KeyValue, RequestContent, RequestKind};
use crate::storage::validate::check_request;

const MAX_COMMAND_BYTES: usize = 1024 * 1024;
const MAX_TOKENS: usize = 5_000;
const MAX_NAME_CHARS: usize = 120;

/// Options that change nothing about the request itself.
const SILENT_FLAGS: [&str; 25] = [
    "--compressed",
    "-s",
    "--silent",
    "-S",
    "--show-error",
    "-L",
    "--location",
    "-v",
    "--verbose",
    "-i",
    "--include",
    "--http1.0",
    "--http1.1",
    "--http2",
    "--http2-prior-knowledge",
    "-N",
    "--no-buffer",
    "-f",
    "--fail",
    "-g",
    "--globoff",
    "-#",
    "--progress-bar",
    "--no-keepalive",
    "--location-trusted",
];
const SILENT_SHORT_FLAGS: &str = "sSLviNfg#";

/// Options whose value is read but not used; they produce a warning.
const IGNORED_WITH_VALUE: [&str; 22] = [
    "-o",
    "--output",
    "-m",
    "--max-time",
    "--connect-timeout",
    "--retry",
    "-w",
    "--write-out",
    "-x",
    "--proxy",
    "--cacert",
    "-E",
    "--cert",
    "--key",
    "--resolve",
    "-c",
    "--cookie-jar",
    "-D",
    "--dump-header",
    "--max-redirs",
    "-T",
    "--upload-file",
];

/// Short options that take a value, which may be attached (`-XPOST`, `-HAccept: x`).
const SHORT_WITH_VALUE: &str = "XHdubAeomxFwTcDE";

#[derive(Default)]
struct Parsed {
    method: Option<String>,
    url: Option<String>,
    headers: Vec<KeyValue>,
    data: Vec<String>,
    form: Vec<FormField>,
    json: bool,
    get: bool,
    head: bool,
    /// `-u user:password`; Basic auth unless `--digest` or `--aws-sigv4` says otherwise.
    user: Option<String>,
    digest: bool,
    /// The provider string of `--aws-sigv4`, such as `aws:amz:eu-west-1:s3`.
    aws_sigv4: Option<String>,
    warnings: Vec<String>,
}

pub fn parse(command: &str) -> Result<CurlImport, AppError> {
    if command.len() > MAX_COMMAND_BYTES {
        return Err(AppError::invalid("The command is larger than 1 MB"));
    }
    let tokens = tokenize(command)?;
    let program = tokens.first().map(String::as_str).unwrap_or_default();
    let program = program.rsplit(['/', '\\']).next().unwrap_or_default();
    if !program.eq_ignore_ascii_case("curl") && !program.eq_ignore_ascii_case("curl.exe") {
        return Err(AppError::invalid("Paste a command that starts with curl"));
    }

    let args = tokens.get(1..).unwrap_or_default();
    let mut parsed = Parsed::default();
    let mut index = 0;
    while let Some(token) = args.get(index) {
        index += 1;
        let (option, inline) = split_option(token);
        if !option.starts_with('-') || option == "-" {
            if parsed.url.is_none() {
                parsed.url = Some(token.clone());
            } else {
                parsed
                    .warnings
                    .push(format!("Ignored extra argument “{token}”"));
            }
            continue;
        }
        apply_option(&mut parsed, &option, inline, args, &mut index)?;
    }
    build(parsed)
}

fn apply_option(
    parsed: &mut Parsed,
    option: &str,
    inline: Option<String>,
    args: &[String],
    index: &mut usize,
) -> Result<(), AppError> {
    let mut value = || take_value(option, inline.clone(), args, index);
    match option {
        "-X" | "--request" => parsed.method = Some(value()?.to_uppercase()),
        "--url" => parsed.url = Some(value()?),
        "-H" | "--header" => add_header_line(parsed, &value()?),
        "-d" | "--data" | "--data-ascii" | "--data-binary" => {
            let data = value()?;
            if data.starts_with('@') {
                parsed.warnings.push(format!(
                    "Skipped “{data}”: reading request data from a file is not supported"
                ));
            } else {
                parsed.data.push(data);
            }
        }
        "--data-raw" => parsed.data.push(value()?),
        "--data-urlencode" => {
            let data = value()?;
            match url_encode_data(&data) {
                Some(encoded) => parsed.data.push(encoded),
                None => parsed.warnings.push(format!(
                    "Skipped “{data}”: reading request data from a file is not supported"
                )),
            }
        }
        "--json" => {
            parsed.data.push(value()?);
            parsed.json = true;
        }
        "-u" | "--user" => parsed.user = Some(value()?),
        "--digest" => parsed.digest = true,
        "--basic" => parsed.digest = false,
        "--aws-sigv4" => parsed.aws_sigv4 = Some(value()?),
        "-b" | "--cookie" => {
            let cookie = value()?;
            if cookie.contains('=') {
                set_header(parsed, "Cookie", &cookie);
            } else {
                parsed.warnings.push(format!(
                    "Skipped cookie file “{cookie}”: only inline cookies are imported"
                ));
            }
        }
        "-A" | "--user-agent" => set_header(parsed, "User-Agent", &value()?),
        "-e" | "--referer" => set_header(parsed, "Referer", &value()?),
        "-G" | "--get" => parsed.get = true,
        "-I" | "--head" => parsed.head = true,
        "-k" | "--insecure" => parsed
            .warnings
            .push("Ignored --insecure: Codestamp always verifies TLS certificates".to_owned()),
        "-F" | "--form" => add_form_field(parsed, &value()?, true),
        "--form-string" => add_form_field(parsed, &value()?, false),
        _ if SILENT_FLAGS.contains(&option) || is_silent_flag_group(option) => {}
        _ if IGNORED_WITH_VALUE.contains(&option) => {
            let ignored = value()?;
            parsed
                .warnings
                .push(format!("Ignored option {option} {ignored}"));
        }
        _ => parsed
            .warnings
            .push(format!("Ignored unknown option {option}")),
    }
    Ok(())
}

fn build(parsed: Parsed) -> Result<CurlImport, AppError> {
    let Parsed {
        method,
        url,
        mut headers,
        mut data,
        form,
        json,
        get,
        head,
        user,
        digest,
        aws_sigv4,
        mut warnings,
    } = parsed;
    let auth = match user {
        Some(credentials) => user_auth(&credentials, digest, aws_sigv4.as_deref(), &mut headers),
        None => Auth::Inherit,
    };
    if !form.is_empty() && !data.is_empty() {
        // curl itself refuses to mix the two.
        warnings.push(
            "Both --form and --data were given; only the form fields were imported".to_owned(),
        );
        data.clear();
    }
    let raw_url = url.ok_or_else(|| AppError::invalid("The command has no URL"))?;
    // curl itself assumes http:// when no scheme is given.
    let raw_url = if raw_url.contains("://") || raw_url.starts_with("{{") {
        raw_url
    } else {
        format!("http://{raw_url}")
    };
    let (url, mut params) = split_url(&raw_url);
    let mut body = data.join("&");
    if get && !body.is_empty() {
        params.extend(parse_query(&body));
        body.clear();
    }
    let has_body = !body.is_empty() || !form.is_empty();
    let method = if head {
        "HEAD".to_owned()
    } else {
        method.unwrap_or_else(|| if has_body { "POST" } else { "GET" }.to_owned())
    };
    if json {
        add_missing(&mut headers, "Content-Type", "application/json");
        add_missing(&mut headers, "Accept", "application/json");
    }
    let body_mode = if form.is_empty() {
        body_mode(&body, &headers)
    } else {
        "formdata"
    };
    let content = check_request(RequestContent {
        kind: RequestKind::Http,
        name: request_name(&method, &url),
        method,
        url,
        params,
        headers,
        body_mode: body_mode.to_owned(),
        body,
        form,
        body_file: String::new(),
        pre_request_script: String::new(),
        test_script: String::new(),
        auth,
        mcp_operation: None,
        path_variables: Vec::new(),
        graphql_variables: String::new(),
        grpc: None,
        soap: None,
    })?;
    Ok(CurlImport {
        request: content,
        warnings,
    })
}

/// `-u` becomes Digest or AWS Signature auth with `--digest` or `--aws-sigv4`, and otherwise the
/// Basic header curl itself would send.
fn user_auth(
    credentials: &str,
    digest: bool,
    aws_sigv4: Option<&str>,
    headers: &mut Vec<KeyValue>,
) -> Auth {
    let (username, password) = credentials.split_once(':').unwrap_or((credentials, ""));
    if let Some(provider) = aws_sigv4 {
        // aws:amz:<region>:<service>
        let mut parts = provider.split(':').skip(2);
        return Auth::AwsSigV4 {
            access_key: username.to_owned(),
            secret_key: password.to_owned(),
            session_token: String::new(),
            region: parts.next().unwrap_or_default().to_owned(),
            service: parts.next().unwrap_or_default().to_owned(),
        };
    }
    if digest {
        return Auth::Digest {
            username: username.to_owned(),
            password: password.to_owned(),
        };
    }
    add_missing(
        headers,
        "Authorization",
        &format!(
            "Basic {}",
            base64_encode(format!("{username}:{password}").as_bytes())
        ),
    );
    Auth::Inherit
}

fn body_mode(body: &str, headers: &[KeyValue]) -> &'static str {
    if body.is_empty() {
        return "none";
    }
    let content_type = headers
        .iter()
        .find(|header| header.key.eq_ignore_ascii_case("content-type"))
        .map(|header| header.value.as_str());
    raw_body_mode(content_type, body)
}

fn request_name(method: &str, url: &str) -> String {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme
        .find('/')
        .map_or(after_scheme, |slash| &after_scheme[slash..]);
    format!("{method} {path}")
        .chars()
        .take(MAX_NAME_CHARS)
        .collect()
}

/// `-F name=value`, `-F name=@file` or `--form-string name=value`. File paths are not kept:
/// only files picked on this device may be uploaded, so the user chooses them again.
fn add_form_field(parsed: &mut Parsed, field: &str, special: bool) {
    let Some((name, content)) = field.split_once('=') else {
        parsed
            .warnings
            .push(format!("Skipped form field “{field}”: expected name=value"));
        return;
    };
    let name = name.trim();
    if special && content.starts_with('<') {
        parsed.warnings.push(format!(
            "Skipped form field “{name}”: reading a field's text from a file is not supported"
        ));
        return;
    }
    let is_file = special && content.starts_with('@');
    if is_file {
        parsed.warnings.push(format!(
            "Form field “{name}” uploads a file: choose it again in the Body tab"
        ));
    }
    parsed.form.push(FormField {
        enabled: true,
        key: name.to_owned(),
        value: if is_file {
            String::new()
        } else {
            content.to_owned()
        },
        kind: if is_file {
            FormFieldKind::File
        } else {
            FormFieldKind::Text
        },
    });
}

fn add_header_line(parsed: &mut Parsed, line: &str) {
    // curl uses `Name;` for a header with an empty value.
    let pair = line
        .split_once(':')
        .or_else(|| line.strip_suffix(';').map(|name| (name, "")));
    match pair {
        Some((name, value)) if !name.trim().is_empty() => parsed.headers.push(KeyValue {
            enabled: true,
            key: name.trim().to_owned(),
            value: value.trim_start().to_owned(),
        }),
        _ => parsed
            .warnings
            .push(format!("Skipped header “{line}”: expected Name: value")),
    }
}

fn set_header(parsed: &mut Parsed, name: &str, value: &str) {
    parsed
        .headers
        .retain(|header| !header.key.eq_ignore_ascii_case(name));
    parsed.headers.push(KeyValue {
        enabled: true,
        key: name.to_owned(),
        value: value.to_owned(),
    });
}

fn add_missing(headers: &mut Vec<KeyValue>, name: &str, value: &str) {
    if !headers
        .iter()
        .any(|header| header.key.eq_ignore_ascii_case(name))
    {
        headers.push(KeyValue {
            enabled: true,
            key: name.to_owned(),
            value: value.to_owned(),
        });
    }
}

/// `--data-urlencode` forms: `content`, `=content`, `name=content` (files are not supported).
fn url_encode_data(data: &str) -> Option<String> {
    let encode = |text: &str| byte_serialize(text.as_bytes()).collect::<String>();
    if let Some(content) = data.strip_prefix('=') {
        return Some(encode(content));
    }
    if let Some((name, content)) = data.split_once('=') {
        return Some(format!("{name}={}", encode(content)));
    }
    if data.contains('@') {
        return None;
    }
    Some(encode(data))
}

fn split_option(token: &str) -> (String, Option<String>) {
    if let Some(long) = token.strip_prefix("--") {
        return match long.split_once('=') {
            Some((name, value)) => (format!("--{name}"), Some(value.to_owned())),
            None => (token.to_owned(), None),
        };
    }
    let mut chars = token.chars();
    if chars.next() == Some('-')
        && let Some(short) = chars.next()
    {
        let rest: String = chars.collect();
        if SHORT_WITH_VALUE.contains(short) && !rest.is_empty() {
            return (format!("-{short}"), Some(rest));
        }
    }
    (token.to_owned(), None)
}

fn is_silent_flag_group(option: &str) -> bool {
    option.strip_prefix('-').is_some_and(|flags| {
        !flags.starts_with('-')
            && !flags.is_empty()
            && flags.chars().all(|c| SILENT_SHORT_FLAGS.contains(c))
    })
}

fn take_value(
    option: &str,
    inline: Option<String>,
    args: &[String],
    index: &mut usize,
) -> Result<String, AppError> {
    if let Some(value) = inline {
        return Ok(value);
    }
    let value = args
        .get(*index)
        .cloned()
        .ok_or_else(|| AppError::InvalidInput(format!("The option {option} needs a value")))?;
    *index += 1;
    Ok(value)
}

/// Splits a shell command into arguments. Supports bash quoting (`'…'`, `"…"`, `$'…'`,
/// backslash line continuations) and Windows cmd copies (`^"…^"`, `^` continuations).
fn tokenize(input: &str) -> Result<Vec<String>, AppError> {
    let windows = input.contains("^\"") || input.contains("^\n") || input.contains("^\r\n");
    let text = if windows {
        strip_caret_escapes(input)
    } else {
        input.to_owned()
    };
    let unterminated = || AppError::invalid("The command has an unterminated quote");

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if !windows => match chars.next() {
                Some('\n') => {}
                Some('\r') => {
                    chars.next_if_eq(&'\n');
                }
                Some(next) => {
                    current.push(next);
                    in_token = true;
                }
                None => {}
            },
            '\'' if !windows => {
                in_token = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => current.push(ch),
                        None => return Err(unterminated()),
                    }
                }
            }
            '$' if !windows && chars.peek() == Some(&'\'') => {
                chars.next();
                in_token = true;
                read_ansi_c(&mut chars, &mut current).ok_or_else(unterminated)?;
            }
            '"' => {
                in_token = true;
                read_double_quoted(&mut chars, &mut current).ok_or_else(unterminated)?;
            }
            c if c.is_whitespace() => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                    if tokens.len() > MAX_TOKENS {
                        return Err(AppError::invalid("The command has too many arguments"));
                    }
                }
            }
            c => {
                current.push(c);
                in_token = true;
            }
        }
    }
    if in_token {
        tokens.push(current);
    }
    Ok(tokens)
}

/// cmd.exe escapes: `^X` means a literal X and `^` at a line end continues the line.
fn strip_caret_escapes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '^' {
            output.push(c);
            continue;
        }
        match chars.next() {
            Some('\r') => {
                chars.next_if_eq(&'\n');
            }
            Some('\n') | None => {}
            Some(next) => output.push(next),
        }
    }
    output
}

fn read_double_quoted(chars: &mut Peekable<Chars<'_>>, current: &mut String) -> Option<()> {
    loop {
        match chars.next()? {
            '"' => return Some(()),
            '\\' => match chars.next()? {
                next @ ('"' | '\\' | '$' | '`') => current.push(next),
                '\n' => {}
                other => {
                    current.push('\\');
                    current.push(other);
                }
            },
            ch => current.push(ch),
        }
    }
}

/// Bash ANSI-C quoting (`$'…'`), used by "Copy as cURL" for bodies with special characters.
fn read_ansi_c(chars: &mut Peekable<Chars<'_>>, current: &mut String) -> Option<()> {
    loop {
        match chars.next()? {
            '\'' => return Some(()),
            '\\' => match chars.next()? {
                'n' => current.push('\n'),
                't' => current.push('\t'),
                'r' => current.push('\r'),
                '0' => current.push('\0'),
                'x' => push_code_point(chars, current, 2),
                'u' => push_code_point(chars, current, 4),
                'U' => push_code_point(chars, current, 8),
                other => current.push(other),
            },
            ch => current.push(ch),
        }
    }
}

fn push_code_point(chars: &mut Peekable<Chars<'_>>, current: &mut String, max_digits: usize) {
    let mut digits = String::new();
    while digits.len() < max_digits {
        match chars.next_if(char::is_ascii_hexdigit) {
            Some(digit) => digits.push(digit),
            None => break,
        }
    }
    match u32::from_str_radix(&digits, 16)
        .ok()
        .and_then(char::from_u32)
    {
        Some(decoded) => current.push(decoded),
        None => current.push('\u{fffd}'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header<'a>(request: &'a RequestContent, name: &str) -> Option<&'a str> {
        request
            .headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }

    #[test]
    fn imports_a_chrome_bash_copy() {
        let command = "curl 'https://api.example.com/v1/users?page=1' \\\n  -H 'accept: application/json' \\\n  -H 'authorization: Bearer abc' \\\n  --data-raw '{\"name\":\"Budi\"}' \\\n  --compressed";
        let imported = parse(command).expect("parse").request;
        assert_eq!(imported.method, "POST");
        assert_eq!(imported.url, "https://api.example.com/v1/users");
        assert_eq!(
            imported
                .params
                .first()
                .map(|p| (p.key.as_str(), p.value.as_str())),
            Some(("page", "1"))
        );
        assert_eq!(header(&imported, "authorization"), Some("Bearer abc"));
        assert_eq!(imported.body_mode, "json");
        assert_eq!(imported.body, "{\"name\":\"Budi\"}");
        assert_eq!(imported.name, "POST /v1/users");
    }

    #[test]
    fn imports_a_windows_cmd_copy() {
        let command = "curl ^\"https://api.example.com/x?a=1^\" ^\r\n  -H ^\"content-type: application/json^\" ^\r\n  --data-raw ^\"^{^\\^\"a^\\^\":1^}^\"";
        let imported = parse(command).expect("parse").request;
        assert_eq!(imported.url, "https://api.example.com/x");
        assert_eq!(header(&imported, "Content-Type"), Some("application/json"));
        assert_eq!(imported.body, "{\"a\":1}");
    }

    #[test]
    fn handles_ansi_c_quoting_and_attached_values() {
        let command = "curl -XPUT https://api.test/items/1 -H'X-Note: hi' --data-raw $'line1\\nline2 \\'quoted\\''";
        let imported = parse(command).expect("parse").request;
        assert_eq!(imported.method, "PUT");
        assert_eq!(header(&imported, "X-Note"), Some("hi"));
        assert_eq!(imported.body, "line1\nline2 'quoted'");
        assert_eq!(imported.body_mode, "text");
    }

    #[test]
    fn converts_user_get_json_and_cookie_options() {
        let basic = parse("curl -u user:p@ss https://api.test/me")
            .expect("parse")
            .request;
        assert_eq!(header(&basic, "Authorization"), Some("Basic dXNlcjpwQHNz"));
        let digest = parse("curl --digest -u ada:s3cret https://api.test/me")
            .expect("parse")
            .request;
        assert_eq!(
            digest.auth,
            Auth::Digest {
                username: "ada".to_owned(),
                password: "s3cret".to_owned()
            }
        );
        assert_eq!(header(&digest, "Authorization"), None);
        let aws = parse("curl --aws-sigv4 aws:amz:eu-west-1:s3 -u AKID:secret https://b.s3.test/k")
            .expect("parse")
            .request;
        assert!(matches!(
            aws.auth,
            Auth::AwsSigV4 { ref region, ref service, .. } if region == "eu-west-1" && service == "s3"
        ));

        let get = parse("curl -G https://api.test/search -d q=rina -d limit=5")
            .expect("parse")
            .request;
        assert_eq!(get.method, "GET");
        assert_eq!(get.body_mode, "none");
        assert_eq!(get.params.len(), 2);

        let json = parse("curl --json '{\"a\":1}' api.test/items -b 'session=xyz'")
            .expect("parse")
            .request;
        assert_eq!(json.method, "POST");
        assert_eq!(json.url, "http://api.test/items");
        assert_eq!(header(&json, "Accept"), Some("application/json"));
        assert_eq!(header(&json, "Cookie"), Some("session=xyz"));
    }

    #[test]
    fn warns_about_unsupported_options_instead_of_dropping_them_silently() {
        let imported =
            parse("curl -sSL -k -F file=@a.png --proxy http://p:8080 --weird https://api.test")
                .expect("parse");
        assert_eq!(imported.warnings.len(), 4);
        assert!(imported.warnings.iter().any(|w| w.contains("--insecure")));
        assert!(
            imported
                .warnings
                .iter()
                .any(|w| w.contains("choose it again"))
        );
    }

    #[test]
    fn imports_form_fields_without_file_paths() {
        let imported = parse(
            "curl https://api.test/upload -F 'title=Q3 report' -F 'doc=@/home/rina/q3.pdf' --form-string 'note=@not-a-file'",
        )
        .expect("parse");
        let request = imported.request;
        assert_eq!(request.method, "POST");
        assert_eq!(request.body_mode, "formdata");
        let fields: Vec<_> = request
            .form
            .iter()
            .map(|f| (f.key.as_str(), f.value.as_str(), f.kind))
            .collect();
        assert_eq!(
            fields,
            [
                ("title", "Q3 report", FormFieldKind::Text),
                ("doc", "", FormFieldKind::File),
                ("note", "@not-a-file", FormFieldKind::Text)
            ]
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("wget https://api.test").is_err());
        assert!(parse("curl -H 'Accept: x'").is_err());
        assert!(parse("curl 'https://api.test").is_err());
        assert!(parse("curl -X TRACE https://api.test").is_err());
    }
}
