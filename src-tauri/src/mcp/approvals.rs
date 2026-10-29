use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::encoding::hex_lower;
use crate::error::AppError;
use crate::storage::db::now_ms;

const MAX_APPROVALS: i64 = 200;

/// Identifies exactly what would run: the program, its arguments and its environment after
/// variables are filled in. A collection shared through Git, or a variable changed by a
/// teammate, therefore cannot start a different program without being approved here again.
/// Only a hash is stored, since environment values may be secrets.
pub fn fingerprint(program: &str, args: &[String], env: &[(String, String)]) -> String {
    let launch = json!({ "program": program, "args": args, "env": env });
    hex_lower(&Sha256::digest(launch.to_string().as_bytes()))
}

pub fn is_approved(conn: &Connection, fingerprint: &str) -> Result<bool, AppError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM mcp_approvals WHERE fingerprint = ?1",
            params![fingerprint],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn approve(conn: &Connection, fingerprint: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO mcp_approvals (fingerprint, approved_at) VALUES (?1, ?2)
         ON CONFLICT(fingerprint) DO UPDATE SET approved_at = excluded.approved_at",
        params![fingerprint, now_ms()],
    )?;
    conn.execute(
        "DELETE FROM mcp_approvals WHERE fingerprint IN (
             SELECT fingerprint FROM mcp_approvals ORDER BY approved_at DESC LIMIT -1 OFFSET ?1)",
        params![MAX_APPROVALS],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    #[test]
    fn approval_follows_exactly_what_runs() {
        let args = vec!["server.js".to_owned()];
        let env = vec![("TOKEN".to_owned(), "a".to_owned())];
        let first = fingerprint("node", &args, &env);
        assert_eq!(first.len(), 64);
        assert_eq!(first, fingerprint("node", &args, &env));
        let other_env = vec![("TOKEN".to_owned(), "b".to_owned())];
        assert_ne!(first, fingerprint("node", &args, &other_env));
        assert_ne!(first, fingerprint("node", &["evil.js".to_owned()], &env));

        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            assert!(!is_approved(conn, &first)?);
            approve(conn, &first)?;
            assert!(is_approved(conn, &first)?);
            Ok(())
        })
        .expect("approvals");
    }
}
