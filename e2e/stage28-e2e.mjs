// End-to-end check: a request opened from quick search (or Docs) is scrolled into view and
// focused, even far down a long collection or inside a folded section. Opening a row that is
// already on screen must not move the page. The title bar stays on top however far the page
// scrolls, and Docs buttons keep the space before the request name. Creates "E2E Jump" and
// deletes it afterwards.
const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Jump';
const SECTION = 'Folded part';
const TITLEBAR = 36;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

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
    await sleep(150);
  }
  return false;
}
async function key(name, code, keyCode) {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', { type, key: name, code, windowsVirtualKeyCode: keyCode });
  }
  await sleep(150);
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$sideItem = (text) => [...document.querySelectorAll('nav.sidebar .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$activeTitle = () => document.querySelector('dialog.quick-search [role="option"][aria-selected="true"] .title')?.textContent.trim() ?? null;
  window.$option = (title) => [...document.querySelectorAll('dialog.quick-search [role="option"]')].find((o) => o.querySelector('.title')?.textContent.trim() === title);
  window.$card = () => document.querySelector('article.card');
  window.$docOpen = (name) => [...document.querySelectorAll('main button.open')].find((b) => b.querySelector('.visually-hidden')?.textContent.trim() === name);
  window.$sc = () => document.querySelector('.scroller');
  window.$cardState = () => { const c = $card(); if (!c) return null; const r = c.getBoundingClientRect(); return { name: c.querySelector('.name input')?.value ?? null, top: Math.round(r.top), viewport: innerHeight, scrollY: Math.round($sc().scrollTop), focused: c === document.activeElement || c.contains(document.activeElement) }; };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 400);
const request = (name) => ({
  kind: 'http',
  name,
  method: 'GET',
  url: `{{baseUrl}}/${name.toLowerCase().replace(/\s+/g, '-')}`,
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
// The card header sits below the sticky title bar and well inside the window.
const inView = (state) =>
  state !== null && state.top >= TITLEBAR - 1 && state.top <= state.viewport - 80;

async function settle(name) {
  const opened = await waitFor(`$cardState()?.name === ${JSON.stringify(name)}`);
  // Room for a smooth scroll to finish.
  await sleep(900);
  return opened ? await run('$cardState()') : null;
}
async function search(text) {
  await run(`document.querySelector('nav.sidebar button.search').click()`);
  await waitFor(`!!document.querySelector('dialog.quick-search[open]')`);
  await cdp('Input.insertText', { text });
  await sleep(400);
}
async function pickWithEnter(name) {
  await search(name);
  const active = await run('$activeTitle()');
  await key('Enter', 'Enter', 13);
  return { active, state: await settle(name) };
}
async function pickWithMouse(name) {
  await search(name);
  await run(
    `$option(${JSON.stringify(name)}).dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }))`,
  );
  return await settle(name);
}

await connect();
await run('location.reload()');
await sleep(1800);
await run(helpers);
for (const stale of await run(`$invoke('list_collections')`)) {
  if (stale.name === NAME) await run(`$invoke('delete_collection', { id: '${stale.id}' })`);
}
let collectionId = null;
try {
  collectionId = (
    await run(
      `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: 'https://api.example.test' })`,
    )
  ).id;
  for (let i = 1; i <= 40; i++) {
    const created = await run(
      `$invoke('create_request', { collectionId: '${collectionId}', sectionId: null, content: ${JSON.stringify(request(`Jump ${String(i).padStart(2, '0')}`))} })`,
    );
    if (i === 36)
      await run(
        `$invoke('save_description', { item: 'request', id: '${created.id}', text: 'Jumps far.' })`,
      );
  }
  const section = await run(
    `$invoke('create_section', { collectionId: '${collectionId}', name: ${JSON.stringify(SECTION)} })`,
  );
  for (let i = 1; i <= 30; i++) {
    await run(
      `$invoke('create_request', { collectionId: '${collectionId}', sectionId: '${section.id}', content: ${JSON.stringify(request(`Folded ${i}`))} })`,
    );
  }
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$sideItem(${JSON.stringify(NAME)}).click()`);
  await waitFor(`document.querySelectorAll('main [data-request-id]').length >= 40`);
  await run(`$btn('Collapse section ${SECTION}')?.click()`);
  await sleep(300);
  const pageHeight = await run('({ page: $sc().scrollHeight, viewport: $sc().clientHeight })');
  check(
    'the fixture is longer than the window',
    pageHeight.page > pageHeight.viewport * 2,
    short(pageHeight),
  );

  // ---------- the title bar stays while the page scrolls ----------
  const bar = [];
  for (const y of [500, 1500, 99999]) {
    await run(`$sc().scrollTo(0, ${y})`);
    await sleep(200);
    bar.push(
      await run(
        `({ scrollY: Math.round($sc().scrollTop), top: Math.round(document.querySelector('header.titlebar').getBoundingClientRect().top), atCorner: document.elementFromPoint(innerWidth - 20, 18)?.closest('button')?.getAttribute('aria-label') ?? null })`,
      ),
    );
  }
  check(
    'the title bar and its Close button stay at the top while scrolling',
    bar.every((b) => b.top === 0 && b.atCorner === 'Close') && bar.at(-1).scrollY > 1500,
    short(bar),
  );
  const scrollbar = await run(
    `({ windowScroll: document.scrollingElement.scrollTop, windowOverflow: document.scrollingElement.scrollHeight - innerHeight, scrollerTop: Math.round($sc().getBoundingClientRect().top), scrollerBottom: Math.round($sc().getBoundingClientRect().bottom), statusBarTop: Math.round(document.querySelector('footer.statusbar').getBoundingClientRect().top), viewport: innerHeight })`,
  );
  check(
    'only the area between the title bar and the status bar scrolls, so its scrollbar starts under the window buttons',
    scrollbar.windowScroll === 0 &&
      scrollbar.windowOverflow <= 0 &&
      scrollbar.scrollerTop === TITLEBAR &&
      scrollbar.scrollerBottom === scrollbar.statusBarTop,
    short(scrollbar),
  );

  // ---------- quick search, Enter, far below ----------
  await run('$sc().scrollTo(0, 0)');
  await sleep(200);
  const below = await pickWithEnter('Jump 38');
  check('Enter opens the highlighted request', below.active === 'Jump 38', short(below.active));
  check('a request far below scrolls into view', inView(below.state), short(below.state));
  check('the opened card takes keyboard focus', below.state?.focused === true, short(below.state));

  // ---------- quick search, mouse, far above ----------
  await run('$sc().scrollTo(0, $sc().scrollHeight)');
  await sleep(200);
  const above = await pickWithMouse('Jump 03');
  check('a click on a result far above scrolls back up to it', inView(above), short(above));

  // ---------- a request inside a folded section ----------
  await run('$sc().scrollTo(0, 0)');
  await sleep(200);
  const folded = await pickWithEnter('Folded 4');
  const expanded = await run(`$btn('Collapse section ${SECTION}') !== undefined`);
  check(
    'a request in a folded section unfolds it and scrolls into view',
    expanded && inView(folded.state),
    short({ expanded, state: folded.state }),
  );

  // ---------- from another page ----------
  await run(`$sideItem('Home').click()`);
  await sleep(600);
  const home = await run(
    `({ visible: $sc().clientHeight, page: $sc().scrollHeight, shell: Math.round(document.querySelector('.shell').getBoundingClientRect().height), content: Math.round(document.querySelector('main .content').scrollHeight) })`,
  );
  // Home fits a tall window exactly; a short one scrolls only as far as Home's own content needs.
  const homeFits =
    home.shell >= home.visible - 1 &&
    (home.visible < 860 ? home.page <= home.content + 80 : home.page === home.visible);
  check('a short page fills the window without extra scroll', homeFits, short(home));
  const fromHome = await pickWithEnter('Jump 30');
  check(
    'from Home, the collection opens on the request',
    inView(fromHome.state),
    short(fromHome.state),
  );

  // ---------- Docs › Open request ----------
  await run(`$sideItem('Docs').click()`);
  // One doc shows at a time: pick it in the index first.
  await waitFor(
    `[...document.querySelectorAll('nav[aria-label="Docs index"] button.entry .name')].some((n) => n.textContent.trim() === 'Jump 36')`,
    10000,
  );
  await run(
    `[...document.querySelectorAll('nav[aria-label="Docs index"] button.entry')].find((b) => b.querySelector('.name')?.textContent.trim() === 'Jump 36').click()`,
  );
  const docsReady = await waitFor(`!!$docOpen('Jump 36')`, 10000);
  const docsPage = docsReady
    ? null
    : await run(
        `({ h1: document.querySelector('main h1')?.textContent.trim(), opens: [...document.querySelectorAll('main button.open')].slice(0, 3).map((b) => b.textContent.trim()), text: document.querySelector('main').innerText.slice(0, 200) })`,
      );
  const labels = await run(
    `(() => { const doc = $docOpen('Jump 36')?.closest('article'); return { open: $docOpen('Jump 36')?.textContent ?? null, edit: [...(doc?.querySelectorAll('button') ?? [])].find((b) => b.textContent.startsWith('Edit description'))?.textContent ?? null }; })()`,
  );
  check(
    'Docs buttons name their request with spaces intact',
    labels.open === 'Open request Jump 36' && /^Edit description of \S/.test(labels.edit ?? ''),
    short(labels),
  );
  await run(`$docOpen('Jump 36')?.click()`);
  const fromDocs = await settle('Jump 36');
  check(
    'Docs › Open request scrolls the card into view',
    docsReady && inView(fromDocs),
    short({ docsReady, fromDocs, docsPage }),
  );

  // ---------- a row already on screen ----------
  if (!(await run(`!!$card()`))) {
    await run(`$sideItem(${JSON.stringify(NAME)}).click()`);
    await waitFor(`document.querySelectorAll('main [data-request-id]').length >= 40`);
  }
  await run(
    `document.querySelector('article.card button[aria-label="Collapse request"]')?.click()`,
  );
  await sleep(400);
  await run('$sc().scrollTo(0, 0)');
  await sleep(300);
  const rowTop = await run(
    `[...document.querySelectorAll('main [data-request-id]')].find((row) => row.textContent.includes('Jump 02'))?.getBoundingClientRect().top ?? null`,
  );
  await run(
    `[...document.querySelectorAll('main [data-request-id]')].find((row) => row.textContent.includes('Jump 02'))?.click()`,
  );
  const inPlace = await settle('Jump 02');
  check(
    'opening a row already on screen leaves the page where it is',
    inPlace?.scrollY === 0 && rowTop !== null,
    short({ rowTop, inPlace }),
  );

  // ---------- focus mode still fills the window ----------
  await run(`document.querySelector('article.card button[aria-label="Focus mode"]')?.click()`);
  await sleep(500);
  const focusMode = await run(
    `({ focused: !!document.querySelector('main.focus'), mainBottom: Math.round(document.querySelector('main').getBoundingClientRect().bottom - $sc().getBoundingClientRect().top + $sc().scrollTop), page: $sc().scrollHeight, visible: $sc().clientHeight })`,
  );
  await key('Escape', 'Escape', 27);
  await sleep(300);
  check(
    'focus mode fills the window below the title bar',
    focusMode.focused &&
      focusMode.mainBottom >= focusMode.visible - 1 &&
      focusMode.mainBottom === focusMode.page,
    short(focusMode),
  );

  // ---------- adding a request at the end of a long section ----------
  await run(
    `document.querySelector('article.card button[aria-label="Collapse request"]')?.click()`,
  );
  await sleep(400);
  const tools = `document.querySelector('[aria-label="Section ${SECTION}: add or change"]')`;
  await run(`(() => { ${tools}.scrollIntoView({ block: 'start' }); $sc().scrollBy(0, -80); })()`);
  await sleep(300);
  const headAt = await run('Math.round($sc().scrollTop)');
  await run(`${tools}.querySelector('button[aria-label="Add request"]').click()`);
  const added = await settle('New request');
  check(
    'a request added from a section head scrolls into view and takes focus',
    inView(added) && added.focused && added.scrollY > headAt + 1000,
    short({ headAt, added }),
  );
} catch (error) {
  check('the suite ran to the end', false, error.message.slice(0, 300));
} finally {
  if (collectionId) await run(`$invoke('delete_collection', { id: '${collectionId}' })`);
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
  const leftovers = await run(`$invoke('list_collections')`);
  check('test data cleaned up', !leftovers.some((collection) => collection.name === NAME));
  socket.close();
}

console.log(results.join('\n'));
process.exit(results.some((line) => line.startsWith('FAIL')) ? 1 : 0);
