//! Access groups as this device keeps them: the files the next sync shares, the keys it has
//! checked, and what members do with them. A new version of a group only counts when it comes
//! from someone who held the key this device already trusts; anything else locks the group here.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::access::{
    self, AccessRequest, DeviceKey, GroupFile, GroupKey, MAX_GROUPS, Member, SealedValue,
    ValuePlace,
};
use super::files::{Snapshot, VariableEntry};
use super::models::Author;
use super::repo;
use crate::encoding::{base64_decode, base64_encode};
use crate::environment::models::VariableInput;
use crate::environment::repo as environments;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::storage::validate::{check_id, clean_name};

/// The set name values of the collection's own variables are bound to.
pub const COLLECTION_SET: &str = "collection";

const UNREADABLE: &str = "the key given to this device does not open";
const UNSTAMPED: &str = "it was changed by someone without the group key";
const OLDER: &str = "it was set back to an older key";
const NOT_FROM_MEMBER: &str = "its new key did not come from a member";
const VALUES_CLOSED: &str = "its values do not open with the key given to this device";

/// One group as stored here.
struct Stored {
    file: GroupFile,
    /// The newest version this device checked with a key it holds.
    trusted: Option<GroupFile>,
    keys: BTreeMap<u32, GroupKey>,
    access: bool,
    problem: Option<String>,
}

impl Stored {
    fn first_seen(file: GroupFile) -> Self {
        Self {
            file,
            trusted: None,
            keys: BTreeMap::new(),
            access: false,
            problem: None,
        }
    }

    fn newest_known(&self) -> u32 {
        let trusted = self.trusted.as_ref().map_or(0, |file| file.key_version);
        let known = self.keys.keys().next_back().copied().unwrap_or(0);
        self.file.key_version.max(trusted).max(known)
    }
}

fn keys_owner(collection_id: &str, group_id: &str) -> String {
    format!("access-keys:{collection_id}:{group_id}")
}

fn to_text<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|_| AppError::Storage)
}

fn from_text<T: for<'a> Deserialize<'a>>(text: &str) -> Result<T, AppError> {
    serde_json::from_str(text).map_err(|_| AppError::Storage)
}

fn load(
    conn: &Connection,
    vault: &Vault,
    collection_id: &str,
) -> Result<BTreeMap<String, Stored>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, file, trusted_file, keys, access, problem FROM team_access_groups
         WHERE collection_id = ?1",
    )?;
    let rows = statement.query_map(params![collection_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<Vec<u8>>>(3)?,
            row.get::<_, bool>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;
    let mut groups = BTreeMap::new();
    for row in rows {
        let (id, file, trusted, keys, access, problem) = row?;
        let keys = match keys {
            Some(blob) => {
                let text = vault.decrypt(&keys_owner(collection_id, &id), &blob)?;
                let encoded: BTreeMap<u32, String> = from_text(&text)?;
                encoded
                    .into_iter()
                    .map(|(version, key)| {
                        base64_decode(&key)
                            .and_then(|bytes| GroupKey::try_from(bytes.as_slice()).ok())
                            .map(|key| (version, key))
                            .ok_or(AppError::Storage)
                    })
                    .collect::<Result<_, _>>()?
            }
            None => BTreeMap::new(),
        };
        groups.insert(
            id,
            Stored {
                file: from_text(&file)?,
                trusted: trusted.as_deref().map(from_text).transpose()?,
                keys,
                access,
                problem,
            },
        );
    }
    Ok(groups)
}

fn save(
    conn: &Connection,
    vault: &Vault,
    collection_id: &str,
    group: &Stored,
) -> Result<(), AppError> {
    let id = &group.file.id;
    let keys = if group.keys.is_empty() {
        None
    } else {
        let encoded: BTreeMap<u32, String> = group
            .keys
            .iter()
            .map(|(version, key)| (*version, base64_encode(key)))
            .collect();
        Some(vault.encrypt(&keys_owner(collection_id, id), &to_text(&encoded)?)?)
    };
    conn.execute(
        "INSERT INTO team_access_groups
             (collection_id, id, file, trusted_file, keys, access, problem)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(collection_id, id) DO UPDATE SET file = excluded.file,
             trusted_file = excluded.trusted_file, keys = excluded.keys,
             access = excluded.access, problem = excluded.problem",
        params![
            collection_id,
            id,
            to_text(&group.file)?,
            group.trusted.as_ref().map(to_text).transpose()?,
            keys,
            group.access,
            group.problem,
        ],
    )?;
    Ok(())
}

/// Groups by id and requests for access by path.
pub type AccessFiles = (BTreeMap<String, GroupFile>, BTreeMap<String, AccessRequest>);

/// The group and request files to share, as this device has them.
pub fn stored_files(conn: &Connection, collection_id: &str) -> Result<AccessFiles, AppError> {
    let mut statement =
        conn.prepare("SELECT id, file FROM team_access_groups WHERE collection_id = ?1")?;
    let groups = statement
        .query_map(params![collection_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| {
            let (id, file) = row?;
            Ok((id, from_text(&file)?))
        })
        .collect::<Result<_, AppError>>()?;
    let mut statement =
        conn.prepare("SELECT path, file FROM team_access_requests WHERE collection_id = ?1")?;
    let requests = statement
        .query_map(params![collection_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| {
            let (path, file) = row?;
            Ok((path, from_text(&file)?))
        })
        .collect::<Result<_, AppError>>()?;
    Ok((groups, requests))
}

/// Stored ciphertext of a set's restricted variables, by variable id.
pub fn sealed_values(
    conn: &Connection,
    set_id: &str,
) -> Result<BTreeMap<String, SealedValue>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, sealed FROM variables
         WHERE environment_id = ?1 AND restricted IS NOT NULL AND sealed IS NOT NULL",
    )?;
    let rows = statement.query_map(params![set_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut values = BTreeMap::new();
    for row in rows {
        let (id, text) = row?;
        // A value this version cannot read is left out and sealed again by a member.
        if let Ok(sealed) = serde_json::from_str(&text) {
            values.insert(id, sealed);
        }
    }
    Ok(values)
}

/// The variable sets of a collection that travel with it, with the name their values are
/// bound to.
fn shared_sets(conn: &Connection, collection_id: &str) -> Result<Vec<(String, String)>, AppError> {
    let mut sets = Vec::new();
    if let Some(set) = environments::find_collection_set(conn, collection_id)? {
        sets.push((set, COLLECTION_SET.to_owned()));
    }
    for (id, _) in environments::owned(conn, collection_id)? {
        sets.push((id.clone(), id));
    }
    Ok(sets)
}

/// The keys of the groups this device can use now, plus the groups that failed their checks.
pub struct Keyring {
    usable: BTreeMap<String, (u32, BTreeMap<u32, GroupKey>)>,
    refused: BTreeSet<String>,
}

impl Keyring {
    pub fn load(conn: &Connection, vault: &Vault, collection_id: &str) -> Result<Self, AppError> {
        let mut keyring = Self {
            usable: BTreeMap::new(),
            refused: BTreeSet::new(),
        };
        for (id, group) in load(conn, vault, collection_id)? {
            if group.problem.is_some() {
                keyring.refused.insert(id);
            } else if group.access && group.keys.contains_key(&group.file.key_version) {
                keyring
                    .usable
                    .insert(id, (group.file.key_version, group.keys));
            }
        }
        Ok(keyring)
    }

    fn current(&self, group_id: &str) -> Option<(u32, &GroupKey)> {
        let (version, keys) = self.usable.get(group_id)?;
        keys.get(version).map(|key| (*version, key))
    }

    fn open(&self, sealed: &SealedValue, place: ValuePlace<'_>) -> Option<String> {
        let (_, keys) = self.usable.get(place.group_id)?;
        access::open_value(keys.get(&sealed.version)?, sealed, place)
    }

    /// The value to store for a shared restricted variable: opened for members, the one stored
    /// here kept while it cannot be opened by a member (`None`), and empty for everyone else.
    pub fn value_for(
        &self,
        entry: &VariableEntry,
        place: ValuePlace<'_>,
        has_own: bool,
    ) -> Option<String> {
        if let Some(plain) = entry
            .sealed
            .as_ref()
            .and_then(|sealed| self.open(sealed, place))
        {
            return Some(plain);
        }
        let member =
            self.usable.contains_key(place.group_id) || self.refused.contains(place.group_id);
        if member && has_own {
            None
        } else {
            Some(String::new())
        }
    }
}

struct RestrictedRow {
    id: String,
    key: String,
    secret: Option<Vec<u8>>,
    group_id: String,
    sealed: Option<String>,
}

/// Encrypts restricted values changed here since the last sync, with each group's current key.
/// A value that still matches what is shared keeps its ciphertext, so nothing changes without
/// reason.
pub fn seal_values(conn: &Connection, vault: &Vault, collection_id: &str) -> Result<(), AppError> {
    let keyring = Keyring::load(conn, vault, collection_id)?;
    if keyring.usable.is_empty() {
        return Ok(());
    }
    for (set_id, set) in shared_sets(conn, collection_id)? {
        let mut statement = conn.prepare(
            "SELECT id, key, secret, restricted, sealed FROM variables
             WHERE environment_id = ?1 AND restricted IS NOT NULL",
        )?;
        let rows: Vec<RestrictedRow> = statement
            .query_map(params![set_id], |row| {
                Ok(RestrictedRow {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    secret: row.get(2)?,
                    group_id: row.get(3)?,
                    sealed: row.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        for RestrictedRow {
            id,
            key,
            secret,
            group_id,
            sealed,
        } in rows
        {
            let Some((version, current)) = keyring.current(&group_id) else {
                continue;
            };
            let place = ValuePlace {
                collection_id,
                set: &set,
                key: &key,
                group_id: &group_id,
            };
            let plain = match secret {
                Some(blob) => vault.decrypt(&id, &blob)?,
                None => String::new(),
            };
            let sealed: Option<SealedValue> =
                sealed.and_then(|text| serde_json::from_str(&text).ok());
            let fresh = sealed.as_ref().is_some_and(|sealed| {
                sealed.version == version
                    && access::open_value(current, sealed, place).as_deref() == Some(plain.as_str())
            });
            if !fresh {
                let sealed = access::seal_value(current, version, place, &plain)?;
                conn.execute(
                    "UPDATE variables SET sealed = ?2 WHERE id = ?1",
                    params![id, to_text(&sealed)?],
                )?;
            }
        }
    }
    Ok(())
}

/// Records the shared ciphertext of a set's restricted variables after they were stored.
pub fn store_sealed(
    conn: &Connection,
    set_id: &str,
    shared: &[VariableEntry],
) -> Result<(), AppError> {
    for entry in shared.iter().filter(|entry| entry.restricted.is_some()) {
        conn.execute(
            "UPDATE variables SET sealed = ?3 WHERE environment_id = ?1 AND key = ?2",
            params![
                set_id,
                entry.key,
                entry.sealed.as_ref().map(to_text).transpose()?
            ],
        )?;
    }
    Ok(())
}

enum Step {
    /// Checked with a key this device holds.
    Verified {
        keys: BTreeMap<u32, GroupKey>,
        member: bool,
    },
    /// A newer version this device is not in, so it cannot check it and has no access.
    Outside,
    Refused(&'static str),
}

fn step(
    device: &DeviceKey,
    fingerprint: &str,
    collection_id: &str,
    stored: &Stored,
    file: &GroupFile,
) -> Result<Step, AppError> {
    let mine = file
        .members
        .iter()
        .find(|member| member.fingerprint == fingerprint);
    let trusted = stored.trusted.as_ref().map(|trusted| trusted.key_version);
    if let Some(version) = trusted
        && file.key_version <= version
    {
        if file.key_version < version {
            return Ok(Step::Refused(OLDER));
        }
        let Some(key) = stored.keys.get(&version) else {
            return Ok(Step::Refused(UNREADABLE));
        };
        return Ok(if access::stamp_holds(file, collection_id, key) {
            Step::Verified {
                keys: stored.keys.clone(),
                member: mine.is_some(),
            }
        } else {
            Step::Refused(UNSTAMPED)
        });
    }
    let Some(mine) = mine else {
        return Ok(Step::Outside);
    };
    let Some(key) = device.unwrap_key(&mine.key, collection_id, &file.id, file.key_version)? else {
        return Ok(Step::Refused(UNREADABLE));
    };
    if !access::stamp_holds(file, collection_id, &key) {
        return Ok(Step::Refused(UNSTAMPED));
    }
    let Some(previous) = access::open_previous(file, collection_id, &key) else {
        return Ok(Step::Refused(UNSTAMPED));
    };
    // Every key this device knows from before must be in the chain, the trusted one included.
    let chained = stored
        .keys
        .iter()
        .filter(|(version, _)| **version < file.key_version)
        .all(|(version, known)| previous.get(version) == Some(known));
    if !chained || trusted.is_some_and(|version| !stored.keys.contains_key(&version)) {
        return Ok(Step::Refused(NOT_FROM_MEMBER));
    }
    let mut keys = stored.keys.clone();
    keys.extend(previous);
    keys.insert(file.key_version, key);
    Ok(Step::Verified { keys, member: true })
}

fn apply_step(stored: &mut Stored, file: &GroupFile, step: Step) {
    stored.file = file.clone();
    match step {
        Step::Verified { keys, member } => {
            stored.trusted = Some(file.clone());
            stored.keys = keys;
            stored.access = member;
            stored.problem = None;
        }
        Step::Outside => {
            stored.access = false;
            stored.problem = None;
        }
        Step::Refused(reason) => {
            stored.access = false;
            stored.problem = Some(reason.to_owned());
        }
    }
}

/// Every restricted value of a group in a snapshot, with its place.
fn values_of<'a>(
    snapshot: &'a Snapshot,
    group_id: &'a str,
) -> Vec<(ValuePlace<'a>, &'a SealedValue)> {
    let collection_id = snapshot.collection.id.as_str();
    let sets = std::iter::once((COLLECTION_SET, &snapshot.collection.variables)).chain(
        snapshot
            .environments
            .values()
            .map(|environment| (environment.id.as_str(), &environment.variables)),
    );
    sets.flat_map(|(set, variables)| {
        variables.iter().filter_map(move |entry| {
            let sealed = entry.sealed.as_ref()?;
            (entry.restricted.as_deref() == Some(group_id)).then_some((
                ValuePlace {
                    collection_id,
                    set,
                    key: &entry.key,
                    group_id,
                },
                sealed,
            ))
        })
    })
    .collect()
}

/// Takes in the access files that arrived. `history` holds the versions of each group committed
/// in between, oldest first: each is checked against the one before, so a key slipped in on
/// top of a newer one this device has not seen yet is caught.
pub fn absorb(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    after: &Snapshot,
    history: &BTreeMap<String, Vec<GroupFile>>,
) -> Result<(), AppError> {
    let collection_id = &after.collection.id;
    let mut stored = load(conn, vault, collection_id)?;
    let fingerprint = if after.groups.is_empty() {
        String::new()
    } else {
        device.fingerprint()?
    };
    for (id, file) in &after.groups {
        let versions = history
            .get(id)
            .into_iter()
            .flatten()
            .chain(std::iter::once(file));
        let mut group = stored
            .remove(id)
            .unwrap_or_else(|| Stored::first_seen(file.clone()));
        let trusted_before = group.trusted.is_some();
        // A group never checked here is checked even when its file did not change.
        let mut unchecked = !trusted_before;
        for version in versions {
            if *version == group.file && !unchecked {
                continue;
            }
            let outcome = step(device, &fingerprint, collection_id, &group, version)?;
            apply_step(&mut group, version, outcome);
            unchecked = false;
        }
        // Without an earlier key to check against, the values must open with the one given.
        if !trusted_before && group.access {
            let opens = values_of(after, id).into_iter().all(|(place, sealed)| {
                group
                    .keys
                    .get(&sealed.version)
                    .is_some_and(|key| access::open_value(key, sealed, place).is_some())
            });
            if !opens {
                group.trusted = None;
                group.keys.clear();
                apply_step(&mut group, file, Step::Refused(VALUES_CLOSED));
            }
        }
        save(conn, vault, collection_id, &group)?;
    }
    // A group missing from the repository stays here, so the next sync puts it back.
    conn.execute(
        "DELETE FROM team_access_requests WHERE collection_id = ?1",
        params![collection_id],
    )?;
    for (path, request) in &after.access_requests {
        conn.execute(
            "INSERT INTO team_access_requests (collection_id, path, file) VALUES (?1, ?2, ?3)",
            params![collection_id, path, to_text(request)?],
        )?;
    }
    Ok(())
}

/// The author written on this device's member entries and requests.
fn author_of(conn: &Connection, collection_id: &str) -> Result<Author, AppError> {
    let link = repo::link(conn, collection_id)?.ok_or_else(|| {
        AppError::invalid("Share the collection first; access groups travel with it")
    })?;
    match link.author {
        Some(author) => Ok(author),
        None => repo::author(conn)?
            .ok_or_else(|| AppError::invalid("Add your name and email for commits first")),
    }
}

fn group_of(
    conn: &Connection,
    vault: &Vault,
    collection_id: &str,
    group_id: &str,
) -> Result<Stored, AppError> {
    check_id(collection_id)?;
    check_id(group_id)?;
    load(conn, vault, collection_id)?
        .remove(group_id)
        .ok_or(AppError::NotFound("Access group"))
}

/// The group's current key, for members whose copy of the group passed its checks.
fn member_key(group: &Stored) -> Result<GroupKey, AppError> {
    let key = group.keys.get(&group.file.key_version).copied();
    match key {
        Some(key) if group.access && group.problem.is_none() => Ok(key),
        _ => Err(AppError::invalid(
            "Only members of this group can change who has access",
        )),
    }
}

pub fn create_group(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    collection_id: &str,
    name: &str,
) -> Result<(), AppError> {
    check_id(collection_id)?;
    let author = author_of(conn, collection_id)?;
    let name = clean_name(name)?;
    let existing = load(conn, vault, collection_id)?;
    if existing.len() >= MAX_GROUPS {
        return Err(AppError::invalid(
            "A collection can have up to 20 access groups",
        ));
    }
    if existing
        .values()
        .any(|group| group.file.name.to_lowercase() == name.to_lowercase())
    {
        return Err(AppError::invalid(
            "Another access group already has this name",
        ));
    }
    let id = crate::storage::db::new_id();
    let key = access::new_group_key();
    let mut file = GroupFile {
        members: vec![device.member(&author, &author.name, &key, collection_id, (&id, 1))?],
        id,
        name,
        key_version: 1,
        previous_keys: String::new(),
        stamp: String::new(),
    };
    access::restamp(&mut file, collection_id, &key)?;
    save(
        conn,
        vault,
        collection_id,
        &Stored {
            trusted: Some(file.clone()),
            file,
            keys: BTreeMap::from([(1, key)]),
            access: true,
            problem: None,
        },
    )
}

pub fn request_access(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    collection_id: &str,
    group_id: &str,
) -> Result<(), AppError> {
    let group = group_of(conn, vault, collection_id, group_id)?;
    let fingerprint = device.fingerprint()?;
    if group.access {
        return Err(AppError::invalid("This device already has access"));
    }
    let author = author_of(conn, collection_id)?;
    let request = AccessRequest {
        group_id: group_id.to_owned(),
        name: author.name,
        email: author.email,
        public_key: device.public_key()?,
        fingerprint: fingerprint.clone(),
    };
    conn.execute(
        "INSERT INTO team_access_requests (collection_id, path, file) VALUES (?1, ?2, ?3)
         ON CONFLICT(collection_id, path) DO UPDATE SET file = excluded.file",
        params![
            collection_id,
            access::request_path(group_id, &fingerprint),
            to_text(&request)?
        ],
    )?;
    Ok(())
}

fn pending_request(
    conn: &Connection,
    collection_id: &str,
    group_id: &str,
    fingerprint: &str,
) -> Result<Option<AccessRequest>, AppError> {
    let text: Option<String> = conn
        .query_row(
            "SELECT file FROM team_access_requests WHERE collection_id = ?1 AND path = ?2",
            params![collection_id, access::request_path(group_id, fingerprint)],
            |row| row.get(0),
        )
        .optional()?;
    text.as_deref().map(from_text).transpose()
}

fn drop_request(
    conn: &Connection,
    collection_id: &str,
    group_id: &str,
    fingerprint: &str,
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM team_access_requests WHERE collection_id = ?1 AND path = ?2",
        params![collection_id, access::request_path(group_id, fingerprint)],
    )?;
    Ok(())
}

/// Adds the device that asked, with the current key wrapped for it.
pub fn grant(
    conn: &Connection,
    vault: &Vault,
    collection_id: &str,
    group_id: &str,
    fingerprint: &str,
) -> Result<(), AppError> {
    let mut group = group_of(conn, vault, collection_id, group_id)?;
    let key = member_key(&group)?;
    let request = pending_request(conn, collection_id, group_id, fingerprint)?
        .ok_or(AppError::NotFound("Access request"))?;
    let request = access::check_request(&access::request_path(group_id, fingerprint), request)?;
    if group
        .file
        .members
        .iter()
        .any(|member| member.fingerprint == request.fingerprint)
    {
        return drop_request(conn, collection_id, group_id, fingerprint);
    }
    if group.file.members.len() >= access::MAX_MEMBERS {
        return Err(AppError::invalid("A group can have up to 100 members"));
    }
    let author = author_of(conn, collection_id)?;
    let version = group.file.key_version;
    group.file.members.push(Member {
        key: access::wrap_key(&key, &request.public_key, collection_id, group_id, version)?,
        name: request.name,
        email: request.email,
        public_key: request.public_key,
        fingerprint: request.fingerprint,
        added_by: author.name,
    });
    access::restamp(&mut group.file, collection_id, &key)?;
    group.trusted = Some(group.file.clone());
    save(conn, vault, collection_id, &group)?;
    drop_request(conn, collection_id, group_id, fingerprint)
}

/// Turns a request down: members do it for anyone, and a device can withdraw its own.
pub fn decline(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    collection_id: &str,
    group_id: &str,
    fingerprint: &str,
) -> Result<(), AppError> {
    let group = group_of(conn, vault, collection_id, group_id)?;
    if member_key(&group).is_err() && device.fingerprint()? != fingerprint {
        return Err(AppError::invalid(
            "Only members of this group can turn down a request",
        ));
    }
    drop_request(conn, collection_id, group_id, fingerprint)
}

/// A new key for the given members, with every earlier one sealed under it. Versions this
/// device never had get a random stand-in; no member can hold a real one it lacks.
fn rotate(
    collection_id: &str,
    group: &Stored,
    base: &GroupFile,
    members: Vec<Member>,
) -> Result<(GroupFile, GroupKey), AppError> {
    let version = group.newest_known() + 1;
    let key = access::new_group_key();
    let previous: Vec<GroupKey> = (1..version)
        .map(|old| {
            group
                .keys
                .get(&old)
                .copied()
                .unwrap_or_else(access::new_group_key)
        })
        .collect();
    let members = members
        .into_iter()
        .map(|member| {
            Ok(Member {
                key: access::wrap_key(&key, &member.public_key, collection_id, &base.id, version)?,
                ..member
            })
        })
        .collect::<Result<_, AppError>>()?;
    let mut file = GroupFile {
        id: base.id.clone(),
        name: base.name.clone(),
        key_version: version,
        members,
        previous_keys: access::seal_previous(&previous, &key, collection_id, &base.id, version)?,
        stamp: String::new(),
    };
    access::restamp(&mut file, collection_id, &key)?;
    Ok((file, key))
}

fn finish_rotation(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    collection_id: &str,
    mut group: Stored,
    file: GroupFile,
    key: GroupKey,
) -> Result<(), AppError> {
    let fingerprint = device.fingerprint()?;
    group.access = file
        .members
        .iter()
        .any(|member| member.fingerprint == fingerprint);
    group.keys.insert(file.key_version, key);
    group.trusted = Some(file.clone());
    group.file = file;
    group.problem = None;
    save(conn, vault, collection_id, &group)
}

/// Removes a member and gives the rest a new key; the next sync encrypts every value of the
/// group with it. What the removed member already saw stays known to them, so the values to
/// change at their source are returned.
pub fn remove(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    collection_id: &str,
    group_id: &str,
    fingerprint: &str,
) -> Result<Vec<RestrictedUse>, AppError> {
    let group = group_of(conn, vault, collection_id, group_id)?;
    member_key(&group)?;
    let members: Vec<Member> = group
        .file
        .members
        .iter()
        .filter(|member| member.fingerprint != fingerprint)
        .cloned()
        .collect();
    if members.len() == group.file.members.len() {
        return Err(AppError::NotFound("Member"));
    }
    if members.is_empty() {
        return Err(AppError::invalid("A group needs at least one member"));
    }
    let base = group.file.clone();
    let (file, key) = rotate(collection_id, &group, &base, members)?;
    finish_rotation(conn, vault, device, collection_id, group, file, key)?;
    Ok(restricted_uses(conn, collection_id)?
        .remove(group_id)
        .unwrap_or_default())
}

/// After a change this device refused: the last version it checked comes back with a new key,
/// so a key someone slipped in is never used.
pub fn repair(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    collection_id: &str,
    group_id: &str,
) -> Result<(), AppError> {
    let group = group_of(conn, vault, collection_id, group_id)?;
    let Some(trusted) = repairable(&group, &device.fingerprint()?).cloned() else {
        return Err(AppError::invalid(
            "Only a member whose device checked this group earlier can repair it",
        ));
    };
    let (file, key) = rotate(collection_id, &group, &trusted, trusted.members.clone())?;
    finish_rotation(conn, vault, device, collection_id, group, file, key)
}

/// The version to bring back after a refused change: the last one this device checked as a
/// member.
fn repairable<'a>(group: &'a Stored, fingerprint: &str) -> Option<&'a GroupFile> {
    group.trusted.as_ref().filter(|trusted| {
        group.problem.is_some()
            && group.keys.contains_key(&trusted.key_version)
            && trusted
                .members
                .iter()
                .any(|member| member.fingerprint == fingerprint)
    })
}

/// A restricted variable, for the list of values to change after someone leaves.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestrictedUse {
    pub key: String,
    /// `None` for the collection's own variables, else the environment's name.
    pub environment: Option<String>,
}

fn restricted_uses(
    conn: &Connection,
    collection_id: &str,
) -> Result<BTreeMap<String, Vec<RestrictedUse>>, AppError> {
    let mut sets: Vec<(String, Option<String>)> = Vec::new();
    if let Some(set) = environments::find_collection_set(conn, collection_id)? {
        sets.push((set, None));
    }
    for (id, name) in environments::owned(conn, collection_id)? {
        sets.push((id, Some(name)));
    }
    let mut uses: BTreeMap<String, Vec<RestrictedUse>> = BTreeMap::new();
    for (set_id, environment) in sets {
        for variable in environments::variables(conn, &set_id)? {
            if let Some(group) = variable.restricted {
                uses.entry(group).or_default().push(RestrictedUse {
                    key: variable.key,
                    environment: environment.clone(),
                });
            }
        }
    }
    Ok(uses)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessOverview {
    /// This device's fingerprint, for others to compare before granting. `None` until the
    /// collection has a group, so the device key is only made once it is needed.
    pub fingerprint: Option<String>,
    pub groups: Vec<GroupView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupView {
    pub id: String,
    pub name: String,
    pub key_version: u32,
    pub access: bool,
    /// Why this device does not trust the group's current version.
    pub problem: Option<String>,
    /// Whether this device can bring back the version it checked last.
    pub can_repair: bool,
    pub members: Vec<PersonView>,
    pub requests: Vec<PersonView>,
    pub variables: Vec<RestrictedUse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonView {
    pub name: String,
    pub email: String,
    pub fingerprint: String,
    /// Who added a member; `None` for requests.
    pub added_by: Option<String>,
    pub this_device: bool,
}

pub fn overview(
    conn: &Connection,
    vault: &Vault,
    device: &DeviceKey,
    collection_id: &str,
) -> Result<AccessOverview, AppError> {
    check_id(collection_id)?;
    let (stored, requests) = stored_files(conn, collection_id)?;
    if stored.is_empty() {
        return Ok(AccessOverview {
            fingerprint: None,
            groups: Vec::new(),
        });
    }
    let fingerprint = device.fingerprint()?;
    let mut uses = restricted_uses(conn, collection_id)?;
    let mut groups: Vec<GroupView> = load(conn, vault, collection_id)?
        .into_iter()
        .map(|(id, group)| {
            let can_repair = repairable(&group, &fingerprint).is_some();
            GroupView {
                members: group
                    .file
                    .members
                    .iter()
                    .map(|member| PersonView {
                        name: member.name.clone(),
                        email: member.email.clone(),
                        fingerprint: member.fingerprint.clone(),
                        added_by: Some(member.added_by.clone()),
                        this_device: member.fingerprint == fingerprint,
                    })
                    .collect(),
                requests: requests
                    .values()
                    .filter(|request| request.group_id == id)
                    .map(|request| PersonView {
                        name: request.name.clone(),
                        email: request.email.clone(),
                        fingerprint: request.fingerprint.clone(),
                        added_by: None,
                        this_device: request.fingerprint == fingerprint,
                    })
                    .collect(),
                variables: uses.remove(&id).unwrap_or_default(),
                name: group.file.name,
                key_version: group.file.key_version,
                access: group.access,
                problem: group.problem,
                can_repair,
                id,
            }
        })
        .collect();
    groups.sort_by_key(|group| group.name.to_lowercase());
    Ok(AccessOverview {
        fingerprint: Some(fingerprint),
        groups,
    })
}

/// What the variable editor may save: a locked value stays as it is, and only groups this device
/// can use are offered for new restrictions.
pub fn check_edit(
    conn: &Connection,
    set_id: &str,
    inputs: &[VariableInput],
) -> Result<(), AppError> {
    let existing = environments::variables(conn, set_id)?;
    for locked in existing.iter().filter(|variable| variable.locked) {
        let kept = inputs.iter().any(|input| {
            input.id.as_deref() == Some(locked.id.as_str())
                && input.key == locked.key
                && input.restricted == locked.restricted
                && input.value.is_none()
        });
        if !kept {
            return Err(AppError::InvalidInput(format!(
                "“{}” is restricted to a group this device is not in. Only its members can change or remove it",
                locked.key
            )));
        }
    }
    let owner: Option<String> = conn
        .query_row(
            "SELECT COALESCE(collection_id, owner_collection_id) FROM environments WHERE id = ?1",
            params![set_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    for input in inputs {
        let Some(group_id) = &input.restricted else {
            continue;
        };
        let unchanged = existing.iter().any(|variable| {
            Some(&variable.id) == input.id.as_ref()
                && variable.restricted.as_ref() == Some(group_id)
        });
        if unchanged {
            continue;
        }
        let usable = match &owner {
            Some(collection_id) => conn
                .query_row(
                    "SELECT 1 FROM team_access_groups
                     WHERE collection_id = ?1 AND id = ?2 AND access = 1 AND problem IS NULL",
                    params![collection_id, group_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some(),
            None => false,
        };
        if !usable {
            return Err(AppError::invalid(
                "Values can only be restricted to a group of this collection you are in",
            ));
        }
    }
    Ok(())
}
