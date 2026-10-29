// End-to-end check of the 2026-10-28 batches: quick search opens without a blurred backdrop, no
// shortcut text on screen or in tooltips, rail tooltips, the title bar tooltip inside the window,
// the status bar, Export in the collection (Codestamp, Postman, Docs), Import of a Codestamp file,
// History per collection, and Recently deleted with Undo. Launches the app.
import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const shots = path.join(here, '.work', 'screens');
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'codestamp-e2e40-'));
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const NAME = 'E2E Forty';
const COPY = 'E2E Forty copy';
const SECRET = 'TOPSECRET-40';

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
function cdp(method, params = {}) {
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
    await sleep(150);
  }
  return false;
}
async function shot(name) {
  const image = await cdp('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync(path.join(shots, name), Buffer.from(image.result.data, 'base64'));
}
async function key(name, code, vk, modifiers = 0, text) {
  const down = text ? { type: 'keyDown', text } : { type: 'rawKeyDown' };
  for (const event of [down, { type: 'keyUp' }]) {
    await cdp('Input.dispatchKeyEvent', {
      ...event,
      key: name,
      code,
      windowsVirtualKeyCode: vk,
      modifiers,
    });
  }
  await sleep(200);
}
async function hover(expression) {
  const box = await run(
    `(() => { const r = (${expression}).getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`,
  );
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: box.x, y: box.y });
  await sleep(200);
}
async function reload() {
  await run('location.reload()');
  await sleep(1500);
  await waitFor(
    `!document.getElementById('launch') && !!document.querySelector('nav.sidebar')`,
    8000,
  );
  await run(helpers);
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
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim() === text || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$row = (name) => [...document.querySelectorAll('.row')].find((r) => r.querySelector('.name')?.textContent.trim() === name);
  window.$banner = () => document.querySelector('.banner')?.textContent.replace(/\\s+/g, ' ').trim() ?? '';
  window.$confirm = (label) => [...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === label);
  window.$exportPanel = () => document.querySelector('section[aria-label="Export"]');
  window.$historyPanel = () => document.querySelector('section[aria-label="History"]');
  window.$pick = async (trigger, label) => { trigger.click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label).dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 100)); };
  window.$goTo = async (title) => { document.querySelector('nav.sidebar button.search').click(); await new Promise((r) => setTimeout(r, 200)); const input = document.querySelector('dialog.quick-search input'); input.value = title; input.dispatchEvent(new Event('input', { bubbles: true })); await new Promise((r) => setTimeout(r, 200)); [...document.querySelectorAll('dialog.quick-search .result')].find((r) => r.querySelector('.title')?.textContent.trim() === title)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); };
  window.$tip = () => { const tip = document.querySelector('nav.sidebar + .tip'); if (!tip) return null; const r = tip.getBoundingClientRect(); return { text: tip.textContent.trim(), left: Math.round(r.left), mid: Math.round(r.top + r.height / 2) }; };
  window.$shortcutText = () => {
    const visible = (el) => el.getClientRects().length > 0 && getComputedStyle(el).visibility !== 'hidden';
    const kbd = [...document.querySelectorAll('kbd')].filter(visible).map((k) => k.textContent.trim());
    const attrs = [...document.querySelectorAll('[title], [aria-label], [data-tip]')].flatMap((el) => ['title', 'aria-label', 'data-tip'].map((a) => el.getAttribute(a) ?? '')).filter((t) => /Ctrl|Shift\\+|Alt \\+|\\(Esc\\)|\\(Enter\\)/.test(t));
    const tips = [...document.querySelectorAll('.tip')].map((t) => t.textContent).filter((t) => /Ctrl/.test(t));
    const text = /Ctrl ?[+↵]|Ctrl Space|Win \\+/.test(document.body.innerText);
    return { kbd, attrs, tips, text };
  };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const request = (name) => ({
  kind: 'http',
  name,
  method: 'GET',
  url: `https://api.test/${name.toLowerCase().replace(/\s+/g, '-')}`,
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
const noShortcuts = (found) =>
  found.kbd.length === 0 && found.attrs.length === 0 && found.tips.length === 0 && !found.text;

const server = http.createServer((req, res) => {
  res.writeHead(req.url.startsWith('/missing') ? 404 : 200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ path: req.url }));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
const sendFrom = (collectionId, requestId, url) =>
  invoke('send_http_request', {
    request: {
      method: 'GET',
      url,
      params: [],
      headers: [],
      body: null,
      timeoutMs: null,
      collectionId,
      requestId,
      recordHistory: true,
      pathVariables: [],
    },
  });

execFileSync(process.execPath, [path.join(here, 'launch.mjs')], { stdio: 'inherit' });
await connect();
const created = [];
const envIds = [];
try {
  const collectionId = (
    await invoke('create_collection', { name: NAME, icon: null, baseUrl: null })
  ).ok.id;
  created.push(collectionId);
  const section = (await invoke('create_section', { collectionId, name: 'Users' })).ok;
  const requestIds = {};
  for (const name of ['List users', 'Get user']) {
    requestIds[name] = (
      await invoke('create_request', {
        collectionId,
        sectionId: section.id,
        content: request(name),
      })
    ).ok.id;
  }
  const set = (await invoke('get_collection_variable_set', { collectionId })).ok;
  const setId = typeof set === 'string' ? set : set.id;
  await invoke('save_environment_variables', {
    id: setId,
    variables: [{ id: null, key: 'token', value: SECRET, isSecret: true, enabled: true }],
  });
  const staging = (await invoke('create_environment', { name: 'E2E Forty staging' })).ok.id;
  envIds.push(staging);
  await invoke('set_environment_collection', { id: staging, collectionId });
  await reload();

  // ---------- no shortcut text anywhere ----------
  await run(`$nav('Home').click()`);
  await sleep(900);
  const home = await run('$shortcutText()');
  check('Home shows no shortcut hints', noShortcuts(home), short(home));
  const search = await run(
    `(() => { const b = document.querySelector('nav.sidebar button.search'); return { text: b.textContent.trim(), title: b.getAttribute('title'), kbd: b.querySelectorAll('kbd').length }; })()`,
  );
  check(
    'the sidebar search button just says Search',
    search.text === 'Search' && search.title === null && search.kbd === 0,
    short(search),
  );
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  await run(`$row('Get user').click()`);
  await waitFor(`!!document.querySelector('article.card')`);
  const send = await run(
    `[...document.querySelectorAll('article.card button.send')].map((b) => b.textContent.trim())`,
  );
  const card = await run('$shortcutText()');
  check(
    'the Send button and the card show no shortcut',
    short(send) === short(['Send']) && noShortcuts(card),
    short({ send, card }),
  );
  await key(' ', 'Space', 32, 2, ' ');
  await waitFor(`!!document.querySelector('dialog.quick-search[open]')`);
  const quick = await run(
    `(() => { const d = document.querySelector('dialog.quick-search'); const back = getComputedStyle(d, '::backdrop'); const style = getComputedStyle(d); return { kbd: d.querySelectorAll('kbd').length, footer: d.querySelector('.footer').textContent.trim(), blur: back.backdropFilter, animation: style.animationName, duration: style.animationDuration }; })()`,
  );
  check(
    'quick search has no key hints and no blurred backdrop',
    quick.kbd === 0 &&
      !/Esc|Enter|↑/.test(quick.footer) &&
      (quick.blur === 'none' || quick.blur === '') &&
      quick.animation.endsWith('rise-in'),
    short(quick),
  );
  await key('Escape', 'Escape', 27);
  await waitFor(`!document.querySelector('dialog.quick-search[open]')`);

  // ---------- quick search opens smoothly ----------
  const frames = await run(`new Promise((resolve) => {
    const stamps = [];
    const start = performance.now();
    const tick = (now) => { stamps.push(now); if (now - start < 450) requestAnimationFrame(tick); else { const gaps = stamps.slice(1).map((t, i) => t - stamps[i]); resolve({ frames: stamps.length, worst: Math.round(Math.max(...gaps)), typical: Math.round(gaps.sort((a, b) => a - b)[Math.floor(gaps.length / 2)]) }); } };
    requestAnimationFrame(tick);
    setTimeout(() => document.querySelector('nav.sidebar button.search').click(), 60);
  })`);
  check(
    'opening quick search keeps the frame rate (no long frame)',
    frames.worst <= 50 && frames.frames >= 15,
    short(frames),
  );
  await shot('stage40-quick-search.png');
  await key('Escape', 'Escape', 27);
  await waitFor(`!document.querySelector('dialog.quick-search[open]')`);

  // ---------- the sidebar toggle's tooltip stays inside the window ----------
  await hover(`document.querySelector('header.titlebar button[aria-label="Collapse sidebar"]')`);
  const toggleTip = await run(
    `(() => { const tip = document.querySelector('header.titlebar button[aria-label="Collapse sidebar"]').parentElement.querySelector('.tip'); const r = tip.getBoundingClientRect(); return { text: tip.textContent.trim(), left: Math.round(r.left), right: Math.round(r.right), visible: getComputedStyle(tip).visibility }; })()`,
  );
  check(
    'the Collapse sidebar tooltip is whole and has no shortcut',
    toggleTip.text === 'Collapse sidebar' && toggleTip.left >= 0 && toggleTip.visible === 'visible',
    short(toggleTip),
  );
  await shot('stage40-toggle-tip.png');

  // ---------- rail tooltips ----------
  await run(
    `document.querySelector('header.titlebar button[aria-label="Collapse sidebar"]').click()`,
  );
  await waitFor(`document.querySelector('nav.sidebar')?.classList.contains('rail')`);
  await hover(`$nav(${JSON.stringify(NAME)})`);
  const itemBox = await run(
    `(() => { const r = $nav(${JSON.stringify(NAME)}).getBoundingClientRect(); return { right: Math.round(r.right), mid: Math.round(r.top + r.height / 2) }; })()`,
  );
  const collectionTip = await run('$tip()');
  check(
    'hovering a collection in the rail shows its name beside it',
    collectionTip?.text === NAME &&
      collectionTip.left >= itemBox.right &&
      Math.abs(collectionTip.mid - itemBox.mid) <= 2,
    short({ collectionTip, itemBox }),
  );
  await shot('stage40-rail-tip.png');
  await hover(`$nav('Catalog')`);
  check(
    'rail pages show their names too',
    (await run('$tip()'))?.text === 'Catalog',
    short(await run('$tip()')),
  );
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 600, y: 400 });
  await sleep(200);
  check('the rail tooltip goes when the pointer leaves', (await run('$tip()')) === null);
  await run(`document.querySelector('nav.sidebar button.search').focus()`);
  await key('Tab', 'Tab', 9);
  const focusTip = await run('$tip()');
  check(
    'keyboard focus on a rail item shows its tooltip',
    focusTip?.text === 'Home',
    short(focusTip),
  );
  const railTitles = await run(
    `[...document.querySelectorAll('nav.sidebar [title]')].map((el) => el.getAttribute('title'))`,
  );
  check(
    'the rail has no native titles on top of its tooltips',
    railTitles.length === 0,
    short(railTitles),
  );
  await run(
    `document.querySelector('header.titlebar button[aria-label="Expand sidebar"]').click()`,
  );
  await waitFor(`!document.querySelector('nav.sidebar')?.classList.contains('rail')`);
  check(
    'expanding the sidebar drops a rail tooltip still on the focused item',
    (await run('$tip()')) === null,
    short(await run('$tip()')),
  );

  // ---------- the status bar ----------
  const about = (await invoke('about_app', {})).ok;
  const statusBar = await run(
    `(() => { const footer = document.querySelector('footer.statusbar'); const box = footer.getBoundingClientRect(); return { places: [...footer.querySelectorAll('button')].map((b) => b.getAttribute('aria-label')), version: footer.querySelector('.version')?.textContent.trim(), bottom: Math.round(innerHeight - box.bottom), full: Math.round(box.width) === innerWidth, groups: [...document.querySelectorAll('nav.sidebar h2')].map((h) => h.textContent.trim()) }; })()`,
  );
  check(
    'a status bar across the bottom holds Settings, Recently deleted, Cookies and Environments',
    short(statusBar.places) ===
      short(['Settings', 'Recently deleted', 'Cookies', 'Environments']) &&
      statusBar.bottom === 0 &&
      statusBar.full,
    short(statusBar),
  );
  check(
    'the status bar ends with the running version',
    statusBar.version === `v${about.version}`,
    `${statusBar.version} · ${about.version}`,
  );
  check(
    'the sidebar no longer has a Settings section',
    short(statusBar.groups) === short(['Collections', 'Tools']),
    short(statusBar.groups),
  );
  await hover(`document.querySelector('footer.statusbar button[aria-label="Settings"]')`);
  const barTip = await run(
    `(() => { const button = document.querySelector('footer.statusbar button[aria-label="Settings"]'); const tip = button.parentElement.querySelector('.tip'); const r = tip.getBoundingClientRect(); return { text: tip.textContent.trim(), above: r.bottom <= button.getBoundingClientRect().top, left: Math.round(r.left), visible: getComputedStyle(tip).visibility }; })()`,
  );
  check(
    'status bar tooltips open above their buttons, inside the window',
    barTip.text === 'Settings' && barTip.above && barTip.left >= 0 && barTip.visible === 'visible',
    short(barTip),
  );
  await shot('stage40-statusbar-tip.png');
  await run(`$nav('Cookies').click()`);
  await waitFor(`document.querySelector('h1.page-title')?.textContent.trim() === 'Cookies'`);
  check(
    'a status bar button opens its page and shows it is open',
    (await run(`$nav('Cookies').getAttribute('aria-current')`)) === 'page',
  );

  // ---------- Export in the collection ----------
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(800);
  await run(`$btn('Export collection').click()`);
  await waitFor(`!!$exportPanel()`);
  const panel = await run(
    `({ format: $exportPanel().querySelector('button[aria-label="Export format"]')?.textContent.replace(/\\s+/g, ' ').trim(), hint: $exportPanel().querySelector('.hint')?.textContent.trim(), radios: $exportPanel().querySelectorAll('input[type="radio"]').length, pressed: $btn('Export collection').getAttribute('aria-expanded'), title: document.querySelector('main h1')?.textContent.trim() })`,
  );
  check(
    'Export opens a panel in the collection with a format dropdown',
    panel.format === 'Codestamp' &&
      panel.hint?.startsWith('For another Codestamp') &&
      panel.radios === 0 &&
      panel.pressed === 'true',
    short(panel),
  );
  await run(
    `$pick($exportPanel().querySelector('button[aria-label="Export format"]'), 'Postman Collection v2.1')`,
  );
  const options = await run(
    `(async () => { $exportPanel().querySelector('button[aria-label="Export format"]').click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); const labels = [...document.querySelectorAll('[role="listbox"] [role="option"]')].map((o) => o.textContent.trim()); document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); return labels; })()`,
  );
  await key('Escape', 'Escape', 27);
  check(
    'the dropdown offers Codestamp, Postman Collection v2.1 and Docs as Markdown',
    short(options) === short(['Codestamp', 'Postman Collection v2.1', 'Docs as Markdown']),
    short(options),
  );
  await run(
    `$pick($exportPanel().querySelector('button[aria-label="Export format"]'), 'Codestamp')`,
  );
  await shot('stage40-export.png');

  const codestampFile = path.join(work, 'forty.codestamp.json');
  let answered = answerDialog('Export collection', codestampFile);
  await run(`$btn('Export…', $exportPanel()).click()`);
  await answered;
  await waitFor(`!!$exportPanel()?.querySelector('.exported')`, 6000);
  const codestampNote = await run(
    `$exportPanel().querySelector('.exported')?.innerText.replace(/\\s+/g, ' ') ?? ''`,
  );
  const codestampText = fs.existsSync(codestampFile) ? fs.readFileSync(codestampFile, 'utf8') : '';
  const bundle = codestampText ? JSON.parse(codestampText) : null;
  const paths = Object.keys(bundle?.files ?? {});
  check(
    'a Codestamp file holds the collection, its requests and its environment',
    bundle?.codestamp === 'collection' &&
      paths.filter((p) => p.startsWith('requests/')).length === 2 &&
      paths.some((p) => p.startsWith('environments/')),
    short(paths),
  );
  check(
    'the Codestamp file leaves the secret out and says so',
    !codestampText.includes(SECRET) && codestampNote.includes('secret value'),
    codestampNote.slice(0, 160),
  );

  await run(
    `$pick($exportPanel().querySelector('button[aria-label="Export format"]'), 'Postman Collection v2.1')`,
  );
  const postmanFile = path.join(work, 'forty.collection.json');
  answered = answerDialog('Export collection', postmanFile);
  await run(`$btn('Export…', $exportPanel()).click()`);
  await answered;
  await waitFor(
    `($exportPanel()?.querySelector('.exported')?.textContent ?? '').includes('forty.collection.json')`,
    6000,
  );
  const postman = fs.existsSync(postmanFile)
    ? JSON.parse(fs.readFileSync(postmanFile, 'utf8'))
    : null;
  check(
    'Postman Collection v2.1 is there for other tools',
    postman?.info?.schema?.includes('v2.1.0') === true,
    short(postman?.info),
  );

  await run(
    `$pick($exportPanel().querySelector('button[aria-label="Export format"]'), 'Docs as Markdown')`,
  );
  const docsFile = path.join(work, 'forty.docs.md');
  answered = answerDialog('Save docs', docsFile);
  await run(`$btn('Export…', $exportPanel()).click()`);
  await answered;
  await waitFor(
    `($exportPanel()?.querySelector('.exported')?.textContent ?? '').includes('forty.docs.md')`,
    6000,
  );
  check(
    'Docs export as Markdown from the same panel',
    fs.existsSync(docsFile) && fs.readFileSync(docsFile, 'utf8').includes(NAME),
  );

  // ---------- Import of a Codestamp file ----------
  await run(`$goTo('Import')`);
  await waitFor(`document.querySelector('main h1')?.textContent.trim() === 'Import'`);
  check(
    'Import is its own page again, without an Export tab',
    (await run(`document.querySelectorAll('[role="tab"]').length`)) === 0,
  );
  await run(
    `(() => { const input = document.querySelector('input[aria-label="Collection or environment file"]'); const dt = new DataTransfer(); dt.items.add(new File([${JSON.stringify(codestampText)}], 'forty.codestamp.json', { type: 'application/json' })); input.files = dt.files; input.dispatchEvent(new Event('change', { bubbles: true })); })()`,
  );
  await waitFor(`!!$btn('Import 2 requests')`, 6000);
  const preview = await run(
    `({ source: document.querySelector('.file-text .hint')?.textContent.trim(), stats: [...document.querySelectorAll('.stat')].map((s) => s.textContent.replace(/\\s+/g, ' ').trim()) })`,
  );
  check(
    'Import reads the Codestamp file with its environment',
    preview.source?.endsWith('Codestamp collection') && preview.stats.includes('1environments'),
    short(preview),
  );
  await run(
    `(() => { const input = [...document.querySelectorAll('label.field input')].find((i) => i.value === ${JSON.stringify(NAME)}); input.value = ${JSON.stringify(COPY)}; input.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await run(`$btn('Import 2 requests').click()`);
  await waitFor(`!!$nav(${JSON.stringify(COPY)})`, 6000);
  const copyId = (await invoke('list_collections', {})).ok.find((c) => c.name === COPY)?.id;
  if (copyId) created.push(copyId);
  const copyRows = await run(
    `[...document.querySelectorAll('.row .name')].map((n) => n.textContent.trim())`,
  );
  check(
    'the imported copy has the same requests next to the original',
    !!copyId && copyId !== collectionId && short(copyRows) === short(['List users', 'Get user']),
    short({ copyRows }),
  );

  // ---------- History per collection ----------
  const copyRequest = (await invoke('get_collection', { id: copyId })).ok.requests[0].id;
  await sendFrom(collectionId, requestIds['List users'], `${base}/users`);
  await sendFrom(collectionId, requestIds['List users'], `${base}/missing`);
  await sendFrom(copyId, copyRequest, `${base}/copy`);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(800);
  await run(`$btn('History').click()`);
  await waitFor(`$historyPanel()?.querySelectorAll('.item').length === 2`, 6000);
  const own = await run(
    `[...$historyPanel().querySelectorAll('.item')].map((i) => i.querySelector('.url').textContent.trim())`,
  );
  check(
    'History in the collection lists only what it sent',
    own.length === 2 && own.every((url) => !url.endsWith('/copy')) && own[0].endsWith('/missing'),
    short(own),
  );
  await shot('stage40-history.png');
  await run(
    `(() => { const input = $historyPanel().querySelector('input[type="search"]'); input.value = 'missing'; input.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await waitFor(`$historyPanel().querySelectorAll('.item').length === 1`, 4000);
  check(
    'search narrows the collection history',
    (await run(`$historyPanel().querySelectorAll('.item').length`)) === 1,
  );
  await run(`$btn('Open', $historyPanel()).click()`);
  await waitFor(`document.querySelector('article.card .name input')?.value === 'List users'`, 5000);
  check(
    'Open in History opens the request card',
    (await run(`document.querySelector('article.card .name input')?.value`)) === 'List users',
  );
  await run(
    `(() => { const input = $historyPanel().querySelector('input[type="search"]'); input.value = ''; input.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await waitFor(`$historyPanel().querySelectorAll('.item').length === 2`, 4000);
  await run(`$btn('Clear history', $historyPanel()).click()`);
  await waitFor(`!!$confirm('Clear')`);
  await run(`$confirm('Clear').click()`);
  await waitFor(
    `($historyPanel()?.textContent ?? '').includes('Nothing sent from this collection yet.')`,
    4000,
  );
  const copyHistory = (
    await invoke('list_history', { filter: { query: null, status: 'all', collectionId: copyId } })
  ).ok;
  check(
    'Clear history empties this collection only',
    copyHistory.length === 1,
    short(copyHistory.map((entry) => entry.url)),
  );
  await run(`$btn('History').click()`);
  await run(`$nav('Home').click()`);
  await sleep(700);
  await run(
    `[...document.querySelectorAll('.home [role="tab"]')].find((t) => t.textContent.trim() === 'Workspace').click()`,
  );
  await sleep(300);
  await run(`document.querySelector('.home .tiles input[aria-label="History"]').click()`);
  await sleep(300);
  const homeHistory = await run(
    `[...document.querySelectorAll('.home .stage button.primary')].map((b) => b.textContent.trim())[0]`,
  );
  await run(`document.querySelector('.home .stage button.primary').click()`);
  const fromHome = await waitFor(`!!$historyPanel()`, 5000);
  check(
    'Home’s History tile opens the collection’s History panel',
    homeHistory === `Open History of ${NAME}` && fromHome,
    homeHistory,
  );
  await run(`$btn('Close', $historyPanel())?.click()`);

  // ---------- Recently deleted: Undo right after deleting ----------
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(800);
  await run(`$row('Get user').click()`);
  await waitFor(`!!document.querySelector('article.card')`);
  await run(`$btn('Delete request').click()`);
  await waitFor(`!!$confirm('Delete')`);
  const warning = await run(
    `document.querySelector('dialog[open]:not(.panel-dialog)')?.textContent.replace(/\\s+/g, ' ') ?? ''`,
  );
  check(
    'the delete dialog says where it can be restored from',
    warning.includes('Recently deleted for 30 days') && !warning.includes('cannot be undone'),
    warning.slice(0, 160),
  );
  await run(`$confirm('Delete').click()`);
  await waitFor(`$banner().includes('was deleted')`);
  const banner = await run('$banner()');
  check(
    'deleting shows a banner with Undo',
    banner.includes('“Get user” was deleted') && !!(await run(`$btn('Undo')`)),
    banner,
  );
  await shot('stage40-undo.png');
  await run(`$btn('Undo').click()`);
  // Undo opens what it restored, and an open request shows as its card instead of its row.
  const opened = `document.querySelector('article.card .name input')?.value === 'Get user'`;
  await waitFor(opened, 6000);
  const undone = await run(`({ open: ${opened}, banner: $banner() })`);
  check(
    'Undo puts the request back and opens it',
    undone.open && !undone.banner.includes('was deleted'),
    short(undone),
  );

  // ---------- Recently deleted page ----------
  await run(`$btn('Delete collection').click()`);
  await waitFor(`!!$confirm('Delete')`);
  await run(`$confirm('Delete').click()`);
  await waitFor(`!$nav(${JSON.stringify(NAME)})`);
  await run(`$nav('Recently deleted').click()`);
  await waitFor(
    `[...document.querySelectorAll('.list .item .name')].some((n) => n.textContent.trim() === ${JSON.stringify(NAME)})`,
  );
  const listed = await run(
    `[...document.querySelectorAll('.list .item')].map((i) => i.textContent.replace(/\\s+/g, ' ').trim()).find((t) => t.includes(${JSON.stringify(NAME)}))`,
  );
  check(
    'Recently deleted lists the collection with its requests and days left',
    listed.includes('Collection') &&
      listed.includes('2 requests') &&
      listed.includes('30 days left'),
    listed,
  );
  await shot('stage40-recently-deleted.png');
  await run(
    `[...document.querySelectorAll('.list .item')].find((i) => i.textContent.includes(${JSON.stringify(NAME)})).querySelector('button.restore').click()`,
  );
  await waitFor(`!!$nav(${JSON.stringify(NAME)})`, 6000);
  const back = await run(
    `({ banner: $banner(), listed: [...document.querySelectorAll('.list .item .name')].map((n) => n.textContent.trim()) })`,
  );
  check(
    'Restore brings the collection back and offers to open it',
    back.banner.includes('is back') &&
      !back.listed.includes(NAME) &&
      !!(await run(`$btn('Open', document.querySelector('.banner'))`)),
    short(back),
  );
  const variables = await invoke('get_environment_variables', { id: setId });
  const owned = (await invoke('list_environments', {})).ok;
  const ownedList = Array.isArray(owned) ? owned : owned.environments;
  check(
    'the restored collection keeps its secret and its environment',
    JSON.stringify(variables.ok ?? variables).includes('token') &&
      ownedList.some((env) => env.id === staging && env.collectionId === collectionId),
    short({ variables: variables.ok?.length ?? variables }),
  );

  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(800);
  await run(`$row('List users').click()`);
  await waitFor(`!!document.querySelector('article.card')`);
  await run(`$btn('Delete request').click()`);
  await waitFor(`!!$confirm('Delete')`);
  await run(`$confirm('Delete').click()`);
  await waitFor(`$banner().includes('was deleted')`);
  await run(`$nav('Recently deleted').click()`);
  await waitFor(
    `[...document.querySelectorAll('.list .item .name')].some((n) => n.textContent.trim() === 'List users')`,
  );
  const where = await run(
    `[...document.querySelectorAll('.list .item')].find((i) => i.textContent.includes('List users'))?.querySelector('.where')?.textContent.trim()`,
  );
  check('a deleted request says which collection it goes back to', where === `In “${NAME}”`, where);
  await run(`$btn('Delete “List users” for good').click()`);
  await waitFor(`!!$confirm('Delete')`);
  await run(`$confirm('Delete').click()`);
  await waitFor(
    `![...document.querySelectorAll('.list .item .name')].some((n) => n.textContent.trim() === 'List users')`,
  );
  check(
    'Delete for good removes the copy',
    !(await run(
      `[...document.querySelectorAll('.list .item .name')].some((n) => n.textContent.trim() === 'List users')`,
    )),
  );

  await key(' ', 'Space', 32, 2, ' ');
  await waitFor(`!!document.querySelector('dialog.quick-search[open]')`);
  await run(
    `(() => { const input = document.querySelector('dialog.quick-search input'); input.value = 'restore deleted'; input.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(300);
  const found = await run(
    `[...document.querySelectorAll('dialog.quick-search .result .title')].map((t) => t.textContent.trim())`,
  );
  check('quick search finds Recently deleted', found.includes('Recently deleted'), short(found));
  await key('Escape', 'Escape', 27);
} catch (error) {
  console.log(results.join('\n'));
  throw error;
} finally {
  for (const id of created) await invoke('delete_collection', { id });
  for (const id of envIds) await invoke('delete_environment', { id });
  const left = (await invoke('list_deleted', {})).ok ?? [];
  for (const item of left.filter(
    (entry) => entry.name.startsWith('E2E') || ['List users', 'Get user'].includes(entry.name),
  )) {
    await invoke('forget_deleted', { id: item.id });
  }
  fs.rmSync(work, { recursive: true, force: true });
  server.close();
  await run('location.reload()').catch(() => null);
  await sleep(1500);
  socket.close();
}
console.log(results.join('\n'));
console.log(`${results.filter((line) => line.startsWith('PASS')).length}/${results.length} passed`);
