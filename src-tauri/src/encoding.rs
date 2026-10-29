const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Base64url without padding (RFC 4648 §5), as PKCE code challenges use.
pub fn base64_url_encode(input: &[u8]) -> String {
    base64_encode(input)
        .trim_end_matches('=')
        .replace('+', "-")
        .replace('/', "_")
}

/// Lowercase hex, as hashes and signatures are written in headers and fingerprints.
pub fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(HEX_DIGITS[usize::from(byte & 0x0F)]));
    }
    output
}

/// Standard base64 with padding, used for `Authorization: Basic` headers.
pub fn base64_encode(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let bytes = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let packed = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);
        for position in 0..4 {
            if position <= chunk.len() {
                let index = (packed >> (18 - 6 * position)) & 0x3f;
                output.push(char::from(BASE64_TABLE[index as usize]));
            } else {
                output.push('=');
            }
        }
    }
    output
}

/// Standard base64 with padding, read strictly: anything `base64_encode` would not write is
/// refused, so one value has one spelling.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let mut output = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let padding = chunk.iter().rev().take_while(|&&byte| byte == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return None;
        }
        let mut packed: u32 = 0;
        for &byte in &chunk[..4 - padding] {
            let value = BASE64_TABLE.iter().position(|&known| known == byte)?;
            packed = (packed << 6) | u32::try_from(value).ok()?;
        }
        packed <<= 6 * padding;
        let decoded = packed.to_be_bytes();
        let kept = &decoded[1..4 - padding];
        // Bits a padded chunk does not use must be zero, as the encoder writes them.
        if packed & ((1 << (8 * padding)) - 1) != 0 {
            return None;
        }
        output.extend_from_slice(kept);
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"user:p@ss"), "dXNlcjpwQHNz");
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
        assert_eq!(base64_url_encode(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn decodes_what_it_encodes_and_nothing_else() {
        for sample in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"user:p@ss",
            &[0, 255, 128, 7],
        ] {
            assert_eq!(
                base64_decode(&base64_encode(sample)).as_deref(),
                Some(sample)
            );
        }
        assert_eq!(base64_decode("Zg"), None, "missing padding");
        assert_eq!(base64_decode("Zh=="), None, "unused bits set");
        assert_eq!(base64_decode("Zg==Zg=="), None, "padding in the middle");
        assert_eq!(base64_decode("Z===",), None);
        assert_eq!(base64_decode("Zm9-"), None, "url alphabet");
    }
}
