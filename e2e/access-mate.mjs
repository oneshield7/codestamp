// A second Codestamp identity written from the format alone (RFC 9180 HPKE with DHKEM X25519,
// HKDF-SHA256 and ChaCha20-Poly1305), so the e2e also checks that another implementation can
// read and write access groups.
import crypto from 'node:crypto';

const ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
const KEM_SUITE = Buffer.concat([Buffer.from('KEM'), u16(0x20)]);
const HPKE_SUITE = Buffer.concat([Buffer.from('HPKE'), u16(0x20), u16(1), u16(3)]);

function u16(value) {
  return Buffer.from([value >> 8, value & 0xff]);
}

const b64url = (bytes) => Buffer.from(bytes).toString('base64url');

function publicKeyObject(raw) {
  return crypto.createPublicKey({
    key: { kty: 'OKP', crv: 'X25519', x: b64url(raw) },
    format: 'jwk',
  });
}

export function newIdentity() {
  const { privateKey, publicKey } = crypto.generateKeyPairSync('x25519');
  const raw = Buffer.from(publicKey.export({ format: 'jwk' }).x, 'base64url');
  return { privateKey, raw, publicKey: raw.toString('base64'), fingerprint: fingerprint(raw) };
}

export function fingerprint(raw) {
  const digest = crypto.createHash('sha256').update(raw).digest();
  let bits = 0n;
  for (const byte of digest.subarray(0, 10)) bits = (bits << 8n) | BigInt(byte);
  let text = '';
  for (let index = 0; index < 16; index++) {
    if (index > 0 && index % 4 === 0) text += '-';
    text += ALPHABET[Number((bits >> BigInt(5 * (15 - index))) & 31n)];
  }
  return text;
}

function hmac(key, data) {
  return crypto.createHmac('sha256', key).update(data).digest();
}

function expand(prk, info, length) {
  let block = Buffer.alloc(0);
  let output = Buffer.alloc(0);
  for (let counter = 1; output.length < length; counter++) {
    block = hmac(prk, Buffer.concat([block, info, Buffer.from([counter])]));
    output = Buffer.concat([output, block]);
  }
  return output.subarray(0, length);
}

const labeledExtract = (suite, salt, label, ikm) =>
  hmac(
    salt.length > 0 ? salt : Buffer.alloc(32),
    Buffer.concat([Buffer.from('HPKE-v1'), suite, Buffer.from(label), ikm]),
  );
const labeledExpand = (suite, prk, label, info, length) =>
  expand(
    prk,
    Buffer.concat([u16(length), Buffer.from('HPKE-v1'), suite, Buffer.from(label), info]),
    length,
  );

function schedule(dh, enc, recipient, info) {
  const kemContext = Buffer.concat([enc, recipient]);
  const eae = labeledExtract(KEM_SUITE, Buffer.alloc(0), 'eae_prk', dh);
  const shared = labeledExpand(KEM_SUITE, eae, 'shared_secret', kemContext, 32);
  const empty = Buffer.alloc(0);
  const context = Buffer.concat([
    Buffer.from([0]),
    labeledExtract(HPKE_SUITE, empty, 'psk_id_hash', empty),
    labeledExtract(HPKE_SUITE, empty, 'info_hash', info),
  ]);
  const secret = labeledExtract(HPKE_SUITE, shared, 'secret', empty);
  return {
    key: labeledExpand(HPKE_SUITE, secret, 'key', context, 32),
    nonce: labeledExpand(HPKE_SUITE, secret, 'base_nonce', context, 12),
  };
}

function aeadSeal(key, nonce, aad, plaintext) {
  const cipher = crypto.createCipheriv('chacha20-poly1305', key, nonce, { authTagLength: 16 });
  cipher.setAAD(aad, { plaintextLength: plaintext.length });
  return Buffer.concat([cipher.update(plaintext), cipher.final(), cipher.getAuthTag()]);
}

function aeadOpen(key, nonce, aad, sealed) {
  const decipher = crypto.createDecipheriv('chacha20-poly1305', key, nonce, { authTagLength: 16 });
  decipher.setAAD(aad, { plaintextLength: sealed.length - 16 });
  decipher.setAuthTag(sealed.subarray(sealed.length - 16));
  return Buffer.concat([decipher.update(sealed.subarray(0, sealed.length - 16)), decipher.final()]);
}

const context = (label, parts) =>
  Buffer.concat([
    Buffer.from(label),
    ...parts.map((part) => Buffer.concat([Buffer.from([0]), Buffer.from(String(part))])),
  ]);
const wrapInfo = (collectionId, groupId, version) =>
  context('codestamp group key v1', [collectionId, groupId, version]);

export function wrapKey(groupKey, publicKeyBase64, collectionId, groupId, version) {
  const recipient = Buffer.from(publicKeyBase64, 'base64');
  const ephemeral = crypto.generateKeyPairSync('x25519');
  const enc = Buffer.from(ephemeral.publicKey.export({ format: 'jwk' }).x, 'base64url');
  const dh = crypto.diffieHellman({
    privateKey: ephemeral.privateKey,
    publicKey: publicKeyObject(recipient),
  });
  const { key, nonce } = schedule(dh, enc, recipient, wrapInfo(collectionId, groupId, version));
  return Buffer.concat([enc, aeadSeal(key, nonce, Buffer.alloc(0), groupKey)]).toString('base64');
}

export function unwrapKey(identity, wrapped, collectionId, groupId, version) {
  const blob = Buffer.from(wrapped, 'base64');
  const enc = blob.subarray(0, 32);
  const dh = crypto.diffieHellman({
    privateKey: identity.privateKey,
    publicKey: publicKeyObject(enc),
  });
  const { key, nonce } = schedule(dh, enc, identity.raw, wrapInfo(collectionId, groupId, version));
  return aeadOpen(key, nonce, Buffer.alloc(0), blob.subarray(32));
}

const subkey = (groupKey, purpose) =>
  Buffer.from(
    crypto.hkdfSync('sha256', groupKey, Buffer.alloc(0), context('codestamp', [purpose]), 32),
  );

const valueAad = (place, version) =>
  context('codestamp value v1', [place.collectionId, place.set, place.key, place.groupId, version]);

export function sealValue(groupKey, version, place, plaintext) {
  const nonce = crypto.randomBytes(12);
  const data = aeadSeal(
    subkey(groupKey, 'value'),
    nonce,
    valueAad(place, version),
    Buffer.from(plaintext),
  );
  return { version, nonce: nonce.toString('base64'), data: data.toString('base64') };
}

export function openValue(groupKey, sealed, place) {
  return aeadOpen(
    subkey(groupKey, 'value'),
    Buffer.from(sealed.nonce, 'base64'),
    valueAad(place, sealed.version),
    Buffer.from(sealed.data, 'base64'),
  ).toString('utf8');
}

/** The stamp over everything but itself, in the field order Codestamp signs. */
export function restamp(group, collectionId, groupKey) {
  const content = JSON.stringify({
    collectionId,
    id: group.id,
    name: group.name,
    keyVersion: group.keyVersion,
    members: group.members.map((m) => ({
      name: m.name,
      email: m.email,
      publicKey: m.publicKey,
      fingerprint: m.fingerprint,
      key: m.key,
      addedBy: m.addedBy,
    })),
    previousKeys: group.previousKeys ?? '',
  });
  group.stamp = hmac(subkey(groupKey, 'stamp'), Buffer.from(content)).toString('base64');
}

export function member(identity, person, addedBy, groupKey, collectionId, groupId, version) {
  return {
    name: person.name,
    email: person.email,
    publicKey: identity.publicKey,
    fingerprint: identity.fingerprint,
    key: wrapKey(groupKey, identity.publicKey, collectionId, groupId, version),
    addedBy,
  };
}

export const pretty = (value) => `${JSON.stringify(value, null, 2)}\n`;
