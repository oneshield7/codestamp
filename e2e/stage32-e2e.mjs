// End-to-end check of sharing into a repository that already has files: review, replace, join.
// launch.mjs maps https://git.test/ to bare repositories in .work/git-hosts, standing in for GitLab.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Legacy';
const REMOTE = 'https://git.test/team/legacy-api-collection.git';
const PLAIN = 'https://git.test/team/plain.git';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const hosts = path.join(path.dirname(fileURLToPath(import.meta.url)), '.work', 'git-hosts');
const bare = path.join(hosts, 'team', 'legacy-api-collection.git');
const plain = path.join(hosts, 'team', 'plain.git');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'cs-e2e-replace-'));
const reposDir = path.join(process.env.APPDATA, 'dev.codestamp.desktop', 'repos');
const git = (args, cwd = work) =>
  execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();

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
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$goTo = (text) => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); };
  window.$team = () => document.querySelector('section[aria-label="Collaboration"]');
  window.$field = (label, root = $team()) => [...root.querySelectorAll('label')].find((l) => l.textContent.trim().startsWith(label))?.querySelector('input');
  window.$review = () => $team()?.querySelector('[aria-labelledby="replace-title"]');
  window.$error = (root = $team()) => root?.querySelector('.error')?.textContent.trim() ?? '';
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 260);
let collectionId = null;
let joinedId = null;

// A GitLab-style repository with files that were not made by Codestamp.
fs.mkdirSync(path.dirname(bare), { recursive: true });
git(['init', '-q', '--bare', '--initial-branch', 'master', bare]);
git(['clone', '-q', bare, 'seed']);
const seed = path.join(work, 'seed');
fs.writeFileSync(path.join(seed, 'README.md'), '# Legacy API\n');
fs.mkdirSync(path.join(seed, 'postman'));
fs.writeFileSync(path.join(seed, 'postman', 'legacy.postman_collection.json'), '{}\n');
git(['add', '--all'], seed);
git(
  [
    '-c',
    'user.name=Seed',
    '-c',
    'user.email=seed@example.com',
    'commit',
    '-q',
    '-m',
    'Postman export',
  ],
  seed,
);
git(['push', '-q', 'origin', 'master'], seed);
git(['clone', '-q', '--bare', bare, plain]);
const files = () => git(['--git-dir', bare, 'ls-tree', '-r', '--name-only', 'master']).split('\n');
const commits = () => Number(git(['--git-dir', bare, 'rev-list', '--count', 'master']));

await connect();
try {
  const profile = (await invoke('team_setup', {})).ok?.author ?? {
    name: 'Ana Work',
    email: 'ana@acme.test',
  };
  const request = {
    kind: 'http',
    name: 'Policy',
    method: 'GET',
    url: 'https://api.test/policy',
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

  // ---------- Join explains what is wrong with a repository Codestamp did not share ----------
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$goTo('Import')`);
  await sleep(900);
  const joinPanel = `document.querySelector('section[aria-labelledby="join-title"]')`;
  await run(`$set(${joinPanel}.querySelector('input.mono'), ${JSON.stringify(PLAIN)})`);
  await sleep(200);
  await run(`$btn('Join', ${joinPanel}).click()`);
  await waitFor(`!!${joinPanel}.querySelector('.error')`, 30000);
  const joinError = await run(`${joinPanel}.querySelector('.error')?.textContent.trim() ?? ''`);
  check(
    'Join says the repository was not shared from Codestamp',
    /was not shared from Codestamp/.test(joinError) &&
      /share it from Collaboration/.test(joinError),
    joinError,
  );

  // ---------- Share reviews the files first ----------
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  await run(`document.querySelector('header.top button[aria-label^="Collaboration"]').click()`);
  await sleep(500);
  await run(`$set($field('Repository address'), ${JSON.stringify(REMOTE)})`);
  await sleep(150);
  await run(`$set($field('Git account'), 'anggi.saputra')`);
  await run(`$set($field('Your name'), ${JSON.stringify(profile.name)})`);
  await run(`$set($field('Email'), ${JSON.stringify(profile.email)})`);
  await sleep(150);
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`!!$review()`, 30000);
  const review = await run(
    `({ title: $review().querySelector('h3')?.textContent.trim(), text: $review().textContent.replace(/\\s+/g, ' '), files: [...$review().querySelectorAll('.files li')].map((li) => li.textContent.trim()), buttons: [...$review().querySelectorAll('button')].map((b) => b.textContent.trim()) })`,
  );
  check(
    'a repository with files is shown for review first',
    review.title === 'This repository already has files' &&
      short(review.files) === short(['README.md', 'postman/legacy.postman_collection.json']) &&
      review.text.includes('2 files to remove') &&
      review.text.includes('branch master') &&
      review.text.includes('Nothing is force-pushed'),
    short(review),
  );
  check(
    'nothing is pushed before the review is confirmed',
    commits() === 1 && !(await run(`!!$btn('Sync now', $team())`)),
    String(commits()),
  );
  check(
    'the review offers Cancel and Replace files and share',
    short(review.buttons) === short(['Cancel', 'Replace files and share']),
    short(review.buttons),
  );

  await run(`$btn('Cancel', $review()).click()`);
  await sleep(200);
  check(
    'Cancel goes back to the Share button',
    !(await run(`!!$review()`)) && (await run(`!!$btn('Share', $team())`)),
  );
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`!!$review()`, 30000);
  await run(`$set($field('Repository address'), ${JSON.stringify(REMOTE + ' ')})`);
  await sleep(150);
  check('editing the address drops the review', !(await run(`!!$review()`)));
  await run(`$set($field('Repository address'), ${JSON.stringify(REMOTE)})`);
  await sleep(150);
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`!!$review()`, 30000);

  // Someone pushes after the review: replacing stops and asks for a new look.
  fs.writeFileSync(path.join(seed, 'CHANGELOG.md'), 'later\n');
  git(['add', '--all'], seed);
  git(
    [
      '-c',
      'user.name=Seed',
      '-c',
      'user.email=seed@example.com',
      'commit',
      '-q',
      '-m',
      'Later change',
    ],
    seed,
  );
  git(['push', '-q', 'origin', 'master'], seed);
  await run(`$btn('Replace files and share', $review()).click()`);
  await waitFor(`$error().length > 0`, 30000);
  check(
    'a repository that moved after the review is not replaced',
    /changed after you checked it/.test(await run(`$error()`)) &&
      files().includes('CHANGELOG.md') &&
      commits() === 2,
    await run(`$error()`),
  );

  await run(`$btn('Cancel', $review()).click()`);
  await sleep(200);
  await run(`$btn('Share', $team()).click()`);
  await waitFor(`!!$review()`, 30000);
  const again = await run(
    `[...$review().querySelectorAll('.files li')].map((li) => li.textContent.trim())`,
  );
  check('checking again lists the new file too', again.includes('CHANGELOG.md'), short(again));
  await run(`$btn('Replace files and share', $review()).click()`);
  await waitFor(`!!$btn('Sync now', $team())`, 30000);
  const after = files();
  const facts = await run(
    `Object.fromEntries([...$team().querySelectorAll('dt')].map((dt) => [dt.textContent.trim(), dt.nextElementSibling.textContent.replace(/\\s+/g, ' ').trim()]))`,
  );
  check(
    'Replace files and share links the collection on the repository branch',
    facts.Branch === 'master' && facts['Signs in as'] === 'anggi.saputra',
    short(facts),
  );
  check(
    'the old files are gone and the collection is in their place',
    after.includes('codestamp.json') &&
      after.includes('collection.json') &&
      !after.some(
        (file) => file === 'README.md' || file === 'CHANGELOG.md' || file.startsWith('postman/'),
      ),
    short(after),
  );
  check(
    'one ordinary commit on top keeps the old ones in history',
    commits() === 3 &&
      git(['--git-dir', bare, 'log', '-1', '--format=%s', 'master']).startsWith(
        "Replace the repository's files with",
      ),
    git(['--git-dir', bare, 'log', '-3', '--format=%s', 'master']),
  );

  // ---------- the replaced repository can now be joined ----------
  const unlinked = await invoke('unlink_team_collection', { collectionId });
  const joined = await invoke('delete_collection', { id: collectionId });
  collectionId = null;
  const join = await invoke('join_team_collection', {
    remote: REMOTE,
    account: { author: profile, username: 'anggi.saputra' },
  });
  joinedId = join.ok?.id ?? null;
  check(
    'the replaced repository can be joined',
    !unlinked.error && !joined.error && join.ok?.name === NAME,
    short(join.error ?? join.ok),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    for (const id of [collectionId, joinedId].filter(Boolean)) {
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
    const leftovers = fs.existsSync(reposDir)
      ? fs.readdirSync(reposDir).filter((name) => name.startsWith('joining-'))
      : [];
    check(
      'test data cleaned up',
      !left.some((c) => c.name === NAME) && leftovers.length === 0,
      short(leftovers),
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
