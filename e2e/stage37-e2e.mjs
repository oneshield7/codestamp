// End-to-end check of access groups (2026-10-27). This device makes a group and restricts a value
// to it; a teammate written from the format alone (access-mate.mjs) asks for access, reads the value
// once granted, makes a group of its own, and lets this device in after it asks. Launches the app.
import { execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import * as mate from './access-mate.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const bare = path.join(here, '.work', 'git-hosts', 'team', 'access.git');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'cs-e2e-access-'));
const clone = path.join(work, 'mate');
const shots = path.join(here, '.work', 'screens');
const git = (args, cwd = work) =>
  execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
const mateGit = (args) =>
  git(['-c', 'user.name=Mate', '-c', 'user.email=mate@example.com', ...args], clone);
const inClone = (file) => path.join(clone, ...file.split('/'));
const readJson = (file) => JSON.parse(fs.readFileSync(inClone(file), 'utf8'));
const writeJson = (file, value) => {
  fs.mkdirSync(path.dirname(inClone(file)), { recursive: true });
  fs.writeFileSync(inClone(file), mate.pretty(value));
};
const MATE = { name: 'Mate', email: 'mate@example.com' };
const TOKEN = 's3cr3t-e2e';
const OPS_VALUE = 'ops-s3cr3t';
const ENV_VALUE = 'ops-env-s3cr3t';

let socket;
let nextId = 1;
const pending = new Map();
async function connect() {
  const pages = await (await fetch(`${CDP}/json`)).json();
  socket = new WebSocket(pages.find((p) => p.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve) => socket.addEventListener('open', resolve));
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    pending.get(message.id)?.(message);
  });
  await cdp('Emulation.setFocusEmulationEnabled', { enabled: true });
  await run(helpers);
}
function cdp(method, params) {
  const id = nextId++;
  socket.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve) => pending.set(id, resolve));
}
async function run(expression) {
  const reply = await cdp('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (reply.result.exceptionDetails)
    throw new Error(
      `${expression.slice(0, 90)} → ${reply.result.exceptionDetails.exception?.description ?? 'eval failed'}`,
    );
  return reply.result.result.value;
}
async function waitFor(expression, timeout = 10000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(200);
  }
  return false;
}
async function shot(name) {
  const image = await cdp('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync(path.join(shots, name), Buffer.from(image.result.data, 'base64'));
}
async function key(name, code, vk, text) {
  const down = text ? { type: 'keyDown', text } : { type: 'rawKeyDown' };
  for (const event of [down, { type: 'keyUp' }]) {
    await cdp('Input.dispatchKeyEvent', { ...event, key: name, code, windowsVirtualKeyCode: vk });
  }
  await sleep(150);
}
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$team = () => document.querySelector('section[aria-label="Collaboration"]');
  window.$teamButton = () => document.querySelector('header.top button[aria-label^="Collaboration"]');
  window.$vars = () => document.querySelector('section[aria-label="Collection variables"]');
  window.$access = () => $team()?.querySelector('section.access');
  window.$group = (name) => [...($access()?.querySelectorAll('article.group') ?? [])].find((a) => a.querySelector('h4')?.textContent.trim() === name);
  window.$chip = (name) => $group(name)?.querySelector('.chip')?.textContent.trim() ?? null;
  window.$facts = () => Object.fromEntries([...($access()?.querySelectorAll('dt') ?? [])].map((dt) => [dt.textContent.trim(), dt.nextElementSibling.querySelector('.mono')?.textContent.trim()]));
  window.$row = (key) => [...($vars()?.querySelectorAll('tbody tr') ?? [])].find((tr) => tr.querySelector('input[aria-label="Variable name"]')?.value === key);
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 120)); };
  window.$envRow = (key) => [...document.querySelectorAll('tbody tr')].find((tr) => tr.querySelector('input[aria-label="Variable name"]')?.value === key);
  window.$preview = () => document.querySelector('p.preview')?.textContent.replace(/\\s+/g, ' ').trim() ?? '';
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 400);
const request = {
  kind: 'http',
  name: 'Health',
  method: 'GET',
  url: 'https://api.test/health?key={{opsKey}}',
  params: [{ enabled: true, key: 'key', value: '{{opsKey}}' }],
  headers: [],
  bodyMode: 'none',
  body: '',
  form: [],
  bodyFile: '',
  preRequestScript: '',
  testScript: '',
  auth: { type: 'inherit' },
  mcpOperation: null,
  pathVariables: [],
  graphqlVariables: '',
  grpc: null,
};

async function openPanel(label) {
  const button = `document.querySelector('header.top button[aria-label^="${label}"]')`;
  if ((await run(`${button}?.getAttribute('aria-expanded')`)) !== 'true') {
    await run(`${button}.click()`);
    await sleep(600);
  }
}
async function syncNow() {
  await openPanel('Collaboration');
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  await run(`$btn('Sync now', $team()).click()`);
  await sleep(400);
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  await sleep(700);
}
/** Clicks a button inside the Access section and waits for the change and its sync. */
async function accessAction(group, label) {
  await run(`$btn(${JSON.stringify(label)}, $group(${JSON.stringify(group)})).click()`);
  await sleep(300);
  await waitFor(
    `!$access().querySelector('button:disabled') || !!$access().querySelector('.error')`,
    30000,
  );
  await sleep(500);
}
const headFile = (file) => git(['--git-dir', bare, 'show', `main:${file}`]);
const pull = () => mateGit(['pull', '-q', '--ff-only']);
const push = (message) => {
  mateGit(['add', '--all']);
  mateGit(['commit', '-q', '-m', message]);
  mateGit(['push', '-q', 'origin', 'main']);
};

fs.rmSync(bare, { recursive: true, force: true });
fs.mkdirSync(path.dirname(bare), { recursive: true });
git(['init', '-q', '--bare', '--initial-branch', 'main', bare]);
execFileSync(process.execPath, [path.join(here, 'launch.mjs')], { stdio: 'inherit' });
await connect();
let collectionId = null;
try {
  const profile = (await invoke('team_setup', {})).ok?.author ?? {
    name: 'Ana Work',
    email: 'ana@acme.test',
  };
  collectionId = (
    await invoke('create_collection', { name: 'E2E Access', icon: null, baseUrl: null })
  ).ok.id;
  await invoke('create_request', { collectionId, sectionId: null, content: request });
  const set = (await invoke('get_collection_variable_set', { collectionId })).ok;
  await invoke('save_environment_variables', {
    id: set,
    variables: [
      { id: null, key: 'token', value: TOKEN, isSecret: true, enabled: true, restricted: null },
    ],
  });
  const shared = await invoke('share_collection', {
    collectionId,
    remote: 'https://git.test/team/access.git',
    account: { author: profile, username: null },
    replaceHead: null,
  });
  check('the collection is shared', !!shared.ok, short(shared.error ?? 'ok'));
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
  await waitFor(`!!$nav('E2E Access')`);
  await run(`$nav('E2E Access').click()`);
  await sleep(900);
  await openPanel('Collaboration');
  await waitFor(`!!$access()`);

  // ---------- a group made here ----------
  const empty = await run(
    `({ heading: $access().querySelector('h3')?.textContent.trim(), facts: $facts(), groups: $access().querySelectorAll('article').length, field: !!$access().querySelector('input[placeholder="e.g. Production"]') })`,
  );
  check(
    'the Access section starts empty, without a device key yet',
    empty.heading === 'Access groups' &&
      empty.groups === 0 &&
      !('This device' in empty.facts) &&
      empty.field,
    short(empty),
  );
  await run(`$set($access().querySelector('input[placeholder="e.g. Production"]'), 'Prod')`);
  await sleep(150);
  await run(`$access().querySelector('input[placeholder="e.g. Production"]').focus()`);
  await key('Tab', 'Tab', 9);
  const focused = await run(`document.activeElement?.textContent.trim()`);
  check('Tab moves from the name to Create group', focused === 'Create group', focused);
  await key('Enter', 'Enter', 13, String.fromCharCode(13));
  await waitFor(`!!$group('Prod')`, 30000);
  await sleep(600);
  const made = await run(
    `({ chip: $chip('Prod'), facts: $facts(), members: [...$group('Prod').querySelectorAll('.people li')].map((li) => li.textContent.replace(/\\s+/g, ' ').trim()) })`,
  );
  const devicePrint = made.facts['This device'];
  check(
    'the new group lists this device as its member',
    made.chip === 'You are a member' &&
      made.members.length === 1 &&
      made.members[0].includes('(this device)') &&
      made.members[0].includes(devicePrint ?? '?'),
    short(made),
  );
  check(
    'making a group here is not announced as a grant',
    !(await run(
      `document.querySelector('.banner.info')?.textContent.includes('gave this device access') ?? false`,
    )),
  );
  check(
    'the device key shows once a group exists',
    /^[0-9A-Z]{4}-[0-9A-Z]{4}-[0-9A-Z]{4}-[0-9A-Z]{4}$/.test(devicePrint ?? ''),
    devicePrint,
  );
  await shot('stage37-group.png');

  // ---------- a value restricted to it travels encrypted ----------
  await openPanel('Variables');
  await waitFor(`!!$row('token')`);
  const typeButton = `$row('token').querySelector('[aria-label="Variable type"]')`;
  await run(`$pick(${typeButton}, 'Restricted: Prod')`);
  await run(`$btn('Save', $vars()).click()`);
  await sleep(800);
  check(
    'the type reads Restricted: Prod after saving',
    (await run(`${typeButton}.textContent.trim()`)) === 'Restricted: Prod',
    await run(`${typeButton}.textContent.trim()`),
  );
  await syncNow();
  const collectionFile = headFile('collection.json');
  const manifest = headFile('codestamp.json');
  const prodId = git(['--git-dir', bare, 'ls-tree', '--name-only', 'main', 'access/'])
    .split('\n')
    .map((p) => p.trim().replace('access/', '').replace('.json', ''))[0];
  check(
    'the repository holds the token only as ciphertext',
    !collectionFile.includes(TOKEN) &&
      collectionFile.includes('"restricted"') &&
      collectionFile.includes('"sealed"'),
    collectionFile.slice(0, 300),
  );
  check(
    'the repository moves to format 8 once it has a group',
    /"format": 8/.test(manifest),
    manifest,
  );

  // ---------- the teammate makes a group of its own and asks to join Prod ----------
  git(['clone', '-q', bare, clone]);
  const identity = mate.newIdentity();
  const opsId = crypto.randomUUID();
  const opsKey = crypto.randomBytes(32);
  const ops = {
    id: opsId,
    name: 'Ops',
    keyVersion: 1,
    members: [mate.member(identity, MATE, 'Mate', opsKey, collectionId, opsId, 1)],
    stamp: '',
  };
  mate.restamp(ops, collectionId, opsKey);
  writeJson(`access/${opsId}.json`, ops);
  const collection = readJson('collection.json');
  collection.variables.push({
    key: 'opsKey',
    value: '',
    secret: true,
    enabled: true,
    restricted: opsId,
    sealed: mate.sealValue(
      opsKey,
      1,
      { collectionId, set: 'collection', key: 'opsKey', groupId: opsId },
      OPS_VALUE,
    ),
  });
  writeJson('collection.json', collection);
  // An environment of the collection holds a value for Ops too, so the Environments page is checked.
  const envId = crypto.randomUUID();
  writeJson(`environments/${envId}.json`, {
    id: envId,
    name: 'Ops env',
    variables: [
      {
        key: 'opsSecret',
        value: '',
        secret: true,
        enabled: true,
        restricted: opsId,
        sealed: mate.sealValue(
          opsKey,
          1,
          { collectionId, set: envId, key: 'opsSecret', groupId: opsId },
          ENV_VALUE,
        ),
      },
    ],
  });
  writeJson(`access/requests/${prodId}.${identity.fingerprint.replaceAll('-', '')}.json`, {
    groupId: prodId,
    name: MATE.name,
    email: MATE.email,
    publicKey: identity.publicKey,
    fingerprint: identity.fingerprint,
  });
  push('Mate: Ops group and a request for Prod');

  await syncNow();
  await waitFor(`!!$group('Ops')`, 10000);
  const seen = await run(
    `({ ops: $chip('Ops'), request: !!$btn('Request access', $group('Ops')), asking: [...$group('Prod').querySelectorAll('.people')].at(-1)?.textContent.replace(/\\s+/g, ' ').trim(), grant: !!$btn('Grant', $group('Prod')) })`,
  );
  check(
    "the teammate's group shows with no access and a way to ask",
    seen.ops === 'No access' && seen.request,
    short(seen),
  );
  check(
    'the request shows the fingerprint the teammate computed',
    (seen.asking ?? '').includes('Mate') &&
      (seen.asking ?? '').includes(identity.fingerprint) &&
      seen.grant,
    short(seen),
  );
  await shot('stage37-asking.png');

  // The request card names the group behind a value this device cannot read.
  if (!(await run(`!!document.querySelector('input[id^="url-"]')`))) {
    await run(
      `[...document.querySelectorAll('.row')].find((r) => r.querySelector('.name')?.textContent.trim() === 'Health').click()`,
    );
    await sleep(700);
  }
  await waitFor(`$preview().startsWith('Not defined')`, 6000);
  const preview = await run(`$preview()`);
  check(
    'an unreadable value is named as restricted, not as a typo',
    preview.includes('{{opsKey}} (restricted to Ops)'),
    preview,
  );
  await openPanel('Variables');
  await waitFor(`!!$row('opsKey')`);
  const locked = await run(
    `(() => { const row = $row('opsKey'); return { text: row.querySelector('.locked')?.textContent.replace(/\\s+/g, ' ').trim(), type: row.querySelector('td.type')?.textContent.trim(), readonly: row.querySelector('input[aria-label="Variable name"]').readOnly, remove: !!row.querySelector('button[aria-label="Remove variable"]') }; })()`,
  );
  check(
    'the locked value is shown read-only with the way in',
    locked.text === 'Restricted to Ops. Ask a member for access in Collaboration.' &&
      locked.type === 'Restricted: Ops' &&
      locked.readonly &&
      !locked.remove,
    short(locked),
  );
  await shot('stage37-locked.png');

  // The Environments page shows the environment's value locked too.
  async function envState() {
    await run(`$nav('Environments').click()`);
    await sleep(700);
    await run(
      `[...document.querySelectorAll('.env')].find((b) => b.textContent.includes('Ops env')).click()`,
    );
    await waitFor(`!!$envRow('opsSecret')`);
    await sleep(300);
    const state = await run(
      `(() => { const row = $envRow('opsSecret'); return { locked: row.querySelector('.locked')?.textContent.replace(/\\s+/g, ' ').trim() ?? null, type: row.querySelector('td.type')?.textContent.trim() }; })()`,
    );
    await run(`$nav('E2E Access').click()`);
    await sleep(800);
    return state;
  }
  const envBefore = await envState();
  check(
    'a value of an environment is locked as well',
    envBefore.locked === 'Restricted to Ops. Ask a member for access in Collaboration.',
    short(envBefore),
  );

  // ---------- granting the teammate lets it read the value ----------
  await openPanel('Collaboration');
  await accessAction('Prod', 'Grant');
  pull();
  const prod = readJson(`access/${prodId}.json`);
  const mine = prod.members.find((m) => m.fingerprint === identity.fingerprint);
  let opened = null;
  try {
    const prodKey = mate.unwrapKey(identity, mine.key, collectionId, prodId, prod.keyVersion);
    const token = readJson('collection.json').variables.find((v) => v.key === 'token');
    opened = mate.openValue(prodKey, token.sealed, {
      collectionId,
      set: 'collection',
      key: 'token',
      groupId: prodId,
    });
  } catch (error) {
    opened = `failed: ${error.message}`;
  }
  check('the granted teammate opens the token with its own key', opened === TOKEN, opened);
  check(
    'the request is gone after granting',
    !fs.existsSync(
      inClone(`access/requests/${prodId}.${identity.fingerprint.replaceAll('-', '')}.json`),
    ),
  );

  // ---------- this device asks to join Ops and the teammate lets it in ----------
  await accessAction('Ops', 'Request access');
  const waiting = await run(
    `$group('Ops').textContent.includes('Waiting for a member to grant access')`,
  );
  check('asking shows that a member has to grant it', waiting);
  pull();
  const askFile = `access/requests/${opsId}.${(devicePrint ?? '').replaceAll('-', '')}.json`;
  const ask = fs.existsSync(inClone(askFile)) ? readJson(askFile) : null;
  check(
    "the request carries this device's key and fingerprint",
    ask !== null &&
      ask.fingerprint === devicePrint &&
      mate.fingerprint(Buffer.from(ask.publicKey, 'base64')) === devicePrint,
    short(ask),
  );
  if (ask) {
    const group = readJson(`access/${opsId}.json`);
    group.members.push({
      name: ask.name,
      email: ask.email,
      publicKey: ask.publicKey,
      fingerprint: ask.fingerprint,
      key: mate.wrapKey(opsKey, ask.publicKey, collectionId, opsId, 1),
      addedBy: 'Mate',
    });
    mate.restamp(group, collectionId, opsKey);
    writeJson(`access/${opsId}.json`, group);
    fs.rmSync(inClone(askFile));
    push('Mate: grant Ops');
  }
  await syncNow();
  await waitFor(`$chip('Ops') === 'You are a member'`, 10000);
  check(
    'after the grant this device is a member of Ops',
    (await run(`$chip('Ops')`)) === 'You are a member',
    await run(`$chip('Ops')`),
  );
  const notice = await run(
    `document.querySelector('.banner.info span')?.textContent.trim() ?? null`,
  );
  check(
    'the grant is announced with who gave it',
    notice === 'Mate gave this device access to Ops. Its restricted values are ready to use.',
    notice,
  );
  await shot('stage37-granted.png');
  await run(`$btn('Open variables', document.querySelector('.banner.info')).click()`);
  await waitFor(`!!$vars()`, 5000);
  const afterOpen = await run(
    `({ vars: !!$vars(), banner: document.querySelector('.banner.info')?.textContent.trim() ?? null, panels: [...document.querySelectorAll('section.panel')].map((p) => p.getAttribute('aria-label')) })`,
  );
  check(
    'Open variables opens the collection variables and closes the notice',
    afterOpen.vars && afterOpen.banner === null,
    short(afterOpen),
  );
  const opsRow = (await invoke('get_environment_variables', { id: set })).ok.find(
    (v) => v.key === 'opsKey',
  );
  const revealed = opsRow ? (await invoke('reveal_secret', { id: opsRow.id })).ok : null;
  check(
    "the teammate's value opens here",
    opsRow && !opsRow.locked && revealed === OPS_VALUE,
    short({ locked: opsRow?.locked, revealed }),
  );
  const envAfter = await envState();
  check(
    'the Environments page opens the value after Sync now, without a restart',
    envAfter.locked === null && envAfter.type === 'Restricted: Ops',
    short(envAfter),
  );
  const envValue = (
    await invoke('reveal_secret', {
      id: (await invoke('get_environment_variables', { id: envId })).ok.find(
        (v) => v.key === 'opsSecret',
      ).id,
    })
  ).ok;
  check("the environment's value is the teammate's", envValue === ENV_VALUE, envValue);
  if (!(await run(`!!document.querySelector('input[id^="url-"]')`))) {
    await run(
      `[...document.querySelectorAll('.row')].find((r) => r.querySelector('.name')?.textContent.trim() === 'Health').click()`,
    );
    await sleep(700);
  }
  await waitFor(`!$preview().startsWith('Not defined')`, 6000);
  check(
    'the request no longer reports the value as missing',
    !(await run(`$preview()`)).startsWith('Not defined'),
    await run(`$preview()`),
  );

  // ---------- removing the teammate gives Prod a new key ----------
  await openPanel('Collaboration');
  const mateRow = `[...$group('Prod').querySelectorAll('.people li')].find((li) => li.textContent.includes('Mate'))`;
  await run(`$btn('Remove', ${mateRow}).click()`);
  await sleep(300);
  const dialog = await run(
    `({ title: document.querySelector('dialog[open]:not(.panel-dialog) h2')?.textContent.trim(), message: document.querySelector('dialog[open]:not(.panel-dialog) p')?.textContent.trim() })`,
  );
  check(
    'removing asks first and says what the removed member keeps',
    dialog.title === 'Remove Mate?' && /already seen the current values/.test(dialog.message ?? ''),
    short(dialog),
  );
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Remove').click()`,
  );
  await sleep(300);
  await waitFor(`!!$access().querySelector('.done')`, 30000);
  const done = await run(
    `$access().querySelector('.done p')?.textContent.replace(/\\s+/g, ' ').trim()`,
  );
  check(
    'the values to change at their source are listed',
    /Change token where they are issued/.test(done ?? ''),
    done,
  );
  await shot('stage37-removed.png');
  pull();
  const rotated = readJson(`access/${prodId}.json`);
  const sealedToken = readJson('collection.json').variables.find((v) => v.key === 'token').sealed;
  let stale = 'opened';
  try {
    const oldKey = mate.unwrapKey(identity, mine.key, collectionId, prodId, 1);
    mate.openValue(oldKey, sealedToken, {
      collectionId,
      set: 'collection',
      key: 'token',
      groupId: prodId,
    });
  } catch {
    stale = 'refused';
  }
  check(
    'the removed teammate is out and the token moved to the new key',
    rotated.keyVersion === 2 &&
      !rotated.members.some((m) => m.fingerprint === identity.fingerprint) &&
      sealedToken.version === 2 &&
      stale === 'refused',
    short({
      version: rotated.keyVersion,
      members: rotated.members.length,
      sealed: sealedToken.version,
      stale,
    }),
  );
} catch (error) {
  console.log(results.join('\n'));
  console.log(
    'state',
    short(
      await run(
        `({ error: $access()?.querySelector('.error')?.textContent, groups: [...($access()?.querySelectorAll('article h4') ?? [])].map((h) => h.textContent), notice: document.querySelector('[role="alert"]')?.textContent })`,
      ).catch(() => null),
    ),
  );
  throw error;
} finally {
  if (collectionId && !process.env.KEEP) await invoke('delete_collection', { id: collectionId });
  socket.close();
  fs.rmSync(work, { recursive: true, force: true });
  if (!process.env.KEEP) fs.rmSync(bare, { recursive: true, force: true });
}
console.log(results.join('\n'));
console.log(`${results.filter((line) => line.startsWith('PASS')).length}/${results.length} passed`);
