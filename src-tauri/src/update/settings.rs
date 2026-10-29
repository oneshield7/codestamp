use serde_json::Value;
use tauri_plugin_updater::Error;

use crate::error::AppError;
use crate::text::clip;

const MAX_NOTES_BYTES: usize = 4000;
const MAX_DATE_BYTES: usize = 40;

/// A build checks for updates only once both a public key and an update address are set.
pub fn is_configured(plugin_config: Option<&Value>) -> bool {
    let Some(config) = plugin_config else {
        return false;
    };
    let has_key = config
        .get("pubkey")
        .and_then(Value::as_str)
        .is_some_and(|key| !key.trim().is_empty());
    let has_endpoint = config
        .get("endpoints")
        .and_then(Value::as_array)
        .is_some_and(|endpoints| !endpoints.is_empty());
    has_key && has_endpoint
}

/// Release notes are shown as plain text, so only their length is limited.
pub fn release_notes(body: Option<&str>) -> Option<String> {
    let notes = body?.trim();
    if notes.is_empty() {
        return None;
    }
    let (kept, cut) = clip(notes, MAX_NOTES_BYTES);
    Some(if cut {
        format!("{}…", kept.trim_end())
    } else {
        kept.to_owned()
    })
}

pub fn release_date(raw: &Value) -> Option<String> {
    let date = raw.get("pub_date")?.as_str()?.trim();
    (!date.is_empty() && date.len() <= MAX_DATE_BYTES).then(|| date.to_owned())
}

// Messages stay general: the server address, paths and key details are not shown in the app.
pub fn describe(error: Error) -> AppError {
    match error {
        Error::Reqwest(error) if error.is_timeout() => AppError::Timeout,
        Error::Reqwest(_) | Error::Network(_) => {
            AppError::Connect("the update server did not answer".to_owned())
        }
        Error::ReleaseNotFound
        | Error::Serialization(_)
        | Error::Semver(_)
        | Error::UrlParse(_)
        | Error::TargetNotFound(_)
        | Error::TargetsNotFound(_) => {
            AppError::Request("the update server has no valid release for this device".to_owned())
        }
        Error::Minisign(_)
        | Error::Base64(_)
        | Error::SignatureUtf8(_)
        | Error::SignedVersionMismatch { .. }
        | Error::MissingSignedVersion => AppError::invalid(
            "The downloaded update failed its signature check, so it was not installed.",
        ),
        Error::InvalidUpdaterFormat => {
            AppError::invalid("The downloaded update is not a Codestamp installer.")
        }
        Error::InsecureTransportProtocol => {
            AppError::invalid("The update server address must start with https://.")
        }
        Error::Io(_) | Error::TempDirNotFound | Error::FailedToDetermineExtractPath => {
            AppError::FileAccess
        }
        _ => AppError::Request("the update could not be installed".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn needs_a_key_and_an_endpoint() {
        assert!(!is_configured(None));
        assert!(!is_configured(Some(
            &json!({ "pubkey": "", "endpoints": [] })
        )));
        assert!(!is_configured(Some(
            &json!({ "pubkey": "  ", "endpoints": ["https://u.test/latest.json"] })
        )));
        assert!(!is_configured(Some(
            &json!({ "pubkey": "abc", "endpoints": [] })
        )));
        assert!(is_configured(Some(
            &json!({ "pubkey": "abc", "endpoints": ["https://u.test/latest.json"] })
        )));
    }

    #[test]
    fn notes_are_trimmed_and_capped() {
        assert_eq!(release_notes(None), None);
        assert_eq!(release_notes(Some("  \n ")), None);
        assert_eq!(
            release_notes(Some(" Fixes sync \n")).as_deref(),
            Some("Fixes sync")
        );
        let long = "é".repeat(MAX_NOTES_BYTES);
        let notes = release_notes(Some(&long)).expect("notes");
        assert!(notes.ends_with('…') && notes.len() <= MAX_NOTES_BYTES + '…'.len_utf8());
    }

    #[test]
    fn keeps_a_short_publish_date_only() {
        assert_eq!(
            release_date(&json!({ "pub_date": "2026-10-27T09:00:00Z" })).as_deref(),
            Some("2026-10-27T09:00:00Z")
        );
        assert_eq!(release_date(&json!({ "pub_date": "x".repeat(80) })), None);
        assert_eq!(release_date(&json!({})), None);
    }

    #[test]
    fn signature_failures_say_nothing_was_installed() {
        let message = describe(Error::SignatureUtf8("bad".to_owned())).to_string();
        assert!(message.contains("not installed"), "{message}");
        assert!(matches!(
            describe(Error::ReleaseNotFound),
            AppError::Request(_)
        ));
    }
}
