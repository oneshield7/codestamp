use rusqlite::{Connection, OptionalExtension, Row, params};

use super::icon::{self, StoredIcon};
use super::models::{Workspace, WorkspaceList};
use crate::error::AppError;
use crate::storage::db::{new_id, now_ms};

/// The settings key the `current_workspace` view reads.
const CURRENT_KEY: &str = "current_workspace";
/// What `from_row` reads, for the workspaces `w`.
const COLUMNS: &str = "w.id, w.name, w.hue,
    w.icon_kind, w.icon_emoji, w.icon_image, w.icon_type, w.icon_source,
    (SELECT COUNT(*) FROM collections c WHERE c.workspace_id = w.id),
    (SELECT COUNT(*) FROM environments e
     WHERE e.workspace_id = w.id AND e.is_global = 0 AND e.collection_id IS NULL),
    (SELECT COUNT(*) FROM flows f WHERE f.workspace_id = w.id)";

/// The open workspace. The view falls back to the first one when the saved choice is gone.
pub fn current(conn: &Connection) -> Result<String, AppError> {
    conn.query_row("SELECT id FROM current_workspace", [], |row| {
        row.get::<_, Option<String>>(0)
    })?
    .ok_or(AppError::Storage)
}

pub fn list(conn: &Connection) -> Result<WorkspaceList, AppError> {
    let mut statement = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM workspaces w ORDER BY w.sort_order, w.created_at"
    ))?;
    let workspaces = statement
        .query_map([], from_row)?
        .collect::<Result<_, _>>()?;
    Ok(WorkspaceList {
        workspaces,
        current_id: current(conn)?,
    })
}

fn get(conn: &Connection, id: &str) -> Result<Workspace, AppError> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM workspaces w WHERE w.id = ?1"),
        params![id],
        from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("Workspace"))
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<Workspace> {
    let kind: String = row.get(3)?;
    Ok(Workspace {
        id: row.get(0)?,
        name: row.get(1)?,
        hue: row.get(2)?,
        icon: icon::to_icon(&kind, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?),
        collection_count: row.get(8)?,
        environment_count: row.get(9)?,
        flow_count: row.get(10)?,
    })
}

/// How a workspace looks: its name, color and icon, all checked.
pub struct Look<'a> {
    pub name: &'a str,
    pub hue: &'a str,
    pub icon: &'a StoredIcon,
}

/// The icon columns, `icon_kind` to `icon_source`, as stored.
struct IconColumns<'a> {
    kind: &'static str,
    emoji: Option<&'a str>,
    image: Option<&'a [u8]>,
    mime: Option<&'a str>,
    source: Option<&'a str>,
}

impl Look<'_> {
    fn icon_columns(&self) -> IconColumns<'_> {
        let letter = IconColumns {
            kind: "letter",
            emoji: None,
            image: None,
            mime: None,
            source: None,
        };
        match self.icon {
            StoredIcon::Letter => letter,
            StoredIcon::Emoji(emoji) => IconColumns {
                kind: "emoji",
                emoji: Some(emoji),
                ..letter
            },
            StoredIcon::Image {
                bytes,
                mime,
                source,
            } => IconColumns {
                kind: "image",
                image: Some(bytes),
                mime: Some(mime),
                source: source.as_deref(),
                ..letter
            },
        }
    }
}

/// A new, empty workspace with its own Globals.
pub fn insert(conn: &Connection, look: &Look<'_>) -> Result<Workspace, AppError> {
    let tx = conn.unchecked_transaction()?;
    let id = new_id();
    let now = now_ms();
    let icon = look.icon_columns();
    tx.execute(
        "INSERT INTO workspaces (id, name, hue, sort_order, created_at,
                                 icon_kind, icon_emoji, icon_image, icon_type, icon_source)
         VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM workspaces), ?4,
                 ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            look.name,
            look.hue,
            now,
            icon.kind,
            icon.emoji,
            icon.image,
            icon.mime,
            icon.source
        ],
    )?;
    tx.execute(
        "INSERT INTO environments (id, name, is_global, sort_order, created_at, workspace_id)
         VALUES (?1, 'Globals', 1, 0, ?2, ?3)",
        params![new_id(), now, id],
    )?;
    let created = get(&tx, &id)?;
    tx.commit()?;
    Ok(created)
}

/// A workspace with a letter icon, for tests elsewhere in the core.
#[cfg(test)]
pub fn insert_named(conn: &Connection, name: &str) -> Result<Workspace, AppError> {
    insert(
        conn,
        &Look {
            name,
            hue: "green",
            icon: &StoredIcon::Letter,
        },
    )
}

pub fn update(conn: &Connection, id: &str, look: &Look<'_>) -> Result<Workspace, AppError> {
    let icon = look.icon_columns();
    let changed = conn.execute(
        "UPDATE workspaces SET name = ?2, hue = ?3, icon_kind = ?4, icon_emoji = ?5,
                icon_image = ?6, icon_type = ?7, icon_source = ?8
         WHERE id = ?1",
        params![
            id,
            look.name,
            look.hue,
            icon.kind,
            icon.emoji,
            icon.image,
            icon.mime,
            icon.source
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound("Workspace"));
    }
    get(conn, id)
}

/// Remembers the open workspace. Swapping the cookie jar is up to the caller.
pub fn set_current(conn: &Connection, id: &str) -> Result<(), AppError> {
    check_exists(conn, id)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![CURRENT_KEY, id],
    )?;
    Ok(())
}

/// The workspace that opens when `id` is deleted while it is open.
pub fn other_than(conn: &Connection, id: &str) -> Result<String, AppError> {
    conn.query_row(
        "SELECT id FROM workspaces WHERE id <> ?1 ORDER BY sort_order, created_at LIMIT 1",
        params![id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::invalid("Codestamp needs at least one workspace"))
}

/// Only an empty workspace can go, and never the last one.
pub fn check_deletable(conn: &Connection, id: &str) -> Result<(), AppError> {
    let (collections, flows): (i64, i64) = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM collections WHERE workspace_id = ?1),
                    (SELECT COUNT(*) FROM flows WHERE workspace_id = ?1)
             FROM workspaces WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound("Workspace"))?;
    if collections > 0 || flows > 0 {
        return Err(AppError::invalid(
            "Move or delete this workspace's collections and flows first",
        ));
    }
    other_than(conn, id).map(|_| ())
}

/// Deletes an empty workspace with its environments, Globals, cookies and Recently deleted items.
pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    check_deletable(conn, id)?;
    conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
    Ok(())
}

/// Moves a collection and the environments it owns. An environment kept on this device stays in
/// the old workspace, so the collection stops using it.
pub fn move_collection(
    conn: &Connection,
    collection_id: &str,
    workspace_id: &str,
) -> Result<(), AppError> {
    check_exists(conn, workspace_id)?;
    let from: Option<String> = conn
        .query_row(
            "SELECT workspace_id FROM collections WHERE id = ?1",
            params![collection_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(AppError::NotFound("Collection"))?;
    if from.as_deref() == Some(workspace_id) {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    place_collection(&tx, collection_id, workspace_id)?;
    tx.execute(
        "DELETE FROM active_environments WHERE collection_id = ?1
           AND environment_id IN (SELECT id FROM environments WHERE owner_collection_id IS NULL)",
        params![collection_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// Puts a collection and the environments it owns in a workspace.
pub fn place_collection(
    conn: &Connection,
    collection_id: &str,
    workspace_id: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE collections SET workspace_id = ?2 WHERE id = ?1",
        params![collection_id, workspace_id],
    )?;
    conn.execute(
        "UPDATE environments SET workspace_id = ?2
         WHERE owner_collection_id = ?1 AND collection_id IS NULL",
        params![collection_id, workspace_id],
    )?;
    Ok(())
}

pub fn check_exists(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.query_row(
        "SELECT 1 FROM workspaces WHERE id = ?1",
        params![id],
        |_| Ok(()),
    )
    .optional()?
    .ok_or(AppError::NotFound("Workspace"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::models::VariableInput;
    use crate::environment::repo as environments;
    use crate::environment::vault::{FixedKey, Vault};
    use crate::storage::db::Database;
    use crate::storage::repo as collections;

    /// Flows are made here the way Orchestration makes them: in the open workspace.
    fn add_flow(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO flows (id, name, sort_order, created_at, updated_at, workspace_id)
             VALUES (?1, 'Checkout', 0, 0, 0, (SELECT id FROM current_workspace))",
            params![crate::storage::db::new_id()],
        )?;
        Ok(())
    }

    fn names_of(list: &WorkspaceList) -> Vec<&str> {
        list.workspaces.iter().map(|w| w.name.as_str()).collect()
    }

    fn set_global(conn: &Connection, vault: &Vault, value: &str) -> Result<(), AppError> {
        let globals = environments::globals_for(conn, None)?;
        environments::replace_variables(
            conn,
            vault,
            &globals,
            vec![VariableInput {
                id: None,
                key: "host".to_owned(),
                value: Some(value.to_owned()),
                is_secret: false,
                enabled: true,
                restricted: None,
            }],
        )?;
        Ok(())
    }

    #[test]
    fn keeps_the_chosen_icon_and_shows_it_as_a_data_url() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
            let logo = StoredIcon::Image {
                bytes: png.clone(),
                mime: "image/png",
                source: Some("telkom.co.id".to_owned()),
            };
            let created = insert(
                conn,
                &Look {
                    name: "Telkom",
                    hue: "rose",
                    icon: &logo,
                },
            )?;
            assert_eq!(
                created.icon,
                crate::workspace::models::WorkspaceIcon::Image {
                    data_url: icon::data_url("image/png", &png),
                    source: Some("telkom.co.id".to_owned()),
                }
            );
            let emoji = update(
                conn,
                &created.id,
                &Look {
                    name: "Telkom",
                    hue: "rose",
                    icon: &StoredIcon::Emoji("🛒"),
                },
            )?;
            assert_eq!(
                emoji.icon,
                crate::workspace::models::WorkspaceIcon::Emoji {
                    emoji: "🛒".to_owned()
                }
            );
            let image_left: Option<Vec<u8>> = conn.query_row(
                "SELECT icon_image FROM workspaces WHERE id = ?1",
                params![created.id],
                |row| row.get(0),
            )?;
            assert!(image_left.is_none(), "the old logo is not kept");
            Ok(())
        })
        .expect("icons");
    }

    #[test]
    fn a_new_database_opens_my_workspace() {
        let db = Database::in_memory().expect("database");
        let list = db.with(list).expect("list");
        assert_eq!(names_of(&list), ["My workspace"]);
        assert_eq!(list.current_id, list.workspaces[0].id);
    }

    #[test]
    fn each_workspace_lists_only_what_was_made_in_it() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let first = current(conn)?;
            collections::insert_collection(conn, "Shop")?;
            add_flow(conn)?;
            environments::insert(conn, "Staging")?;

            let other = insert_named(conn, "Client demo")?;
            set_current(conn, &other.id)?;
            assert!(collections::list_collections(conn)?.is_empty());
            let envs = environments::list(conn)?.environments;
            assert_eq!(envs.len(), 1, "only its own Globals");
            assert!(envs[0].is_global);
            collections::insert_collection(conn, "Demo")?;

            let counts = list(conn)?;
            let first_row = &counts.workspaces[0];
            assert_eq!(
                (
                    first_row.collection_count,
                    first_row.environment_count,
                    first_row.flow_count
                ),
                (1, 1, 1)
            );
            assert_eq!(counts.workspaces[1].collection_count, 1);

            set_current(conn, &first)?;
            let shown: Vec<String> = collections::list_collections(conn)?
                .into_iter()
                .map(|c| c.name)
                .collect();
            assert_eq!(shown, ["Shop"]);
            Ok(())
        })
        .expect("workspaces");
    }

    #[test]
    fn globals_come_from_the_collections_own_workspace() {
        let db = Database::in_memory().expect("database");
        let vault = Vault::new(Box::new(FixedKey([4; 32])));
        db.with(|conn| {
            set_global(conn, &vault, "first.test")?;
            let shop = collections::insert_collection(conn, "Shop")?;
            let other = insert_named(conn, "Other")?;
            set_current(conn, &other.id)?;
            set_global(conn, &vault, "other.test")?;
            let demo = collections::insert_collection(conn, "Demo")?;

            let names = std::iter::once("host".to_owned()).collect();
            let host_in = |collection: &str| -> Result<Option<String>, AppError> {
                Ok(
                    environments::lookup_values(conn, &vault, &names, Some(collection), true)?
                        .remove("host"),
                )
            };
            assert_eq!(host_in(&shop.id)?.as_deref(), Some("first.test"));
            assert_eq!(host_in(&demo.id)?.as_deref(), Some("other.test"));
            Ok(())
        })
        .expect("globals");
    }

    #[test]
    fn only_an_empty_workspace_that_is_not_the_last_can_be_deleted() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let first = current(conn)?;
            assert!(matches!(
                delete(conn, &first),
                Err(AppError::InvalidInput(message)) if message.contains("at least one")
            ));
            let other = insert_named(conn, "Other")?;
            set_current(conn, &other.id)?;
            add_flow(conn)?;
            assert!(matches!(
                delete(conn, &other.id),
                Err(AppError::InvalidInput(message)) if message.contains("flows first")
            ));
            conn.execute("DELETE FROM flows", [])?;
            environments::insert(conn, "Staging")?;
            delete(conn, &other.id)?;
            let left: i64 = conn.query_row(
                "SELECT COUNT(*) FROM environments WHERE workspace_id = ?1",
                params![other.id],
                |row| row.get(0),
            )?;
            assert_eq!(left, 0, "its environments and Globals went with it");
            assert_eq!(current(conn)?, first, "the first workspace opens again");
            Ok(())
        })
        .expect("delete");
    }

    #[test]
    fn a_moved_collection_takes_its_own_environments_but_not_this_devices() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let first = current(conn)?;
            let shop = collections::insert_collection(conn, "Shop")?;
            let device = environments::insert(conn, "Local")?;
            let owned = environments::insert(conn, "Shop staging")?;
            environments::set_owner(conn, &owned.id, Some(&shop.id))?;
            environments::set_active(conn, &shop.id, Some(&device.id))?;

            let other = insert_named(conn, "Other")?;
            move_collection(conn, &shop.id, &other.id)?;
            let active = environments::list(conn)?.active_ids;
            assert!(
                !active.contains_key(&shop.id),
                "this device's one stays behind"
            );
            let in_first: Vec<String> = environments::list(conn)?
                .environments
                .into_iter()
                .map(|e| e.name)
                .collect();
            assert_eq!(in_first, ["Globals", "Local"]);

            set_current(conn, &other.id)?;
            environments::set_active(conn, &shop.id, Some(&owned.id))?;
            assert!(matches!(
                environments::set_active(conn, &shop.id, Some(&device.id)),
                Err(AppError::InvalidInput(message)) if message.contains("another workspace")
            ));
            let in_other: Vec<String> = environments::list(conn)?
                .environments
                .into_iter()
                .map(|e| e.name)
                .collect();
            assert_eq!(in_other, ["Globals", "Shop staging"]);
            assert!(
                collections::list_collections(conn)?
                    .iter()
                    .any(|c| c.id == shop.id)
            );
            assert_ne!(first, other.id);
            Ok(())
        })
        .expect("move");
    }
}
