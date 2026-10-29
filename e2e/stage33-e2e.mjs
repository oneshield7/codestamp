// End-to-end check of the 2026-10-27 notes: the status bar pages, Ctrl+S for variables,
// manual sync, collection variables replaced by the active environment, and environments that
// belong to a shared collection. launch.mjs maps https://git.test/ to .work/git-hosts.
import { execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Notes';
const REMOTE = 'https://git.test/team/notes.git';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const hosts = path.join(path.dirname(fileURLToPath(import.meta.url)), '.work', 'git-hosts');
const bare = path.join(hosts, 'team', 'notes.git');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'cs-e2e-notes-'));
const mate = path.join(work, 'mate');
const git = (args, cwd = work) =>
  execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
const commits = () => Number(git(['--git-dir', bare, 'rev-list', '--count', 'main']));

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
async function waitFor(expression, timeout = 20000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(250);
  }
  return false;
}
async function ctrlS() {
  for (const type of ['keyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: 's',
      code: 'KeyS',
      windowsVirtualKeyCode: 83,
      modifiers: 2,
    });
  }
  await sleep(600);
}
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$section = (title) => document.querySelector('nav.sidebar [role="group"][aria-label="' + title + '"]');
  window.$place = (name) => document.querySelector('footer.statusbar button[aria-label="' + name + '"]');
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim().endsWith(label)); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 150)); };
  window.$env = (name) => [...document.querySelectorAll('nav.list .env')].find((b) => b.querySelector('.name')?.textContent.trim() === name);
  window.$envPanel = () => document.querySelector('.layout section.panel');
  window.$saveLabel = (root) => [...root.querySelectorAll('button.primary')].map((b) => b.textContent.trim()).find((t) => /^Sav/.test(t));
  window.$rowsOf = (root) => [...root.querySelectorAll('tbody tr')].map((tr) => ({ key: tr.querySelector('input[aria-label="Variable name"]')?.value, value: tr.querySelector('input[aria-label="Variable value"]')?.value, replaced: tr.querySelector('.replaced')?.textContent.trim() ?? '' })).filter((r) => r.key);
  window.$teamButton = () => document.querySelector('header.top button[aria-label^="Collaboration"]');
  window.$team = () => document.querySelector('section[aria-label="Collaboration"]');
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 260);
let collectionId = null;
const environmentIds = [];

async function openCollection() {
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await waitFor(
    `document.querySelector('input[aria-label="Collection name"]')?.value === ${JSON.stringify(NAME)}`,
  );
  await sleep(400);
}
async function syncNow() {
  if (!(await run(`!!$team()`))) await run(`$teamButton().click()`);
  await waitFor(
    `!!$team() && !!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`,
    30000,
  );
  await run(`$btn('Sync now', $team()).click()`);
  await sleep(300);
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  await sleep(400);
}

fs.mkdirSync(path.dirname(bare), { recursive: true });
git(['init', '-q', '--bare', '--initial-branch', 'main', bare]);
await connect();
try {
  const profile = (await invoke('team_setup', {})).ok?.author ?? {
    name: 'Ana Work',
    email: 'ana@acme.test',
  };
  const request = {
    kind: 'http',
    name: 'Ping',
    method: 'GET',
    url: '{{baseUrl}}/ping',
    params: [],
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
  collectionId = (await invoke('create_collection', { name: NAME, icon: null, baseUrl: null })).ok
    .id;
  await invoke('create_request', { collectionId, sectionId: null, content: request });
  const set = (await invoke('get_collection_variable_set', { collectionId })).ok;
  await invoke('save_environment_variables', {
    id: set,
    variables: [
      {
        id: null,
        key: 'baseUrl',
        value: 'https://collection.test',
        isSecret: false,
        enabled: true,
      },
      { id: null, key: 'extra', value: '1', isSecret: false, enabled: true },
    ],
  });
  const dev = (await invoke('create_environment', { name: 'DEV' })).ok;
  environmentIds.push(dev.id);
  await invoke('save_environment_variables', {
    id: dev.id,
    variables: [
      { id: null, key: 'baseUrl', value: 'https://dev.test', isSecret: false, enabled: true },
      { id: null, key: 'token', value: 'dev-secret', isSecret: true, enabled: true },
    ],
  });
  const shared = await invoke('share_collection', {
    collectionId,
    remote: REMOTE,
    account: { author: profile, username: null },
    replaceHead: null,
  });
  check(
    'the collection is shared over the stand-in host',
    !!shared.ok && commits() === 1,
    short(shared.error ?? 'ok'),
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);

  // ---------- note 5, moved on 2026-10-28: Settings and data pages sit in the status bar ----------
  const bar = await run(
    `(() => { const footer = document.querySelector('footer.statusbar'); const box = footer.getBoundingClientRect(); return { groups: [...document.querySelectorAll('nav.sidebar h2')].map((h) => h.textContent.trim()), places: [...footer.querySelectorAll('button')].map((b) => b.getAttribute('aria-label')), gapBelow: Math.round(innerHeight - box.bottom), width: Math.round(box.width) }; })()`,
  );
  check(
    'the status bar holds Settings, Recently deleted, Cookies and Environments',
    short(bar.groups) === short(['Collections', 'Tools']) &&
      short(bar.places) === short(['Settings', 'Recently deleted', 'Cookies', 'Environments']),
    short(bar),
  );
  check(
    'the status bar runs along the bottom of the window',
    bar.gapBelow === 0 && bar.width === (await run('innerWidth')),
    short(bar),
  );
  for (const page of ['Cookies', 'Recently deleted', 'Settings', 'Environments']) {
    await run(`$place(${JSON.stringify(page)}).click()`);
    await sleep(500);
  }
  check(
    'its buttons open their pages',
    (await run(`document.querySelector('main h1')?.textContent.trim()`)) === 'Environments' &&
      (await run(`$place('Environments').getAttribute('aria-current')`)) === 'page',
  );

  // ---------- note 2: Ctrl+S saves an environment ----------
  await run(`$env('DEV').click()`);
  await waitFor(`$rowsOf($envPanel()).length === 2`);
  await run(
    `$set($envPanel().querySelectorAll('input[aria-label="Variable value"]')[0], 'https://dev2.test')`,
  );
  await sleep(200);
  const dirty = await run(`$saveLabel($envPanel())`);
  await ctrlS();
  const savedLabel = await run(`$saveLabel($envPanel())`);
  const stored = (await invoke('get_environment_variables', { id: dev.id })).ok?.[0]?.value;
  check(
    'Ctrl+S saves the environment being edited',
    dirty === 'Save' && savedLabel === 'Saved' && stored === 'https://dev2.test',
    short({ dirty, savedLabel, stored }),
  );

  // ---------- note 4: collection variables the active environment replaces ----------
  await invoke('set_active_environment', { id: dev.id });
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await openCollection();
  await run(`$btn('Variables').click()`);
  const variablesPanel = `document.querySelector('section[aria-label="Collection variables"]')`;
  await waitFor(`$rowsOf(${variablesPanel}).some((r) => r.replaced)`);
  const marked = await run(`$rowsOf(${variablesPanel})`);
  check(
    'a collection variable the active environment also has is marked replaced',
    marked.find((r) => r.key === 'baseUrl')?.replaced === 'Replaced by “DEV”' &&
      marked.find((r) => r.key === 'extra')?.replaced === '',
    short(marked),
  );

  // ---------- notes 2 and 3: Ctrl+S in the collection panel, and nothing syncs on its own ----------
  const before = commits();
  await run(
    `$set([...${variablesPanel}.querySelectorAll('tbody tr')].find((tr) => tr.querySelector('input[aria-label="Variable name"]').value === 'extra').querySelector('input[aria-label="Variable value"]'), '2')`,
  );
  await sleep(200);
  await ctrlS();
  check(
    'Ctrl+S saves collection variables too',
    (await run(`$saveLabel(${variablesPanel})`)) === 'Saved',
  );
  await run(`$btn('Close', ${variablesPanel}).click()`);
  await sleep(6500);
  const label = await run(`$teamButton().getAttribute('aria-label').trim()`);
  check(
    'a saved change waits for Sync now',
    commits() === before && /not synced/.test(label),
    `${label} · ${commits()} commits`,
  );
  await syncNow();
  check(
    'Sync now sends it',
    commits() === before + 1 &&
      /synced/.test(await run(`$teamButton().getAttribute('aria-label')`)),
    String(commits()),
  );

  // ---------- note 1: an environment that belongs to the collection is shared ----------
  await run(`$place('Environments').click()`);
  await sleep(600);
  await run(`$env('DEV').click()`);
  await sleep(400);
  await run(
    `$pick(document.querySelector('button[aria-label="Belongs to"]'), ${JSON.stringify(NAME)})`,
  );
  await waitFor(
    `!!$envPanel().querySelector('.owner-hint')?.textContent.includes('Shared with everyone')`,
  );
  const owned = await run(
    `({ hint: $envPanel().querySelector('.owner-hint').textContent.replace(/\\s+/g, ' ').trim(), groups: [...document.querySelectorAll('nav.list .group-title')].map((h) => h.textContent.trim()) })`,
  );
  check(
    'Belongs to gives the environment to the collection',
    owned.hint.startsWith(`Shared with everyone in “${NAME}”`) &&
      owned.groups.some((g) => g.endsWith(NAME)),
    short(owned),
  );
  await openCollection();
  check(
    'the collection then has changes to sync',
    /not synced/.test(await run(`$teamButton().getAttribute('aria-label')`)),
  );
  const beforeEnv = commits();
  await syncNow();
  git(['clone', '-q', bare, mate]);
  const envFile = path.join(mate, 'environments', `${dev.id}.json`);
  const sent = fs.existsSync(envFile) ? JSON.parse(fs.readFileSync(envFile, 'utf8')) : null;
  check(
    'Sync now writes it to environments/ without the secret value',
    commits() === beforeEnv + 1 &&
      sent?.name === 'DEV' &&
      sent.variables.find((v) => v.key === 'token')?.value === '' &&
      sent.variables.find((v) => v.key === 'token')?.secret === true &&
      sent.variables[0].value === 'https://dev2.test',
    short(sent),
  );

  // A collaborator adds a variable to DEV and a new PROD environment.
  const prodId = crypto.randomUUID();
  environmentIds.push(prodId);
  sent.variables.push({ key: 'region', value: 'id', secret: false, enabled: true });
  fs.writeFileSync(envFile, `${JSON.stringify(sent, null, 2)}\n`);
  fs.writeFileSync(
    path.join(mate, 'environments', `${prodId}.json`),
    `${JSON.stringify({ id: prodId, name: 'PROD', variables: [{ key: 'baseUrl', value: 'https://prod.test', secret: false, enabled: true }] }, null, 2)}\n`,
  );
  git(['add', '--all'], mate);
  git(
    [
      '-c',
      'user.name=Mate',
      '-c',
      'user.email=mate@example.com',
      'commit',
      '-q',
      '-m',
      'Add PROD and region',
    ],
    mate,
  );
  git(['push', '-q', 'origin', 'main'], mate);
  await syncNow();
  await run(`$place('Environments').click()`);
  await sleep(700);
  const listed = await run(
    `[...document.querySelectorAll('nav.list .env .name')].map((n) => n.textContent.trim())`,
  );
  await run(`$env('DEV').click()`);
  await sleep(500);
  const devRows = await run(`$rowsOf($envPanel()).map((r) => r.key)`);
  const tokenKept = (await invoke('get_environment_variables', { id: dev.id })).ok?.find(
    (v) => v.key === 'token',
  );
  const tokenValue = tokenKept ? (await invoke('reveal_secret', { id: tokenKept.id })).ok : null;
  check(
    'a collaborator’s environments and variables arrive with Sync now',
    listed.includes('PROD') && short(devRows) === short(['baseUrl', 'token', 'region']),
    short({ listed, devRows }),
  );
  check('the secret value on this device is kept', tokenValue === 'dev-secret', String(tokenValue));

  // Git changes names the environments in the collaborator's commit.
  await run(`$nav('Git changes').click()`);
  await waitFor(
    `!![...document.querySelectorAll('button.commit .message')].find((m) => m.textContent.includes('PROD'))`,
    20000,
  );
  await run(
    `[...document.querySelectorAll('button.commit')].find((b) => b.textContent.includes('PROD')).click()`,
  );
  await waitFor(`!!document.querySelector('section[aria-label^="Commit "] li.file')`);
  const files = await run(
    `[...document.querySelectorAll('section[aria-label^="Commit "] li.file')].map((li) => li.querySelector('.chip').textContent.trim() + ' ' + li.querySelector('.name').textContent.replace(/\\s+/g, ' ').trim())`,
  );
  check(
    'Git changes lists the environment files of a commit',
    files.includes('Updated Environment “DEV”') && files.includes('Added Environment “PROD”'),
    short(files),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await invoke('set_active_environment', { id: null });
    if (collectionId) {
      await invoke('unlink_team_collection', { collectionId });
      await invoke('delete_collection', { id: collectionId });
    }
    for (const id of environmentIds) {
      await invoke('delete_environment', { id });
    }
    await run('location.reload()');
    await sleep(1500);
    const left = (await invoke('list_collections', {})).ok ?? [];
    const envsLeft = ((await invoke('list_environments', {})).ok?.environments ?? []).filter((e) =>
      environmentIds.includes(e.id),
    );
    check(
      'test data cleaned up',
      !left.some((c) => c.id === collectionId) && envsLeft.length === 0,
      short(envsLeft.map((e) => e.name)),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  fs.rmSync(work, { recursive: true, force: true });
  fs.rmSync(hosts, { recursive: true, force: true });
  process.exit(0);
}
