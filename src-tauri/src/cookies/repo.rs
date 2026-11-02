use rusqlite::{Connection, OptionalExtension, params};

use super::jar::CookieJar;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::db::{Database, now_ms};
use crate::workspace::repo as workspaces;

/// Binds the encrypted jar to its purpose, the way a variable id binds a secret to its row.
const VAULT_CONTEXT: &str = "cookie-jar";

/// Cookies are credentials (session tokens), so the jar is stored encrypted like secrets. The
/// jar in memory always holds the open workspace's cookies.
pub fn save(conn: &Connection, vault: &Vault, jar: &CookieJar) -> Result<(), AppError> {
    let blob = vault.encrypt(VAULT_CONTEXT, &jar.to_json()?)?;
    conn.execute(
        "INSERT INTO workspace_cookies (workspace_id, data, updated_at)
         VALUES ((SELECT id FROM current_workspace), ?1, ?2)
         ON CONFLICT(workspace_id) DO UPDATE
         SET data = excluded.data, updated_at = excluded.updated_at",
        params![blob, now_ms()],
    )?;
    Ok(())
}

/// Keeps the open workspace's cookies, then fills the jar with the ones `to` saved. The caller
/// opens `to` afterwards.
pub fn switch(conn: &Connection, vault: &Vault, jar: &CookieJar, to: &str) -> Result<(), AppError> {
    save(conn, vault, jar)?;
    jar.clear(None)?;
    jar.take_changed();
    load(conn, vault, jar, to)
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

/// Loads the open workspace's jar at startup.
pub fn restore(conn: &Connection, vault: &Vault, jar: &CookieJar) -> Result<(), AppError> {
    load(conn, vault, jar, &workspaces::current(conn)?)
}

/// The key is only read when there is a jar to decrypt.
fn load(
    conn: &Connection,
    vault: &Vault,
    jar: &CookieJar,
    workspace_id: &str,
) -> Result<(), AppError> {
    let blob: Option<Vec<u8>> = conn
        .query_row(
            "SELECT data FROM workspace_cookies WHERE workspace_id = ?1",
            params![workspace_id],
            |row| row.get(0),
        )
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
            .with(|conn| {
                Ok(conn.query_row("SELECT data FROM workspace_cookies", [], |row| row.get(0))?)
            })
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

    #[test]
    fn each_workspace_keeps_its_own_cookies() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([5; 32])));
        let jar = CookieJar::default();
        let site = url::Url::parse("https://shop.test/").expect("url");
        let header = HeaderValue::from_static("session=first; Path=/");
        jar.set_cookies(&mut std::iter::once(&header), &site);
        let (first, other) = db
            .with(|conn| {
                let first = workspaces::current(conn)?;
                let other = workspaces::insert_named(conn, "Other")?.id;
                switch(conn, &vault, &jar, &other)?;
                workspaces::set_current(conn, &other)?;
                Ok((first, other))
            })
            .expect("switch");
        assert!(
            jar.list().expect("list").is_empty(),
            "the new workspace starts empty"
        );

        db.with(|conn| {
            switch(conn, &vault, &jar, &first)?;
            workspaces::set_current(conn, &first)
        })
        .expect("back");
        let kept = &jar.list().expect("list")[0];
        let key = crate::cookies::models::CookieKey {
            domain: kept.domain.clone(),
            path: kept.path.clone(),
            name: kept.name.clone(),
        };
        assert_eq!(jar.value(&key).expect("value"), "first");
        assert_ne!(first, other);
    }
}
