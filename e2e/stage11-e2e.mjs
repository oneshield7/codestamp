// End-to-end check of stage 11 (team collections over Git) through the real release build.
// A local bare repository stands in for the Git host; the teammate is simulated with the git CLI.
import { execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Stage11';
const SECRET = 'team-secret-7781';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'pc-e2e-'));
const bare = path.join(work, 'team.git');
const mate = path.join(work, 'mate');
const reposDir = path.join(process.env.APPDATA, 'dev.codestamp.desktop', 'repos');

function git(args, cwd = work) {
  return execFileSync('git', args, {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}
function mateCommit(message) {
  git(['add', '--all'], mate);
  git(
    ['-c', 'user.name=Mate', '-c', 'user.email=mate@example.com', 'commit', '-q', '-m', message],
    mate,
  );
  git(['push', '-q', 'origin', 'main'], mate);
}
function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(mate, file), 'utf8'));
}
function writeJson(file, value) {
  fs.writeFileSync(path.join(mate, file), `${JSON.stringify(value, null, 2)}\n`);
}
function requestFileNamed(name) {
  const dir = path.join(mate, 'requests');
  for (const file of fs.readdirSync(dir)) {
    const value = JSON.parse(fs.readFileSync(path.join(dir, file), 'utf8'));
    if (value.name === name) return { file: `requests/${file}`, value };
  }
  return null;
}
const remoteHead = () => git(['--git-dir', bare, 'rev-parse', 'main']);

// ---------- CDP ----------
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
async function waitFor(expression, timeout = 15000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(250);
  }
  return false;
}
async function waitUntil(test, timeout = 20000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (test()) return true;
    await sleep(500);
  }
  return false;
}
const helpers = `
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 100)); };
  window.$picked = (trigger) => trigger.querySelector('.text').textContent.trim();
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$card = () => document.querySelector('article.card');
  window.$dialog = () => document.querySelector('dialog[open]:not(.panel-dialog)');
  window.$row = (name) => [...document.querySelectorAll('.row')].find((r) => r.querySelector('.name')?.textContent.trim() === name);
  window.$rows = () => [...document.querySelectorAll('.row .name')].map((n) => n.textContent.trim());
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$team = () => document.querySelector('section[aria-label="Collaboration"]');
  window.$teamButton = () => document.querySelector('header.top button[aria-label^="Collaboration"]');
  window.$banner = () => document.querySelector('.banner')?.textContent ?? '';
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 200);

async function addRequest(name, url) {
  await run(`(document.querySelector('.adder') ?? document.querySelector('.start .tile')).click()`);
  await sleep(500);
  await run(`$set($card().querySelector('.name input'), ${JSON.stringify(name)})`);
  await run(`$set(document.querySelector('input[id^="url-"]'), ${JSON.stringify(url)})`);
  await sleep(300);
}
async function collapse() {
  await run(`$card()?.querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(800);
}
async function openRow(name) {
  await run(`$row(${JSON.stringify(name)}).click()`);
  await sleep(800);
}
async function openTeam() {
  if (!(await run(`!!$team()`))) {
    await run(`$teamButton().click()`);
    await sleep(500);
  }
}
async function closeTeam() {
  if (await run(`!!$team()`)) {
    await run(`$btn('Close', $team()).click()`);
    await sleep(300);
  }
}
async function syncNow() {
  await openTeam();
  // A sync may still be running; wait until Sync now can be pressed.
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  await run(`$btn('Sync now', $team()).click()`);
  await sleep(400);
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  await sleep(500);
}

git(['init', '-q', '--bare', '--initial-branch', 'main', bare]);
await connect();
try {
  // ---------- a local collection ----------
  await run(`(async () => {
    const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    document.querySelector('nav button.new').click();
    await wait(400);
    const input = document.getElementById('new-collection-name');
    input.value = 'E2E temp';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await wait(300);
    [...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Create collection').click();
    await wait(900);
  })()`);
  await sleep(800);
  await run(
    `(() => { const t = document.querySelector('input[aria-label="Collection name"]'); $set(t, ${JSON.stringify(NAME)}); t.dispatchEvent(new Event('blur')); })()`,
  );
  await sleep(500);
  await run(`$btn('Variables').click()`);
  await sleep(600);
  const vars = `document.querySelectorAll('section[aria-label="Collection variables"] tbody tr')`;
  await run(`$set(${vars}[0].querySelector('input[aria-label="Variable name"]'), 'baseUrl')`);
  await run(
    `$set(${vars}[0].querySelector('input[aria-label="Variable value"]'), 'https://api.test')`,
  );
  await sleep(200);
  await run(`$set(${vars}[1].querySelector('input[aria-label="Variable name"]'), 'token')`);
  await sleep(200);
  await run(`$pick(${vars}[1].querySelector('button[aria-label="Variable type"]'), 'Secret')`);
  await sleep(200);
  await run(`$set(${vars}[1].querySelector('input[aria-label="Variable value"]'), '${SECRET}')`);
  await sleep(200);
  await run(
    `$btn('Save', document.querySelector('section[aria-label="Collection variables"]')).click()`,
  );
  await sleep(700);
  await run(
    `$btn('Close', document.querySelector('section[aria-label="Collection variables"]')).click()`,
  );
  await sleep(300);
  await addRequest('List users', 'https://api.test/users');
  await collapse();
  await addRequest('Create user', 'https://api.test/users/new');
  await collapse();

  // ---------- share ----------
  await openTeam();
  const prefilled = await run(`[...$team().querySelectorAll('.author input')].map((i) => i.value)`);
  const gitName = git(['config', '--global', 'user.name']);
  check(
    'Team panel suggests the Git identity of this device',
    prefilled[0] === gitName,
    short(prefilled),
  );
  await run(`$set($team().querySelector('input.mono'), ${JSON.stringify(bare)})`);
  await sleep(200);
  await run(`$btn('Share', $team()).click()`);
  const shared = await waitFor(`!!$btn('Sync now', $team())`, 30000);
  const status = await run(`$team().querySelector('.status')?.textContent.trim()`);
  check(
    'Share pushes the collection and shows its status',
    shared && /Up to date/.test(status ?? ''),
    status,
  );
  check(
    'the header shows the team state',
    /Collaboration · synced/.test(await run(`$teamButton().getAttribute('aria-label')`)),
    await run(`$teamButton().getAttribute('aria-label').trim()`),
  );

  git(['clone', '-q', bare, mate]);
  const files = git(['ls-files'], mate).split('\n');
  const repoText = files.map((f) => fs.readFileSync(path.join(mate, f), 'utf8')).join('\n');
  check(
    'the repository has one file per request',
    files.includes('codestamp.json') &&
      files.includes('collection.json') &&
      files.filter((f) => f.startsWith('requests/')).length === 2,
    files.join(', '),
  );
  check(
    'secret values stay on this device',
    !repoText.includes(SECRET) &&
      readJson('collection.json').variables.some(
        (v) => v.key === 'token' && v.secret && v.value === '',
      ),
    short(readJson('collection.json').variables),
  );
  check(
    'the first commit is written as the user',
    git(['log', '-1', '--format=%an'], mate) === gitName,
    git(['log', '-1', '--format=%an · %s'], mate),
  );
  const reposEntry = fs.readdirSync(reposDir);
  check(
    'the local clone lives in the app data folder',
    reposEntry.length >= 1,
    reposEntry.join(', '),
  );

  // ---------- a teammate's changes arrive (open card reloads) ----------
  const listUsers = requestFileNamed('List users');
  writeJson(listUsers.file, { ...listUsers.value, url: 'https://api.test/v2/users' });
  const healthId = crypto.randomUUID();
  writeJson(`requests/${healthId}.json`, {
    ...listUsers.value,
    id: healthId,
    name: 'Health check',
    url: 'https://api.test/health',
  });
  const collection = readJson('collection.json');
  writeJson('collection.json', { ...collection, order: [...collection.order, healthId] });
  mateCommit('Mate: v2 users and a health check');

  await openRow('List users');
  await syncNow();
  const cardUrl = await run(`document.querySelector('input[id^="url-"]')?.value`);
  check(
    "a teammate's edit reloads the open card",
    cardUrl === 'https://api.test/v2/users',
    cardUrl,
  );
  check(
    "a teammate's new request appears",
    (await run(`$rows()`)).includes('Health check'),
    short(await run(`$rows()`)),
  );
  const incoming = await run(
    `[...$team().querySelectorAll('.muted')].map((p) => p.textContent.trim()).find((t) => t.startsWith('Last update')) ?? ''`,
  );
  check(
    'the panel says who changed what',
    incoming.includes('Mate') && incoming.includes('updated') && incoming.includes('added'),
    incoming,
  );
  await collapse();
  await closeTeam();

  // ---------- local edits wait for Sync now ----------
  const before = remoteHead();
  await openRow('Create user');
  await run(`$set(document.querySelector('input[id^="url-"]'), 'https://api.test/users/create')`);
  await sleep(300);
  await collapse();
  await sleep(6000);
  const waiting = await run(`$teamButton().getAttribute('aria-label').trim()`);
  check(
    'a saved edit is not pushed on its own and the header says so',
    remoteHead() === before && /not synced/.test(waiting),
    waiting,
  );
  await syncNow();
  const pushed = await waitUntil(() => remoteHead() !== before, 25000);
  git(['pull', '-q'], mate);
  const created = requestFileNamed('Create user');
  check(
    'Sync now pushes the edit',
    pushed &&
      created?.value.url === 'https://api.test/users/create' &&
      /synced/.test(await run(`$teamButton().getAttribute('aria-label')`)),
    created?.value.url,
  );
  check(
    'the commit says what changed and who changed it',
    git(['log', '-1', '--format=%an|%s'], mate) === `${gitName}|Update “Create user”`,
    git(['log', '-1', '--format=%an|%s'], mate),
  );

  // ---------- a chosen icon syncs both ways ----------
  const sidebarIcon = () =>
    run(
      `[...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === ${JSON.stringify(NAME)})?.querySelector('.icon')?.textContent.trim()`,
    );
  const beforeIcon = remoteHead();
  await run(`document.querySelector('.icon-tile').click()`);
  await sleep(500);
  await run(`$set(document.querySelector('[aria-label="Collection icon"] .search input'), 'fox')`);
  await sleep(200);
  await run(
    `document.querySelector('[aria-label="Collection icon"] .scroll .emoji-button').click()`,
  );
  await sleep(500);
  await syncNow();
  await closeTeam();
  const iconPushed = await waitUntil(() => remoteHead() !== beforeIcon, 25000);
  git(['pull', '-q'], mate);
  check(
    'a chosen icon is pushed in collection.json',
    iconPushed &&
      readJson('collection.json').icon === '🦊' &&
      git(['log', '-1', '--format=%s'], mate) === 'Update collection settings',
    `${readJson('collection.json').icon} · ${git(['log', '-1', '--format=%s'], mate)}`,
  );
  check(
    'the repository is written in format 7',
    readJson('codestamp.json').format === 7,
    JSON.stringify(readJson('codestamp.json')),
  );
  writeJson('collection.json', { ...readJson('collection.json'), icon: '🚀' });
  mateCommit('Mate: rocket icon');
  await syncNow();
  await closeTeam();
  const arrived = {
    tile: await run(`document.querySelector('.icon-tile')?.textContent.trim()`),
    sidebar: await sidebarIcon(),
  };
  check(
    "a teammate's icon arrives",
    arrived.tile === '🚀' && arrived.sidebar === '🚀',
    JSON.stringify(arrived),
  );

  // ---------- both rename the same request: conflict, keep both ----------
  writeJson(created.file, { ...created.value, name: 'Create member' });
  mateCommit('Mate: rename');
  await openRow('Create user');
  await run(`$set($card().querySelector('.name input'), 'Add user')`);
  await sleep(300);
  await collapse();
  await syncNow();
  const inPanel = await waitFor(`!!$btn('Resolve', $team())`, 25000);
  await closeTeam();
  const overlap = await waitFor(`!!document.querySelector('.overlap')`, 5000);
  check(
    'overlapping edits wait for a choice, in the panel and above the collection',
    inPanel &&
      overlap &&
      /1 to resolve/.test(await run(`$teamButton().getAttribute('aria-label')`)),
    await run(`$teamButton().getAttribute('aria-label').trim()`),
  );
  await run(`$btn('Resolve', document.querySelector('.overlap')).click()`);
  await sleep(500);
  const fields = await run(
    `[...$dialog().querySelectorAll('.grid')].map((g) => [...g.children].map((c) => c.textContent.trim()).slice(3))`,
  );
  check(
    'the conflict dialog shows both sides',
    short(fields) === short([['Name', 'Add user', 'Create member']]),
    short(fields),
  );
  await run(
    `[...$dialog().querySelectorAll('label.choice')].find((l) => l.textContent.includes('Keep both')).click()`,
  );
  await sleep(200);
  await run(`$btn('Apply and sync', $dialog()).click()`);
  await waitFor(`!document.querySelector('dialog[open]:not(.panel-dialog)')`, 30000);
  await sleep(800);
  const rowsAfter = await run(`$rows()`);
  check(
    'keep both: their name wins and mine becomes a copy',
    rowsAfter.includes('Create member') &&
      rowsAfter.includes('Add user (my copy)') &&
      !rowsAfter.includes('Add user'),
    short(rowsAfter),
  );
  git(['pull', '-q'], mate);
  check(
    'the merge and the copy reach the teammate',
    !!requestFileNamed('Add user (my copy)') &&
      git(['log', '-1', '--format=%s'], mate) === 'Merge changes from collaborators',
    git(['log', '-1', '--format=%s'], mate),
  );
  await waitFor(`!/syncing/.test($teamButton().getAttribute('aria-label'))`, 30000);
  check(
    'nothing is left to resolve',
    !(await run(`!!document.querySelector('.overlap')`)) &&
      /synced/.test(await run(`$teamButton().getAttribute('aria-label')`)),
    await run(`$teamButton().getAttribute('aria-label').trim()`),
  );
  // Idle: nothing syncs without Sync now, so no sync may start in 8 s.
  let syncStarts = 0;
  let wasSyncing = false;
  for (let i = 0; i < 32; i++) {
    const syncingNow = /syncing/.test(await run(`$teamButton().getAttribute('aria-label')`));
    if (syncingNow && !wasSyncing) syncStarts += 1;
    wasSyncing = syncingNow;
    await sleep(250);
  }
  check(
    'an idle collection does not sync on its own',
    syncStarts === 0,
    `${syncStarts} sync(s) in 8 s`,
  );

  // ---------- history and restore ----------
  await openRow('List users');
  await run(`$card().querySelector('button[aria-label="Request history"]').click()`);
  await waitFor(`!!$dialog()?.querySelector('.version')`);
  const versionCount = await run(`$dialog().querySelectorAll('.version').length`);
  await run(`[...$dialog().querySelectorAll('.version')].at(-1).click()`);
  await waitFor(`!!$dialog().querySelector('.grid')`);
  const diff = await run(
    `[...$dialog().querySelector('.grid').children].map((c) => c.textContent.trim()).slice(3)`,
  );
  check(
    'history lists versions and compares one with now',
    versionCount >= 2 &&
      short(diff) === short(['URL', 'https://api.test/users', 'https://api.test/v2/users']),
    `${versionCount} versions · ${short(diff)}`,
  );
  const beforeRestore = remoteHead();
  await run(`$btn('Restore this version', $dialog()).click()`);
  await waitFor(`!document.querySelector('dialog[open]:not(.panel-dialog)')`);
  const restoredUrl = await run(`document.querySelector('input[id^="url-"]')?.value`);
  await syncNow();
  await closeTeam();
  const restoredPushed = await waitUntil(() => remoteHead() !== beforeRestore, 25000);
  git(['pull', '-q'], mate);
  check(
    'restore brings the old version back and shares it',
    restoredUrl === 'https://api.test/users' &&
      restoredPushed &&
      requestFileNamed('List users')?.value.url === 'https://api.test/users',
    `${restoredUrl} · pushed ${restoredPushed}`,
  );
  await collapse();

  const settledHead = remoteHead();
  await syncNow();
  check('a sync without changes adds no commit', remoteHead() === settledHead);
  await closeTeam();

  // ---------- join: refused for bad addresses and for a collection already here ----------
  await run(`$nav('Import').click()`);
  await sleep(700);
  const joinPanel = `document.querySelector('section[aria-labelledby="join-title"]')`;
  await run(`$set(${joinPanel}.querySelector('input.mono'), 'ext::calc.exe')`);
  await sleep(200);
  await run(`$btn('Join', ${joinPanel}).click()`);
  await waitFor(`!!${joinPanel}.querySelector('.error')`);
  check(
    'join refuses an ext:: address',
    /https:\/\/ or SSH/.test(await run(`${joinPanel}.querySelector('.error')?.textContent ?? ''`)),
    await run(`${joinPanel}.querySelector('.error')?.textContent.trim()`),
  );
  await run(`$set(${joinPanel}.querySelector('input.mono'), ${JSON.stringify(bare)})`);
  await sleep(200);
  await run(`$btn('Join', ${joinPanel}).click()`);
  await waitFor(
    `/already in Codestamp/.test(${joinPanel}.querySelector('.error')?.textContent ?? '')`,
    30000,
  );
  check(
    'join refuses a collection that is already on this device',
    /already in Codestamp/.test(
      await run(`${joinPanel}.querySelector('.error')?.textContent ?? ''`),
    ),
    await run(`${joinPanel}.querySelector('.error')?.textContent.trim()`),
  );
  check(
    'the refused join leaves no folder behind',
    !fs.readdirSync(reposDir).some((name) => name.startsWith('joining-')),
    fs.readdirSync(reposDir).join(', '),
  );

  // ---------- stop syncing ----------
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  await openTeam();
  await run(`$btn('Stop syncing', $team()).click()`);
  await sleep(300);
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Stop syncing').click()`,
  );
  await waitFor(`!!$btn('Share', $team())`);
  const left = fs.readdirSync(reposDir);
  check(
    'stop syncing keeps the collection and removes the clone',
    (await run(`$rows()`)).length === 4 && left.length === 0,
    `${short(await run(`$rows()`))} · repos: ${left.join(', ')}`,
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(
      `document.querySelector('dialog[open]:not(.panel-dialog) button[aria-label="Close"]')?.click()`,
    );
    if (await run(`!!$nav(${JSON.stringify(NAME)})`)) {
      await run(`$nav(${JSON.stringify(NAME)}).click()`);
      await sleep(900);
      if (
        (await run(`document.querySelector('input[aria-label="Collection name"]')?.value`)) === NAME
      ) {
        await run(`$btn('Delete collection').click()`);
        await sleep(300);
        await run(
          `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Delete').click()`,
        );
        await sleep(900);
      }
    }
    check('test data cleaned up', !(await run(`!!$nav(${JSON.stringify(NAME)})`)));
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  fs.rmSync(work, { recursive: true, force: true });
  process.exit(0);
}
