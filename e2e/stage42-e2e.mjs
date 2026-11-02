// End-to-end check of workspaces: the switcher in the title bar, a new workspace that starts
// empty, Quick search switching back, moving a collection with the environment it owns, cookies
// kept per workspace, Settings › Workspaces, and workspace icons (a logo found by name or
// website, an image file, an emoji) shown on the launch screen. A local server stands in for
// Google's icon service. Launches the app.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import zlib from 'node:zlib';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const shots = path.join(here, '.work', 'screens');
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const SHOP = 'E2E FortyTwo Shop';
const DEMO = 'E2E FortyTwo demo';
const RENAMED = 'E2E FortyTwo renamed';
const EMPTY = 'E2E FortyTwo empty';
const OWN_ENV = 'E2E FortyTwo own';
const DEVICE_ENV = 'E2E FortyTwo device';
const COOKIE = 'e2e42';

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
    if (await run(expression).catch(() => false)) return true;
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
/** Waits for the page a workspace switch reloaded, then puts the helpers back. */
async function settled(expectedName) {
  await sleep(1200);
  await waitFor(
    `!document.getElementById('launch') && !!document.querySelector('nav.sidebar') && !!document.querySelector('header.titlebar button[aria-label^="Workspace:"]')?.textContent.includes(${JSON.stringify(expectedName)})`,
    10000,
  );
  await run(helpers);
  await waitFor(`$switcher()?.textContent.includes(${JSON.stringify(expectedName)})`, 8000);
}
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim() === text || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$switcher = () => document.querySelector('header.titlebar button[aria-haspopup="menu"][aria-label^="Workspace:"]');
  window.$menu = () => document.querySelector('ul[role="menu"][aria-label="Workspaces"]');
  window.$entries = () => [...($menu()?.querySelectorAll('[role="menuitemradio"], [role="menuitem"]') ?? [])].map((li) => ({ role: li.getAttribute('role'), text: li.querySelector('.label')?.textContent.trim(), checked: li.getAttribute('aria-checked') }));
  window.$choose = (label) => [...$menu().querySelectorAll('[role="menuitemradio"], [role="menuitem"]')].find((li) => li.querySelector('.label')?.textContent.trim() === label).dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));
  window.$set = (input, value) => { const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; setter.call(input, value); input.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$dialog = (titleId) => document.querySelector('dialog[open][aria-labelledby="' + titleId + '"]');
  window.$search = async (text) => { document.querySelector('nav.sidebar button.search').click(); await new Promise((r) => setTimeout(r, 250)); const input = document.querySelector('dialog.quick-search input'); input.value = text; input.dispatchEvent(new Event('input', { bubbles: true })); await new Promise((r) => setTimeout(r, 400)); return [...document.querySelectorAll('dialog.quick-search .result .title')].map((t) => t.textContent.trim()); };
  window.$pickResult = (title) => [...document.querySelectorAll('dialog.quick-search .result')].find((r) => r.querySelector('.title')?.textContent.trim() === title)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));
  window.$rows = () => [...document.querySelectorAll('ul[aria-label="Workspaces"] li.row')].map((li) => ({ name: li.querySelector('.about .name')?.firstChild?.textContent.trim(), open: !!li.querySelector('.chip'), detail: li.querySelector('.about .muted')?.textContent.trim(), deletable: !li.querySelector('button.danger')?.disabled }));
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const names = (list) => list.map((item) => item.name);

/** A solid square PNG, enough for a logo the app must show. */
function png(size, [red, green, blue]) {
  const chunk = (type, data) => {
    const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(zlib.crc32(body));
    return Buffer.concat([length, body, crc]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(size, 0);
  header.writeUInt32BE(size, 4);
  header.set([8, 2, 0, 0, 0], 8);
  const row = Buffer.concat([
    Buffer.from([0]),
    Buffer.from(Array(size).fill([red, green, blue]).flat()),
  ]);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', zlib.deflateSync(Buffer.concat(Array(size).fill(row)))),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}
const RED = png(32, [200, 40, 40]);
const BLUE = png(32, [40, 80, 200]);
const GREEN = png(32, [30, 150, 70]);
// What the stand-in icon service knows: the renamed workspace's .com and .id share one icon, its
// .co.id is only known with www., and a typed website has its own.
const ICONS = new Map([
  ['e2efortytworenamed.com', RED],
  ['e2efortytworenamed.id', RED],
  ['www.e2efortytworenamed.co.id', BLUE],
  ['e2e-site.org', GREEN],
]);
const iconLookups = [];
const server = http.createServer((req, res) => {
  if (req.url.startsWith('/s2/favicons')) {
    const domain = new URL(req.url, 'http://localhost').searchParams.get('domain');
    iconLookups.push(domain);
    const icon = ICONS.get(domain);
    res.writeHead(icon ? 200 : 404, { 'Content-Type': 'image/png' });
    res.end(icon ?? png(16, [128, 128, 128]));
    return;
  }
  res.writeHead(200, {
    'Content-Type': 'application/json',
    'Set-Cookie': `${COOKIE}=first; Path=/`,
  });
  res.end('{}');
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
const cookieHere = async () =>
  ((await invoke('list_cookies', {})).ok ?? []).some((cookie) => cookie.name === COOKIE);

process.env.CODESTAMP_ICON_SERVICE = `${base}/s2/favicons`;
const imageFile = path.join(here, '.work', 'stage42-icon.png');
fs.mkdirSync(path.dirname(imageFile), { recursive: true });
fs.writeFileSync(imageFile, png(300, [120, 60, 160]));
execFileSync(process.execPath, [path.join(here, 'launch.mjs')], { stdio: 'inherit' });
await connect();
const start = (await invoke('list_workspaces', {})).ok;
const original = start.workspaces.find((item) => item.id === start.currentId);
const madeWorkspaces = [];
let shopId = null;
let deviceId = null;
try {
  shopId = (await invoke('create_collection', { name: SHOP, icon: null, baseUrl: null })).ok.id;
  const own = (await invoke('create_environment', { name: OWN_ENV })).ok.id;
  deviceId = (await invoke('create_environment', { name: DEVICE_ENV })).ok.id;
  await invoke('set_environment_collection', { id: own, collectionId: shopId });
  await invoke('set_active_environment', { collectionId: shopId, id: deviceId });
  await invoke('send_http_request', {
    request: {
      method: 'GET',
      url: `${base}/login`,
      params: [],
      headers: [],
      body: null,
      timeoutMs: null,
      collectionId: null,
      requestId: null,
      name: null,
      preRequestScript: null,
      testScript: null,
      auth: null,
      recordHistory: false,
      pathVariables: [],
    },
  });
  await run('location.reload()');
  await settled(original.name);

  // --- the switcher in the title bar ---
  const trigger = await run(`(() => {
    const r = $switcher().getBoundingClientRect();
    return { center: Math.round(r.left + r.width / 2), window: Math.round(innerWidth / 2), label: $switcher().getAttribute('aria-label'), inBar: !!$switcher().closest('header.titlebar') };
  })()`);
  check(
    'the switcher sits in the middle of the title bar and names the open workspace',
    trigger.inBar &&
      Math.abs(trigger.center - trigger.window) <= 2 &&
      trigger.label === `Workspace: ${original.name}`,
    short(trigger),
  );
  await run(`$switcher().focus()`);
  await key('ArrowDown', 'ArrowDown', 40);
  await waitFor(`!!$menu()`, 2000);
  const menu = await run(`({ entries: $entries(), focused: document.activeElement === $menu() })`);
  const radios = menu.entries.filter((entry) => entry.role === 'menuitemradio');
  check(
    'the keyboard opens a menu of workspaces with the open one checked, then New and Manage',
    menu.focused &&
      radios.some((entry) => entry.text === original.name && entry.checked === 'true') &&
      menu.entries.at(-2)?.text === 'New workspace' &&
      menu.entries.at(-1)?.text === 'Manage workspaces',
    short(menu),
  );
  await shot('stage42-switcher-menu.png');
  await key('Escape', 'Escape', 27);
  const closed = await run(
    `({ open: !!$menu(), back: document.activeElement === $switcher(), expanded: $switcher().getAttribute('aria-expanded') })`,
  );
  check(
    'Escape closes the menu and gives the keyboard back to the switcher',
    !closed.open && closed.back && closed.expanded === 'false',
    short(closed),
  );

  // --- a new workspace starts empty ---
  await run(`$switcher().click()`);
  await waitFor(`!!$menu()`, 2000);
  await run(`$choose('New workspace')`);
  await waitFor(`!!$dialog('workspace-dialog-title')`, 3000);
  const dialog = await run(`(() => {
    const d = $dialog('workspace-dialog-title');
    const colors = [...d.querySelectorAll('input[name="workspace-hue"]')];
    return { focused: document.activeElement?.id === 'workspace-name', colors: colors.length, picked: colors.find((c) => c.checked)?.value, button: $btn('Create workspace', d)?.disabled };
  })()`);
  check(
    'New workspace asks for a name and offers 8 colors, starting on one no workspace uses yet',
    dialog.focused &&
      dialog.colors === 8 &&
      dialog.picked !== original.hue &&
      dialog.button === true,
    short(dialog),
  );
  await run(`$set(document.getElementById('workspace-name'), ${JSON.stringify(DEMO)})`);
  await shot('stage42-new-workspace.png');
  await run(`$btn('Create workspace', $dialog('workspace-dialog-title')).click()`);
  await settled(DEMO);
  const demoList = (await invoke('list_workspaces', {})).ok;
  const demo = demoList.workspaces.find((item) => item.name === DEMO);
  if (demo) madeWorkspaces.push(demo.id);
  const fresh = await run(`({
    shop: !!$nav(${JSON.stringify(SHOP)}),
    collections: document.querySelectorAll('nav.sidebar .item .icon.emoji').length,
    none: document.querySelector('nav.sidebar p.none')?.textContent.trim(),
  })`);
  const freshEnvs = (await invoke('list_environments', {})).ok.environments;
  check(
    'creating a workspace opens it, empty: no collections and only its own Globals',
    demoList.currentId === demo?.id &&
      !fresh.shop &&
      fresh.collections === 0 &&
      fresh.none === 'No collections yet.' &&
      freshEnvs.length === 1 &&
      freshEnvs[0].isGlobal,
    short({ fresh, envs: names(freshEnvs) }),
  );
  check('cookies stay with the workspace that received them', !(await cookieHere()));

  const found = await run(`$search(${JSON.stringify('FortyTwo')})`);
  const offered = await run(`$search('workspace')`);
  check(
    'Quick search finds nothing of the other workspace, and offers switching to it',
    !found.includes(SHOP) && offered.includes(`Switch to ${original.name}`),
    short({ found, offered }),
  );
  await run(`$pickResult(${JSON.stringify(`Switch to ${original.name}`)})`);
  await settled(original.name);
  check(
    'switching back from Quick search shows the first workspace again, cookies included',
    (await run(`!!$nav(${JSON.stringify(SHOP)})`)) && (await cookieHere()),
  );

  // --- moving a collection to another workspace ---
  await run(`$nav(${JSON.stringify(SHOP)}).click()`);
  await waitFor(`!!$btn('Move to workspace') && !$btn('Move to workspace').disabled`, 3000);
  await run(`$btn('Move to workspace').click()`);
  await waitFor(`!!$dialog('move-collection-title')`, 3000);
  const moveDialog = await run(`(() => {
    const d = $dialog('move-collection-title');
    return { options: [...d.querySelectorAll('label.option .name')].map((n) => n.textContent.trim()), checked: d.querySelector('input:checked')?.closest('label')?.querySelector('.name')?.textContent.trim() };
  })()`);
  check(
    'Move to workspace lists the other workspaces, one already chosen',
    moveDialog.options.includes(DEMO) &&
      !moveDialog.options.includes(original.name) &&
      moveDialog.checked !== undefined,
    short(moveDialog),
  );
  await run(`(() => {
    const d = $dialog('move-collection-title');
    [...d.querySelectorAll('label.option')].find((l) => l.querySelector('.name').textContent.trim() === ${JSON.stringify(DEMO)}).querySelector('input').click();
  })()`);
  await shot('stage42-move.png');
  await run(`$btn('Move collection', $dialog('move-collection-title')).click()`);
  await waitFor(`!$dialog('move-collection-title')`, 4000);
  await sleep(400);
  const moved = await run(`({
    shop: !!$nav(${JSON.stringify(SHOP)}),
    banners: [...document.querySelectorAll('.banner > span')].map((s) => s.textContent.trim()),
    action: [...document.querySelectorAll('.banner button')].map((b) => b.textContent.trim()),
  })`);
  const leftHere = names((await invoke('list_environments', {})).ok.environments);
  check(
    'the moved collection leaves the sidebar, with a note offering to open its workspace',
    !moved.shop &&
      moved.banners.includes(`“${SHOP}” moved to ${DEMO}.`) &&
      moved.action.includes(`Open ${DEMO}`),
    short(moved),
  );
  check(
    'its own environment went with it, and this device’s one stayed',
    !leftHere.includes(OWN_ENV) && leftHere.includes(DEVICE_ENV),
    short(leftHere),
  );
  await run(
    `[...document.querySelectorAll('.banner button')].find((b) => b.textContent.trim() === ${JSON.stringify(`Open ${DEMO}`)}).click()`,
  );
  await settled(DEMO);
  const there = await run(`({ shop: !!$nav(${JSON.stringify(SHOP)}) })`);
  const envsThere = (await invoke('list_environments', {})).ok;
  check(
    'in the new workspace the collection has its own environment and no longer uses the device one',
    there.shop &&
      names(envsThere.environments).includes(OWN_ENV) &&
      !names(envsThere.environments).includes(DEVICE_ENV) &&
      envsThere.activeIds[shopId] === undefined,
    short({ there, envs: names(envsThere.environments), active: envsThere.activeIds[shopId] }),
  );

  // --- Settings › Workspaces ---
  const empty = (
    await invoke('create_workspace', { name: EMPTY, hue: 'slate', icon: { kind: 'letter' } })
  ).ok;
  madeWorkspaces.push(empty.id);
  await run(`$switcher().click()`);
  await waitFor(`!!$menu()`, 2000);
  await run(`$choose('Manage workspaces')`);
  await waitFor(`$rows().length >= 3`, 4000);
  const rows = await run(`$rows()`);
  const demoRow = rows.find((row) => row.name === DEMO);
  const emptyRow = rows.find((row) => row.name === EMPTY);
  check(
    'Settings › Workspaces marks the open one and counts what each holds',
    demoRow?.open === true &&
      demoRow.detail === '1 collection · 1 environment' &&
      emptyRow?.detail === 'Empty',
    short(rows),
  );
  check(
    'only an empty workspace can be deleted',
    demoRow?.deletable === false && emptyRow?.deletable === true,
    short(rows),
  );
  await shot('stage42-settings.png');
  await run(`$btn('Delete ${EMPTY}').click()`);
  await waitFor(`!!document.querySelector('dialog[open][aria-labelledby="confirm-title"]')`, 3000);
  await run(
    `$btn('Delete workspace', document.querySelector('dialog[open][aria-labelledby="confirm-title"]')).click()`,
  );
  await waitFor(`!$rows().some((row) => row.name === ${JSON.stringify(EMPTY)})`, 4000);
  const afterDelete = (await invoke('list_workspaces', {})).ok;
  check(
    'deleting the empty workspace removes it after a confirmation',
    !names(afterDelete.workspaces).includes(EMPTY),
    short(names(afterDelete.workspaces)),
  );

  await run(`$btn('Edit ${DEMO}').click()`);
  await waitFor(`!!$dialog('workspace-dialog-title')`, 3000);
  await run(`$set(document.getElementById('workspace-name'), ${JSON.stringify(RENAMED)})`);
  await run(`$dialog('workspace-dialog-title').querySelector('input[value="rose"]').click()`);
  await run(`$btn('Save', $dialog('workspace-dialog-title')).click()`);
  await waitFor(`!$dialog('workspace-dialog-title')`, 3000);
  await sleep(300);
  const renamed = await run(`({
    switcher: $switcher().querySelector('.name').textContent.trim(),
    hue: $switcher().querySelector('[data-hue]')?.getAttribute('data-hue'),
    row: $rows().some((row) => row.name === ${JSON.stringify(RENAMED)}),
  })`);
  check(
    'Edit renames and recolors the workspace, and the switcher follows',
    renamed.switcher === RENAMED && renamed.hue === 'rose' && renamed.row,
    short(renamed),
  );

  // --- workspace icons ---
  const openEdit = async () => {
    await run(`$btn('Edit ${RENAMED}').click()`);
    await waitFor(`!!$dialog('workspace-dialog-title')`, 3000);
  };
  const chooseIcon = (label) =>
    run(
      `[...$dialog('workspace-dialog-title').querySelectorAll('input[name="workspace-icon"]')].find((i) => i.value === ${JSON.stringify(label)}).click()`,
    );
  const candidates = () =>
    run(
      `[...$dialog('workspace-dialog-title').querySelectorAll('.candidate')].map((b) => ({ website: b.querySelector('.website').textContent.trim(), chosen: b.getAttribute('aria-pressed') === 'true' }))`,
    );
  await openEdit();
  await sleep(900);
  const before = iconLookups.length;
  await chooseIcon('logo');
  await waitFor(
    `$dialog('workspace-dialog-title').querySelectorAll('.candidate').length >= 2`,
    8000,
  );
  const logos = await candidates();
  check(
    'choosing Logo looks the name up, and nothing is sent before',
    before === 0 &&
      iconLookups.includes('e2efortytworenamed.com') &&
      iconLookups.includes('www.e2efortytworenamed.co.id'),
    short({ before, lookups: iconLookups.slice(0, 10) }),
  );
  check(
    'logos found for the name are offered, without repeats, the first one chosen',
    logos.length === 2 &&
      logos[0].website === 'e2efortytworenamed.com' &&
      logos[0].chosen &&
      logos[1].website === 'e2efortytworenamed.co.id',
    short(logos),
  );
  await run(
    `(() => { const input = $dialog('workspace-dialog-title').querySelector('input[aria-label="Website"]'); $set(input, 'https://www.e2e-site.org/about'); input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); })()`,
  );
  await waitFor(
    `$dialog('workspace-dialog-title').querySelector('.candidate')?.textContent.includes('e2e-site.org')`,
    5000,
  );
  const typed = await candidates();
  check(
    'a typed website adds its logo first and chooses it',
    typed[0]?.website === 'e2e-site.org' && typed[0].chosen && typed.length === 3,
    short(typed),
  );
  await shot('stage42-logo.png');
  await run(`$btn('Save', $dialog('workspace-dialog-title')).click()`);
  await waitFor(`!$dialog('workspace-dialog-title')`, 3000);
  const logoShown = await run(`({
    switcher: $switcher().querySelector('img')?.getAttribute('src')?.slice(0, 22),
    row: !!document.querySelector('ul[aria-label="Workspaces"] li.row img'),
  })`);
  const stored = (await invoke('list_workspaces', {})).ok.workspaces.find(
    (w) => w.name === RENAMED,
  );
  check(
    'the logo is saved with its website and shows in the switcher and Settings',
    logoShown.switcher === 'data:image/png;base64,' &&
      logoShown.row &&
      stored?.icon.kind === 'image' &&
      stored.icon.source === 'e2e-site.org',
    short({ logoShown, icon: { ...stored?.icon, dataUrl: undefined } }),
  );

  await openEdit();
  await chooseIcon('image');
  const nodes = await cdp('DOM.getDocument', { depth: -1, pierce: true });
  const fileInput = await cdp('DOM.querySelector', {
    nodeId: nodes.result.root.nodeId,
    selector: 'dialog[open] input[type="file"]',
  });
  await cdp('DOM.setFileInputFiles', { nodeId: fileInput.result.nodeId, files: [imageFile] });
  await waitFor(
    `$dialog('workspace-dialog-title').querySelector('.icon-row img')?.getAttribute('src')?.startsWith('data:image/png')`,
    5000,
  );
  const preview = await run(`(async () => {
    const img = $dialog('workspace-dialog-title').querySelector('.icon-row img');
    const probe = new Image();
    probe.src = img.src;
    await probe.decode();
    return { width: probe.naturalWidth, height: probe.naturalHeight };
  })()`);
  check(
    'an image file is shrunk to 128 pixels before it is saved',
    preview.width === 128 && preview.height === 128,
    short(preview),
  );
  await run(`$btn('Save', $dialog('workspace-dialog-title')).click()`);
  await waitFor(`!$dialog('workspace-dialog-title')`, 3000);
  const fromFile = (await invoke('list_workspaces', {})).ok.workspaces.find(
    (w) => w.name === RENAMED,
  );
  check(
    'the image is saved without a website',
    fromFile?.icon.kind === 'image' && fromFile.icon.source === null,
    short({ ...fromFile?.icon, dataUrl: undefined }),
  );

  await openEdit();
  await chooseIcon('emoji');
  await run(`$btn('Choose emoji', $dialog('workspace-dialog-title')).click()`);
  await waitFor(
    `!!document.querySelector('.picker[aria-label="Workspace icon"] .emoji-button')`,
    3000,
  );
  const picker = await run(`({
    automatic: !!$btn('Use automatic', document.querySelector('.picker')),
    emoji: document.querySelector('.picker .emoji-button .emoji').textContent,
  })`);
  await run(`document.querySelector('.picker .emoji-button').click()`);
  await sleep(300);
  await run(`$btn('Save', $dialog('workspace-dialog-title')).click()`);
  await waitFor(`!$dialog('workspace-dialog-title')`, 3000);
  const emojiShown = await run(`$switcher().querySelector('.tile').textContent.trim()`);
  check(
    'an emoji from the picker becomes the icon, with no automatic choice to go back to',
    !picker.automatic && emojiShown === picker.emoji,
    short({ picker, emojiShown }),
  );

  // Back to the logo, then away and back, to see it on the launch screen.
  await invoke('update_workspace', {
    id: stored.id,
    name: RENAMED,
    hue: 'rose',
    icon: stored.icon,
  });
  await run(`$switcher().click()`);
  await waitFor(`!!$menu()`, 2000);
  await run(`$choose(${JSON.stringify(original.name)})`);
  await settled(original.name);
  const away = await run(`({
    opening: document.documentElement.dataset.opening,
    name: getComputedStyle(document.documentElement).getPropertyValue('--opening-name').trim(),
    text: getComputedStyle(document.documentElement).getPropertyValue('--opening-text').trim(),
  })`);
  await run(
    `$search('workspace').then(() => $pickResult(${JSON.stringify(`Switch to ${RENAMED}`)}))`,
  );
  await sleep(250);
  await shot('stage42-launch.png');
  await settled(RENAMED);
  const back = await run(`({
    opening: document.documentElement.dataset.opening,
    name: getComputedStyle(document.documentElement).getPropertyValue('--opening-name').trim(),
    image: getComputedStyle(document.documentElement).getPropertyValue('--opening-image').trim().slice(0, 27),
  })`);
  check(
    'the launch screen shows the workspace being opened: its letter, or its logo',
    away.opening === 'text' &&
      away.name === JSON.stringify(original.name) &&
      away.text === JSON.stringify(original.name.charAt(0).toUpperCase()) &&
      back.opening === 'image' &&
      back.name === JSON.stringify(RENAMED) &&
      back.image === 'url("data:image/png;base64,',
    short({ away, back }),
  );
} catch (error) {
  console.log(results.join('\n'));
  throw error;
} finally {
  if (shopId) await invoke('delete_collection', { id: shopId });
  for (const id of madeWorkspaces) await invoke('delete_workspace', { id });
  if (original) await invoke('open_workspace', { id: original.id });
  if (deviceId) await invoke('delete_environment', { id: deviceId });
  await invoke('delete_cookie', { key: { domain: '127.0.0.1', path: '/', name: COOKIE } });
  const left = (await invoke('list_deleted', {})).ok ?? [];
  for (const item of left.filter((entry) => entry.name.startsWith('E2E FortyTwo'))) {
    await invoke('forget_deleted', { id: item.id });
  }
  server.close();
  await run('location.reload()').catch(() => null);
  await sleep(1500);
  socket.close();
}
console.log(results.join('\n'));
console.log(`${results.filter((line) => line.startsWith('PASS')).length}/${results.length} passed`);
