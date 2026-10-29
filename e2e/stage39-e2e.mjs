// End-to-end check of the 2026-10-28 fixes: the window opens centred, only the active environment
// is marked green, Docs sections fold, the sidebar's Tools group folds, and the status bar's gear
// opens Settings. Launches the app.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const shots = path.join(here, '.work', 'screens');
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const powershell = (file) =>
  execFileSync(
    'powershell',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(here, file)],
    { encoding: 'utf8' },
  ).trim();
const KEYS = ['codestamp.foldedNavGroups', 'codestamp.openDocsSections'];

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
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$heading = (title) => [...document.querySelectorAll('nav.sidebar h2 button')].find((b) => b.textContent.trim() === title);
  window.$docsIndex = () => document.querySelector('nav.index');
  window.$docsFold = (name) => $docsIndex()?.querySelector('button.fold[aria-label$="section ' + name + '"]');
  window.$nested = () => [...($docsIndex()?.querySelectorAll('.entry.nested') ?? [])].map((e) => e.textContent.replace(/\\s+/g, ' ').trim());
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

execFileSync(process.execPath, [path.join(here, 'launch.mjs')], { stdio: 'inherit' });
// Read before anything resizes the window.
const place = JSON.parse(powershell('window-rect.ps1'));
await connect();
let collectionId = null;
const envIds = [];
try {
  const [left, top, right, bottom] = place.window;
  const [workLeft, workTop, workRight, workBottom] = place.work;
  const [monitorLeft, monitorTop, monitorRight, monitorBottom] = place.monitor;
  // Windows counts an invisible resize frame (up to 8 px a side) in the window's rectangle.
  const FRAME = 12;
  const offsetX = Math.abs((left + right) / 2 - (workLeft + workRight) / 2);
  const offsetY = Math.abs((top + bottom) / 2 - (workTop + workBottom) / 2);
  if (!process.env.E2E_SIZE)
    check(
      'the window opens in the middle of its screen',
      offsetX <= FRAME &&
        offsetY <= FRAME &&
        left >= monitorLeft &&
        right <= monitorRight &&
        top >= monitorTop &&
        bottom <= monitorBottom,
      short({ ...place, offsetX, offsetY }),
    );

  await run(`(${JSON.stringify(KEYS)}).forEach((k) => localStorage.removeItem(k))`);
  collectionId = (
    await invoke('create_collection', { name: 'E2E Fold', icon: null, baseUrl: null })
  ).ok.id;
  for (const name of ['Alpha', 'Beta']) {
    const section = (await invoke('create_section', { collectionId, name })).ok;
    for (const step of ['one', 'two', 'three']) {
      await invoke('create_request', {
        collectionId,
        sectionId: section.id,
        content: request(`${name} ${step}`),
      });
    }
  }
  for (const name of ['E2E Dev', 'E2E Prod']) {
    envIds.push((await invoke('create_environment', { name })).ok.id);
  }
  await invoke('set_active_environment', { id: envIds[1] });
  await reload();
  await run(`$nav('E2E Fold').click()`);
  await sleep(900);

  // ---------- only the active environment is green ----------
  const trigger = `[...document.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith('E2E Prod'))`;
  await run(`${trigger}.click()`);
  await sleep(250);
  const dots = await run(
    `Object.fromEntries([...document.querySelectorAll('[role="listbox"] [role="option"]')].map((o) => [o.textContent.trim(), o.querySelector('.dot')?.getAttribute('style') ?? '']))`,
  );
  await shot('stage39-environments.png');
  await key('Escape', 'Escape', 27);
  const green = Object.entries(dots)
    .filter(([, style]) => style.includes('--dot-ok'))
    .map(([label]) => label);
  check(
    'only the active environment has a green dot',
    green.length === 1 &&
      green[0].startsWith('E2E Prod') &&
      Object.keys(dots).some((label) => label.startsWith('E2E Dev')),
    short(dots),
  );

  // ---------- Docs sections start folded and remember what was opened ----------
  await run(`$nav('Docs').click()`);
  await waitFor(`!!$docsFold('Alpha')`);
  const folded = await run(
    `({ nested: $nested(), alpha: $docsFold('Alpha').getAttribute('aria-expanded'), beta: $docsFold('Beta').getAttribute('aria-expanded') })`,
  );
  check(
    'Docs sections start folded, so the index stays short',
    folded.nested.length === 0 && folded.alpha === 'false' && folded.beta === 'false',
    short(folded),
  );
  await run(`$docsFold('Alpha').focus()`);
  await key('Enter', 'Enter', 13, 0, String.fromCharCode(13));
  const opened = await run(
    `({ nested: $nested(), alpha: $docsFold('Alpha').getAttribute('aria-expanded'), label: $docsFold('Alpha').getAttribute('aria-label') })`,
  );
  check(
    'the chevron opens a section from the keyboard',
    opened.nested.length === 3 &&
      opened.alpha === 'true' &&
      opened.label === 'Collapse section Alpha',
    short(opened),
  );
  await shot('stage39-docs.png');
  await reload();
  // After a restart the first collection opens; Docs follows the open collection.
  await run(`$nav('E2E Fold').click()`);
  await sleep(700);
  await run(`$nav('Docs').click()`);
  await waitFor(`!!$docsFold('Alpha')`);
  const remembered = await run(
    `({ alpha: $docsFold('Alpha').getAttribute('aria-expanded'), beta: $docsFold('Beta').getAttribute('aria-expanded') })`,
  );
  check(
    'an opened section stays open after a restart',
    remembered.alpha === 'true' && remembered.beta === 'false',
    short(remembered),
  );
  await run(
    `[...$docsIndex().querySelectorAll('.entry.group')].find((b) => b.textContent.includes('Beta')).click()`,
  );
  await sleep(400);
  const picked = await run(
    `({ beta: $docsFold('Beta').getAttribute('aria-expanded'), title: document.getElementById('doc-section-title')?.textContent.trim() })`,
  );
  check(
    'picking a section shows its doc and its requests',
    picked.beta === 'true' && picked.title === 'Beta',
    short(picked),
  );
  await run(`$docsFold('Beta').click(); $docsFold('Alpha').click()`);
  await sleep(200);
  await run(
    `$set($docsIndex().querySelector('input[type="search"], input[aria-label^="Search the docs"]'), 'three')`,
  );
  await sleep(500);
  const searching = await run(`({ nested: $nested(), disabled: $docsFold('Alpha')?.disabled })`);
  check(
    'while searching, matches show even in folded sections',
    searching.nested.length === 2 && searching.disabled === true,
    short(searching),
  );
  await run(
    `$set($docsIndex().querySelector('input[type="search"], input[aria-label^="Search the docs"]'), '')`,
  );
  await sleep(400);
  check('clearing the search folds them again', (await run(`$nested().length`)) === 0);

  // ---------- the sidebar's Tools and Settings fold ----------
  const forcedRail = await run(`innerWidth < 900`);
  if (forcedRail) {
    const rail = await run(`({ flows: !!$nav('Orchestration'), env: !!$nav('Environments') })`);
    check(
      'in a narrow window the icon rail and the status bar show every page',
      rail.flows && rail.env,
      short(rail),
    );
  } else {
    await run(`$heading('Tools').click()`);
    await sleep(200);
    const tools = await run(
      `({ expanded: $heading('Tools').getAttribute('aria-expanded'), flows: !!$nav('Orchestration'), env: !!$nav('Environments') })`,
    );
    check(
      'Tools folds to its heading',
      tools.expanded === 'false' && !tools.flows && tools.env,
      short(tools),
    );
    await shot('stage39-sidebar-folded.png');
    await reload();
    const after = await run(
      `({ tools: $heading('Tools')?.getAttribute('aria-expanded'), flows: !!$nav('Orchestration'), settings: !!$btn('Settings', document.querySelector('footer.statusbar')) })`,
    );
    check(
      'a folded group stays folded after a restart',
      after.tools === 'false' && !after.flows && after.settings,
      short(after),
    );
    await key('b', 'KeyB', 66, 2);
    await sleep(400);
    const rail = await run(
      `({ rail: document.querySelector('nav.sidebar').classList.contains('rail'), flows: !!$nav('Orchestration'), env: !!$nav('Environments') })`,
    );
    check('the icon rail still shows every page', rail.rail && rail.flows && rail.env, short(rail));
    await key('b', 'KeyB', 66, 2);
    await sleep(400);
    await run(`$heading('Tools').click()`);
    await sleep(200);
  }

  // ---------- the heading buttons line up with the counts of the rows ----------
  if (!forcedRail) {
    const aligned = await run(
      `(() => { const nav = document.querySelector('nav.sidebar'); const right = (el) => Math.round(el.getBoundingClientRect().right); const count = nav.querySelector('.item .count'); return { count: right(count), plus: right(nav.querySelector('button.new svg')) }; })()`,
    );
    check(
      'the + icon ends where the row counts end',
      Math.abs(aligned.plus - aligned.count) <= 1,
      short(aligned),
    );
  }

  // ---------- the status bar's gear opens Settings ----------
  await run(`$btn('Settings', document.querySelector('footer.statusbar')).click()`);
  await waitFor(`document.querySelector('h1.page-title')?.textContent.trim() === 'Settings'`);
  const gear = await run(
    `({ title: document.querySelector('h1.page-title')?.textContent.trim(), current: $btn('Settings', document.querySelector('footer.statusbar')).getAttribute('aria-current') })`,
  );
  check(
    'the status bar gear opens the Settings page',
    gear.title === 'Settings' && gear.current === 'page',
    short(gear),
  );
  await shot('stage39-settings.png');
} catch (error) {
  console.log(results.join('\n'));
  throw error;
} finally {
  await run(`(${JSON.stringify(KEYS)}).forEach((k) => localStorage.removeItem(k))`).catch(
    () => null,
  );
  for (const id of envIds) await invoke('delete_environment', { id });
  if (collectionId) await invoke('delete_collection', { id: collectionId });
  // The next suite reads the sidebar, so it should not still list what was deleted here.
  await run('location.reload()').catch(() => null);
  await sleep(1500);
  socket.close();
}
console.log(results.join('\n'));
console.log(`${results.filter((line) => line.startsWith('PASS')).length}/${results.length} passed`);
