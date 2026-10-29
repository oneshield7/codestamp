// End-to-end check of Settings › Git accounts, Tools › Git changes and Import › replace a collection.
// launch.mjs maps https://git.test/ to bare repositories in .work/git-hosts, standing in for a Git host.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Changes';
const REMOTE = 'https://git.test/acme/changes.git';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const hosts = path.join(path.dirname(fileURLToPath(import.meta.url)), '.work', 'git-hosts');
const bare = path.join(hosts, 'acme', 'changes.git');
const reposDir = path.join(process.env.APPDATA, 'dev.codestamp.desktop', 'repos');
const git = (args) =>
  execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
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
async function waitFor(expression, timeout = 15000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(250);
  }
  return false;
}
async function waitUntil(test, timeout = 30000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (test()) return true;
    await sleep(400);
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
  window.$goTo = (text) => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); };
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim().endsWith(label)); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 100)); };
  window.$accounts = () => document.querySelector('section[aria-labelledby="git-accounts-title"]');
  window.$row = (username) => [...$accounts().querySelectorAll('li.account')].find((li) => li.querySelector('.name')?.textContent.trim() === username);
  window.$rowState = (username) => { const li = $row(username); return li ? { chips: [...li.querySelectorAll('.chip')].map((c) => c.textContent.trim()), buttons: [...li.querySelectorAll('button')].map((b) => b.textContent.trim()), used: [...li.querySelectorAll('.used')].map((p) => p.textContent.trim()), label: li.querySelector('.label')?.textContent.trim() ?? '' } : null; };
  window.$formField = (label) => [...document.querySelectorAll('form[aria-label="Add a Git account"] label')].find((l) => l.textContent.trim().startsWith(label))?.querySelector('input');
  window.$team = () => document.querySelector('section[aria-label="Collaboration"]');
  window.$page = () => [...document.querySelectorAll('h1')].find((h) => h.textContent.trim() === 'Git changes')?.closest('.view');
  window.$tab = (name) => [...document.querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim() === name);
  window.$detail = () => document.querySelector('section[aria-label^="Commit "]');
  window.$files = () => [...($detail()?.querySelectorAll('li.file') ?? [])].map((li) => ({ change: li.querySelector('.chip')?.textContent.trim(), name: li.querySelector('.name')?.textContent.trim(), fields: [...li.querySelectorAll('.grid .label')].map((l) => l.textContent.trim()) }));
  window.$rows = () => [...document.querySelectorAll('.row .name')].map((n) => n.textContent.trim());
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 240);
let collectionId = null;

async function openSettings() {
  await run(`$goTo('Settings')`);
  await sleep(900);
  await run(
    `[...document.querySelectorAll('nav[aria-label="Settings sections"] button')].find((b) => b.textContent.trim() === 'Git accounts').click()`,
  );
  await waitFor(`!!$accounts() && !$accounts().textContent.includes('Loading accounts')`);
  await sleep(300);
}

fs.mkdirSync(path.dirname(bare), { recursive: true });
git(['init', '-q', '--bare', '--initial-branch', 'main', bare]);
await connect();
try {
  // A shared collection that signs in as ana-work, committed under the profile so it stays as it was.
  const profile = (await invoke('team_setup', {})).ok?.author ?? {
    name: 'Ana Work',
    email: 'ana@acme.test',
  };
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
  collectionId = (await invoke('create_collection', { name: NAME, icon: null, baseUrl: null })).ok
    .id;
  const health = (
    await invoke('create_request', {
      collectionId,
      sectionId: null,
      content: request('Health', 'https://api.test/health'),
    })
  ).ok;
  await invoke('create_request', {
    collectionId,
    sectionId: null,
    content: request('Users', 'https://api.test/users'),
  });
  const shared = await invoke('share_collection', {
    collectionId,
    remote: REMOTE,
    account: { author: profile, username: 'ana-work' },
  });
  check(
    'the collection is shared over the stand-in https host',
    !!shared.ok && commits() === 1,
    short(shared.error ?? shared.ok?.remote),
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);

  // ---------- Settings › Git accounts ----------
  await openSettings();
  const work = await run(`$rowState('ana-work')`);
  check(
    'the account a collection uses is listed with it',
    work?.chips.includes('1 collection') &&
      work.used.includes(`Used by ${NAME}`) &&
      !work.buttons.includes('Remove') &&
      work.buttons.includes('Check'),
    short(work),
  );

  await run(`$btn('Add account', $accounts()).click()`);
  await sleep(200);
  await run(`$set($formField('Host'), 'git.test')`);
  await run(`$set($formField('Username'), 'ana-home')`);
  await run(`$set($formField('Label'), 'Home')`);
  await sleep(100);
  await run(
    `document.querySelector('form[aria-label="Add a Git account"] button[type="submit"]').click()`,
  );
  await waitFor(`!!$row('ana-home')`);
  const home = await run(`$rowState('ana-home')`);
  check(
    'Add account adds a labelled account not used yet',
    home?.label === 'Home' &&
      home.chips.includes('Not used yet') &&
      home.buttons.includes('Remove') &&
      !home.buttons.includes('Check'),
    short(home),
  );

  await run(`$btn('Add account', $accounts()).click()`);
  await sleep(200);
  await run(`$set($formField('Host'), 'git.test')`);
  await run(`$set($formField('Username'), 'ana-home')`);
  await sleep(100);
  await run(
    `document.querySelector('form[aria-label="Add a Git account"] button[type="submit"]').click()`,
  );
  await waitFor(`!!$accounts().querySelector('.error')`);
  check(
    'the same account cannot be added twice',
    /already in the list/.test(await run(`$accounts().querySelector('.error')?.textContent ?? ''`)),
    await run(`$accounts().querySelector('.error')?.textContent.trim()`),
  );
  await run(`$btn('Cancel', $accounts()).click()`);

  await run(`$btn('Check', $row('ana-work')).click()`);
  await waitFor(`$rowState('ana-work').chips.includes('Signed in')`, 30000);
  check(
    'Check reaches the repository with that account',
    (await run(`$rowState('ana-work')`)).chips.includes('Signed in'),
    short(await run(`$rowState('ana-work')`)),
  );

  // ---------- Collaboration picks from the list ----------
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  await run(`document.querySelector('header.top button[aria-label^="Collaboration"]').click()`);
  await sleep(500);
  await run(`$btn('Change account', $team()).click()`);
  await sleep(400);
  const picker = await run(
    `(async () => { const trigger = $team().querySelector('button[aria-label="Git account"]'); if (!trigger) return null; const picked = trigger.textContent.trim(); trigger.click(); await new Promise((r) => setTimeout(r, 150)); const names = [...document.querySelectorAll('[role="listbox"] [role="option"]')].map((o) => o.textContent.trim()); trigger.click(); await new Promise((r) => setTimeout(r, 150)); return { picked, names }; })()`,
  );
  check(
    'Change account offers the saved accounts of the host',
    picker?.picked === 'ana-work' &&
      short(picker.names) ===
        short(["Git's default account", 'ana-home · Home', 'ana-work', 'Another account…']),
    short(picker),
  );
  await run(`$btn('Cancel', $team()).click()`);
  await sleep(200);
  await openSettings();
  await run(`$btn('Remove', $row('ana-home')).click()`);
  await waitFor(`!$row('ana-home')`);
  check('an unused account can be removed', !(await run(`!!$row('ana-home')`)));

  // ---------- Tools › Git changes ----------
  // get_request answers with the content fields beside the ids; update_request takes the content only.
  const {
    id: _id,
    collectionId: _collection,
    sectionId: _section,
    ...content
  } = (await invoke('get_request', { id: health.id })).ok;
  const edited = await invoke('update_request', {
    id: health.id,
    content: { ...content, url: 'https://api.test/v2/health' },
  });
  if (edited.error) throw new Error(`update_request: ${edited.error}`);
  await run(`$nav('Git changes').click()`);
  await waitFor(`!!$page() && !!$detail()`, 20000);
  const header = await run(
    `({ picked: $page().querySelector('button[aria-label="Shared collection"]')?.textContent.trim(), chips: [...$page().querySelectorAll('.bar .chip')].map((c) => c.textContent.replace(/\\s+/g, ' ').trim()), tabs: [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent.trim()) })`,
  );
  check(
    'Git changes opens on the shared collection with its local edit',
    header.picked?.endsWith(NAME) &&
      header.chips.includes('1 local edit') &&
      short(header.tabs) === short(['Commits', 'Requests']),
    short(header),
  );

  const before = commits();
  await run(`$btn('Sync now', $page()).click()`);
  await waitUntil(() => commits() === before + 1);
  await waitFor(
    `[...$page().querySelectorAll('button.commit .message')][0]?.textContent.includes('Health')`,
    20000,
  );
  await run(`$page().querySelector('button.commit').click()`);
  await waitFor(`$detail()?.querySelector('h3')?.textContent.startsWith('Update')`);
  const files = await run(`$files()`);
  const listed = await run(
    `({ day: $page().querySelector('h2.day')?.textContent.trim(), count: $page().querySelectorAll('button.commit').length, chips: [...$page().querySelectorAll('.bar .chip')].map((c) => c.textContent.trim()) })`,
  );
  check(
    'Sync now adds the commit to the list, under Today',
    listed.day === 'Today' && listed.count === 2 && listed.chips.length === 0,
    short(listed),
  );
  const seen = await run(
    `({ list: [...$page().querySelectorAll('button.commit')].map((b) => [b.querySelector('.message').textContent.trim(), b.getAttribute('aria-pressed')]), title: $detail()?.querySelector('h3')?.textContent })`,
  );
  check(
    'the commit shows the request and the field it changed',
    short(files) === short([{ change: 'Updated', name: 'Health', fields: ['URL'] }]),
    short({ files, seen }),
  );
  const diff = await run(
    `[...$detail().querySelectorAll('.grid pre')].map((p) => p.textContent.trim())`,
  );
  check(
    'before and after are side by side',
    short(diff) === short(['https://api.test/health', 'https://api.test/v2/health']),
    short(diff),
  );
  const first = await run(
    `(() => { const buttons = [...$page().querySelectorAll('button.commit')]; buttons.at(-1).click(); return buttons.at(-1).querySelector('.message').textContent.trim(); })()`,
  );
  await waitFor(`$detail()?.textContent.includes('Share')`);
  const shareFiles = await run(`$files()`);
  check(
    'the first commit lists what Share added',
    first.startsWith('Share') &&
      shareFiles.some((f) => f.name === 'Collection settings') &&
      shareFiles.filter((f) => f.change === 'Added').length >= 2,
    short({ first, shareFiles }),
  );

  await run(`$tab('Requests').click()`);
  await waitFor(`!!$page().querySelector('table')`);
  const table = await run(
    `[...$page().querySelectorAll('tbody tr')].map((tr) => [...tr.querySelectorAll('td')].slice(0, 3).map((td) => td.textContent.replace(/\\s+/g, ' ').trim()))`,
  );
  check(
    'the Requests tab shows who changed each request and its versions',
    table[0]?.[0].endsWith('Health') &&
      table[0][1].startsWith(profile.name) &&
      table[0][2] === '2' &&
      table[1]?.[2] === '1',
    short(table),
  );
  await run(`$btn('History of Health', $page()).click()`);
  await waitFor(`!!document.querySelector('dialog[open]:not(.panel-dialog)')`);
  const dialog = await run(
    `document.querySelector('dialog[open]:not(.panel-dialog) h2')?.textContent.trim()`,
  );
  check('History opens the versions of the request', dialog === 'History of “Health”', dialog);
  await run(
    `$btn('Close', document.querySelector('dialog[open]:not(.panel-dialog) .foot')).click()`,
  );
  await sleep(300);
  await run(`$tab('Commits').click()`);
  await sleep(200);
  await run(`$tab('Commits').focus()`);
  await cdp('Input.dispatchKeyEvent', {
    type: 'keyDown',
    key: 'ArrowRight',
    code: 'ArrowRight',
    windowsVirtualKeyCode: 39,
  });
  await cdp('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: 'ArrowRight',
    code: 'ArrowRight',
    windowsVirtualKeyCode: 39,
  });
  await sleep(200);
  check(
    'arrow keys move between the tabs',
    await run(
      `document.activeElement?.textContent.trim() === 'Requests' && $tab('Requests').getAttribute('aria-selected') === 'true'`,
    ),
  );

  // ---------- Import › replace the shared collection ----------
  const file = {
    info: {
      name: 'From another tool',
      schema: 'https://schema.getpostman.com/json/collection/v2.1.0/collection.json',
    },
    variable: [{ key: 'baseUrl', value: 'https://new.test' }],
    item: [
      { name: 'Alpha', request: { method: 'GET', url: '{{baseUrl}}/alpha' } },
      {
        name: 'Admin',
        item: [{ name: 'Beta', request: { method: 'POST', url: '{{baseUrl}}/beta' } }],
      },
      { name: 'Gamma', request: { method: 'DELETE', url: '{{baseUrl}}/gamma' } },
    ],
  };
  await run(`$goTo('Import')`);
  await sleep(900);
  await run(
    `(() => { const input = document.querySelector('input[aria-label="Collection or environment file"]'); const dt = new DataTransfer(); dt.items.add(new File([${JSON.stringify(JSON.stringify(file))}], 'other.collection.json', { type: 'application/json' })); input.files = dt.files; input.dispatchEvent(new Event('change', { bubbles: true })); })()`,
  );
  await waitFor(`!!document.querySelector('fieldset.mode')`);
  await run(`document.querySelector('fieldset.mode input[type="radio"]:not(:checked)').click()`);
  await sleep(200);
  await run(
    `$pick(document.querySelector('button[aria-label="Collection to replace"]'), ${JSON.stringify(NAME)})`,
  );
  const replaceUi = await run(
    `({ hint: [...document.querySelectorAll('.hint')].map((h) => h.textContent.trim()).find((t) => t.includes('keeps its name')) ?? '', button: $btn('Replace with')?.textContent.trim() })`,
  );
  check(
    'Import offers to replace an existing collection',
    replaceUi.hint.startsWith(`“${NAME}” keeps its name`) &&
      replaceUi.button === 'Replace with 3 requests',
    short(replaceUi),
  );
  await run(`$btn('Replace with').click()`);
  await waitFor(`!!document.querySelector('dialog[open]:not(.panel-dialog)')`);
  const confirmText = await run(
    `document.querySelector('dialog[open]:not(.panel-dialog)').textContent.replace(/\\s+/g, ' ').trim()`,
  );
  check(
    'replacing asks first and says what goes',
    confirmText.includes(`Replace “${NAME}”?`) &&
      confirmText.includes('Its 2 requests') &&
      confirmText.includes('the 3 requests from other.collection.json'),
    confirmText.slice(0, 220),
  );
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Replace').click()`,
  );
  await waitFor(
    `document.querySelector('input[aria-label="Collection name"]')?.value === ${JSON.stringify(NAME)}`,
  );
  await sleep(600);
  const after = await run(
    `({ name: document.querySelector('input[aria-label="Collection name"]')?.value, rows: $rows(), sidebar: [...document.querySelectorAll('nav .item .name')].filter((n) => n.textContent.trim() === ${JSON.stringify(NAME)}).length })`,
  );
  const kept = (await invoke('list_team_links', {})).ok.some(
    (link) => link.collectionId === collectionId,
  );
  check(
    'the collection keeps its name and link, with the file’s requests',
    after.name === NAME &&
      short(after.rows) === short(['Alpha', 'Gamma', 'Beta']) &&
      after.sidebar === 1 &&
      kept,
    short({ ...after, kept }),
  );
  await sleep(5000);
  const notYet = commits() === before + 1;
  await run(
    `if (!$team()) document.querySelector('header.top button[aria-label^="Collaboration"]').click()`,
  );
  await waitFor(`!!$team() && !!$btn('Sync now', $team())`);
  await run(`$btn('Sync now', $team()).click()`);
  const pushed = notYet && (await waitUntil(() => commits() === before + 2, 30000));
  const changed = pushed
    ? git([
        '--git-dir',
        bare,
        'show',
        '--name-status',
        '--no-renames',
        '--format=%s',
        'main',
      ]).split('\n')
    : [];
  const added = changed.filter((line) => line.startsWith('A\trequests/')).length;
  const removed = changed.filter((line) => line.startsWith('D\trequests/')).length;
  check(
    'Sync now sends the replacement as one ordinary commit, not before',
    pushed && added === 3 && removed === 2,
    short({ pushed, added, removed, subject: changed[0] }),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    if (collectionId) {
      await invoke('unlink_team_collection', { collectionId });
      await invoke('delete_collection', { id: collectionId });
    }
    for (const account of (await invoke('list_git_accounts', {})).ok ?? []) {
      if (account.host === 'git.test' && account.id)
        await invoke('remove_git_account', { id: account.id });
    }
    await run('location.reload()');
    await sleep(1500);
    const left = (await invoke('list_collections', {})).ok ?? [];
    const accountsLeft = ((await invoke('list_git_accounts', {})).ok ?? []).filter(
      (a) => a.host === 'git.test',
    ).length;
    check(
      'test data cleaned up',
      !left.some((c) => c.id === collectionId) &&
        accountsLeft === 0 &&
        !fs.existsSync(path.join(reposDir, collectionId ?? 'none')),
      short({ accountsLeft }),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  fs.rmSync(hosts, { recursive: true, force: true });
  process.exit(0);
}
