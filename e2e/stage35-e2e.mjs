// End-to-end check of the 2026-10-27 Git fixes: Sync now rebuilds a clone that lost its .git, and
// a collection whose sign-in is gone offers Sign in. https://auth.test/ goes to a local Git HTTP
// server that answers 401 until the test "signs in"; Git's credential helpers are switched off
// for the test copy, so no sign-in window opens on the desktop. The suite launches the app itself.
import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const AUTH_PORT = 9466;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const hosts = path.join(here, '.work', 'git-hosts');
const bare = path.join(hosts, 'team', 'repair.git');
const authRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'cs-e2e-auth-'));
const authBare = path.join(authRoot, 'team', 'auth.git');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'cs-e2e-repair-'));
const mate = path.join(work, 'mate');
const reposDir = path.join(process.env.APPDATA, 'dev.codestamp.desktop', 'repos');
const git = (args, cwd = work) =>
  execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
const commits = (dir) => Number(git(['--git-dir', dir, 'rev-list', '--count', 'main']));

// A Git smart-HTTP server (git http-backend) that refuses everyone until `signedIn` is true.
let signedIn = true;
const authServer = http.createServer((request, response) => {
  if (!signedIn) {
    response.writeHead(401, { 'WWW-Authenticate': 'Basic realm="e2e"' }).end();
    return;
  }
  const url = new URL(request.url, 'http://local');
  const backend = spawn('git', ['http-backend'], {
    env: {
      ...process.env,
      GIT_PROJECT_ROOT: authRoot,
      GIT_HTTP_EXPORT_ALL: '1',
      PATH_INFO: url.pathname,
      QUERY_STRING: url.search.slice(1),
      REQUEST_METHOD: request.method,
      CONTENT_TYPE: request.headers['content-type'] ?? '',
      HTTP_CONTENT_ENCODING: request.headers['content-encoding'] ?? '',
      GIT_PROTOCOL: request.headers['git-protocol'] ?? '',
      REMOTE_USER: 'ana',
      REMOTE_ADDR: '127.0.0.1',
    },
  });
  request.pipe(backend.stdin);
  let head = Buffer.alloc(0);
  let started = false;
  backend.stdout.on('data', (chunk) => {
    if (started) {
      response.write(chunk);
      return;
    }
    head = Buffer.concat([head, chunk]);
    const end = head.indexOf('\r\n\r\n');
    if (end < 0) return;
    const headers = {};
    let status = 200;
    for (const line of head.subarray(0, end).toString().split('\r\n')) {
      const at = line.indexOf(':');
      const name = line.slice(0, at).trim();
      const value = line.slice(at + 1).trim();
      if (name.toLowerCase() === 'status') status = Number(value.split(' ')[0]);
      else headers[name] = value;
    }
    response.writeHead(status, headers);
    response.write(head.subarray(end + 4));
    started = true;
  });
  backend.stdout.on('end', () => response.end());
});

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
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim() === text || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$teamButton = () => document.querySelector('header.top button[aria-label^="Collaboration"]');
  window.$team = () => document.querySelector('section[aria-label="Collaboration"]');
  window.$status = () => $team()?.querySelector('.status')?.textContent.replace(/\\s+/g, ' ').trim() ?? null;
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const created = [];
const request = (name, url) => ({
  kind: 'http',
  name,
  method: 'GET',
  url,
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
});

async function reloadApp() {
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
  await waitFor(`!!document.querySelector('nav button.new')`);
  await sleep(500);
}
async function openCollection(name) {
  await waitFor(`!!$nav(${JSON.stringify(name)})`);
  await run(`$nav(${JSON.stringify(name)}).click()`);
  await waitFor(
    `document.querySelector('input[aria-label="Collection name"]')?.value === ${JSON.stringify(name)}`,
  );
  await sleep(400);
}
async function openTeam() {
  if (!(await run(`!!$team()`))) await run(`$teamButton().click()`);
  await waitFor(`!!$team() && !!$btn('Sync now', $team())`);
}
async function syncNow() {
  await openTeam();
  await waitFor(`$btn('Sync now', $team())?.disabled === false`, 30000);
  await run(`$btn('Sync now', $team()).click()`);
  await sleep(300);
  await waitFor(`$btn('Sync now', $team())?.disabled === false`, 60000);
  await sleep(400);
}
async function edit(requestId, url) {
  const saved = (await invoke('get_request', { id: requestId })).ok;
  const { id, collectionId, sectionId, ...content } = saved;
  await invoke('update_request', { id: requestId, content: { ...content, url } });
}

fs.mkdirSync(path.dirname(bare), { recursive: true });
fs.mkdirSync(path.dirname(authBare), { recursive: true });
git(['init', '-q', '--bare', '--initial-branch', 'main', bare]);
git(['init', '-q', '--bare', '--initial-branch', 'main', authBare]);
await new Promise((resolve) => authServer.listen(AUTH_PORT, '127.0.0.1', resolve));
execFileSync(process.execPath, [path.join(here, 'launch.mjs')], {
  stdio: 'inherit',
  env: {
    ...process.env,
    E2E_GIT_CONFIG: JSON.stringify([
      ['url.http://127.0.0.1:9466/.insteadOf', 'https://auth.test/'],
      ['credential.helper', ''],
    ]),
  },
});
await connect();
try {
  const profile = (await invoke('team_setup', {})).ok?.author ?? {
    name: 'Ana Work',
    email: 'ana@acme.test',
  };
  const account = { author: profile, username: null };

  // ---------- a clone that lost its .git is rebuilt by Sync now ----------
  const repairId = (
    await invoke('create_collection', { name: 'E2E Repair', icon: null, baseUrl: null })
  ).ok.id;
  created.push(repairId);
  const login = (
    await invoke('create_request', {
      collectionId: repairId,
      sectionId: null,
      content: request('Login', 'https://api.test/login'),
    })
  ).ok.id;
  const users = (
    await invoke('create_request', {
      collectionId: repairId,
      sectionId: null,
      content: request('Users', 'https://api.test/users'),
    })
  ).ok.id;
  const shared = await invoke('share_collection', {
    collectionId: repairId,
    remote: 'https://git.test/team/repair.git',
    account,
    replaceHead: null,
  });
  check('the collection is shared', !!shared.ok, short(shared.error ?? 'ok'));
  git(['clone', '-q', bare, mate]);
  const usersFile = path.join(mate, 'requests', `${users}.json`);
  const usersJson = JSON.parse(fs.readFileSync(usersFile, 'utf8'));
  fs.writeFileSync(
    usersFile,
    `${JSON.stringify({ ...usersJson, url: 'https://api.test/people' }, null, 2)}\n`,
  );
  git(
    ['-c', 'user.name=Mate', '-c', 'user.email=mate@example.com', 'commit', '-qam', 'Mate: people'],
    mate,
  );
  git(['push', '-q', 'origin', 'main'], mate);
  await edit(login, 'https://api.test/sign-in');
  fs.rmSync(path.join(reposDir, repairId, '.git'), { recursive: true, force: true });
  await reloadApp();
  await openCollection('E2E Repair');
  const before = commits(bare);
  await syncNow();
  const status = await run(`$status()`);
  git(['pull', '-q'], mate);
  const pushedLogin = JSON.parse(
    fs.readFileSync(path.join(mate, 'requests', `${login}.json`), 'utf8'),
  ).url;
  const arrived = (await invoke('get_request', { id: users })).ok?.url;
  check(
    'Sync now rebuilds the lost clone and syncs',
    /Up to date/.test(status ?? '') && fs.existsSync(path.join(reposDir, repairId, '.git')),
    String(status),
  );
  check(
    "the change saved here is sent and the collaborator's arrives",
    pushedLogin === 'https://api.test/sign-in' &&
      arrived === 'https://api.test/people' &&
      commits(bare) > before,
    short({ pushedLogin, arrived }),
  );
  check(
    'no temporary clone is left behind',
    !fs.readdirSync(reposDir).some((name) => name.startsWith('joining-')),
    fs.readdirSync(reposDir).join(', '),
  );

  // ---------- a lost sign-in offers Sign in ----------
  const authId = (
    await invoke('create_collection', { name: 'E2E Sign In', icon: null, baseUrl: null })
  ).ok.id;
  created.push(authId);
  const ping = (
    await invoke('create_request', {
      collectionId: authId,
      sectionId: null,
      content: request('Ping', 'https://api.test/ping'),
    })
  ).ok.id;
  const authShared = await invoke('share_collection', {
    collectionId: authId,
    remote: 'https://auth.test/team/auth.git',
    account,
    replaceHead: null,
  });
  check(
    'a collection shares over the HTTP test host',
    !!authShared.ok && commits(authBare) === 1,
    short(authShared.error ?? 'ok'),
  );
  signedIn = false;
  await edit(ping, 'https://api.test/ping2');
  await reloadApp();
  await openCollection('E2E Sign In');
  await syncNow();
  const refused = await run(
    `({ status: $status(), header: $teamButton().getAttribute('aria-label'), button: $btn('Sign in', $team())?.textContent.trim() ?? null })`,
  );
  check(
    'a missing sign-in shows sign in with a Sign in button',
    /sign in/.test(refused.header) &&
      /Git could not sign in\..*Sign in opens Git's sign-in window\./.test(refused.status ?? '') &&
      refused.button === 'Sign in',
    short(refused),
  );
  await run(`$btn('Sign in', $team()).click()`);
  await waitFor(
    `$btn('Sign in', $team())?.textContent.trim() === 'Sign in' && !!$team().querySelector('.error')`,
    30000,
  );
  const stillRefused = await run(`$team().querySelector('.error')?.textContent.trim() ?? null`);
  check(
    'Sign in that Git refuses says so',
    /^Git could not sign in/.test(stillRefused ?? '') && commits(authBare) === 1,
    String(stillRefused),
  );
  signedIn = true;
  await run(`$btn('Sign in', $team()).click()`);
  await waitFor(`/Up to date/.test($status() ?? '')`, 60000);
  const after = await run(
    `({ status: $status(), header: $teamButton().getAttribute('aria-label'), button: !!$btn('Sign in', $team()), error: !!$team().querySelector('.error') })`,
  );
  check(
    'once signed in, Sign in syncs and the button goes away',
    /Up to date/.test(after.status ?? '') &&
      /synced/.test(after.header) &&
      !after.button &&
      !after.error &&
      commits(authBare) === 2,
    short(after),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
  results.push(
    `      page: ${short(await run(`({ title: document.querySelector('input[aria-label="Collection name"]')?.value ?? null, header: $teamButton()?.getAttribute('aria-label') ?? null, panel: !!$team(), status: $status(), buttons: $team() ? [...$team().querySelectorAll('button')].map((b) => b.textContent.trim()) : null, banner: document.querySelector('main .banner')?.textContent.trim() ?? null })`).catch(() => null))}`,
  );
} finally {
  try {
    signedIn = true;
    for (const id of created) {
      await invoke('unlink_team_collection', { collectionId: id });
      await invoke('delete_collection', { id });
    }
    await run('location.reload()');
    await sleep(1500);
    const left = (await invoke('list_collections', {})).ok ?? [];
    check('test data cleaned up', !left.some((c) => created.includes(c.id)));
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket?.close();
  authServer.close();
  fs.rmSync(work, { recursive: true, force: true });
  fs.rmSync(authRoot, { recursive: true, force: true });
  fs.rmSync(hosts, { recursive: true, force: true });
  process.exit(0);
}
