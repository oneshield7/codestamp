use crate::error::AppError;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const MAX_CAPTURE_BYTES: usize = 50 * 1024 * 1024;

pub fn check_png(bytes: &[u8]) -> Result<(), AppError> {
    if bytes.len() > MAX_CAPTURE_BYTES {
        return Err(AppError::invalid("The image is larger than 50 MB"));
    }
    if !bytes.starts_with(&PNG_SIGNATURE) {
        return Err(AppError::invalid("The image is not a PNG file"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_png_data() {
        let mut png = PNG_SIGNATURE.to_vec();
        png.extend_from_slice(b"IHDR");
        assert!(check_png(&png).is_ok());
        assert!(check_png(b"GIF89a").is_err());
        assert!(check_png(&[]).is_err());
    }
}
