// End-to-end check of the 2026-10-27 Git changes layout: with a long history the page itself does
// not scroll; the commit list and the commit detail scroll on their own under a fixed header.
// A collaborator clone adds 120 commits through a local bare repository. Launches the app itself.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const bare = path.join(here, '.work', 'git-hosts', 'team', 'history.git');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'cs-e2e-history-'));
const mate = path.join(work, 'mate');
const shots = path.join(here, '.work', 'screens');
const git = (args, cwd = work, env = {}) =>
  execFileSync('git', args, {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, ...env },
  }).trim();
const COMMITS = 120;

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
async function waitFor(expression, timeout = 8000) {
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
async function tab() {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: 'Tab',
      code: 'Tab',
      windowsVirtualKeyCode: 9,
    });
  }
  await sleep(150);
}
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim() === text || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$list = () => document.querySelector('#changes-panel .list');
  window.$side = () => document.querySelector('#changes-panel .side');
  window.$scroller = () => document.querySelector('.scroller');
  window.$layout = () => {
    const box = (el) => { const r = el.getBoundingClientRect(); return { top: Math.round(r.top), bottom: Math.round(r.bottom) }; };
    const list = $list(); const side = $side(); const scroller = $scroller();
    return {
      page: { scrolls: scroller.scrollHeight > scroller.clientHeight + 1, scrollTop: scroller.scrollTop },
      window: innerHeight,
      tabs: box(document.querySelector('.tabs')),
      picker: box(document.querySelector('.bar')),
      panel: box(document.getElementById('changes-panel')),
      list: list && { ...box(list), scrolls: list.scrollHeight > list.clientHeight + 1, scrollTop: Math.round(list.scrollTop), overflow: getComputedStyle(list).overflowY },
      side: side && { ...box(side), overflow: getComputedStyle(side).overflowY },
      commits: document.querySelectorAll('#changes-panel .commit').length,
    };
  };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 400);
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

/** A small window: the list runs in one column and everything stays reachable by scrolling. */
async function smallWindow(first) {
  const reachable = await run(
    `(() => { const panel = document.getElementById('changes-panel'); const scroller = $scroller(); return { panel: panel.scrollHeight > panel.clientHeight, page: scroller.scrollHeight > scroller.clientHeight }; })()`,
  );
  check(
    'in a small window the history scrolls as a whole',
    first.commits === 50 && (reachable.panel || reachable.page),
    short({ commits: first.commits, ...reachable }),
  );
  await run(`$btn('Load older commits').scrollIntoView({ block: 'center' })`);
  await run(`$btn('Load older commits').click()`);
  await waitFor(`document.querySelectorAll('#changes-panel .commit').length >= 100`, 10000);
  check(
    'older commits load in a small window too',
    (await run(`document.querySelectorAll('#changes-panel .commit').length`)) === 100,
  );
  const lastMessage = await run(
    `(() => { const last = [...document.querySelectorAll('#changes-panel .commit')].at(-1); last.scrollIntoView({ block: 'center' }); last.click(); return last.querySelector('.message').textContent.trim(); })()`,
  );
  await waitFor(`$side()?.textContent.includes(${JSON.stringify(lastMessage)} + ' ')`, 6000);
  check(
    'a commit far down shows its detail in a small window',
    await run(`$side()?.textContent.includes(${JSON.stringify(lastMessage)} + ' ') ?? false`),
    lastMessage,
  );
  await shot('stage36-small.png');
}

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
    await invoke('create_collection', { name: 'E2E History', icon: null, baseUrl: null })
  ).ok.id;
  await invoke('create_request', {
    collectionId,
    sectionId: null,
    content: request('Health', 'https://api.test/health'),
  });
  const shared = await invoke('share_collection', {
    collectionId,
    remote: 'https://git.test/team/history.git',
    account: { author: profile, username: null },
    replaceHead: null,
  });
  check('the collection is shared', !!shared.ok, short(shared.error ?? 'ok'));

  // A collaborator adds a long history, a few commits a day over the last weeks.
  git(['clone', '-q', bare, mate]);
  const start = Math.floor(Date.now() / 1000) - COMMITS * 8 * 3600;
  for (let index = 1; index <= COMMITS; index++) {
    const date = `@${start + index * 8 * 3600} +0700`;
    git(
      [
        '-c',
        'user.name=Mate',
        '-c',
        'user.email=mate@example.com',
        'commit',
        '-q',
        '--allow-empty',
        '-m',
        `Mate change ${index}`,
      ],
      mate,
      { GIT_AUTHOR_DATE: date, GIT_COMMITTER_DATE: date },
    );
  }
  git(['push', '-q', 'origin', 'main'], mate);

  // Shared through invoke, so the page is reloaded to pick up the link like a real share does.
  const synced = await invoke('sync_team_collection', { collectionId, decisions: [] });
  check("the collaborator's history arrives", synced.ok?.status === 'synced', short(synced));
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
  await waitFor(`!!$nav('Git changes')`);
  await run(`$nav('Git changes').click()`);
  await waitFor(`document.querySelectorAll('#changes-panel .commit').length >= 50`, 20000);
  await sleep(600);

  const first = await run(`$layout()`);
  // Side by side needs 760 px for the panel (DESIGN.md); the page fits the window from 480 px of
  // panel height. Smaller windows scroll the panel, or the page, as a whole.
  const wide = await run(
    `document.getElementById('changes-panel').getBoundingClientRect().width >= 760 && innerHeight >= 700`,
  );
  if (!wide) {
    await smallWindow(first);
  } else {
    check('a long history does not make the page scroll', !first.page.scrolls, short(first.page));
    check(
      'the commit list scrolls on its own',
      first.list?.scrolls && first.list.overflow === 'auto' && first.commits === 50,
      short(first.list),
    );
    check(
      'the panel ends inside the window',
      first.panel.bottom <= first.window,
      short({ panel: first.panel, window: first.window }),
    );
    await shot('stage36-top.png');

    await run(`$list().scrollTop = $list().scrollHeight`);
    await sleep(300);
    const bottom = await run(
      `(() => { const more = $btn('Load older commits'); const r = more?.getBoundingClientRect(); const l = $list().getBoundingClientRect(); return { more: !!more, inside: !!r && r.top >= l.top && r.bottom <= l.bottom, layout: $layout() }; })()`,
    );
    check(
      'Load older commits sits at the end of the list and comes into view',
      bottom.more && bottom.inside,
      short(bottom),
    );
    check(
      'scrolling the list keeps the picker and tabs in place',
      bottom.layout.tabs.top === first.tabs.top &&
        bottom.layout.picker.top === first.picker.top &&
        !bottom.layout.page.scrolls &&
        bottom.layout.page.scrollTop === 0,
      short({ before: first.tabs, after: bottom.layout.tabs, page: bottom.layout.page }),
    );

    const listTop = bottom.layout.list.scrollTop;
    await run(`$btn('Load older commits').click()`);
    await waitFor(`document.querySelectorAll('#changes-panel .commit').length >= 100`, 10000);
    await sleep(300);
    const loaded = await run(`$layout()`);
    check(
      'older commits load into the same scroll area',
      loaded.commits === 100 && loaded.list.scrollTop >= listTop - 2 && !loaded.page.scrolls,
      short({ commits: loaded.commits, listTop, after: loaded.list.scrollTop }),
    );

    // Picking a commit far down shows its detail at the top of the right column, no scrolling needed.
    const lastMessage = await run(
      `(() => { const last = [...document.querySelectorAll('#changes-panel .commit')].at(-1); last.click(); return last.querySelector('.message').textContent.trim(); })()`,
    );
    await waitFor(`$side()?.textContent.includes(${JSON.stringify(lastMessage)} + ' ')`, 6000);
    const picked = await run(
      `(() => { const s = $side(); const r = s.querySelector('h2, h3, .title, strong')?.getBoundingClientRect() ?? s.getBoundingClientRect(); return { heading: s.textContent.trim().slice(0, 60), visible: r.top >= s.getBoundingClientRect().top - 1 && r.bottom <= innerHeight, side: $layout().side }; })()`,
    );
    check(
      'the detail of a commit far down shows without scrolling',
      picked.heading.startsWith(lastMessage + ' ') &&
        picked.visible &&
        picked.side.overflow === 'auto',
      short({ lastMessage, ...picked }),
    );
    await shot('stage36-picked.png');

    // Tab moves through the list; the focused commit scrolls into view and its ring is not cut off.
    await run(
      `$list().scrollTop = 0; document.querySelectorAll('#changes-panel .commit')[5].focus()`,
    );
    for (let step = 0; step < 14; step++) await tab();
    const focus = await run(
      `(() => { const el = document.activeElement; const r = el.getBoundingClientRect(); const l = $list().getBoundingClientRect(); return { commit: el.classList.contains('commit'), text: el.querySelector('.message')?.textContent, inView: r.top >= l.top && r.bottom <= l.bottom, ringRoom: Math.round(Math.min(r.left - l.left, l.right - r.right, r.top - l.top)) }; })()`,
    );
    check(
      'keyboard focus keeps the commit in view with room for its ring',
      focus.commit && focus.inView && focus.ringRoom >= 4,
      short(focus),
    );

    await run(`document.getElementById('changes-tab-requests').click()`);
    await sleep(800);
    const requests = await run(`$layout()`);
    check(
      'the Requests tab keeps the same fixed header',
      !requests.page.scrolls && requests.tabs.top === first.tabs.top,
      short(requests),
    );
  }
} catch (error) {
  console.log(results.join('\n'));
  console.log(
    'state',
    short(
      await run(
        `({ layout: $layout(), buttons: [...document.querySelectorAll('#changes-panel button')].map((b) => b.textContent.trim()).filter((t) => !t.startsWith('Mate') && !t.startsWith('Share')).slice(0, 8), count: document.querySelectorAll('#changes-panel .commit').length })`,
      ),
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
