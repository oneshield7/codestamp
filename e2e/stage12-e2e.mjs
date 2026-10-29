// End-to-end check of stage 12 (Postmark look, Home, New collection dialog, empty collection,
// emoji collection icons) through the real release build.
const CDP = 'http://127.0.0.1:9333';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let currentName = 'E2E Payments API';

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
async function waitFor(expression, timeout = 5000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(150);
  }
  return false;
}
async function pressKey(key) {
  const codes = {
    Enter: ['Enter', 13],
    ArrowRight: ['ArrowRight', 39],
    ArrowLeft: ['ArrowLeft', 37],
  };
  const [code, vk] = codes[key] ?? [`Digit${key}`, key.charCodeAt(0)];
  const text = key.length === 1 ? key : key === 'Enter' ? '\r' : undefined;
  await cdp('Input.dispatchKeyEvent', {
    type: 'keyDown',
    key,
    code,
    windowsVirtualKeyCode: vk,
    text,
  });
  await cdp('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: vk });
  await sleep(250);
}
async function pressEscape() {
  for (const type of ['keyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: 'Escape',
      code: 'Escape',
      windowsVirtualKeyCode: 27,
    });
  }
  await sleep(300);
}
const helpers = `
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 100)); };
  window.$picked = (trigger) => trigger.querySelector('.text').textContent.trim();
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$dialog = () => document.querySelector('dialog[open]:not(.panel-dialog)');
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$navIcon = (text) => $nav(text)?.querySelector('.icon')?.textContent.trim();
  window.$tile = () => document.querySelector('.icon-tile')?.textContent.trim();
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$hub = (label) => [...document.querySelectorAll('.home [role="tab"]')].find((t) => t.textContent.trim() === label);
  window.$homeTile = (name) => [...document.querySelectorAll('.home .tiles input')].find((i) => i.getAttribute('aria-label') === name);
  window.$stage = () => ({ hub: document.querySelector('.home [role="tab"][aria-selected="true"]')?.textContent.trim(), title: document.querySelector('.home .stage h2')?.textContent.trim(), primary: document.querySelector('.home .primary')?.textContent.trim() });
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 220);

async function rename(name) {
  await run(
    `(() => { const t = document.querySelector('input[aria-label="Collection name"]'); $set(t, ${JSON.stringify(name)}); t.dispatchEvent(new Event('blur')); })()`,
  );
  await sleep(600);
  currentName = name;
}

await connect();
let collectionId;
try {
  // ---------- Postmark look ----------
  const font = await run(`getComputedStyle(document.body).fontFamily`);
  // A font loads when text first uses it, so each is asked for: loading proves it is bundled.
  const loaded = await run(
    `Promise.all([document.fonts.load('14px "Geist Variable"'), document.fonts.load('13px "Geist Mono Variable"')]).then((faces) => faces.every((found) => found.length > 0))`,
  );
  check(
    'Geist and Geist Mono are bundled and used',
    font.startsWith('"Geist Variable"') && loaded,
    font,
  );
  const tokens = await run(
    `(() => { const s = getComputedStyle(document.documentElement); return ['--accent', '--bg', '--focus', '--method-get-bg'].map((t) => s.getPropertyValue(t).trim()); })()`,
  );
  check(
    'Postmark tokens are in place',
    short(tokens) === short(['#111318', '#f3f4f6', '#2f6bff', '#e8f6ef']),
    short(tokens),
  );
  const icons = await run(
    `[...document.querySelectorAll('nav .item .icon')].map((s) => s.textContent.trim())`,
  );
  check(
    'every collection in the sidebar has an emoji',
    icons.every((t) => t.length > 0),
    short(icons),
  );

  // ---------- Home ----------
  await run(`$nav('Home').click()`);
  await sleep(500);
  const home = await run(
    `({ ...$stage(), tabs: [...document.querySelectorAll('.home [role="tab"]')].map((t) => t.textContent.trim()), tiles: [...document.querySelectorAll('.home .tiles input')].map((i) => i.getAttribute('aria-label')), keys: document.querySelectorAll('.home .keys li').length, active: document.querySelector('nav .item.on .name')?.textContent.trim() })`,
  );
  check(
    'Home shows sections, tiles and a stage',
    short(home.tabs) === short(['Start', 'Requests', 'Testing', 'Workspace']) &&
      home.hub === 'Start' &&
      short(home.tiles) ===
        short(['New collection', 'Import', 'Join a shared collection', 'Quick search']) &&
      home.title === 'Create a collection' &&
      home.primary === 'Create collection' &&
      home.keys === 0 &&
      home.active === 'Home',
    short(home),
  );
  await run(`document.activeElement?.blur()`);
  await pressKey('4');
  await pressKey('ArrowRight');
  const viaKeys = await run(`$stage()`);
  check(
    'a number key picks a section and the arrows a tile',
    viaKeys.hub === 'Workspace' &&
      viaKeys.title === 'See what changed in shared collections' &&
      viaKeys.primary === 'Open Git changes',
    short(viaKeys),
  );
  await pressKey('Enter');
  await sleep(400);
  check(
    'Enter runs the main action',
    (await run(`document.querySelector('main h1')?.textContent.trim()`)) === 'Git changes',
  );
  await run(`$nav('Home').click()`);
  await sleep(400);
  await run(`$hub('Testing').click()`);
  await sleep(150);
  await run(`$homeTile('Command line').click()`);
  await sleep(200);
  const cli = await run(
    `({ variants: [...document.querySelectorAll('.home .variant')].map((v) => v.textContent.trim()), command: document.querySelector('.home .command code')?.textContent.trim() })`,
  );
  await run(
    `[...document.querySelectorAll('.home .variant')].find((v) => v.textContent.trim() === 'Data file').querySelector('input').click()`,
  );
  await sleep(200);
  const dataCommand = await run(
    `document.querySelector('.home .command code')?.textContent.trim()`,
  );
  check(
    'the command line tile switches its example commands',
    short(cli.variants) === short(['Folder', 'Collection file', 'Data file']) &&
      cli.command.startsWith('codestamp-cli run ./api --environment') &&
      dataCommand === 'codestamp-cli run ./api --data users.csv --bail',
    short({ cli, dataCommand }),
  );
  await run(`$hub('Workspace').click()`);
  await sleep(150);
  await run(`document.querySelector('.home .primary').click()`);
  await sleep(500);
  check(
    'a Home button opens its page',
    (await run(`document.querySelector('main h1')?.textContent.trim()`)) === 'Docs',
  );
  await run(`$nav('Home').click()`);
  await sleep(400);

  // ---------- New collection dialog ----------
  await run(`$homeTile('Join a shared collection').click()`);
  await sleep(200);
  await run(`document.querySelector('.home .primary').click()`);
  await sleep(500);
  const join = await run(
    `({ checked: $dialog()?.querySelector('input[value="join"]')?.checked, remote: !!document.getElementById('new-collection-remote') })`,
  );
  await run(`$dialog().querySelector('input[value="import"]').click()`);
  await sleep(200);
  const importChoice = await run(`!!$btn('Choose a file', $dialog())`);
  check(
    'the dialog starts on the chosen option and switches',
    join.checked && join.remote && importChoice,
    short({ join, importChoice }),
  );
  await run(`$btn('Cancel', $dialog()).click()`);
  await sleep(400);
  check('Cancel closes the dialog', !(await run(`!!$dialog()`)));

  await run(`$homeTile('New collection').click()`);
  await sleep(200);
  await run(`document.querySelector('.home .primary').click()`);
  await sleep(500);
  check(
    'the name field is focused when the dialog opens',
    await run(`document.activeElement?.id === 'new-collection-name'`),
  );
  await run(`$set(document.getElementById('new-collection-name'), ${JSON.stringify(currentName)})`);
  const followed = await waitFor(
    `$dialog()?.querySelector('.icon-button')?.textContent.trim() === '💳'`,
    3000,
  );
  check(
    'the icon follows the typed name',
    followed,
    await run(`$dialog()?.querySelector('.icon-button')?.textContent.trim()`),
  );

  // Escape inside the dialog's picker closes only the picker.
  await run(`$dialog().querySelector('.icon-button').click()`);
  await sleep(400);
  const pickerOpen = await run(`!!$dialog()?.querySelector('[aria-label="Collection icon"]')`);
  await pressEscape();
  const afterFirst = await run(
    `({ picker: !!document.querySelector('[aria-label="Collection icon"]'), dialog: !!$dialog() })`,
  );
  check(
    'Escape closes the picker but keeps the dialog',
    pickerOpen && !afterFirst.picker && afterFirst.dialog,
    short({ pickerOpen, afterFirst }),
  );

  await run(`$set(document.getElementById('new-collection-base-url'), 'ftp://files.test')`);
  await sleep(200);
  const bad = await run(
    `({ disabled: $btn('Create collection', $dialog()).disabled, hint: document.getElementById('new-collection-base-url-hint').textContent.trim() })`,
  );
  check(
    'a base URL that is not http(s) is refused before creating',
    bad.disabled && /http/.test(bad.hint),
    short(bad),
  );
  await run(
    `$set(document.getElementById('new-collection-base-url'), 'https://api.payments.test')`,
  );
  await sleep(200);
  await run(`$btn('Create collection', $dialog()).click()`);
  await waitFor(
    `!$dialog() && document.querySelector('input[aria-label="Collection name"]')?.value === ${JSON.stringify(currentName)}`,
    5000,
  );
  collectionId = await run(
    `$invoke('list_collections').then((list) => list.find((c) => c.name === ${JSON.stringify(currentName)})?.id ?? null)`,
  );
  const created = await run(
    `({ tile: $tile(), sidebar: $navIcon(${JSON.stringify(currentName)}), meta: document.querySelector('.meta')?.textContent.replace(/\\s+/g, ' ').trim(), start: !!document.querySelector('.start'), adders: !!document.querySelector('.adders'), runDisabled: document.querySelector('header.top button[aria-label^="Run all"]')?.disabled, runTitle: document.querySelector('header.top button[aria-label^="Run all"]')?.getAttribute('aria-label') })`,
  );
  check(
    'the new collection opens with its icon and empty state',
    !!collectionId &&
      created.tile === '💳' &&
      created.sidebar === '💳' &&
      created.start &&
      !created.adders &&
      created.runDisabled &&
      /HTTP or GraphQL request/.test(created.runTitle),
    short(created),
  );
  check(
    'the header says it has no requests and was just created',
    created.meta === 'No requests yet · saved automatically Created just now',
    created.meta,
  );
  const card = await waitFor(
    `[...document.querySelectorAll('.ready .card')].some((c) => c.textContent.includes('https://api.payments.test'))`,
    3000,
  );
  await run(`$btn('Edit variables').click()`);
  await sleep(800);
  const vars = await run(
    `[...document.querySelectorAll('section[aria-label="Collection variables"] tbody tr')].map((r) => [r.querySelector('input[aria-label="Variable name"]')?.value, r.querySelector('input[aria-label="Variable value"]')?.value]).filter(([k]) => k)`,
  );
  check(
    'the base URL is saved as the {{baseUrl}} collection variable',
    card && short(vars) === short([['baseUrl', 'https://api.payments.test']]),
    short(vars),
  );
  await run(
    `$btn('Close', document.querySelector('section[aria-label="Collection variables"]')).click()`,
  );
  await sleep(300);

  // ---------- emoji picker ----------
  await run(`document.querySelector('.icon-tile').click()`);
  await sleep(500);
  const picker = await run(
    `(() => { const grids = [...document.querySelectorAll('.picker .grid')]; return { suggested: [...grids[0].querySelectorAll('.emoji-button')].map((b) => b.textContent.trim()), all: document.querySelectorAll('.picker .scroll .emoji-button').length, mode: document.querySelector('.picker .mode')?.textContent.trim(), searchFocused: document.activeElement === document.querySelector('.picker .search input'), automaticDisabled: $btn('Use automatic', document.querySelector('[aria-label="Collection icon"]')).disabled }; })()`,
  );
  check(
    'the picker suggests icons for the name',
    short(picker.suggested) === short(['💳', '💰', '🧾', '🏦', '💸', '🪙']) &&
      picker.all === 58 &&
      picker.mode.startsWith('Automatic') &&
      picker.searchFocused &&
      picker.automaticDisabled,
    short(picker),
  );
  await run(`$set(document.querySelector('.picker .search input'), 'rocket')`);
  await sleep(200);
  const found = await run(
    `[...document.querySelectorAll('.picker .scroll .emoji-button')].map((b) => b.textContent.trim())`,
  );
  check('search finds emoji by keyword', short(found) === short(['🚀']), short(found));
  await run(`document.querySelector('.picker .scroll .emoji-button').click()`);
  await sleep(700);
  const chosen = await run(
    `({ open: !!document.querySelector('[aria-label="Collection icon"]'), tile: $tile(), sidebar: $navIcon(${JSON.stringify(currentName)}) })`,
  );
  const stored = await run(
    `$invoke('get_collection', { id: '${collectionId}' }).then((c) => ({ icon: c.icon, custom: c.iconCustom }))`,
  );
  check(
    'a chosen emoji is shown everywhere and stored',
    !chosen.open &&
      chosen.tile === '🚀' &&
      chosen.sidebar === '🚀' &&
      stored.icon === '🚀' &&
      stored.custom,
    short({ chosen, stored }),
  );

  await rename('E2E Shop API');
  check(
    'a chosen icon stays when the collection is renamed',
    (await run(`$tile()`)) === '🚀' && (await run(`$navIcon('E2E Shop API')`)) === '🚀',
  );
  await run(`document.querySelector('.icon-tile').click()`);
  await sleep(500);
  const mode = await run(`document.querySelector('.picker .mode')?.textContent.trim()`);
  await run(
    `$btn('Use automatic', document.querySelector('[aria-label="Collection icon"]')).click()`,
  );
  await sleep(600);
  check(
    'Use automatic goes back to the icon from the name',
    mode === 'Chosen by you' && (await run(`$tile()`)) === '🛒',
    `${mode} · ${await run(`$tile()`)}`,
  );
  await pressEscape();
  check(
    'Escape closes the picker',
    !(await run(`!!document.querySelector('[aria-label="Collection icon"]')`)),
  );
  await rename('E2E Internal tools');
  check(
    'an automatic icon follows a new name',
    (await run(`$tile()`)) === '🛠️' && (await run(`$navIcon('E2E Internal tools')`)) === '🛠️',
    await run(`$tile()`),
  );

  // ---------- the Rust core checks icons and base URLs ----------
  const refusedIcon = await run(
    `$invoke('set_collection_icon', { id: '${collectionId}', icon: '<b>x</b>' }).then(() => 'accepted', (e) => e.message)`,
  );
  check(
    'icons outside the catalogue are refused',
    refusedIcon === 'Choose an icon from the list',
    refusedIcon,
  );
  const refusedUrl = await run(
    `$invoke('create_collection', { name: 'E2E refused', icon: null, baseUrl: 'javascript:alert(1)' }).then(() => 'accepted', (e) => e.message)`,
  );
  const leftover = await run(
    `$invoke('list_collections').then((list) => list.some((c) => c.name === 'E2E refused'))`,
  );
  check(
    'a non-http base URL is refused and nothing is created',
    refusedUrl !== 'accepted' && /http/.test(refusedUrl) && !leftover,
    refusedUrl,
  );

  // ---------- empty collection tiles ----------
  await run(
    `[...document.querySelectorAll('.start .tile')].find((b) => b.textContent.includes('Paste a cURL')).click()`,
  );
  await sleep(900);
  await run(`$set(document.activeElement, 'curl https://api.payments.test/health')`);
  await waitFor(`!!document.querySelector('button[aria-label="Collection"]')`, 4000);
  const handoff = await run(
    `({ focused: document.activeElement?.tagName, target: $picked(document.querySelector('button[aria-label="Collection"]')) })`,
  );
  check(
    'Paste a cURL command opens Import with this collection chosen',
    handoff.focused === 'TEXTAREA' && handoff.target === currentName,
    short(handoff),
  );
  await run(`$nav(${JSON.stringify(currentName)}).click()`);
  await sleep(800);
  await run(`document.querySelector('.start .tile').click()`);
  await sleep(700);
  await run(`$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/health')`);
  await sleep(300);
  await run(`document.querySelector('article.card button[aria-label="Collapse request"]').click()`);
  await sleep(900);
  const row = await run(
    `(() => { const r = document.querySelector('.row'); const stamp = r?.querySelector('.stamp'); return { step: r?.querySelector('.step')?.textContent.trim(), stamp: stamp?.textContent.trim(), color: stamp && getComputedStyle(stamp).color, background: stamp && getComputedStyle(stamp).backgroundColor, start: !!document.querySelector('.start'), adders: !!document.querySelector('.adders') }; })()`,
  );
  check(
    'rows show a numbered step and a coloured method stamp',
    row.step === '01' &&
      row.stamp === 'GET' &&
      row.color === 'rgb(10, 122, 75)' &&
      row.background === 'rgb(232, 246, 239)',
    short(row),
  );
  check(
    'the start panel gives way to the adders once a request exists',
    !row.start && row.adders,
    short(row),
  );

  // ---------- icon toolbar, tooltips and pressed states ----------
  const toolButton = (label) =>
    `document.querySelector('header.top button[aria-label="${label}"]')`;
  const toolbar = await run(
    `[...document.querySelectorAll('header.top [role="group"] button')].map((b) => ({ label: b.getAttribute('aria-label'), text: b.textContent.trim() }))`,
  );
  check(
    'the collection menu is icon buttons with labels',
    toolbar.length === 9 && toolbar.every((b) => b.text === '' && b.label),
    short(toolbar.map((b) => b.label)),
  );
  const box = await run(
    `(() => { const r = ${toolButton('Variables')}.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`,
  );
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: box.x, y: box.y });
  await sleep(200);
  const tip = await run(
    `(() => { const t = ${toolButton('Variables')}.nextElementSibling; return { text: t.textContent.trim(), visible: getComputedStyle(t).visibility }; })()`,
  );
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 5, y: 5 });
  await sleep(100);
  const hidden = await run(
    `getComputedStyle(${toolButton('Variables')}.nextElementSibling).visibility`,
  );
  check(
    'hovering shows a tooltip, which goes away after',
    tip.text === 'Variables' && tip.visible === 'visible' && hidden === 'hidden',
    short({ tip, hidden }),
  );
  await run(`${toolButton('Variables')}.click()`);
  await sleep(500);
  const pressed = await run(
    `(() => { const b = ${toolButton('Variables')}; const s = getComputedStyle(b); return { expanded: b.getAttribute('aria-expanded'), border: s.borderColor, background: s.backgroundColor }; })()`,
  );
  check(
    "an open panel's button looks pressed",
    pressed.expanded === 'true' &&
      pressed.border === 'rgb(17, 19, 24)' &&
      pressed.background === 'rgb(230, 232, 236)',
    short(pressed),
  );
  await run(`${toolButton('Variables')}.click()`);
  await sleep(300);
  await run(`$nav('Home').click()`);
  await sleep(500);
  const look = await run(
    `(() => { const primary = getComputedStyle(document.querySelector('.home .primary')); return { stage: getComputedStyle(document.querySelector('.home .stage')).backgroundColor, button: primary.backgroundColor, text: primary.color }; })()`,
  );
  check(
    'the Home stage is light with a dark main button',
    look.stage === 'rgb(255, 255, 255)' &&
      look.button === 'rgb(17, 19, 24)' &&
      look.text === 'rgb(255, 255, 255)',
    short(look),
  );
  await run(`document.querySelector('.home .primary').click()`);
  await sleep(500);
  const whileOpen = await run(
    `({ tile: document.querySelector('.home .primary').getAttribute('aria-expanded'), plus: document.querySelector('nav button.new').getAttribute('aria-expanded') })`,
  );
  await run(`$btn('Cancel', $dialog()).click()`);
  await sleep(400);
  const afterClose = await run(
    `({ tile: document.querySelector('.home .primary').getAttribute('aria-expanded'), plus: document.querySelector('nav button.new').getAttribute('aria-expanded') })`,
  );
  check(
    'the tile and the + button stay pressed while the dialog is open',
    whileOpen.tile === 'true' &&
      whileOpen.plus === 'true' &&
      afterClose.tile === 'false' &&
      afterClose.plus === 'false',
    short({ whileOpen, afterClose }),
  );

  // ---------- Send colour, Duplicate dialog, folding sections ----------
  await run(`$nav(${JSON.stringify(currentName)}).click()`);
  await sleep(800);
  await run(`document.querySelector('.row').click()`);
  await sleep(700);
  const sendColours = {};
  for (const method of ['GET', 'POST', 'DELETE']) {
    await run(`$pick(document.querySelector('article.card button[id^="method-"]'), '${method}')`);
    await sleep(150);
    sendColours[method] = await run(
      `getComputedStyle(document.querySelector('article.card .send')).backgroundColor`,
    );
  }
  await run(`$pick(document.querySelector('article.card button[id^="method-"]'), 'GET')`);
  await sleep(300);
  check(
    'Send takes the colour of the method',
    sendColours.GET === 'rgb(10, 122, 75)' &&
      sendColours.POST === 'rgb(164, 75, 7)' &&
      sendColours.DELETE === 'rgb(192, 38, 27)',
    short(sendColours),
  );

  await run(
    `document.querySelector('article.card button[aria-label="Duplicate request"]').click()`,
  );
  await sleep(500);
  const dupInput = `document.getElementById('duplicate-name')`;
  const dupState = () =>
    run(
      `({ value: ${dupInput}.value, hint: document.getElementById('duplicate-name-hint').textContent.trim(), disabled: $btn('Duplicate', $dialog()).disabled, pressed: document.querySelector('article.card button[aria-label="Duplicate request"]').getAttribute('aria-expanded') })`,
    );
  const prefilled = await dupState();
  await run(`$set(${dupInput}, 'new REQUEST')`);
  await sleep(150);
  const taken = await dupState();
  await run(`$set(${dupInput}, '   ')`);
  await sleep(150);
  const empty = await dupState();
  check(
    'Duplicate asks for a name and refuses taken or empty ones',
    prefilled.value === 'New request copy' &&
      prefilled.pressed === 'true' &&
      !prefilled.disabled &&
      /already exists/.test(taken.hint) &&
      taken.disabled &&
      /Enter a name/.test(empty.hint) &&
      empty.disabled,
    short({ prefilled, taken, empty }),
  );
  await run(`$set(${dupInput}, 'Health copy')`);
  await sleep(150);
  await run(`$btn('Duplicate', $dialog()).click()`);
  await waitFor(
    `!$dialog() && document.querySelector('article.card .name input')?.value === 'Health copy'`,
    5000,
  );
  const afterCopy = await run(
    `({ card: document.querySelector('article.card .name input')?.value, rows: [...document.querySelectorAll('.row .name')].map((n) => n.textContent.trim()) })`,
  );
  check(
    'the copy gets the chosen name and opens',
    afterCopy.card === 'Health copy' && afterCopy.rows.includes('New request'),
    short(afterCopy),
  );
  await run(`document.querySelector('article.card button[aria-label="Collapse request"]').click()`);
  await sleep(800);

  await run(
    `[...document.querySelectorAll('.adder')].find((b) => b.textContent.includes('Add section')).click()`,
  );
  await sleep(700);
  await run(
    `[...document.querySelectorAll('.row')].at(-1).dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', altKey: true, bubbles: true }))`,
  );
  await sleep(900);
  const rowsOpen = await run(`document.querySelectorAll('.row').length`);
  await run(`document.querySelector('.section-head .fold').click()`);
  await sleep(300);
  const folded = await run(
    `({ rows: document.querySelectorAll('.row').length, expanded: document.querySelector('.section-head .fold').getAttribute('aria-expanded'), count: document.querySelector('.section-head .count').textContent.trim() })`,
  );
  check(
    'folding a section hides its requests and keeps its count',
    rowsOpen === 2 && folded.rows === 1 && folded.expanded === 'false' && folded.count === '1',
    short({ rowsOpen, folded }),
  );
  await run('location.reload()');
  await sleep(1500);
  await waitFor(`!!document.querySelector('nav button.new')`, 15000);
  await run(helpers);
  await run(`$nav(${JSON.stringify(currentName)}).click()`);
  await sleep(900);
  const remembered = await run(
    `document.querySelector('.section-head .fold')?.getAttribute('aria-expanded')`,
  );
  await run(`document.querySelector('.section-head .fold').click()`);
  await sleep(300);
  check(
    'a fold is remembered after a reload and opens again',
    remembered === 'false' && (await run(`document.querySelectorAll('.row').length`)) === 2,
    String(remembered),
  );

  // ---------- Adding a request from Home ----------
  await run(`$nav('Home').click()`);
  await sleep(500);
  await run(`$hub('Requests').click()`);
  await sleep(150);
  await run(`$homeTile('GraphQL').click()`);
  await sleep(200);
  const addLabel = await run(`document.querySelector('.home .primary').textContent.trim()`);
  await run(`document.querySelector('.home .primary').click()`);
  await waitFor(
    `document.querySelector('article.card')?.getAttribute('aria-label') === 'New GraphQL request'`,
    5000,
  );
  const added = await run(
    `({ collection: document.querySelector('input[aria-label="Collection name"]')?.value, card: document.querySelector('article.card')?.getAttribute('aria-label'), rows: document.querySelectorAll('.row').length })`,
  );
  check(
    'Home adds a request of the chosen kind to the open collection',
    addLabel === `Add GraphQL to ${currentName}` &&
      added.collection === currentName &&
      added.card === 'New GraphQL request',
    short({ addLabel, added }),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    if (await run(`!!$dialog()`)) {
      await pressEscape();
    }
    for (const name of [
      currentName,
      'E2E Payments API',
      'E2E Shop API',
      'E2E Internal tools',
      'E2E refused',
    ]) {
      if (!(await run(`!!$nav(${JSON.stringify(name)})`))) continue;
      await run(`$nav(${JSON.stringify(name)}).click()`);
      await sleep(800);
      if (
        (await run(`document.querySelector('input[aria-label="Collection name"]')?.value`)) !== name
      )
        continue;
      await run(`$btn('Delete collection').click()`);
      await sleep(300);
      await run(
        `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Delete').click()`,
      );
      await sleep(900);
    }
    check(
      'test data cleaned up',
      !(await run(
        `[...document.querySelectorAll('nav .item .name')].some((n) => n.textContent.startsWith('E2E'))`,
      )),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  process.exit(0);
}
