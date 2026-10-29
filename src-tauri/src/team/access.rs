//! Restricted variables: values only the members of an access group can read. Each device has a
//! key pair whose private half stays in the OS credential store. A group's key is wrapped for
//! every member's public key with HPKE (RFC 9180), and values are encrypted with keys derived
//! from it, so the repository only ever holds ciphertext.

use std::collections::BTreeMap;
use std::sync::Mutex;

use chacha20poly1305::aead::{Aead, Generate, Key, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use hpke::aead::ChaCha20Poly1305 as WrapCipher;
use hpke::kdf::HkdfSha256;
use hpke::kem::X25519HkdfSha256;
use hpke::{Deserializable, Kem, OpModeR, OpModeS, Serializable};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::git::check_author;
use super::models::Author;
use crate::encoding::{base64_decode, base64_encode};
use crate::environment::vault::KeySource;
use crate::error::AppError;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::storage::validate::{check_id, clean_name};

pub const GROUP_PREFIX: &str = "access/";
pub const REQUEST_PREFIX: &str = "access/requests/";
const JSON_SUFFIX: &str = ".json";
pub const MAX_GROUPS: usize = 20;
pub const MAX_MEMBERS: usize = 100;
pub const MAX_REQUESTS: usize = 200;
const MAX_KEY_VERSION: u32 = 10_000;
pub const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const STAMP_BYTES: usize = 32;
/// An X25519 encapsulated key followed by the sealed group key.
const WRAPPED_BYTES: usize = 32 + KEY_BYTES + TAG_BYTES;
/// Crockford's base32: no I, L, O or U, so a fingerprint read aloud is not misheard.
const FINGERPRINT_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// 80 bits of the public key's hash, in four groups of four.
const FINGERPRINT_CHARS: usize = 16;

pub type GroupKey = [u8; KEY_BYTES];
type PrivateKey = <X25519HkdfSha256 as Kem>::PrivateKey;
type PublicKey = <X25519HkdfSha256 as Kem>::PublicKey;
type EncappedKey = <X25519HkdfSha256 as Kem>::EncappedKey;

/// `access/<id>.json`: the members of a group, the group key wrapped for each of them, and a
/// stamp only someone holding the key can make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupFile {
    pub id: String,
    pub name: String,
    pub key_version: u32,
    pub members: Vec<Member>,
    /// Every earlier key, encrypted with the current one. A member checks that a new key came
    /// from someone who held the one they already trust.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub previous_keys: String,
    pub stamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Member {
    pub name: String,
    pub email: String,
    pub public_key: String,
    pub fingerprint: String,
    /// The group key, readable only with this member's private key.
    pub key: String,
    pub added_by: String,
}

/// `access/requests/<group id>.<fingerprint>.json`: someone asking a group's members for access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessRequest {
    pub group_id: String,
    pub name: String,
    pub email: String,
    pub public_key: String,
    pub fingerprint: String,
}

/// A restricted value as the repository holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SealedValue {
    pub version: u32,
    pub nonce: String,
    pub data: String,
}

/// Where a restricted value lives. It is bound to its ciphertext, so a value copied to another
/// variable, set or group does not open.
#[derive(Debug, Clone, Copy)]
pub struct ValuePlace<'a> {
    pub collection_id: &'a str,
    /// `collection` for the collection's own variables, else the environment id.
    pub set: &'a str,
    pub key: &'a str,
    pub group_id: &'a str,
}

pub fn group_path(id: &str) -> String {
    format!("{GROUP_PREFIX}{id}{JSON_SUFFIX}")
}

/// The group id of an `access/<id>.json` path.
pub fn group_id_of(path: &str) -> Option<&str> {
    let id = path.strip_prefix(GROUP_PREFIX)?.strip_suffix(JSON_SUFFIX)?;
    check_id(id).is_ok().then_some(id)
}

pub fn request_path(group_id: &str, fingerprint: &str) -> String {
    format!(
        "{REQUEST_PREFIX}{group_id}.{}{JSON_SUFFIX}",
        fingerprint.replace('-', "")
    )
}

/// The group id and fingerprint of an `access/requests/<group id>.<fingerprint>.json` path.
pub fn request_of(path: &str) -> Option<(&str, String)> {
    let name = path
        .strip_prefix(REQUEST_PREFIX)?
        .strip_suffix(JSON_SUFFIX)?;
    let (group_id, compact) = name.rsplit_once('.')?;
    let valid = check_id(group_id).is_ok()
        && compact.len() == FINGERPRINT_CHARS
        && compact
            .bytes()
            .all(|byte| FINGERPRINT_ALPHABET.contains(&byte));
    valid.then(|| (group_id, dashed(compact)))
}

fn dashed(compact: &str) -> String {
    let mut text = String::with_capacity(compact.len() + compact.len() / 4);
    for (index, character) in compact.chars().enumerate() {
        if index > 0 && index % 4 == 0 {
            text.push('-');
        }
        text.push(character);
    }
    text
}

/// What people compare to be sure a request comes from the device they think it does.
pub fn fingerprint(public_key: &[u8]) -> String {
    let digest = Sha256::digest(public_key);
    let mut bits: u128 = 0;
    for byte in digest.iter().take(FINGERPRINT_CHARS * 5 / 8) {
        bits = (bits << 8) | u128::from(*byte);
    }
    let compact: String = (0..FINGERPRINT_CHARS)
        .map(|index| {
            let shift = 5 * (FINGERPRINT_CHARS - 1 - index);
            let value = usize::try_from((bits >> shift) & 0x1f).unwrap_or(0);
            char::from(FINGERPRINT_ALPHABET[value])
        })
        .collect();
    dashed(&compact)
}

pub fn new_group_key() -> GroupKey {
    let mut key = [0; KEY_BYTES];
    key.copy_from_slice(&Key::<ChaCha20Poly1305>::generate());
    key
}

fn to_key(bytes: &[u8]) -> Option<GroupKey> {
    GroupKey::try_from(bytes).ok()
}

/// Joins a label and the parts that bind a key or value to its place; ids and names never
/// contain a zero byte.
fn context(label: &str, parts: &[&str]) -> Vec<u8> {
    let mut bytes = label.as_bytes().to_vec();
    for part in parts {
        bytes.push(0);
        bytes.extend_from_slice(part.as_bytes());
    }
    bytes
}

fn wrap_context(collection_id: &str, group_id: &str, version: u32) -> Vec<u8> {
    context(
        "codestamp group key v1",
        &[collection_id, group_id, &version.to_string()],
    )
}

/// The group key encrypted for one member's public key.
pub fn wrap_key(
    key: &GroupKey,
    public_key: &str,
    collection_id: &str,
    group_id: &str,
    version: u32,
) -> Result<String, AppError> {
    let recipient = base64_decode(public_key)
        .and_then(|bytes| PublicKey::from_bytes(&bytes).ok())
        .ok_or_else(|| AppError::invalid("This device key cannot be used"))?;
    let (encapped, sealed) = hpke::single_shot_seal::<WrapCipher, HkdfSha256, X25519HkdfSha256>(
        &OpModeS::Base,
        &recipient,
        &wrap_context(collection_id, group_id, version),
        key,
        &[],
    )
    .map_err(|_| AppError::SecretStore)?;
    let mut blob = encapped.to_bytes().to_vec();
    blob.extend_from_slice(&sealed);
    Ok(base64_encode(&blob))
}

fn subkey(key: &GroupKey, purpose: &str) -> Result<GroupKey, AppError> {
    let mut output = [0; KEY_BYTES];
    Hkdf::<Sha256>::new(None, key)
        .expand(context("codestamp", &[purpose]).as_slice(), &mut output)
        .map_err(|_| AppError::SecretStore)?;
    Ok(output)
}

fn seal(key: &GroupKey, aad: &[u8], plaintext: &[u8]) -> Result<(String, String), AppError> {
    let cipher = ChaCha20Poly1305::new(&Key::<ChaCha20Poly1305>::from(*key));
    let nonce = Nonce::generate();
    let sealed = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| AppError::SecretStore)?;
    Ok((base64_encode(&nonce), base64_encode(&sealed)))
}

fn open(key: &GroupKey, aad: &[u8], nonce: &str, data: &str) -> Option<Vec<u8>> {
    let nonce = Nonce::try_from(base64_decode(nonce)?.as_slice()).ok()?;
    let data = base64_decode(data)?;
    ChaCha20Poly1305::new(&Key::<ChaCha20Poly1305>::from(*key))
        .decrypt(&nonce, Payload { msg: &data, aad })
        .ok()
}

fn value_context(place: ValuePlace<'_>, version: u32) -> Vec<u8> {
    context(
        "codestamp value v1",
        &[
            place.collection_id,
            place.set,
            place.key,
            place.group_id,
            &version.to_string(),
        ],
    )
}

pub fn seal_value(
    key: &GroupKey,
    version: u32,
    place: ValuePlace<'_>,
    plaintext: &str,
) -> Result<SealedValue, AppError> {
    let (nonce, data) = seal(
        &subkey(key, "value")?,
        &value_context(place, version),
        plaintext.as_bytes(),
    )?;
    Ok(SealedValue {
        version,
        nonce,
        data,
    })
}

/// The plaintext, or `None` when the key is wrong or the value was changed or moved.
pub fn open_value(key: &GroupKey, sealed: &SealedValue, place: ValuePlace<'_>) -> Option<String> {
    let plain = open(
        &subkey(key, "value").ok()?,
        &value_context(place, sealed.version),
        &sealed.nonce,
        &sealed.data,
    )?;
    String::from_utf8(plain).ok()
}

fn history_context(collection_id: &str, group_id: &str, version: u32) -> Vec<u8> {
    context(
        "codestamp previous keys v1",
        &[collection_id, group_id, &version.to_string()],
    )
}

/// Keys 1 to `version - 1`, in order, encrypted with the key of `version`.
pub fn seal_previous(
    previous: &[GroupKey],
    current: &GroupKey,
    collection_id: &str,
    group_id: &str,
    version: u32,
) -> Result<String, AppError> {
    if previous.is_empty() {
        return Ok(String::new());
    }
    let plain: Vec<u8> = previous.iter().flatten().copied().collect();
    let (nonce, data) = seal(
        &subkey(current, "previous keys")?,
        &history_context(collection_id, group_id, version),
        &plain,
    )?;
    Ok(format!("{nonce}.{data}"))
}

/// The earlier keys of a group, by version, or `None` when they do not open with `current`.
pub fn open_previous(
    group: &GroupFile,
    collection_id: &str,
    current: &GroupKey,
) -> Option<BTreeMap<u32, GroupKey>> {
    let mut keys = BTreeMap::new();
    if group.key_version == 1 {
        return group.previous_keys.is_empty().then_some(keys);
    }
    let (nonce, data) = group.previous_keys.split_once('.')?;
    let plain = open(
        &subkey(current, "previous keys").ok()?,
        &history_context(collection_id, &group.id, group.key_version),
        nonce,
        data,
    )?;
    if plain.len() != KEY_BYTES * usize::try_from(group.key_version.checked_sub(1)?).ok()? {
        return None;
    }
    for (version, chunk) in (1..).zip(plain.chunks(KEY_BYTES)) {
        keys.insert(version, to_key(chunk)?);
    }
    Some(keys)
}

/// Everything in the file but the stamp, bound to its collection.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stamped<'a> {
    collection_id: &'a str,
    id: &'a str,
    name: &'a str,
    key_version: u32,
    members: &'a [Member],
    previous_keys: &'a str,
}

fn stamp_mac(
    group: &GroupFile,
    collection_id: &str,
    key: &GroupKey,
) -> Result<Hmac<Sha256>, AppError> {
    let content = serde_json::to_vec(&Stamped {
        collection_id,
        id: &group.id,
        name: &group.name,
        key_version: group.key_version,
        members: &group.members,
        previous_keys: &group.previous_keys,
    })
    .map_err(|_| AppError::Storage)?;
    let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(&subkey(key, "stamp")?)
        .map_err(|_| AppError::SecretStore)?;
    mac.update(&content);
    Ok(mac)
}

/// Sets the stamp after a change; only a holder of the group key can.
pub fn restamp(group: &mut GroupFile, collection_id: &str, key: &GroupKey) -> Result<(), AppError> {
    group.stamp = base64_encode(
        &stamp_mac(group, collection_id, key)?
            .finalize()
            .into_bytes(),
    );
    Ok(())
}

/// Whether the file was last changed by someone holding `key`. Compared in constant time.
pub fn stamp_holds(group: &GroupFile, collection_id: &str, key: &GroupKey) -> bool {
    let Some(expected) = base64_decode(&group.stamp) else {
        return false;
    };
    stamp_mac(group, collection_id, key).is_ok_and(|mac| mac.verify_slice(&expected).is_ok())
}

/// This device's key pair for access groups. The private key is read from the credential store
/// on first use only.
pub struct DeviceKey {
    source: Box<dyn KeySource>,
    secret: Mutex<Option<PrivateKey>>,
}

impl DeviceKey {
    pub fn new(source: Box<dyn KeySource>) -> Self {
        Self {
            source,
            secret: Mutex::new(None),
        }
    }

    fn with_secret<T>(&self, operation: impl FnOnce(&PrivateKey) -> T) -> Result<T, AppError> {
        let mut slot = self.secret.lock().map_err(|_| AppError::SecretStore)?;
        if slot.is_none() {
            let bytes = self.source.load_or_create()?;
            *slot = Some(PrivateKey::from_bytes(&bytes).map_err(|_| AppError::SecretStore)?);
        }
        slot.as_ref().map(operation).ok_or(AppError::SecretStore)
    }

    fn public_bytes(&self) -> Result<Vec<u8>, AppError> {
        self.with_secret(|secret| X25519HkdfSha256::sk_to_pk(secret).to_bytes().to_vec())
    }

    pub fn public_key(&self) -> Result<String, AppError> {
        Ok(base64_encode(&self.public_bytes()?))
    }

    pub fn fingerprint(&self) -> Result<String, AppError> {
        Ok(fingerprint(&self.public_bytes()?))
    }

    /// The group key wrapped for this device, or `None` when it is not wrapped for it.
    pub fn unwrap_key(
        &self,
        wrapped: &str,
        collection_id: &str,
        group_id: &str,
        version: u32,
    ) -> Result<Option<GroupKey>, AppError> {
        let Some(blob) = base64_decode(wrapped).filter(|blob| blob.len() == WRAPPED_BYTES) else {
            return Ok(None);
        };
        let (encapped, sealed) = blob.split_at(WRAPPED_BYTES - KEY_BYTES - TAG_BYTES);
        let Ok(encapped) = EncappedKey::from_bytes(encapped) else {
            return Ok(None);
        };
        self.with_secret(|secret| {
            hpke::single_shot_open::<WrapCipher, HkdfSha256, X25519HkdfSha256>(
                &OpModeR::Base,
                secret,
                &encapped,
                &wrap_context(collection_id, group_id, version),
                sealed,
                &[],
            )
            .ok()
            .and_then(|plain| to_key(&plain))
        })
    }

    /// This device as a group member, with the key wrapped for it.
    pub fn member(
        &self,
        author: &Author,
        added_by: &str,
        key: &GroupKey,
        collection_id: &str,
        group: (&str, u32),
    ) -> Result<Member, AppError> {
        let public_key = self.public_key()?;
        Ok(Member {
            name: author.name.clone(),
            email: author.email.clone(),
            key: wrap_key(key, &public_key, collection_id, group.0, group.1)?,
            fingerprint: self.fingerprint()?,
            public_key,
            added_by: added_by.to_owned(),
        })
    }
}

/// A group file from the repository, checked like any other shared file. The stamp is checked
/// later, against the keys this device trusts.
pub fn check_group(path: &str, group: GroupFile) -> Result<GroupFile, AppError> {
    let invalid = || invalid_file(path);
    if group_id_of(path) != Some(group.id.as_str())
        || group.key_version == 0
        || group.key_version > MAX_KEY_VERSION
        || group.members.is_empty()
        || group.members.len() > MAX_MEMBERS
        || base64_decode(&group.stamp).is_none_or(|stamp| stamp.len() != STAMP_BYTES)
        || (group.key_version == 1) != group.previous_keys.is_empty()
    {
        return Err(invalid());
    }
    let mut seen = Vec::with_capacity(group.members.len());
    for member in &group.members {
        check_person(
            &member.name,
            &member.email,
            &member.public_key,
            &member.fingerprint,
        )
        .map_err(|_| invalid())?;
        let wrapped_fits =
            base64_decode(&member.key).is_some_and(|blob| blob.len() == WRAPPED_BYTES);
        let added_by_fits = !member.added_by.trim().is_empty()
            && member.added_by.chars().count() <= MAX_PERSON_CHARS
            && !member.added_by.chars().any(char::is_control);
        if !wrapped_fits || !added_by_fits || seen.contains(&&member.fingerprint) {
            return Err(invalid());
        }
        seen.push(&member.fingerprint);
    }
    Ok(GroupFile {
        name: clean_name(&group.name).map_err(|_| invalid())?,
        ..group
    })
}

const MAX_PERSON_CHARS: usize = 100;

fn check_person(name: &str, email: &str, public_key: &str, claimed: &str) -> Result<(), AppError> {
    check_author(&Author {
        name: name.to_owned(),
        email: email.to_owned(),
    })?;
    let key = base64_decode(public_key)
        .filter(|key| key.len() == KEY_BYTES)
        .ok_or_else(|| AppError::invalid("Unknown key"))?;
    if fingerprint(&key) != claimed {
        return Err(AppError::invalid("Unknown key"));
    }
    Ok(())
}

pub fn check_request(path: &str, request: AccessRequest) -> Result<AccessRequest, AppError> {
    let invalid = || invalid_file(path);
    let (group_id, fingerprint) = request_of(path).ok_or_else(invalid)?;
    if group_id != request.group_id || fingerprint != request.fingerprint {
        return Err(invalid());
    }
    check_person(
        &request.name,
        &request.email,
        &request.public_key,
        &request.fingerprint,
    )
    .map_err(|_| invalid())?;
    Ok(request)
}

/// A sealed value from the repository: sizes only, the content is checked when it is opened.
pub fn sealed_fits(sealed: &SealedValue) -> bool {
    sealed.version > 0
        && sealed.version <= MAX_KEY_VERSION
        && base64_decode(&sealed.nonce).is_some_and(|nonce| nonce.len() == NONCE_BYTES)
        && base64_decode(&sealed.data)
            .is_some_and(|data| data.len() <= MAX_KEY_VALUE_LENGTH + TAG_BYTES)
}

fn invalid_file(path: &str) -> AppError {
    AppError::InvalidInput(format!(
        "The repository has a file Codestamp cannot use: {path}"
    ))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::environment::vault::FixedKey;
    use crate::storage::db::new_id;

    pub fn device(seed: u8) -> DeviceKey {
        DeviceKey::new(Box::new(FixedKey([seed; 32])))
    }

    fn author(name: &str) -> Author {
        Author {
            name: name.to_owned(),
            email: format!("{}@example.com", name.to_lowercase()),
        }
    }

    fn group(collection_id: &str, key: &GroupKey, devices: &[(&DeviceKey, &str)]) -> GroupFile {
        let id = new_id();
        let mut group = GroupFile {
            id: id.clone(),
            name: "Prod".to_owned(),
            key_version: 1,
            members: devices
                .iter()
                .map(|(device, name)| {
                    device
                        .member(&author(name), "Ana", key, collection_id, (&id, 1))
                        .expect("member")
                })
                .collect(),
            previous_keys: String::new(),
            stamp: String::new(),
        };
        restamp(&mut group, collection_id, key).expect("stamp");
        group
    }

    #[test]
    fn wraps_the_group_key_for_each_member_only() {
        let (ana, budi, eve) = (device(1), device(2), device(3));
        let collection = new_id();
        let key = new_group_key();
        let group = group(&collection, &key, &[(&ana, "Ana"), (&budi, "Budi")]);
        for (device, member) in [(&ana, &group.members[0]), (&budi, &group.members[1])] {
            let opened = device
                .unwrap_key(&member.key, &collection, &group.id, 1)
                .expect("unwrap");
            assert_eq!(opened, Some(key));
        }
        let stolen = eve
            .unwrap_key(&group.members[0].key, &collection, &group.id, 1)
            .expect("unwrap");
        assert_eq!(stolen, None, "another device cannot open a member's copy");
        let moved = ana
            .unwrap_key(&group.members[0].key, &new_id(), &group.id, 1)
            .expect("unwrap");
        assert_eq!(
            moved, None,
            "a wrapped key copied to another collection does not open"
        );
        assert!(check_group(&group_path(&group.id), group).is_ok_and(|g| g.members.len() == 2));
    }

    #[test]
    fn the_stamp_catches_changes_made_without_the_key() {
        let ana = device(1);
        let eve = device(3);
        let collection = new_id();
        let key = new_group_key();
        let mut group = group(&collection, &key, &[(&ana, "Ana")]);
        assert!(stamp_holds(&group, &collection, &key));
        assert!(
            !stamp_holds(&group, &new_id(), &key),
            "bound to its collection"
        );

        // Someone who can push but holds no key adds their own device.
        let fake_key = new_group_key();
        group.members.push(
            eve.member(
                &author("Eve"),
                "Eve",
                &fake_key,
                &collection,
                (&group.id, 1),
            )
            .expect("member"),
        );
        assert!(!stamp_holds(&group, &collection, &key));
        restamp(&mut group, &collection, &fake_key).expect("stamp");
        assert!(
            !stamp_holds(&group, &collection, &key),
            "a stamp needs the real key"
        );
    }

    #[test]
    fn values_open_only_in_their_place_with_their_key() {
        let collection = new_id();
        let group_id = new_id();
        let key = new_group_key();
        let place = ValuePlace {
            collection_id: &collection,
            set: "collection",
            key: "token",
            group_id: &group_id,
        };
        let sealed = seal_value(&key, 2, place, "s3cr3t-token").expect("seal");
        assert!(sealed_fits(&sealed));
        assert!(!sealed.data.contains("s3cr3t"));
        assert_eq!(
            open_value(&key, &sealed, place).as_deref(),
            Some("s3cr3t-token")
        );
        let elsewhere = ValuePlace {
            key: "apiKey",
            ..place
        };
        assert_eq!(open_value(&key, &sealed, elsewhere), None);
        assert_eq!(open_value(&new_group_key(), &sealed, place), None);
        let older = SealedValue {
            version: 1,
            ..sealed.clone()
        };
        assert_eq!(
            open_value(&key, &older, place),
            None,
            "the version is bound too"
        );
        let mut tampered = sealed;
        tampered.data = base64_encode(&[0; 20]);
        assert_eq!(open_value(&key, &tampered, place), None);
    }

    #[test]
    fn earlier_keys_travel_under_the_current_one() {
        let collection = new_id();
        let ana = device(1);
        let (first, second, third) = (new_group_key(), new_group_key(), new_group_key());
        let mut group = group(&collection, &first, &[(&ana, "Ana")]);
        group.key_version = 3;
        group.previous_keys =
            seal_previous(&[first, second], &third, &collection, &group.id, 3).expect("seal");
        let keys = open_previous(&group, &collection, &third).expect("open");
        assert_eq!(keys.get(&1), Some(&first));
        assert_eq!(keys.get(&2), Some(&second));
        assert_eq!(open_previous(&group, &collection, &second), None);
        group.key_version = 2;
        assert_eq!(
            open_previous(&group, &collection, &third),
            None,
            "bound to its version"
        );
    }

    #[test]
    fn fingerprints_and_paths_round_trip() {
        let ana = device(1);
        let print = ana.fingerprint().expect("fingerprint");
        assert_eq!(print.len(), 19);
        assert_eq!(print, ana.fingerprint().expect("again"));
        assert_ne!(print, device(2).fingerprint().expect("other"));
        let group_id = new_id();
        let path = request_path(&group_id, &print);
        assert_eq!(request_of(&path), Some((group_id.as_str(), print.clone())));
        assert_eq!(group_id_of(&group_path(&group_id)), Some(group_id.as_str()));
        assert_eq!(group_id_of("access/../x.json"), None);
        assert_eq!(request_of("access/requests/x.ABCD.json"), None);
        let request = AccessRequest {
            group_id: group_id.clone(),
            name: "Budi".to_owned(),
            email: "budi@example.com".to_owned(),
            public_key: device(2).public_key().expect("key"),
            fingerprint: print,
        };
        assert!(
            check_request(&path, request).is_err(),
            "a fingerprint that is not the key's own is refused"
        );
    }
}
