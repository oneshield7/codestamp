// End-to-end check of per-collection Git accounts in the Collaboration panel.
// Two local bare repositories stand in for two Git hosts; commits show which author each used.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const A = 'E2E Account A';
const B = 'E2E Account B';
const C = 'E2E Account C';
const hosts = path.join(path.dirname(fileURLToPath(import.meta.url)), '.work', 'git-hosts');
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'cs-e2e-account-'));
const bareA = path.join(work, 'a.git');
const bareB = path.join(work, 'b.git');
const reposDir = path.join(process.env.APPDATA, 'dev.codestamp.desktop', 'repos');

const git = (args) =>
  execFileSync('git', args, {
    cwd: work,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
const lastAuthor = (bare) => git(['--git-dir', bare, 'log', '-1', '--format=%an <%ae>', 'main']);
const commits = (bare) => Number(git(['--git-dir', bare, 'rev-list', '--count', 'main']));

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
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$team = () => document.querySelector('section[aria-label="Collaboration"]');
  window.$teamButton = () => document.querySelector('header.top button[aria-label^="Collaboration"]');
  window.$field = (label, root = $team()) => [...root.querySelectorAll('label')].find((l) => l.textContent.trim().startsWith(label))?.querySelector('input');
  window.$facts = () => Object.fromEntries([...$team().querySelectorAll('dt')].map((dt) => [dt.textContent.trim(), dt.nextElementSibling.textContent.replace(/\\s+/g, ' ').trim()]));
  window.$accountPicker = () => $team().querySelector('button[aria-label="Git account"]');
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 100)); };
  window.$error = () => $team()?.querySelector('.error')?.textContent.trim() ?? '';
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 220);
const ids = {};

async function open(name) {
  await run(`$nav(${JSON.stringify(name)}).click()`);
  await sleep(900);
  if (!(await run(`!!$team()`))) {
    await run(`$teamButton().click()`);
    await sleep(500);
  }
}
async function fillShare(remote, username, name, email) {
  await run(`$set($field('Repository address'), ${JSON.stringify(remote)})`);
  await sleep(150);
  if (username !== null) await run(`$set($field('Git account'), ${JSON.stringify(username)})`);
  await run(`$set($field('Your name'), ${JSON.stringify(name)})`);
  await run(`$set($field('Email'), ${JSON.stringify(email)})`);
  await sleep(150);
}
async function syncNow() {
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  await run(`$btn('Sync now', $team()).click()`);
  await sleep(400);
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  await sleep(400);
}
async function rename(name) {
  await run(
    `(() => { const t = document.querySelector('input[aria-label="Collection name"]'); $set(t, ${JSON.stringify(name)}); t.dispatchEvent(new Event('blur')); })()`,
  );
  await sleep(600);
}

git(['init', '-q', '--bare', '--initial-branch', 'main', bareA]);
git(['init', '-q', '--bare', '--initial-branch', 'main', bareB]);
await connect();
try {
  // The last share below uses the profile found here, so the suite leaves the profile as it was.
  const profile = (await invoke('team_setup', {})).ok?.author ?? {
    name: 'Ana Work',
    email: 'ana@acme.test',
  };
  const request = {
    kind: 'http',
    name: 'Health',
    method: 'GET',
    url: 'https://api.test/health',
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
  for (const [key, name] of [
    ['a', A],
    ['b', B],
  ]) {
    const created = await invoke('create_collection', { name, icon: null, baseUrl: null });
    ids[key] = created.ok.id;
    await invoke('create_request', { collectionId: ids[key], sectionId: null, content: request });
  }
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);

  // ---------- the share form asks for an account only where one is used ----------
  await open(A);
  const form = async (remote) => {
    await run(`$set($field('Repository address'), ${JSON.stringify(remote)})`);
    await sleep(150);
    return run(
      `({ account: !!$field('Git account'), hint: [...$team().querySelectorAll('.hint')].map((h) => h.textContent.trim()).find((t) => t.startsWith('The username')) ?? null })`,
    );
  };
  const folder = await form(bareA);
  const https = await form('https://github.com/acme/shop-api.git');
  const ssh = await form('git@github.com:acme/shop-api.git');
  check(
    'the Git account field shows for https addresses only',
    !folder.account && https.account && !ssh.account,
    short({ folder: folder.account, https: https.account, ssh: ssh.account }),
  );
  check(
    'the field says what it is for and what empty means',
    /signs in with for this collection/.test(https.hint ?? '') &&
      /Leave it empty/.test(https.hint ?? ''),
    https.hint,
  );

  await fillShare('https://github.com/acme/shop-api.git', 'ana work', 'Ana Work', 'ana@acme.test');
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`$error() !== ''`, 10000);
  check(
    'a username with a space is refused before Git runs',
    /without spaces/.test(await run(`$error()`)),
    await run(`$error()`),
  );

  await fillShare('https://127.0.0.1:9/acme/shop-api.git', 'ana-work', 'Ana Work', 'ana@acme.test');
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`$error() !== '' && !/without spaces/.test($error())`, 60000);
  const unreachable = await run(`$error()`);
  check(
    'a valid username goes on to Git, which reports the unreachable host',
    /Could not reach|Git failed/.test(unreachable) && !/username|account/i.test(unreachable),
    unreachable,
  );
  check(
    'the failed share leaves no link and no clone',
    !(await run(`!!$btn('Sync now', $team())`)) && !fs.existsSync(path.join(reposDir, ids.a)),
    fs.existsSync(reposDir) ? fs.readdirSync(reposDir).join(', ') : 'no repos folder',
  );

  // ---------- two collections, two authors ----------
  await fillShare(bareA, null, 'Ana Work', 'ana@acme.test');
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`!!$btn('Sync now', $team())`, 30000);
  const factsA = await run(`$facts()`);
  check(
    'a shared collection shows the author it commits as',
    factsA['Your changes as'] === 'Ana Work · ana@acme.test' && !('Signs in as' in factsA),
    short(factsA),
  );
  check(
    'collection A is pushed as its author',
    lastAuthor(bareA) === 'Ana Work <ana@acme.test>',
    lastAuthor(bareA),
  );

  await open(B);
  const prefill = await run(`[$field('Your name').value, $field('Email').value]`);
  check(
    'the next share starts from the last author used',
    prefill[0] === 'Ana Work' && prefill[1] === 'ana@acme.test',
    short(prefill),
  );
  await fillShare(bareB, null, 'Ana Home', 'ana@home.test');
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`!!$btn('Sync now', $team())`, 30000);
  check(
    'collection B is pushed as its own author',
    lastAuthor(bareB) === 'Ana Home <ana@home.test>' &&
      lastAuthor(bareA) === 'Ana Work <ana@acme.test>',
    `${lastAuthor(bareA)} | ${lastAuthor(bareB)}`,
  );

  // ---------- change the account of one collection ----------
  await run(`$btn('Change account', $team()).click()`);
  await sleep(300);
  const editing = await run(
    `({ title: $team().querySelector('#account-title')?.textContent.trim(), name: $field('Your name').value, email: $field('Email').value, account: !!$field('Git account'), facts: $team().querySelectorAll('dt').length })`,
  );
  check(
    'Change account opens a form filled with this collection’s account',
    editing.title === `Account for “${B}”` &&
      editing.name === 'Ana Home' &&
      editing.email === 'ana@home.test' &&
      !editing.account &&
      editing.facts === 2,
    short(editing),
  );
  await run(`$set($field('Your name'), '')`);
  await sleep(100);
  check('Save waits for a name', await run(`$btn('Save', $team()).disabled`));
  await run(`$set($field('Your name'), 'Ana at Home')`);
  await sleep(100);
  await run(`$btn('Save', $team()).click()`);
  await waitFor(`!!$btn('Change account', $team())`);
  const factsB = await run(`$facts()`);
  check(
    'Save shows the new account',
    factsB['Your changes as'] === 'Ana at Home · ana@home.test',
    short(factsB),
  );

  const beforeB = commits(bareB);
  await rename(`${B} v2`);
  await syncNow();
  check(
    'the next sync of B commits as the new account',
    commits(bareB) === beforeB + 1 && lastAuthor(bareB) === 'Ana at Home <ana@home.test>',
    `${commits(bareB)} · ${lastAuthor(bareB)}`,
  );

  await open(A);
  check(
    'collection A keeps its own account',
    (await run(`$facts()`))['Your changes as'] === 'Ana Work · ana@acme.test',
    short(await run(`$facts()`)),
  );
  const beforeA = commits(bareA);
  await rename(`${A} v2`);
  await syncNow();
  check(
    'and its next sync still commits as Ana Work',
    commits(bareA) === beforeA + 1 && lastAuthor(bareA) === 'Ana Work <ana@acme.test>',
    `${commits(bareA)} · ${lastAuthor(bareA)}`,
  );

  // ---------- an https collection with its own sign-in account ----------
  // launch.mjs maps https://git.test/ to .work/git-hosts, standing in for a real Git host.
  const hostRepo = path.join(hosts, 'acme', 'shop.git');
  fs.mkdirSync(path.dirname(hostRepo), { recursive: true });
  git(['init', '-q', '--bare', '--initial-branch', 'main', hostRepo]);
  const created = await invoke('create_collection', { name: C, icon: null, baseUrl: null });
  ids.c = created.ok.id;
  await invoke('create_request', { collectionId: ids.c, sectionId: null, content: request });
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await open(C);
  await fillShare('https://git.test/acme/shop.git', ' ana-work ', profile.name, profile.email);
  await run(`$btn('Share', $team()).click()`);
  const sharedHttps = await waitFor(`!!$btn('Sync now', $team())`, 30000);
  const factsC = sharedHttps ? await run(`$facts()`) : { error: await run(`$error()`) };
  check(
    'an https collection shares with its own Git account',
    sharedHttps &&
      factsC['Signs in as'] === 'ana-work' &&
      commits(hostRepo) === 1 &&
      lastAuthor(hostRepo) === `${profile.name} <${profile.email}>`,
    short(factsC),
  );

  await run(`$btn('Change account', $team()).click()`);
  await sleep(300);
  const editC = await run(
    `({ account: $accountPicker()?.textContent.trim(), name: $field('Your name').value })`,
  );
  check(
    'its account form picks the Git account from the saved list',
    editC.account === 'ana-work' && editC.name === profile.name,
    short(editC),
  );
  const syncedBefore = (await invoke('list_team_links', {})).ok.find(
    (l) => l.collectionId === ids.c,
  ).syncedAt;
  await sleep(1100);
  await run(`$pick($accountPicker(), 'Another account…')`);
  await sleep(150);
  const typing = await run(`!!$field('Git account') && !$accountPicker()`);
  await run(`$set($field('Git account'), 'ana-home')`);
  await sleep(100);
  await run(`$btn('Save', $team()).click()`);
  await waitFor(`!!$btn('Change account', $team())`);
  await sleep(1500);
  const savedOnly = (await invoke('list_team_links', {})).ok.find((l) => l.collectionId === ids.c);
  check('Another account… switches to a text box', typing);
  check(
    'a new Git account is saved without syncing on its own',
    savedOnly.username === 'ana-home' &&
      savedOnly.syncedAt === syncedBefore &&
      (await run(`$facts()`))['Signs in as'] === 'ana-home',
    short({ username: savedOnly.username, before: syncedBefore, after: savedOnly.syncedAt }),
  );
  await run(`$btn('Sync now', $team()).click()`);
  await sleep(300);
  await waitFor(`!!$btn('Sync now', $team()) && !$btn('Sync now', $team()).disabled`, 30000);
  const syncedAfter = (await invoke('list_team_links', {})).ok.find(
    (l) => l.collectionId === ids.c,
  );
  check(
    'Sync now then uses the new account',
    syncedAfter.syncedAt > syncedBefore,
    short({ before: syncedBefore, after: syncedAfter.syncedAt }),
  );

  await run(`$btn('Change account', $team()).click()`);
  await sleep(300);
  const listed = await run(
    `(async () => { $accountPicker().click(); await new Promise((r) => setTimeout(r, 150)); const options = [...document.querySelectorAll('[role="listbox"] [role="option"]')]; const names = options.map((o) => o.textContent.trim()); options[0].dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 150)); return names; })()`,
  );
  check(
    'the list holds both accounts of this host',
    short(listed) === short(["Git's default account", 'ana-home', 'ana-work', 'Another account…']),
    short(listed),
  );
  await run(`$btn('Save', $team()).click()`);
  await waitFor(`!!$btn('Change account', $team())`);
  const cleared = await run(`$facts()['Signs in as']`);
  check(
    'an empty Git account leaves the choice to Git',
    cleared === "Git's default account for this host",
    cleared,
  );

  // ---------- the backend guards the account as well ----------
  const refused = await invoke('set_team_account', {
    collectionId: ids.a,
    account: { author: { name: 'Ana', email: 'ana@acme.test' }, username: 'ana-work' },
  });
  check(
    'a username for a folder repository is refused',
    /Only https:\/\/ addresses use a Git account/.test(refused.error ?? ''),
    short(refused),
  );
  const badEmail = await invoke('set_team_account', {
    collectionId: ids.a,
    account: { author: { name: 'Ana', email: 'not-an-email' }, username: null },
  });
  check('an invalid email is refused', /valid email/.test(badEmail.error ?? ''), short(badEmail));
  const links = (await invoke('list_team_links', {})).ok ?? [];
  const mine = links
    .filter((l) => l.collectionId === ids.a || l.collectionId === ids.b)
    .map((l) => ({ author: l.author?.name, username: l.username }));
  check(
    'each link reports its own author',
    short(mine) ===
      short([
        { author: 'Ana Work', username: null },
        { author: 'Ana at Home', username: null },
      ]),
    short(mine),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    for (const id of Object.values(ids)) {
      await invoke('unlink_team_collection', { collectionId: id });
      await invoke('delete_collection', { id });
    }
    for (const account of (await invoke('list_git_accounts', {})).ok ?? []) {
      if (account.host === 'git.test' && account.id)
        await invoke('remove_git_account', { id: account.id });
    }
    await run('location.reload()');
    await sleep(1500);
    const left = (await invoke('list_collections', {})).ok ?? [];
    check(
      'test data cleaned up',
      !left.some((c) => Object.values(ids).includes(c.id)) &&
        !Object.values(ids).some((id) => fs.existsSync(path.join(reposDir, id))),
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
