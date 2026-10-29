//! Recognising values that are credentials, so they are flagged or hidden, never repeated.

// The same names the screen masking in `src/lib/privacy/secrets.ts` hides; keep both lists equal.
const SENSITIVE_NAMES: [&str; 8] = [
    "authorization",
    "proxyauthorization",
    "cookie",
    "setcookie",
    "auth",
    "key",
    "pwd",
    "sig",
];
const SENSITIVE_PARTS: [&str; 10] = [
    "token",
    "secret",
    "password",
    "passwd",
    "apikey",
    "session",
    "signature",
    "credential",
    "privatekey",
    "accesskey",
];

/// Header, parameter or field names that usually carry credentials.
pub fn is_sensitive_name(name: &str) -> bool {
    let normalized: String = name
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    SENSITIVE_NAMES.contains(&normalized.as_str())
        || SENSITIVE_PARTS.iter().any(|part| normalized.contains(part))
}

/// A value typed in as it is, rather than taken from a `{{variable}}`.
pub fn is_written_out(value: &str) -> bool {
    !value.trim().is_empty() && !value.contains("{{")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_values() {
        assert!(is_sensitive_name("Authorization"));
        assert!(is_sensitive_name("X-Api-Key"));
        assert!(is_sensitive_name("access_token"));
        assert!(!is_sensitive_name("X-Trace"));
        assert!(is_written_out("Bearer abc"));
        assert!(!is_written_out("Bearer {{token}}"));
        assert!(!is_written_out("  "));
    }
}
