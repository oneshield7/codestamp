// End-to-end check of stage 15a: general wording (no "Postman", "Collaboration" instead of
// "Team"), quick search on Ctrl+Space with path suggestions, and saving a response body.
// Creates "E2E Search" and deletes it afterwards.
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Search';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const here = path.dirname(fileURLToPath(import.meta.url));
const shots = path.join(here, '.work', 'screens');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'pc-e2e-'));
const savedPath = path.join(work, 'logo-saved.png');
fs.mkdirSync(shots, { recursive: true });

// A PNG-looking body with bytes that are not valid UTF-8, so a lossy copy would differ.
const logo = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  crypto.randomBytes(4096),
]);
const server = http.createServer((req, res) => {
  if (req.url === '/logo.png') {
    res.writeHead(200, { 'Content-Type': 'image/png', 'Content-Length': logo.length });
    return res.end(logo);
  }
  if (req.url === '/big') {
    const size = 33 * 1024 * 1024;
    res.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': size });
    return res.end(Buffer.alloc(size, 7));
  }
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end('{"ok":true}');
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;

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
async function key(name, code, keyCode, modifiers = 0) {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: name,
      code,
      windowsVirtualKeyCode: keyCode,
      modifiers,
    });
  }
  await sleep(150);
}
const ctrlSpace = () => key(' ', 'Space', 32, 2);
async function type(text) {
  await cdp('Input.insertText', { text });
  await sleep(250);
}
async function clearSearch() {
  await run(
    `(() => { const i = $search(); i.value = ''; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(150);
}
async function shot(file, clip) {
  const reply = await cdp('Page.captureScreenshot', {
    format: 'png',
    ...(clip ? { clip: { ...clip, scale: 1 } } : {}),
  });
  fs.writeFileSync(path.join(shots, file), Buffer.from(reply.result.data, 'base64'));
}
function answerDialog(title, file) {
  return new Promise((resolve) => {
    const child = spawn('powershell', [
      '-NoProfile',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      path.join(here, 'answer-dialog.ps1'),
      '-Title',
      title,
      '-Path',
      file,
    ]);
    let output = '';
    child.stdout.on('data', (chunk) => (output += chunk));
    child.on('close', () => resolve(output.trim()));
  });
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$open = () => document.querySelector('dialog.quick-search[open]');
  window.$search = () => document.querySelector('dialog.quick-search input');
  window.$groups = () => [...document.querySelectorAll('dialog.quick-search .group')].map((g) => ({ label: g.getAttribute('aria-label'), items: [...g.querySelectorAll('[role="option"]')].map((o) => o.querySelector('.title').textContent.trim()) }));
  window.$activeTitle = () => document.querySelector('dialog.quick-search [role="option"][aria-selected="true"] .title')?.textContent.trim() ?? null;
  window.$ghost = () => { const g = document.querySelector('dialog.quick-search .ghost'); return g ? g.textContent.slice(g.querySelector('.typed').textContent.length) : null; };
  window.$card = () => document.querySelector('article.card');
  window.$saveBody = () => [...document.querySelectorAll('.response .save')].find((b) => b.textContent.includes('Save body'));
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const content = (name, method, url) => ({
  kind: 'http',
  name,
  method,
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
});

await connect();
try {
  // ---------- wording ----------
  await run(`$nav('Home').click()`);
  await sleep(500);
  const home = await run(
    `(async () => { const wait = () => new Promise((r) => setTimeout(r, 60)); const texts = []; for (const tab of document.querySelectorAll('.home [role="tab"]')) { tab.click(); await wait(); for (const input of document.querySelectorAll('.home .tiles input')) { input.click(); await wait(); texts.push(input.getAttribute('aria-label') + ': ' + document.querySelector('.home .stage').innerText); } } return texts.join(' | '); })()`,
  );
  check(
    'Home no longer names Postman or teams',
    !/postman/i.test(home) &&
      !/\bteam/i.test(home) &&
      home.includes('Import a collection or cURL') &&
      home.includes('Join a shared collection') &&
      home.includes('Collaboration over Git'),
    short(home.match(/.{0,30}(postman|team).{0,30}/i)?.[0] ?? 'clean'),
  );
  await run(`$nav('Import').click()`);
  await sleep(500);
  const importText = await run(`document.querySelector('main').innerText`);
  check(
    'Import is general',
    !/postman/i.test(importText) &&
      !/\bteam/i.test(importText) &&
      importText.includes('Collection, environment or API spec') &&
      importText.includes('Join a shared collection'),
    short(importText.match(/.{0,30}(postman|team).{0,30}/i)?.[0] ?? 'clean'),
  );

  // ---------- data ----------
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: 'https://api.example.test' })`,
  );
  for (const [name, method, url] of [
    ['List users', 'GET', '{{baseUrl}}/users'],
    ['Get user', 'GET', '{{baseUrl}}/users/:id'],
    ['User orders', 'GET', '{{baseUrl}}/users/:id/orders'],
    ['Create order', 'POST', '{{baseUrl}}/orders'],
    ['Download logo', 'GET', `${base}/logo.png`],
    ['Huge file', 'GET', `${base}/big`],
  ]) {
    await run(
      `$invoke('create_request', { collectionId: '${collection.id}', sectionId: null, content: ${JSON.stringify(content(name, method, url))} })`,
    );
  }
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  const collab = await run(
    `[...document.querySelectorAll('header button, .actions button, button')].map((b) => b.getAttribute('aria-label') ?? '').find((l) => /^(Collaboration|Team)/.test(l)) ?? null`,
  );
  check('the collection header says Collaboration', collab === 'Collaboration', String(collab));
  const exportLabel = await run(
    `!!document.querySelector('button[aria-label="Export collection"]')`,
  );
  check('the export button is general', exportLabel);

  // ---------- open & empty state ----------
  await run(`document.activeElement?.blur()`);
  await ctrlSpace();
  await waitFor(`!!$open()`, 3000);
  const opened = await run(
    `({ open: !!$open(), focus: document.activeElement === $search(), groups: $groups().map((g) => g.label) })`,
  );
  check(
    'Ctrl+Space opens quick search with the field focused',
    opened.open &&
      opened.focus &&
      opened.groups.includes('Go to') &&
      opened.groups.includes('Collections'),
    short(opened),
  );
  const animation = await run(`getComputedStyle($open()).animationName`);
  check('it rises into view', animation.endsWith('rise-in'), animation);

  // ---------- path suggestions ----------
  await type('/us');
  await waitFor(`$groups()[0]?.label === 'Requests'`, 4000);
  const pathResults = await run(
    `({ groups: $groups(), ghost: $ghost(), active: $activeTitle(), marks: [...document.querySelectorAll('dialog.quick-search [aria-selected="true"] .detail mark')].map((m) => m.textContent) })`,
  );
  check(
    'typing a path lists matching requests first',
    pathResults.groups[0]?.label === 'Requests' &&
      pathResults.groups[0].items.slice(0, 3).join('|') === 'List users|Get user|User orders' &&
      pathResults.marks.includes('/us'),
    short(pathResults),
  );
  check('the path is completed inline', pathResults.ghost === 'ers', String(pathResults.ghost));
  await shot('search-path.png');
  await key('Tab', 'Tab', 9);
  const accepted = await run(
    `({ value: $search().value, ghost: $ghost(), focus: document.activeElement === $search() })`,
  );
  check(
    'Tab takes the suggestion and offers the next segment',
    accepted.value === '/users' && accepted.ghost === '/:id' && accepted.focus,
    short(accepted),
  );
  await key('ArrowDown', 'ArrowDown', 40);
  const moved = await run(`$activeTitle()`);
  await key('ArrowUp', 'ArrowUp', 38);
  await key('Enter', 'Enter', 13);
  await waitFor(`!$open() && !!$card()`, 4000);
  const openedCard = await run(
    `({ open: !!$open(), name: $card()?.querySelector('.name input')?.value ?? null })`,
  );
  check(
    'arrows move and Enter opens the request',
    moved === 'Get user' && !openedCard.open && openedCard.name === 'List users',
    short({ moved, openedCard }),
  );

  // ---------- inside an editor ----------
  await run(
    `[...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith('Scripts'))?.click()`,
  );
  await sleep(400);
  await run(`$card().querySelector('.cm-content')?.focus()`);
  await sleep(150);
  const inEditor = await run(`!!document.activeElement?.closest('.cm-editor')`);
  await ctrlSpace();
  await sleep(400);
  const fromEditor = await run(
    `({ open: !!$open(), completion: !!document.querySelector('.cm-tooltip-autocomplete') })`,
  );
  check(
    'Ctrl+Space in a code editor opens search, not code completion',
    inEditor && fromEditor.open && !fromEditor.completion,
    short({ inEditor, fromEditor }),
  );
  await ctrlSpace();
  check('Ctrl+Space again closes it', !(await run(`!!$open()`)));

  // ---------- method words ----------
  await ctrlSpace();
  await type('post orders');
  await sleep(200);
  const post = await run(
    `({ groups: $groups(), stamp: document.querySelector('dialog.quick-search [aria-selected="true"] .stamp')?.textContent.trim() })`,
  );
  check(
    'a method word narrows to that method',
    post.groups[0]?.items[0] === 'Create order' &&
      post.stamp === 'POST' &&
      !post.groups[0].items.includes('User orders'),
    short(post),
  );

  // ---------- settings, pages, environments ----------
  await clearSearch();
  await type('dark');
  const dark = await run(`$activeTitle()`);
  await key('Enter', 'Enter', 13);
  await sleep(700);
  const appearance = await run(
    `({ title: document.querySelector('h1.page-title')?.textContent.trim(), section: document.querySelector('.sections .section.on')?.textContent.trim() })`,
  );
  check(
    '"dark" leads to Settings › Appearance',
    dark === 'Settings › Appearance' &&
      appearance.title === 'Settings' &&
      appearance.section === 'Appearance',
    short({ dark, appearance }),
  );
  await ctrlSpace();
  await type('proxy');
  await key('Enter', 'Enter', 13);
  await sleep(600);
  check(
    'a settings section opens directly',
    (await run(`document.querySelector('.sections .section.on')?.textContent.trim()`)) === 'Proxy',
  );
  await ctrlSpace();
  await type('globals');
  const globalsTitle = await run(`$activeTitle()`);
  await key('Enter', 'Enter', 13);
  await sleep(800);
  const envPage = await run(`document.querySelector('h1.page-title')?.textContent.trim()`);
  check(
    'an environment opens on the Environments page',
    globalsTitle === 'Globals' && envPage === 'Environments',
    short({ globalsTitle, envPage }),
  );
  await ctrlSpace();
  await type('zzzz-nothing');
  const none = await run(
    `document.querySelector('dialog.quick-search .empty')?.textContent.trim()`,
  );
  check('no match says so', none === 'Nothing matches “zzzz-nothing”.', String(none));

  // ---------- closing ----------
  await key('Escape', 'Escape', 27);
  check('Esc closes', !(await run(`!!$open()`)));
  await run(`document.querySelector('nav button.search').click()`);
  await sleep(300);
  const fromSidebar = await run(
    `({ open: !!$open(), expanded: document.querySelector('nav button.search').getAttribute('aria-expanded') })`,
  );
  for (const type of ['mousePressed', 'mouseReleased']) {
    await cdp('Input.dispatchMouseEvent', { type, x: 6, y: 300, button: 'left', clickCount: 1 });
  }
  await sleep(300);
  check(
    'the sidebar button opens it and a click outside closes it',
    fromSidebar.open && fromSidebar.expanded === 'true' && !(await run(`!!$open()`)),
    short(fromSidebar),
  );
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(800);
  await run(`$btn('Delete collection').click()`);
  await sleep(300);
  await ctrlSpace();
  const blocked = await run(
    `({ search: !!$open(), confirm: !!document.querySelector('dialog[open]:not(.panel-dialog)') })`,
  );
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Cancel').click()`,
  );
  await sleep(300);
  check(
    'it stays shut while another dialog is open',
    !blocked.search && blocked.confirm,
    short(blocked),
  );

  // ---------- save response body ----------
  await ctrlSpace();
  await type('logo');
  await key('Enter', 'Enter', 13);
  await waitFor(`$card()?.querySelector('.name input')?.value === 'Download logo'`, 4000);
  await run(`$card().querySelector('.send').click()`);
  await waitFor(`!!$saveBody()`, 15000);
  const save = await run(`({ disabled: $saveBody().disabled, title: $saveBody().title })`);
  check('a response offers Save body', !save.disabled, short(save));
  const answered = answerDialog('Save response body', savedPath);
  await run(`$saveBody().click()`);
  const answer = await answered;
  await waitFor(
    `(document.querySelector('.banner')?.textContent ?? '').includes('saved to')`,
    5000,
  );
  const banner = await run(`document.querySelector('.banner')?.textContent.trim() ?? ''`);
  const same = fs.existsSync(savedPath) && Buffer.compare(fs.readFileSync(savedPath), logo) === 0;
  check(
    'the body is saved byte for byte',
    answer === 'answered' && same && banner.includes(savedPath),
    short({ answer, same, banner }),
  );
  await run(
    `[...document.querySelectorAll('.banner button')].find((b) => b.textContent.trim() === 'Dismiss')?.click()`,
  );
  await ctrlSpace();
  await type('huge');
  await key('Enter', 'Enter', 13);
  await waitFor(`$card()?.querySelector('.name input')?.value === 'Huge file'`, 4000);
  await run(`$card().querySelector('.send').click()`);
  await waitFor(`!!$saveBody()`, 30000);
  const huge = await run(`({ disabled: $saveBody().disabled, title: $saveBody().title })`);
  check(
    'bodies over 32 MB cannot be saved, and say why',
    huge.disabled && huge.title.includes('32 MB'),
    short(huge),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    if (await run(`!!$open()`)) await key('Escape', 'Escape', 27);
    if (await run(`!!$nav(${JSON.stringify(NAME)})`)) {
      await run(`$nav(${JSON.stringify(NAME)}).click()`);
      await sleep(800);
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
  server.close();
  fs.rmSync(work, { recursive: true, force: true });
  process.exit(0);
}
