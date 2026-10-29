//! Limits shared by request sending and request storage, so both enforce the same rules.

pub const ALLOWED_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
pub const MAX_URL_LENGTH: usize = 8_192;
pub const MAX_KEY_VALUE_ROWS: usize = 100;
pub const MAX_KEY_VALUE_LENGTH: usize = 8_192;
pub const MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_SCRIPT_BYTES: usize = 100 * 1024;
/// Uploaded files are streamed from disk, so this caps upload time rather than memory.
pub const MAX_UPLOAD_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_PATH_LENGTH: usize = 4_096;

pub fn is_allowed_method(method: &str) -> bool {
    ALLOWED_METHODS.contains(&method)
}
