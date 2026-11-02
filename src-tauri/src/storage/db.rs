use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::error::AppError;

const SCHEMA_V1: &str = "
CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE sections (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL
);
CREATE TABLE requests (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    section_id TEXT REFERENCES sections(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    params TEXT NOT NULL,
    headers TEXT NOT NULL,
    body_mode TEXT NOT NULL,
    body TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_sections_collection ON sections(collection_id, sort_order);
CREATE INDEX idx_requests_collection ON requests(collection_id, sort_order);
";

/// Secret variables keep `value` empty and store an encrypted `secret` blob instead.
const SCHEMA_V2: &str = "
CREATE TABLE environments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    is_global INTEGER NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE TABLE variables (
    id TEXT PRIMARY KEY,
    environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    secret BLOB,
    enabled INTEGER NOT NULL,
    sort_order INTEGER NOT NULL
);
CREATE INDEX idx_variables_environment ON variables(environment_id, sort_order);
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// A collection's own variables live in a hidden environment row linked to the collection.
/// History keeps the request as typed (with `{{variables}}`), never the resolved secrets.
const SCHEMA_V3: &str = "
ALTER TABLE environments ADD COLUMN collection_id TEXT
    REFERENCES collections(id) ON DELETE CASCADE;
CREATE UNIQUE INDEX idx_environments_collection ON environments(collection_id)
    WHERE collection_id IS NOT NULL;
CREATE TABLE history (
    id TEXT PRIMARY KEY,
    collection_id TEXT REFERENCES collections(id) ON DELETE SET NULL,
    request_id TEXT REFERENCES requests(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    params TEXT NOT NULL,
    headers TEXT NOT NULL,
    body_mode TEXT NOT NULL,
    body TEXT NOT NULL,
    body_omitted INTEGER NOT NULL,
    status INTEGER,
    error TEXT,
    elapsed_ms INTEGER,
    body_size INTEGER,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_history_created ON history(created_at);
";

const SCHEMA_V4: &str = "
ALTER TABLE requests ADD COLUMN pre_request_script TEXT NOT NULL DEFAULT '';
ALTER TABLE requests ADD COLUMN test_script TEXT NOT NULL DEFAULT '';
";

const SCHEMA_V5: &str = "
ALTER TABLE requests ADD COLUMN kind TEXT NOT NULL DEFAULT 'http';
";

/// Imported font files live in the app data folder as `<id>.<format>`; this is their index.
const SCHEMA_V6: &str = "
CREATE TABLE fonts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    size INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
";

/// Auth settings as JSON. History rows from before this version had their auth in headers.
const SCHEMA_V7: &str = r#"
ALTER TABLE collections ADD COLUMN auth TEXT NOT NULL DEFAULT '{"type":"none"}';
ALTER TABLE requests ADD COLUMN auth TEXT NOT NULL DEFAULT '{"type":"inherit"}';
ALTER TABLE history ADD COLUMN auth TEXT NOT NULL DEFAULT '{"type":"none"}';
"#;

/// Form rows and the binary-body file per request; the files the user picked to upload (only
/// those may be read when sending); and the cookie jar, encrypted like secret variables.
const SCHEMA_V8: &str = "
ALTER TABLE requests ADD COLUMN form TEXT NOT NULL DEFAULT '[]';
ALTER TABLE requests ADD COLUMN body_file TEXT NOT NULL DEFAULT '';
CREATE TABLE file_grants (
    path TEXT PRIMARY KEY,
    granted_at INTEGER NOT NULL
);
CREATE TABLE cookie_jar (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    data BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);
";

/// Collections shared with a team through a Git repository. The local clone lives in the app
/// data folder under the collection id; this row says where it syncs to.
/// NULL keeps the icon picked from the collection's name.
const SCHEMA_V10: &str = "
ALTER TABLE collections ADD COLUMN icon TEXT;
";

/// Settings secrets such as the proxy password, and the certificates requests use. A CA
/// certificate is public; a client certificate's private key is stored encrypted.
const SCHEMA_V11: &str = "
CREATE TABLE app_secrets (
    key TEXT PRIMARY KEY,
    data BLOB NOT NULL
);
CREATE TABLE certificates (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('ca', 'client')),
    name TEXT NOT NULL,
    host TEXT,
    cert_pem TEXT NOT NULL,
    key_secret BLOB,
    created_at INTEGER NOT NULL
);
";

/// MCP requests remember the tool, prompt or resource they call. Programs started for STDIO
/// servers are remembered by a hash of exactly what ran, once the user approved them.
const SCHEMA_V12: &str = "
ALTER TABLE requests ADD COLUMN mcp_operation TEXT;
CREATE TABLE mcp_approvals (
    fingerprint TEXT PRIMARY KEY,
    approved_at INTEGER NOT NULL
);
";

/// Values for `:name` path segments, on requests and on the history that keeps them.
const SCHEMA_V13: &str = "
ALTER TABLE requests ADD COLUMN path_vars TEXT NOT NULL DEFAULT '[]';
ALTER TABLE history ADD COLUMN path_vars TEXT NOT NULL DEFAULT '[]';
";

/// GraphQL requests keep their query in `body` and their variables here.
const SCHEMA_V14: &str = "
ALTER TABLE requests ADD COLUMN graphql_variables TEXT NOT NULL DEFAULT '';
";

/// Responses kept with a request, for reference and for the mock server.
const SCHEMA_V15: &str = "
CREATE TABLE request_examples (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    status INTEGER NOT NULL,
    headers TEXT NOT NULL,
    body TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_examples_request ON request_examples(request_id, sort_order);
";

/// gRPC requests remember the method they call and where its descriptors come from.
const SCHEMA_V16: &str = "
ALTER TABLE requests ADD COLUMN grpc TEXT;
";

/// Orchestration: flows and their steps. A step whose request is deleted keeps its place (with no
/// request) so the user sees the gap instead of a silently shorter flow.
const SCHEMA_V17: &str = "
CREATE TABLE flows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE flow_steps (
    id TEXT PRIMARY KEY,
    flow_id TEXT NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    request_id TEXT REFERENCES requests(id) ON DELETE SET NULL,
    extracts TEXT NOT NULL DEFAULT '[]',
    checks TEXT NOT NULL DEFAULT '[]',
    on_failure TEXT NOT NULL DEFAULT 'stop',
    sort_order INTEGER NOT NULL
);
CREATE INDEX idx_flow_steps_flow ON flow_steps(flow_id, sort_order);
CREATE INDEX idx_flow_steps_request ON flow_steps(request_id);
";

/// SOAP requests remember their WSDL operation and how its action is sent.
const SCHEMA_V18: &str = "
ALTER TABLE requests ADD COLUMN soap TEXT;
";

/// Catalog: the owner and tags of each collection's API, and Run all results, which the
/// catalog's Tests numbers come from. Runs keep one row per request, not per iteration.
const SCHEMA_V19: &str = "
CREATE TABLE catalog_info (
    collection_id TEXT PRIMARY KEY REFERENCES collections(id) ON DELETE CASCADE,
    owner TEXT NOT NULL,
    tags TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE collection_runs (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    environment TEXT,
    started_at INTEGER NOT NULL,
    finished_at INTEGER NOT NULL,
    outcome TEXT NOT NULL,
    results TEXT NOT NULL
);
CREATE INDEX idx_collection_runs ON collection_runs(collection_id, finished_at);
CREATE INDEX idx_history_collection ON history(collection_id, created_at);
";

/// Docs: a Markdown description per collection, section and request. It has its own column, so
/// a request card saving its content never overwrites what was written in Docs.
const SCHEMA_V20: &str = "
ALTER TABLE collections ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE sections ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE requests ADD COLUMN description TEXT NOT NULL DEFAULT '';
";

/// Collaboration accounts per shared collection: the username Git signs in with on https hosts,
/// and the commit author. Links from before this step have neither and use the profile.
const SCHEMA_V21: &str = "
ALTER TABLE team_links ADD COLUMN username TEXT;
ALTER TABLE team_links ADD COLUMN author_name TEXT;
ALTER TABLE team_links ADD COLUMN author_email TEXT;
";

/// Git accounts picked for shared collections: host and username only; the sign-in stays with
/// Git's credential manager.
const SCHEMA_V22: &str = "
CREATE TABLE git_accounts (
    id TEXT PRIMARY KEY,
    host TEXT NOT NULL,
    username TEXT NOT NULL,
    label TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (host, username)
);
";

/// Environments that belong to a collection and travel with it through Collaboration. Collection
/// variable sets keep using `collection_id`; this marks a named environment's owner.
const SCHEMA_V23: &str = "
ALTER TABLE environments ADD COLUMN owner_collection_id TEXT REFERENCES collections(id) ON DELETE CASCADE;
CREATE INDEX idx_environments_owner ON environments(owner_collection_id);
";

// The remote commit a shared collection last matched, the base when a lost clone is rebuilt.
const SCHEMA_V24: &str = "
ALTER TABLE team_links ADD COLUMN synced_commit TEXT;
";

/// Access groups of shared collections and restricted variables. `file` is the group as the
/// repository has it; `trusted_file` and the vault-encrypted `keys` are what this device has
/// checked, and `access` says whether it can read and write the group's values now.
const SCHEMA_V25: &str = "
CREATE TABLE team_access_groups (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    file TEXT NOT NULL,
    trusted_file TEXT,
    keys BLOB,
    access INTEGER NOT NULL DEFAULT 0,
    problem TEXT,
    PRIMARY KEY (collection_id, id)
);
CREATE TABLE team_access_requests (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    file TEXT NOT NULL,
    PRIMARY KEY (collection_id, path)
);
ALTER TABLE variables ADD COLUMN restricted TEXT;
ALTER TABLE variables ADD COLUMN sealed TEXT;
";

/// Recently deleted: a deleted collection or request as the rows it had, restorable for 30 days.
/// Secrets in those rows stay vault-encrypted.
const SCHEMA_V26: &str = "
CREATE TABLE deleted_items (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('collection', 'request')),
    name TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    collection_name TEXT NOT NULL,
    request_count INTEGER NOT NULL,
    rows TEXT NOT NULL,
    deleted_at INTEGER NOT NULL
);
CREATE INDEX idx_deleted_items_at ON deleted_items(deleted_at);
";

/// Each collection keeps its own active environment, a choice on this device that is never
/// synced. The one environment that used to be active for everything stays active in every
/// collection that may use it: its own collection, or all of them when it is kept on this device.
const SCHEMA_V27: &str = "
CREATE TABLE active_environments (
    collection_id TEXT PRIMARY KEY REFERENCES collections(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE
);
INSERT INTO active_environments (collection_id, environment_id)
SELECT c.id, e.id
FROM settings s
JOIN environments e ON e.id = s.value AND e.is_global = 0 AND e.collection_id IS NULL
JOIN collections c ON e.owner_collection_id IS NULL OR e.owner_collection_id = c.id
WHERE s.key = 'active_environment_id';
DELETE FROM settings WHERE key = 'active_environment_id';
";

/// Workspaces keep collections, named environments, Globals, flows, cookies and Recently deleted
/// apart. Everything already here goes into the first one. A collection's own variable set
/// (`collection_id` set) follows its collection, so it has no workspace of its own. Collections
/// and flows block deleting their workspace; the rest goes with it.
const SCHEMA_V28: &str = "
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    hue TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
INSERT INTO workspaces (id, name, hue, sort_order, created_at)
VALUES ('00000000-0000-4000-8000-000000000001', 'My workspace', 'blue', 0, 0);
ALTER TABLE collections ADD COLUMN workspace_id TEXT REFERENCES workspaces(id);
ALTER TABLE flows ADD COLUMN workspace_id TEXT REFERENCES workspaces(id);
ALTER TABLE environments ADD COLUMN workspace_id TEXT
    REFERENCES workspaces(id) ON DELETE CASCADE;
ALTER TABLE deleted_items ADD COLUMN workspace_id TEXT
    REFERENCES workspaces(id) ON DELETE CASCADE;
UPDATE collections SET workspace_id = '00000000-0000-4000-8000-000000000001';
UPDATE flows SET workspace_id = '00000000-0000-4000-8000-000000000001';
UPDATE environments SET workspace_id = '00000000-0000-4000-8000-000000000001'
WHERE collection_id IS NULL;
UPDATE deleted_items SET workspace_id = '00000000-0000-4000-8000-000000000001';
CREATE INDEX idx_collections_workspace ON collections(workspace_id, sort_order);
CREATE INDEX idx_flows_workspace ON flows(workspace_id, sort_order);
CREATE UNIQUE INDEX idx_environments_globals ON environments(workspace_id) WHERE is_global = 1;
CREATE TABLE workspace_cookies (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
    data BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);
INSERT INTO workspace_cookies (workspace_id, data, updated_at)
SELECT '00000000-0000-4000-8000-000000000001', data, updated_at FROM cookie_jar;
DROP TABLE cookie_jar;
INSERT INTO settings (key, value)
VALUES ('current_workspace', '00000000-0000-4000-8000-000000000001');
CREATE VIEW current_workspace AS SELECT COALESCE(
    (SELECT w.id FROM settings s JOIN workspaces w ON w.id = s.value
     WHERE s.key = 'current_workspace'),
    (SELECT id FROM workspaces ORDER BY sort_order, created_at LIMIT 1)) AS id;
";

/// A workspace's icon: its first letter (the default), an emoji, or an image, which is either a
/// logo found from a website (`icon_source`) or a file the user chose. Images are kept here, so
/// they show without a connection.
const SCHEMA_V29: &str = "
ALTER TABLE workspaces ADD COLUMN icon_kind TEXT NOT NULL DEFAULT 'letter'
    CHECK (icon_kind IN ('letter', 'emoji', 'image'));
ALTER TABLE workspaces ADD COLUMN icon_emoji TEXT;
ALTER TABLE workspaces ADD COLUMN icon_image BLOB;
ALTER TABLE workspaces ADD COLUMN icon_type TEXT;
ALTER TABLE workspaces ADD COLUMN icon_source TEXT;
";

const SCHEMA_V9: &str = "
CREATE TABLE team_links (
    collection_id TEXT PRIMARY KEY REFERENCES collections(id) ON DELETE CASCADE,
    remote TEXT NOT NULL,
    branch TEXT NOT NULL,
    synced_at INTEGER,
    created_at INTEGER NOT NULL
);
";

/// A single SQLite connection shared by all commands; requests are short, so a mutex is enough.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Self::init(Connection::open(path)?)
    }

    /// A database that lives only as long as the process: tests and the command-line runner.
    pub fn in_memory() -> Result<Self, AppError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, AppError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // Overwrites deleted content, so a value made secret leaves no plain copy in free pages.
        conn.pragma_update(None, "secure_delete", "ON")?;
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn with<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let conn = self.conn.lock().map_err(|_| AppError::Storage)?;
        operation(&conn)
    }

    /// For the few operations that need the connection itself, such as restoring a backup.
    pub fn with_mut<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut conn = self.conn.lock().map_err(|_| AppError::Storage)?;
        operation(&mut conn)
    }
}

/// Schema steps in order. Step N runs once, in its own transaction, and sets `user_version` to N;
/// a new step is simply appended here.
const MIGRATIONS: [&str; 29] = [
    SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4, SCHEMA_V5, SCHEMA_V6, SCHEMA_V7, SCHEMA_V8,
    SCHEMA_V9, SCHEMA_V10, SCHEMA_V11, SCHEMA_V12, SCHEMA_V13, SCHEMA_V14, SCHEMA_V15, SCHEMA_V16,
    SCHEMA_V17, SCHEMA_V18, SCHEMA_V19, SCHEMA_V20, SCHEMA_V21, SCHEMA_V22, SCHEMA_V23, SCHEMA_V24,
    SCHEMA_V25, SCHEMA_V26, SCHEMA_V27, SCHEMA_V28, SCHEMA_V29,
];
/// The `user_version` of a database that has every step.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;
/// The step that also creates the Globals environment.
const GLOBALS_STEP: i64 = 2;

pub fn migrate(conn: &Connection) -> Result<(), AppError> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    for (step, sql) in (1_i64..).zip(MIGRATIONS) {
        if version >= step {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        if step == GLOBALS_STEP {
            tx.execute(
                "INSERT INTO environments (id, name, is_global, sort_order, created_at)
                 VALUES (?1, 'Globals', 1, 0, ?2)",
                rusqlite::params![new_id(), now_ms()],
            )?;
        }
        tx.pragma_update(None, "user_version", step)?;
        tx.commit()?;
    }
    Ok(())
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_an_older_database_step_by_step() {
        let conn = Connection::open_in_memory().expect("database");
        for sql in &MIGRATIONS[..5] {
            conn.execute_batch(sql).expect("older schema");
        }
        conn.pragma_update(None, "user_version", 5)
            .expect("version");

        migrate(&conn).expect("migrate");
        migrate(&conn).expect("a second run changes nothing");

        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(usize::try_from(version).ok(), Some(MIGRATIONS.len()));
        conn.execute(
            "INSERT INTO collections (id, name, sort_order, created_at, updated_at)
             VALUES ('c', 'Users API', 1, 0, 0)",
            [],
        )
        .expect("collection");
        let auth: String = conn
            .query_row("SELECT auth FROM collections", [], |row| row.get(0))
            .expect("auth column");
        assert_eq!(auth, r#"{"type":"none"}"#);
    }

    /// The actives an older database ends up with when `active` was its one active environment.
    fn actives_after_upgrade(active: &str) -> Vec<(String, String)> {
        let conn = Connection::open_in_memory().expect("database");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("keys");
        for sql in &MIGRATIONS[..26] {
            conn.execute_batch(sql).expect("older schema");
        }
        conn.pragma_update(None, "user_version", 26)
            .expect("version");
        conn.execute_batch(
            "INSERT INTO collections (id, name, sort_order, created_at, updated_at)
             VALUES ('a', 'Orders', 1, 0, 0), ('b', 'Users', 2, 0, 0);
             INSERT INTO environments (id, name, is_global, sort_order, created_at)
             VALUES ('device', 'Device', 0, 1, 0);
             INSERT INTO environments (id, name, is_global, owner_collection_id, sort_order, created_at)
             VALUES ('owned', 'Orders staging', 0, 'a', 2, 0);",
        )
        .expect("data");
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('active_environment_id', ?1)",
            [active],
        )
        .expect("old setting");
        migrate(&conn).expect("migrate");
        let left: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = 'active_environment_id'",
                [],
                |row| row.get(0),
            )
            .expect("settings");
        assert_eq!(left, 0, "the old setting is gone");
        let mut statement = conn
            .prepare("SELECT collection_id, environment_id FROM active_environments ORDER BY 1")
            .expect("query");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("rows")
            .collect::<Result<_, _>>()
            .expect("actives")
    }

    #[test]
    fn the_old_active_environment_stays_active_where_it_may_be_used() {
        let pair =
            |collection: &str, environment: &str| (collection.to_owned(), environment.to_owned());
        assert_eq!(
            actives_after_upgrade("device"),
            [pair("a", "device"), pair("b", "device")]
        );
        assert_eq!(actives_after_upgrade("owned"), [pair("a", "owned")]);
    }

    #[test]
    fn everything_from_before_workspaces_lands_in_my_workspace() {
        let conn = Connection::open_in_memory().expect("database");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("keys");
        for sql in &MIGRATIONS[..27] {
            conn.execute_batch(sql).expect("older schema");
        }
        conn.pragma_update(None, "user_version", 27)
            .expect("version");
        conn.execute_batch(
            "INSERT INTO collections (id, name, sort_order, created_at, updated_at)
             VALUES ('a', 'Orders', 1, 0, 0);
             INSERT INTO environments (id, name, is_global, sort_order, created_at)
             VALUES ('globals', 'Globals', 1, 0, 0), ('device', 'Device', 0, 1, 0);
             INSERT INTO environments (id, name, is_global, sort_order, created_at, collection_id)
             VALUES ('set', 'Collection variables', 0, 0, 0, 'a');
             INSERT INTO flows (id, name, sort_order, created_at, updated_at)
             VALUES ('f', 'Checkout', 0, 0, 0);
             INSERT INTO cookie_jar (id, data, updated_at) VALUES (1, x'0102', 5);",
        )
        .expect("data");
        migrate(&conn).expect("migrate");

        let (id, name): (String, String) = conn
            .query_row(
                "SELECT w.id, w.name FROM current_workspace c JOIN workspaces w ON w.id = c.id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("open workspace");
        assert_eq!(name, "My workspace");
        let in_workspace =
            |sql: &str| -> i64 { conn.query_row(sql, [&id], |row| row.get(0)).expect("count") };
        assert_eq!(
            in_workspace("SELECT COUNT(*) FROM collections WHERE workspace_id = ?1"),
            1
        );
        assert_eq!(
            in_workspace("SELECT COUNT(*) FROM flows WHERE workspace_id = ?1"),
            1
        );
        assert_eq!(
            in_workspace("SELECT COUNT(*) FROM environments WHERE workspace_id = ?1"),
            2,
            "Globals and the device environment, not the collection's own set"
        );
        assert_eq!(
            in_workspace("SELECT COUNT(*) FROM workspace_cookies WHERE workspace_id = ?1"),
            1
        );
    }
}
