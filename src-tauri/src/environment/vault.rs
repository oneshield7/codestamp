use std::sync::Mutex;

use chacha20poly1305::aead::{Aead, Generate, Key, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

use crate::error::AppError;
use crate::legacy;

const KEY_SERVICE: &str = "dev.codestamp.desktop";
const KEY_ACCOUNT: &str = "variables-encryption-key";
const DEVICE_ACCOUNT: &str = "access-device-key";
const NONCE_LENGTH: usize = 12;

/// Supplies the 32-byte encryption key. The real source is the OS credential store.
pub trait KeySource: Send + Sync {
    fn load_or_create(&self) -> Result<Vec<u8>, AppError>;
}

/// Encrypts secret values. Each value is bound to its owner (a variable id, or a fixed name such
/// as the cookie jar's), so an encrypted value copied elsewhere fails to decrypt. The key is
/// loaded on first use only.
pub struct Vault {
    source: Box<dyn KeySource>,
    cipher: Mutex<Option<ChaCha20Poly1305>>,
}

impl Vault {
    pub fn new(source: Box<dyn KeySource>) -> Self {
        Self {
            source,
            cipher: Mutex::new(None),
        }
    }

    pub fn encrypt(&self, owner: &str, plaintext: &str) -> Result<Vec<u8>, AppError> {
        self.with_cipher(|cipher| {
            let nonce = Nonce::generate();
            let sealed = cipher
                .encrypt(
                    &nonce,
                    Payload {
                        msg: plaintext.as_bytes(),
                        aad: owner.as_bytes(),
                    },
                )
                .map_err(|_| AppError::SecretStore)?;
            let mut blob = nonce.to_vec();
            blob.extend_from_slice(&sealed);
            Ok(blob)
        })
    }

    pub fn decrypt(&self, owner: &str, blob: &[u8]) -> Result<String, AppError> {
        if blob.len() < NONCE_LENGTH {
            return Err(AppError::SecretStore);
        }
        let (nonce_bytes, sealed) = blob.split_at(NONCE_LENGTH);
        let nonce = Nonce::try_from(nonce_bytes).map_err(|_| AppError::SecretStore)?;
        self.with_cipher(|cipher| {
            let plain = cipher
                .decrypt(
                    &nonce,
                    Payload {
                        msg: sealed,
                        aad: owner.as_bytes(),
                    },
                )
                .map_err(|_| AppError::SecretStore)?;
            String::from_utf8(plain).map_err(|_| AppError::SecretStore)
        })
    }

    fn with_cipher<T>(
        &self,
        operation: impl FnOnce(&ChaCha20Poly1305) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut slot = self.cipher.lock().map_err(|_| AppError::SecretStore)?;
        if slot.is_none() {
            let bytes = self.source.load_or_create()?;
            let key = Key::<ChaCha20Poly1305>::try_from(bytes.as_slice())
                .map_err(|_| AppError::SecretStore)?;
            *slot = Some(ChaCha20Poly1305::new(&key));
        }
        slot.as_ref().map_or(Err(AppError::SecretStore), operation)
    }
}

/// Keeps the key in Windows Credential Manager (via keyring-core), never in the database.
pub struct CredentialStoreKey;

impl KeySource for CredentialStoreKey {
    fn load_or_create(&self) -> Result<Vec<u8>, AppError> {
        let entry = credential(KEY_SERVICE, KEY_ACCOUNT)?;
        match entry.get_secret() {
            Ok(bytes) => Ok(bytes),
            Err(keyring_core::Error::NoEntry) => adopt_or_create(&entry),
            Err(_) => Err(AppError::SecretStore),
        }
    }
}

/// This device's private key for access groups, kept in the same store. It never leaves it, so
/// a new device or a reinstalled system asks for access again.
pub struct DeviceCredential;

impl KeySource for DeviceCredential {
    fn load_or_create(&self) -> Result<Vec<u8>, AppError> {
        let entry = credential(KEY_SERVICE, DEVICE_ACCOUNT)?;
        match entry.get_secret() {
            Ok(bytes) => Ok(bytes),
            Err(keyring_core::Error::NoEntry) => {
                let key = Key::<ChaCha20Poly1305>::generate();
                entry.set_secret(&key).map_err(|_| AppError::SecretStore)?;
                Ok(key.to_vec())
            }
            Err(_) => Err(AppError::SecretStore),
        }
    }
}

/// Moves the key kept under the pre-rename name, so secrets saved before the rename still open.
/// Without one, a new key is made.
fn adopt_or_create(entry: &keyring_core::Entry) -> Result<Vec<u8>, AppError> {
    let old = credential(legacy::IDENTIFIER, KEY_ACCOUNT)?;
    match old.get_secret() {
        Ok(key) => {
            entry.set_secret(&key).map_err(|_| AppError::SecretStore)?;
            // The key is stored under the new name now; an old entry left behind is never read.
            let _ = old.delete_credential();
            Ok(key)
        }
        Err(keyring_core::Error::NoEntry) => {
            let key = Key::<ChaCha20Poly1305>::generate();
            entry.set_secret(&key).map_err(|_| AppError::SecretStore)?;
            Ok(key.to_vec())
        }
        Err(_) => Err(AppError::SecretStore),
    }
}

fn credential(service: &str, account: &str) -> Result<keyring_core::Entry, AppError> {
    keyring_core::Entry::new(service, account).map_err(|_| AppError::SecretStore)
}

/// A random key kept only in memory, for the command-line runner's throwaway database.
pub struct EphemeralKey(Vec<u8>);

impl EphemeralKey {
    pub fn generate() -> Self {
        Self(Key::<ChaCha20Poly1305>::generate().to_vec())
    }
}

impl KeySource for EphemeralKey {
    fn load_or_create(&self) -> Result<Vec<u8>, AppError> {
        Ok(self.0.clone())
    }
}

/// Registers the OS credential store: Credential Manager (Windows), Keychain (macOS) or
/// Secret Service (Linux, e.g. GNOME Keyring / KWallet).
pub fn install_platform_credential_store() -> Result<(), AppError> {
    #[cfg(windows)]
    let store = windows_native_keyring_store::Store::new();
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new();
    #[cfg(target_os = "linux")]
    let store = zbus_secret_service_keyring_store::Store::new();

    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    keyring_core::set_default_store(store.map_err(|_| AppError::SecretStore)?);
    Ok(())
}

#[cfg(test)]
pub struct FixedKey(pub [u8; 32]);

#[cfg(test)]
impl KeySource for FixedKey {
    fn load_or_create(&self) -> Result<Vec<u8>, AppError> {
        Ok(self.0.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> Vault {
        Vault::new(Box::new(FixedKey([7; 32])))
    }

    #[test]
    fn round_trips_a_secret() {
        let vault = vault();
        let blob = vault.encrypt("var-1", "s3cr3t-token").expect("encrypt");
        assert!(!blob.windows(6).any(|window| window == b"s3cr3t"));
        assert_eq!(
            vault.decrypt("var-1", &blob).expect("decrypt"),
            "s3cr3t-token"
        );
    }

    #[test]
    fn rejects_a_blob_moved_to_another_variable() {
        let vault = vault();
        let blob = vault.encrypt("var-1", "token").expect("encrypt");
        assert!(vault.decrypt("var-2", &blob).is_err());
    }

    #[test]
    fn rejects_tampered_or_truncated_blobs() {
        let vault = vault();
        let mut blob = vault.encrypt("var-1", "token").expect("encrypt");
        if let Some(last) = blob.last_mut() {
            *last ^= 0xff;
        }
        assert!(vault.decrypt("var-1", &blob).is_err());
        assert!(vault.decrypt("var-1", &[1, 2, 3]).is_err());
    }

    #[test]
    fn uses_a_fresh_nonce_each_time() {
        let vault = vault();
        let first = vault.encrypt("var-1", "same").expect("encrypt");
        let second = vault.encrypt("var-1", "same").expect("encrypt");
        assert_ne!(first, second);
    }
}
