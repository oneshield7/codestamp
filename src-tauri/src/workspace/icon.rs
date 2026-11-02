use super::models::WorkspaceIcon;
use crate::encoding::{base64_decode, base64_encode};
use crate::error::AppError;
use crate::storage::icons;

/// Icons are drawn at 32px at most and uploads are shrunk to 128px first, so this is generous.
pub const MAX_IMAGE_BYTES: usize = 256 * 1024;
const MAX_WEBSITE_CHARS: usize = 253;

/// A checked icon, ready to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredIcon {
    Letter,
    Emoji(&'static str),
    Image {
        bytes: Vec<u8>,
        mime: &'static str,
        source: Option<String>,
    },
}

/// Emoji come from the collection icon list; images must really be PNG, JPEG, GIF, WebP or ICO,
/// whatever the `data:` URL claims, so nothing else (an SVG with script, say) is ever stored.
pub fn check_icon(icon: WorkspaceIcon) -> Result<StoredIcon, AppError> {
    match icon {
        WorkspaceIcon::Letter => Ok(StoredIcon::Letter),
        WorkspaceIcon::Emoji { emoji } => Ok(StoredIcon::Emoji(icons::check_icon(&emoji)?)),
        WorkspaceIcon::Image { data_url, source } => {
            let bytes = decode_data_url(&data_url)?;
            let mime = image_type(&bytes)
                .ok_or_else(|| AppError::invalid("Use a PNG, JPG, WebP, GIF or ICO image"))?;
            let source = source.as_deref().map(check_website).transpose()?;
            Ok(StoredIcon::Image {
                bytes,
                mime,
                source,
            })
        }
    }
}

/// The icon as the UI receives it. A row whose image is missing shows the letter.
pub fn to_icon(
    kind: &str,
    emoji: Option<String>,
    image: Option<Vec<u8>>,
    mime: Option<String>,
    source: Option<String>,
) -> WorkspaceIcon {
    match (kind, emoji, image, mime) {
        ("emoji", Some(emoji), _, _) => WorkspaceIcon::Emoji { emoji },
        ("image", _, Some(bytes), Some(mime)) => WorkspaceIcon::Image {
            data_url: data_url(&mime, &bytes),
            source,
        },
        _ => WorkspaceIcon::Letter,
    }
}

pub fn data_url(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", base64_encode(bytes))
}

fn decode_data_url(text: &str) -> Result<Vec<u8>, AppError> {
    let not_an_image = || AppError::invalid("This is not an image Codestamp can use");
    let (head, body) = text.split_once(',').ok_or_else(not_an_image)?;
    if !head.starts_with("data:image/") || !head.ends_with(";base64") {
        return Err(not_an_image());
    }
    let too_large = || AppError::invalid("Images can be up to 256 KB");
    if body.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 {
        return Err(too_large());
    }
    let bytes = base64_decode(body).ok_or_else(not_an_image)?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(too_large());
    }
    Ok(bytes)
}

/// The image format, read from the file's own first bytes.
pub fn image_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(&[0, 0, 1, 0]) {
        Some("image/x-icon")
    } else {
        None
    }
}

/// A website as typed (`https://www.telkom.co.id/about`) reduced to its host name, lowercase,
/// without `www.`. Only plain host names pass: no port, no user, no IP-only tricks needed here.
pub fn check_website(raw: &str) -> Result<String, AppError> {
    let invalid = || AppError::invalid("Type a website such as example.com");
    let lower = raw.trim().to_ascii_lowercase();
    let without_scheme = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    let host = host.strip_prefix("www.").unwrap_or(host);
    let labels: Vec<&str> = host.split('.').collect();
    let valid_label = |label: &&str| {
        (1..=63).contains(&label.len())
            && label
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    };
    let top_level_is_word = labels
        .last()
        .is_some_and(|tld| tld.bytes().any(|byte| byte.is_ascii_lowercase()));
    if host.len() > MAX_WEBSITE_CHARS
        || labels.len() < 2
        || !labels.iter().all(valid_label)
        || !top_level_is_word
    {
        return Err(invalid());
    }
    Ok(host.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

    #[test]
    fn keeps_only_real_images_whatever_the_data_url_claims() {
        let png = check_icon(WorkspaceIcon::Image {
            data_url: data_url("image/jpeg", PNG),
            source: Some("https://www.Telkom.co.id/about".to_owned()),
        })
        .expect("png");
        assert_eq!(
            png,
            StoredIcon::Image {
                bytes: PNG.to_vec(),
                mime: "image/png",
                source: Some("telkom.co.id".to_owned()),
            }
        );
        let svg = data_url("image/png", b"<svg onload=\"alert(1)\"/>");
        assert!(
            check_icon(WorkspaceIcon::Image {
                data_url: svg,
                source: None
            })
            .is_err()
        );
        assert!(
            check_icon(WorkspaceIcon::Image {
                data_url: "data:text/html;base64,PGI+".to_owned(),
                source: None
            })
            .is_err()
        );
        let huge = data_url("image/png", &[PNG, &vec![0; MAX_IMAGE_BYTES]].concat());
        assert!(matches!(
            check_icon(WorkspaceIcon::Image { data_url: huge, source: None }),
            Err(AppError::InvalidInput(message)) if message.contains("256 KB")
        ));
    }

    #[test]
    fn emoji_come_from_the_icon_list() {
        assert!(
            check_icon(WorkspaceIcon::Emoji {
                emoji: "🛒".to_owned()
            })
            .is_ok()
        );
        assert!(
            check_icon(WorkspaceIcon::Emoji {
                emoji: "<b>".to_owned()
            })
            .is_err()
        );
    }

    #[test]
    fn a_website_becomes_its_host_name() {
        assert_eq!(
            check_website(" HTTPS://www.telkom.co.id/id?x=1 ")
                .ok()
                .as_deref(),
            Some("telkom.co.id")
        );
        for bad in [
            "telkom",
            "127.0.0.1",
            "telkom.co.id:8080",
            "user@telkom.co.id",
            "-bad.com",
            "",
        ] {
            assert!(check_website(bad).is_err(), "{bad} should be refused");
        }
    }
}
