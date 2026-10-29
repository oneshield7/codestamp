use rusqlite::{Connection, OptionalExtension, params};

use super::jar::CookieJar;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::db::{Database, now_ms};

/// Binds the encrypted jar to its purpose, the way a variable id binds a secret to its row.
const VAULT_CONTEXT: &str = "cookie-jar";

/// Cookies are credentials (session tokens), so the jar is stored encrypted like secrets.
pub fn save(conn: &Connection, vault: &Vault, jar: &CookieJar) -> Result<(), AppError> {
    let blob = vault.encrypt(VAULT_CONTEXT, &jar.to_json()?)?;
    conn.execute(
        "INSERT INTO cookie_jar (id, data, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
        params![blob, now_ms()],
    )?;
    Ok(())
}

/// Saves the jar when responses changed it since the last save.
pub fn save_if_changed(db: &Database, vault: &Vault, jar: &CookieJar) -> Result<(), AppError> {
    if !jar.take_changed() {
        return Ok(());
    }
    let saved = db.with(|conn| save(conn, vault, jar));
    if saved.is_err() {
        // Tried again after the next response.
        jar.mark_changed();
    }
    saved
}

/// Loads the saved jar at startup. The key is only read when there is a jar to decrypt.
pub fn restore(conn: &Connection, vault: &Vault, jar: &CookieJar) -> Result<(), AppError> {
    let blob: Option<Vec<u8>> = conn
        .query_row("SELECT data FROM cookie_jar WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()?;
    let Some(blob) = blob else {
        return Ok(());
    };
    match vault.decrypt(VAULT_CONTEXT, &blob) {
        Ok(json) => jar.restore_json(&json),
        // Without the key (for example after the credential store was reset) the app starts with
        // an empty jar; the unreadable one is replaced by the next save.
        Err(AppError::SecretStore) => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use reqwest::cookie::CookieStore as _;
    use reqwest::header::HeaderValue;

    use super::*;
    use crate::environment::vault::FixedKey;

    #[test]
    fn keeps_the_jar_encrypted_and_restores_it() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([5; 32])));
        let jar = CookieJar::default();
        let site = url::Url::parse("https://shop.test/").expect("url");
        let header = HeaderValue::from_static("session=s3cr3t-session; Path=/");
        jar.set_cookies(&mut std::iter::once(&header), &site);

        save_if_changed(&db, &vault, &jar).expect("save");
        let stored: Vec<u8> = db
            .with(|conn| Ok(conn.query_row("SELECT data FROM cookie_jar", [], |row| row.get(0))?))
            .expect("stored");
        assert!(!stored.windows(6).any(|window| window == b"s3cr3t"));

        let restored = CookieJar::default();
        db.with(|conn| restore(conn, &vault, &restored))
            .expect("restore");
        assert_eq!(restored.list().expect("list").len(), 1);

        let other_key = Vault::new(Box::new(FixedKey([6; 32])));
        let empty = CookieJar::default();
        db.with(|conn| restore(conn, &other_key, &empty))
            .expect("starts empty");
        assert!(empty.list().expect("list").is_empty());
    }
}
