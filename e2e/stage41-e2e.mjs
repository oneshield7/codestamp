// End-to-end check of the 2026-10-30 batch: an HTML response opens as a sandboxed page, the
// popover over a {{variable}} shows and saves its value, and each collection picks its own
// environment from its own ones and the ones kept on this device. Launches the app.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const shots = path.join(here, '.work', 'screens');
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const NAME = 'E2E FortyOne';
const OTHER = 'E2E FortyOne other';
const OWN_ENV = 'E2E FortyOne own';
const DEVICE_ENV = 'E2E FortyOne device';
const TOKEN = 'TOKEN-41';

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
  fs.mkdirSync(shots, { recursive: true });
  const image = await cdp('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync(path.join(shots, name), Buffer.from(image.result.data, 'base64'));
}
async function key(name, code, vk) {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', { type, key: name, code, windowsVirtualKeyCode: vk });
  }
  await sleep(200);
}
async function pointAt(expression) {
  const box = await run(
    `(() => { const r = (${expression}).getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`,
  );
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: box.x, y: box.y });
}
async function pointAway() {
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 4, y: 4 });
  await sleep(500);
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
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim() === text || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$row = (name) => [...document.querySelectorAll('.row')].find((r) => r.querySelector('.name')?.textContent.trim() === name);
  window.$card = () => document.querySelector('article.card');
  window.$popover = (name) => document.querySelector('[role="dialog"][aria-label="Variable ' + name + '"]');
  window.$set = (input, value) => { const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; setter.call(input, value); input.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$envTrigger = () => document.querySelector('header button[aria-label="Active environment"], button[aria-label="Active environment"]');
  window.$envOptions = async () => { $envTrigger().click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); const labels = [...document.querySelectorAll('[role="listbox"] [role="option"]')].map((o) => o.textContent.trim()); $envTrigger().click(); await new Promise((r) => setTimeout(r, 100)); return labels; };
  window.$goTo = async (title) => { document.querySelector('nav.sidebar button.search').click(); await new Promise((r) => setTimeout(r, 200)); const input = document.querySelector('dialog.quick-search input'); input.value = title; input.dispatchEvent(new Event('input', { bubbles: true })); await new Promise((r) => setTimeout(r, 300)); [...document.querySelectorAll('dialog.quick-search .result')].find((r) => r.querySelector('.title')?.textContent.trim() === title)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); };
  window.$tab = (label) => [...$card().querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(label));
  window.$pickEnv = async (label) => { $envTrigger().click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label).dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 300)); };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const request = (name, fields) => ({
  kind: 'http',
  name,
  method: 'GET',
  url: '',
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
  ...fields,
});

const hits = [];
const server = http.createServer((req, res) => {
  hits.push(req.url);
  if (req.url.startsWith('/error')) {
    const page = `<!DOCTYPE html><html lang="en"><head><title>AttributeError at /inet</title>
      <style>body { font: small sans-serif; background: #fff; color: #000 } h1 { color: #b00 }</style>
      </head><body><h1>AttributeError at /inet/loadData</h1><p>The server could not finish.</p>
      <img src="${base}/beacon-image" alt=""><script>fetch('${base}/beacon-script')</script>
      </body></html>`;
    res.writeHead(500, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(page);
    return;
  }
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ path: req.url }));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;

execFileSync(process.execPath, [path.join(here, 'launch.mjs')], { stdio: 'inherit' });
await connect();
const created = [];
const envIds = [];
try {
  const collectionId = (
    await invoke('create_collection', { name: NAME, icon: null, baseUrl: null })
  ).ok.id;
  created.push(collectionId);
  const otherId = (await invoke('create_collection', { name: OTHER, icon: null, baseUrl: null })).ok
    .id;
  created.push(otherId);
  const section = (await invoke('create_section', { collectionId, name: 'Pages' })).ok;
  for (const content of [
    request('Server error', { url: '{{baseUrl}}/error' }),
    request('Items', {
      method: 'POST',
      url: '{{baseUrl}}/items',
      headers: [{ enabled: true, key: 'Authorization', value: 'Bearer {{token}}' }],
      bodyMode: 'json',
      body: '{\n  "home": "{{baseUrl}}"\n}',
    }),
  ]) {
    await invoke('create_request', { collectionId, sectionId: section.id, content });
  }
  const otherSection = (await invoke('create_section', { collectionId: otherId, name: 'Main' })).ok;
  await invoke('create_request', {
    collectionId: otherId,
    sectionId: otherSection.id,
    content: request('Region', { url: 'https://{{region}}.test/x' }),
  });
  const setId = (await invoke('get_collection_variable_set', { collectionId })).ok;
  await invoke('save_environment_variables', {
    id: setId,
    variables: [
      { id: null, key: 'baseUrl', value: base, isSecret: false, enabled: true },
      { id: null, key: 'token', value: TOKEN, isSecret: true, enabled: true },
    ],
  });
  const own = (await invoke('create_environment', { name: OWN_ENV })).ok.id;
  const device = (await invoke('create_environment', { name: DEVICE_ENV })).ok.id;
  envIds.push(own, device);
  await invoke('set_environment_collection', { id: own, collectionId });
  for (const [id, region] of [
    [own, 'own-region'],
    [device, 'device-region'],
  ]) {
    await invoke('save_environment_variables', {
      id,
      variables: [{ id: null, key: 'region', value: region, isSecret: false, enabled: true }],
    });
  }
  await reload();
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);

  // --- an HTML response opens as a page ---
  await run(`$row('Server error').click()`);
  await waitFor(`!!$card()`);
  await run(`$btn('Send', $card()).click()`);
  await waitFor(`$card().querySelector('section.response .chip')?.textContent.includes('500')`);
  const page = await run(`(() => {
    const frame = $card().querySelector('section.response iframe');
    const pressed = [...$card().querySelectorAll('section.response .segmented button')].map((b) => b.textContent.trim() + ':' + b.getAttribute('aria-pressed'));
    return { title: frame?.getAttribute('title'), sandbox: frame?.getAttribute('sandbox'), html: frame?.getAttribute('srcdoc')?.includes('AttributeError at /inet/loadData'), pressed, hint: $card().querySelector('section.response .view-bar .hint')?.textContent.trim() };
  })()`);
  check(
    'an HTML response opens as the page, in an empty sandbox',
    page.title === 'Response page' &&
      page.sandbox === '' &&
      page.html === true &&
      short(page.pressed) === short(['Page:true', 'Source:false']) &&
      page.hint === 'Scripts and files from other sites are not loaded.',
    short(page),
  );
  await sleep(1200);
  await shot('stage41-html-page.png');
  const beacons = hits.filter((url) => url.startsWith('/beacon'));
  check('the page runs no script and loads no outside file', beacons.length === 0, short(beacons));
  await run(`$btn('Source', $card()).click()`);
  await sleep(300);
  const source = await run(
    `(() => ({ frame: !!$card().querySelector('section.response iframe'), text: $card().querySelector('section.response .cm-content')?.textContent ?? '' }))()`,
  );
  check(
    'Source shows the HTML text instead',
    !source.frame && source.text.includes('<title>AttributeError at /inet</title>'),
    short({ frame: source.frame, text: source.text.slice(0, 80) }),
  );
  await run(`$btn('Page', $card()).click()`);
  await sleep(300);
  check(
    'Page brings the rendered page back',
    await run(`!!$card().querySelector('section.response iframe')`),
  );

  // --- the popover over a variable ---
  await run(`$row('Items').click()`);
  await waitFor(`!!$card()?.querySelector('.mirror [data-variable="baseUrl"]')`);
  await pointAt(`$card().querySelector('.mirror [data-variable="baseUrl"]')`);
  await waitFor(`!!$popover('baseUrl')`, 3000);
  const first = await run(
    `(() => { const p = $popover('baseUrl'); return p && { where: p.querySelector('.where')?.textContent.trim(), value: p.querySelector('input')?.value }; })()`,
  );
  check(
    'resting on {{baseUrl}} shows its value and where it comes from',
    first?.where === 'Collection' && first?.value === base,
    short(first),
  );
  await shot('stage41-variable-popover.png');
  await run(`$set($popover('baseUrl').querySelector('input'), ${JSON.stringify(`${base}/v2`)})`);
  await run(`$btn('Save', $popover('baseUrl')).click()`);
  await waitFor(
    `$popover('baseUrl')?.querySelector('[role="status"]')?.textContent.trim() === 'Saved'`,
  );
  const stored = (await invoke('get_environment_variables', { id: setId })).ok;
  check(
    'Save in the popover changes the collection variable',
    stored.find((v) => v.key === 'baseUrl')?.value === `${base}/v2` &&
      stored.find((v) => v.key === 'token')?.isSecret === true,
    short(stored.map((v) => [v.key, v.value])),
  );
  const resolved = await waitFor(
    `$card().querySelector('.preview')?.textContent.includes('/v2/items')`,
    4000,
  );
  check(
    'the Resolves to line follows the new value',
    resolved,
    await run(`$card().querySelector('.preview')?.textContent.trim()`),
  );
  await run(`$popover('baseUrl').querySelector('input').focus()`);
  await key('Escape', 'Escape', 27);
  check('Escape closes the popover', await waitFor(`!$popover('baseUrl')`, 2000));
  await pointAway();

  await run(`$tab('Params & Headers').click()`);
  await waitFor(`!!$card().querySelector('.mirror [data-variable="token"]')`, 3000);
  await pointAt(`$card().querySelector('.mirror [data-variable="token"]')`);
  await waitFor(`!!$popover('token')`, 3000);
  const hidden = await run(
    `(() => { const i = $popover('token')?.querySelector('input'); return i && { type: i.type, value: i.value, show: !!$btn('Show', $popover('token')) }; })()`,
  );
  await run(`$btn('Show', $popover('token'))?.click()`);
  await waitFor(`$popover('token')?.querySelector('input')?.type === 'text'`, 3000);
  const shown = await run(`$popover('token')?.querySelector('input')?.value`);
  check(
    'a secret stays hidden until Show reads it',
    hidden?.type === 'password' && hidden.value === '' && hidden.show && shown === TOKEN,
    short({ hidden, shown }),
  );
  await key('Escape', 'Escape', 27);
  await pointAway();
  await waitFor(`!$popover('token')`, 2000);

  await run(
    `(() => { const i = $card().querySelector('input.url'); i.focus(); i.setSelectionRange(3, 3); })()`,
  );
  await key('ArrowRight', 'ArrowRight', 39);
  check(
    'moving the caret into {{baseUrl}} opens the popover from the keyboard',
    await waitFor(`!!$popover('baseUrl')`, 2000),
  );
  await key('Escape', 'Escape', 27);
  await waitFor(`!$popover('baseUrl')`, 2000);

  await run(`$tab('Body').click()`);
  await waitFor(`!!$card().querySelector('.cm-content [data-variable="baseUrl"]')`, 3000);
  await pointAt(`$card().querySelector('.cm-content [data-variable="baseUrl"]')`);
  await waitFor(`!!$popover('baseUrl')`, 3000);
  check(
    'the body editor shows the same popover',
    (await run(`$popover('baseUrl')?.querySelector('input')?.value`)) === `${base}/v2`,
  );
  await pointAway();

  // --- a collection only offers its own environments ---
  const ownOptions = await run(`$envOptions()`);
  check(
    "a collection offers its own environments and this device's",
    ownOptions.includes(OWN_ENV) && ownOptions.includes(`${DEVICE_ENV} · This device`),
    short(ownOptions),
  );
  await run(`$pickEnv(${JSON.stringify(OWN_ENV)})`);
  await run(`$nav(${JSON.stringify(OTHER)}).click()`);
  await sleep(900);
  const otherOptions = await run(`$envOptions()`);
  const otherShows = await run(`$envTrigger().textContent.trim()`);
  check(
    'another collection does not offer it and keeps its own choice (none yet)',
    !otherOptions.includes(OWN_ENV) &&
      otherOptions.includes(`${DEVICE_ENV} · This device`) &&
      otherShows === 'No environment',
    short({ otherOptions, otherShows }),
  );
  const preview = async (id) =>
    (
      await invoke('preview_url', {
        url: 'https://{{region}}.test/x',
        params: [],
        pathVariables: [],
        collectionId: id,
      })
    ).ok;
  const inOwn = await preview(collectionId);
  const inOther = await preview(otherId);
  check(
    "another collection's environment fills in nothing here",
    inOwn?.url === 'https://own-region.test/x' && inOther?.missing?.includes('region'),
    short({ inOwn, inOther }),
  );
  await run(`$pickEnv(${JSON.stringify(`${DEVICE_ENV} · This device`)})`);
  const fromDevice = await preview(otherId);
  check(
    'an environment kept on this device works in any collection',
    fromDevice?.url === 'https://device-region.test/x',
    short(fromDevice),
  );
  const ownStill = await preview(collectionId);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  const ownShows = await run(`$envTrigger().textContent.trim()`);
  check(
    'each collection keeps its own environment',
    ownStill?.url === 'https://own-region.test/x' && ownShows === OWN_ENV,
    short({ ownStill, ownShows }),
  );

  // --- the collection header stays on top while scrolling ---
  for (let step = 1; step <= 40; step += 1) {
    await invoke('create_request', {
      collectionId: otherId,
      sectionId: otherSection.id,
      content: request(`Long list ${String(step).padStart(2, '0')}`, {
        url: 'https://long.test/x',
      }),
    });
  }
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(600);
  await run(`$nav(${JSON.stringify(OTHER)}).click()`);
  await waitFor(`!!$row('Long list 40')`);
  await run(`document.querySelector('.scroller').scrollTop = 4000`);
  await sleep(400);
  const stuck = await run(`(() => {
    const scroller = document.querySelector('.scroller').getBoundingClientRect();
    const head = document.querySelector('header.top');
    const box = head.getBoundingClientRect();
    const env = $envTrigger().getBoundingClientRect();
    const hit = document.elementFromPoint(env.left + env.width / 2, env.top + env.height / 2);
    return { offset: Math.round(box.top - scroller.top), stuck: head.classList.contains('stuck'), scrolled: document.querySelector('.scroller').scrollTop, envOnTop: !!hit && $envTrigger().contains(hit) };
  })()`);
  await shot('stage41-sticky-header.png');
  check(
    'scrolled down, the header stays at the top with its controls on top',
    stuck.offset === 0 && stuck.stuck && stuck.scrolled > 1000 && stuck.envOnTop,
    short(stuck),
  );
  const scrollTop = () => run(`document.querySelector('.scroller').scrollTop`);
  const before = await scrollTop();
  const trigger = await run(
    `(() => { const r = $envTrigger().getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`,
  );
  for (const type of ['mousePressed', 'mouseReleased']) {
    await cdp('Input.dispatchMouseEvent', { type, ...trigger, button: 'left', clickCount: 1 });
  }
  await waitFor(`!!document.querySelector('[role="listbox"] [role="option"]')`, 2000);
  await key('Escape', 'Escape', 27);
  const afterEscape = await scrollTop();
  await run(`$pickEnv(${JSON.stringify(`${DEVICE_ENV} · This device`)})`);
  await run(`$pickEnv('No environment')`);
  const after = { before, afterEscape, afterPick: await scrollTop() };
  after.shows = await run(`$envTrigger().textContent.trim()`);
  check(
    'the environment list opens, closes and changes from the stuck header without scrolling',
    Math.abs(after.afterEscape - before) <= 2 &&
      Math.abs(after.afterPick - before) <= 2 &&
      after.shows === 'No environment',
    short(after),
  );
  await run(`$goTo('Long list 20')`);
  await waitFor(`$card()?.getAttribute('aria-label')?.includes('Long list 20') ?? !!$card()`, 3000);
  await sleep(1200);
  const below = await run(`(() => {
    const head = document.querySelector('header.top').getBoundingClientRect();
    const card = $card().getBoundingClientRect();
    return { head: Math.round(head.bottom), card: Math.round(card.top), window: innerHeight };
  })()`);
  check(
    'a card opened from Quick search lands below the header, not under it',
    below.card >= below.head && below.card < below.head + 60,
    short(below),
  );
} catch (error) {
  console.log(results.join('\n'));
  throw error;
} finally {
  for (const id of created) await invoke('delete_collection', { id });
  for (const id of envIds) await invoke('delete_environment', { id });
  const left = (await invoke('list_deleted', {})).ok ?? [];
  for (const item of left.filter(
    (entry) =>
      entry.name.startsWith('E2E') ||
      entry.name.startsWith('Long list') ||
      ['Server error', 'Items', 'Region'].includes(entry.name),
  )) {
    await invoke('forget_deleted', { id: item.id });
  }
  server.close();
  await run('location.reload()').catch(() => null);
  await sleep(1500);
  socket.close();
}
console.log(results.join('\n'));
console.log(`${results.filter((line) => line.startsWith('PASS')).length}/${results.length} passed`);
