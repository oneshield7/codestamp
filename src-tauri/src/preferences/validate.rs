use std::ops::RangeInclusive;

use super::models::{Preferences, ProxySettings};
use crate::error::AppError;
use crate::request::network::HostPattern;

const TIMEOUT_SECONDS: RangeInclusive<u32> = 1..=300;
const MAX_RESPONSE_MB: u32 = 1_024;
const HISTORY_ENTRIES: RangeInclusive<u32> = 100..=10_000;
const HISTORY_DAYS: RangeInclusive<u32> = 1..=365;
const MAX_USER_AGENT_CHARS: usize = 200;
const MAX_HOST_CHARS: usize = 253;
const MAX_USERNAME_CHARS: usize = 200;
const MAX_PASSWORD_CHARS: usize = 500;
const MAX_BYPASS_CHARS: usize = 2_000;

pub fn check_preferences(input: Preferences) -> Result<Preferences, AppError> {
    let requests = &input.requests;
    if !TIMEOUT_SECONDS.contains(&requests.timeout_seconds) {
        return Err(AppError::invalid("The timeout must be 1 to 300 seconds"));
    }
    if requests.max_response_mb > MAX_RESPONSE_MB {
        return Err(AppError::invalid(
            "The largest response can be up to 1024 MB (0 means no limit)",
        ));
    }
    let user_agent = requests.user_agent.trim();
    if user_agent.chars().count() > MAX_USER_AGENT_CHARS
        || !user_agent.chars().all(|c| c == ' ' || c.is_ascii_graphic())
    {
        return Err(AppError::invalid(
            "The User-Agent can be up to 200 plain characters",
        ));
    }
    if !HISTORY_ENTRIES.contains(&input.history.max_entries) {
        return Err(AppError::invalid("History can keep 100 to 10000 requests"));
    }
    if !HISTORY_DAYS.contains(&input.history.keep_days) {
        return Err(AppError::invalid("History can keep 1 to 365 days"));
    }
    let user_agent = user_agent.to_owned();
    Ok(Preferences {
        requests: super::models::RequestPreferences {
            user_agent,
            ..input.requests
        },
        ..input
    })
}

/// Checked even while the proxy is off, so switching it on later never meets a broken address.
pub fn check_proxy(input: ProxySettings) -> Result<ProxySettings, AppError> {
    let host = input.host.trim().to_ascii_lowercase();
    if !host.is_empty() && !is_proxy_host(&host) {
        return Err(AppError::invalid(
            "Enter the proxy host as a name or IP address, without http:// or a path",
        ));
    }
    if input.mode == super::models::ProxyMode::Custom && (host.is_empty() || input.port == 0) {
        return Err(AppError::invalid("Enter the proxy host and port"));
    }
    let username = input.username.trim();
    if username.chars().count() > MAX_USERNAME_CHARS
        || username.contains(':')
        || username.chars().any(char::is_control)
    {
        return Err(AppError::invalid(
            "The proxy username can be up to 200 characters, without a colon",
        ));
    }
    let bypass = input.bypass.trim();
    if bypass.len() > MAX_BYPASS_CHARS
        || !bypass
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " ,.-*:/[]".contains(c))
    {
        return Err(AppError::invalid(
            "List the hosts that skip the proxy separated by commas",
        ));
    }
    Ok(ProxySettings {
        host,
        username: username.to_owned(),
        bypass: bypass.to_owned(),
        ..input
    })
}

pub fn check_proxy_password(password: &str) -> Result<(), AppError> {
    if password.chars().count() > MAX_PASSWORD_CHARS || password.chars().any(char::is_control) {
        return Err(AppError::invalid(
            "The proxy password can be up to 500 characters",
        ));
    }
    Ok(())
}

/// A host name, an IPv4 address, or an IPv6 address in brackets.
fn is_proxy_host(host: &str) -> bool {
    if host.len() > MAX_HOST_CHARS {
        return false;
    }
    if let Some(inner) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        return !inner.is_empty() && inner.chars().all(|c| c.is_ascii_hexdigit() || c == ':');
    }
    is_host_name(host)
}

fn is_host_name(host: &str) -> bool {
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// `api.example.com`, `*.example.com`, optionally followed by `:port`.
pub fn parse_host_pattern(raw: &str) -> Result<HostPattern, AppError> {
    let invalid =
        || AppError::invalid("Enter a host such as api.example.com, *.example.com or host:8443");
    let text = raw.trim().to_ascii_lowercase();
    let (host, port) = match text.rsplit_once(':') {
        Some((host, port)) => {
            let port: u16 = port.parse().map_err(|_| invalid())?;
            if port == 0 {
                return Err(invalid());
            }
            (host.to_owned(), Some(port))
        }
        None => (text, None),
    };
    let name = host.strip_prefix("*.").unwrap_or(&host);
    if host.len() > MAX_HOST_CHARS || !is_host_name(name) {
        return Err(invalid());
    }
    Ok(HostPattern { host, port })
}

pub fn host_pattern_text(pattern: &HostPattern) -> String {
    match pattern.port {
        Some(port) => format!("{}:{port}", pattern.host),
        None => pattern.host.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::models::{ProxyMode, RequestPreferences};

    #[test]
    fn accepts_defaults_and_trims_the_user_agent() {
        assert!(check_preferences(Preferences::default()).is_ok());
        let input = Preferences {
            requests: RequestPreferences {
                user_agent: "  MyTool/1.0  ".to_owned(),
                ..RequestPreferences::default()
            },
            ..Preferences::default()
        };
        assert_eq!(
            check_preferences(input).ok().map(|p| p.requests.user_agent),
            Some("MyTool/1.0".to_owned())
        );
    }

    #[test]
    fn refuses_values_out_of_range() {
        let mut input = Preferences::default();
        input.requests.timeout_seconds = 0;
        assert!(check_preferences(input.clone()).is_err());
        input.requests.timeout_seconds = 30;
        input.requests.user_agent = "bad\r\nX-Injected: 1".to_owned();
        assert!(check_preferences(input.clone()).is_err());
        input.requests.user_agent = String::new();
        input.history.max_entries = 50;
        assert!(check_preferences(input).is_err());
    }

    #[test]
    fn checks_proxy_hosts_and_needs_a_port_when_on() {
        let proxy = |host: &str, port: u16| ProxySettings {
            mode: ProxyMode::Custom,
            host: host.to_owned(),
            port,
            ..ProxySettings::default()
        };
        assert!(
            check_proxy(proxy("Proxy.Corp.Example", 8080))
                .is_ok_and(|p| p.host == "proxy.corp.example")
        );
        assert!(check_proxy(proxy("10.0.0.1", 3128)).is_ok());
        assert!(check_proxy(proxy("[::1]", 3128)).is_ok());
        assert!(check_proxy(proxy("http://proxy", 8080)).is_err());
        assert!(check_proxy(proxy("proxy/path", 8080)).is_err());
        assert!(check_proxy(proxy("proxy", 0)).is_err());
        let off = ProxySettings {
            mode: ProxyMode::Off,
            ..ProxySettings::default()
        };
        assert!(check_proxy(off).is_ok());
    }

    #[test]
    fn parses_host_patterns_for_client_certificates() {
        let pattern = parse_host_pattern(" *.Example.com:8443 ").expect("pattern");
        assert_eq!(pattern.host, "*.example.com");
        assert_eq!(pattern.port, Some(8443));
        assert_eq!(host_pattern_text(&pattern), "*.example.com:8443");
        for bad in [
            "",
            "https://a.test",
            "a.test:0",
            "a b.test",
            "*.",
            "a.test/x",
        ] {
            assert!(parse_host_pattern(bad).is_err(), "{bad}");
        }
    }
}
