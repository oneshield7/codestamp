// End-to-end check of the October 26 notes: Ctrl+F in the response and request body editors,
// Orchestration and Catalog as the new names, a Docs page that stays short with many
// collections and can be searched, and Docs results in quick search. Creates "E2E Find" and
// eight "E2E Docs list" collections, deletes them afterwards.
import http from 'node:http';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Find';
const LIST = 'E2E Docs list';
const TITLEBAR = 36;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const BODY = 'alpha needle beta\nneedle gamma\nNEEDLE delta\n';
const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/plain' });
  res.end(BODY);
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
// modifiers: 2 = Ctrl, 8 = Shift.
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
  await sleep(200);
}
const ctrlF = () => key('f', 'KeyF', 70, 2);
async function type(text) {
  await cdp('Input.insertText', { text });
  await sleep(350);
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$sideItem = (text) => [...document.querySelectorAll('nav.sidebar .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$h1 = () => document.querySelector('main h1')?.textContent.trim();
  window.$card = () => document.querySelector('article.card');
  window.$panel = () => document.querySelector('[data-find] .cm-find');
  window.$count = (root) => root?.querySelector('.cm-find-count')?.textContent.trim() ?? null;
  window.$groups = () => [...document.querySelectorAll('dialog.quick-search .group')].map((g) => ({ label: g.getAttribute('aria-label'), items: [...g.querySelectorAll('[role="option"]')].map((o) => ({ title: o.querySelector('.title').textContent.trim(), detail: o.querySelector('.detail').textContent.trim() })) }));
  window.$docs = () => [...document.querySelectorAll('main article.request h4')].map((h) => h.textContent.trim());
  window.$entries = () => [...document.querySelectorAll('nav[aria-label="Docs index"] button.entry')].map((b) => b.querySelector('.name')?.textContent.trim());
  window.$pickCollection = async (name) => { document.querySelector('nav[aria-label="Docs index"] button[aria-haspopup="listbox"]').click(); await new Promise((r) => setTimeout(r, 250)); const options = [...document.querySelectorAll('[role="listbox"] [role="option"]')]; const option = options.find((o) => o.textContent.trim().endsWith(name)); option?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 250)); return options.length; };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 400);
const request = (name, extra = {}) => ({
  kind: 'http',
  name,
  method: 'GET',
  url: `${base}/text`,
  params: [],
  headers: [],
  bodyMode: 'none',
  body: '',
  form: [],
  bodyFile: '',
  preRequestScript: '',
  testScript: '',
  auth: { type: 'none' },
  mcpOperation: null,
  pathVariables: [],
  graphqlVariables: '',
  grpc: null,
  ...extra,
});

async function searchFor(text) {
  await run(`document.querySelector('nav.sidebar button.search').click()`);
  await waitFor(`!!document.querySelector('dialog.quick-search[open]')`);
  await type(text);
  await sleep(400);
}

await connect();
await run('location.reload()');
await sleep(1800);
await run(helpers);
for (const stale of await run(`$invoke('list_collections')`)) {
  if (stale.name === NAME || stale.name.startsWith(LIST))
    await run(`$invoke('delete_collection', { id: '${stale.id}' })`);
}
const created = [];
try {
  // ---------- fixture ----------
  const find = (
    await run(
      `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: null })`,
    )
  ).id;
  created.push(find);
  const plain = await run(
    `$invoke('create_request', { collectionId: '${find}', sectionId: null, content: ${JSON.stringify(request('Plain text', { method: 'POST', bodyMode: 'text', body: 'raw needle body\\nsecond line' }))} })`,
  );
  await run(
    `$invoke('save_description', { item: 'request', id: '${plain.id}', text: 'Returns the **zebracorn** report as plain text.' })`,
  );
  const reports = await run(
    `$invoke('create_section', { collectionId: '${find}', name: 'Reports' })`,
  );
  for (const name of ['Daily report', 'Weekly report']) {
    await run(
      `$invoke('create_request', { collectionId: '${find}', sectionId: '${reports.id}', content: ${JSON.stringify(request(name))} })`,
    );
  }
  const variableSet = await run(
    `$invoke('get_collection_variable_set', { collectionId: '${find}' })`,
  );
  await run(
    `$invoke('save_environment_variables', { id: '${variableSet}', variables: [{ id: null, key: 'botToken', value: '12345:plain-bot-token', isSecret: false, enabled: true }, { id: null, key: 'host', value: 'https://api.example.test', isSecret: false, enabled: true }] })`,
  );
  for (let i = 1; i <= 8; i++) {
    created.push(
      (
        await run(
          `$invoke('create_collection', { name: ${JSON.stringify(`${LIST} ${String(i).padStart(2, '0')}`)}, icon: null, baseUrl: null })`,
        )
      ).id,
    );
  }
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);

  // ---------- new names ----------
  const tools = await run(
    `[...document.querySelectorAll('nav.sidebar [role="group"][aria-label="Tools"] .item .name')].map((n) => n.textContent.trim())`,
  );
  check(
    'the Tools group reads Orchestration, Catalog, Docs and Git changes',
    JSON.stringify(tools) === JSON.stringify(['Orchestration', 'Catalog', 'Docs', 'Git changes']),
    short(tools),
  );
  await run(`$sideItem('Orchestration').click()`);
  const flowsTitle = await waitFor(`$h1() === 'Orchestration'`, 5000);
  await run(`$sideItem('Catalog').click()`);
  const catalogTitle = await waitFor(`$h1() === 'Catalog'`, 8000);
  check('the pages carry the new names', flowsTitle && catalogTitle, short(await run('$h1()')));
  await searchFor('api catalog');
  const oldName = await run(
    `[...document.querySelectorAll('dialog.quick-search [aria-label="Go to"] .title')].map((t) => t.textContent.trim())`,
  );
  await key('Escape', 'Escape', 27);
  check(
    'quick search still finds Catalog by its old name',
    oldName.includes('Catalog'),
    short(oldName),
  );

  // ---------- Ctrl+F in the response body ----------
  await run(`$sideItem(${JSON.stringify(NAME)}).click()`);
  await waitFor(`document.querySelectorAll('main [data-request-id]').length >= 3`);
  await run(
    `[...document.querySelectorAll('main [data-request-id]')].find((row) => row.textContent.includes('Plain text')).click()`,
  );
  await waitFor(`!!$card()`);
  await run(`$card().querySelector('button.send').click()`);
  const answered = await waitFor(`!!$card().querySelector('[data-find] .cm-content')`, 8000);
  await run(`$card().querySelector('button.send').focus()`);
  await ctrlF();
  const opened = await run(
    `({ panel: !!$panel(), focused: document.activeElement?.classList.contains('cm-find-field') && !!document.activeElement.closest('[data-find]') })`,
  );
  check(
    'Ctrl+F outside an editor opens find in the response body',
    answered && opened.panel && opened.focused,
    short({ answered, opened }),
  );
  await type('needle');
  const first = await run(`$count($panel())`);
  await key('Enter', 'Enter', 13);
  const second = await run(`$count($panel())`);
  await key('Enter', 'Enter', 13, 8);
  const back = await run(`$count($panel())`);
  check(
    'typing finds every match, Enter and Shift+Enter step through them',
    first === '1 of 3' && second === '2 of 3' && back === '1 of 3',
    short({ first, second, back }),
  );
  await run(
    `(() => { const f = $panel().querySelector('.cm-find-field'); f.focus(); f.select(); })()`,
  );
  for (const char of 'gamma') {
    await cdp('Input.insertText', { text: char });
    await sleep(120);
  }
  await sleep(300);
  const oneByOne = await run(
    `({ value: $panel().querySelector('.cm-find-field').value, count: $count($panel()) })`,
  );
  await key('Enter', 'Enter', 13);
  await cdp('Input.insertText', { text: '!' });
  await sleep(300);
  const afterEnter = await run(`$panel().querySelector('.cm-find-field').value`);
  check(
    'typing letter by letter keeps the whole word, and after Enter the next letter adds on',
    oneByOne.value === 'gamma' && oneByOne.count === '1 of 1' && afterEnter === 'gamma!',
    short({ oneByOne, afterEnter }),
  );
  await run(
    `(() => { const f = $panel().querySelector('.cm-find-field'); f.focus(); f.select(); })()`,
  );
  await type('needle');
  const marked = await run(`document.querySelectorAll('[data-find] .cm-searchMatch').length`);
  check('every match is marked in the body', marked === 3, short(marked));
  await run(`$panel().querySelector('button[aria-label="Match case"]').click()`);
  await sleep(250);
  const caseOn = await run(
    `({ count: $count($panel()), pressed: $panel().querySelector('button[aria-label="Match case"]').getAttribute('aria-pressed') })`,
  );
  check(
    'Match case narrows the matches',
    caseOn.count === '1 of 2' && caseOn.pressed === 'true',
    short(caseOn),
  );
  await run(
    `(() => { const f = $panel().querySelector('.cm-find-field'); f.focus(); f.select(); })()`,
  );
  await type('[');
  await run(`$panel().querySelector('button[aria-label="Regular expression"]').click()`);
  await sleep(250);
  const invalid = await run(`$count($panel())`);
  check('a broken regular expression says so', invalid === 'Invalid pattern', short(invalid));
  await run(`$panel().querySelector('.cm-find-field').focus()`);
  await key('Escape', 'Escape', 27);
  const closed = await run(
    `({ panel: !!$panel(), inEditor: !!document.activeElement?.closest('[data-find] .cm-editor'), card: !!$card() })`,
  );
  check(
    'Escape closes find and returns to the body, the card stays open',
    !closed.panel && closed.inEditor && closed.card,
    short(closed),
  );
  await run(`[...document.querySelectorAll('nav.sidebar .item')].at(-1).focus()`);
  await ctrlF();
  const fromOutside = await run(
    `({ outside: !document.activeElement?.closest('article.card'), panel: !!$panel(), focused: document.activeElement?.classList.contains('cm-find-field') && !!document.activeElement.closest('[data-find]') })`,
  );
  check(
    'Ctrl+F with the focus outside the card still opens find above the response body',
    fromOutside.panel && fromOutside.focused,
    short(fromOutside),
  );
  await key('Escape', 'Escape', 27);

  // ---------- Ctrl+F in the request body ----------
  await run(
    `[...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith('Body')).click()`,
  );
  await waitFor(`!!$card().querySelector('.cm-content[aria-label="Request body"]')`);
  await run(`$card().querySelector('.cm-content[aria-label="Request body"]').focus()`);
  await ctrlF();
  await type('needle');
  const bodyFind = await run(
    `(() => { const editor = document.activeElement?.closest('.cm-editor'); return { inBody: !!editor?.querySelector('.cm-content[aria-label="Request body"]'), count: $count(editor), responsePanel: !!$panel() }; })()`,
  );
  check(
    'Ctrl+F in the request body searches the body itself',
    bodyFind.inBody && bodyFind.count === '1 of 1' && !bodyFind.responsePanel,
    short(bodyFind),
  );
  await key('Escape', 'Escape', 27);

  // ---------- Docs: an index and one doc at a time, searchable ----------
  await run(`$sideItem('Docs').click()`);
  await waitFor(
    `$h1() === 'Docs' && !!document.querySelector('nav[aria-label="Docs index"]')`,
    8000,
  );
  const offered = await run(`$pickCollection(${JSON.stringify(NAME)})`);
  await waitFor(
    `document.querySelector('#doc-collection-title')?.textContent.trim() === ${JSON.stringify(NAME)}`,
    8000,
  );
  const layout = await run(
    `(() => { const index = document.querySelector('nav[aria-label="Docs index"]').getBoundingClientRect(); return { entries: $entries(), docs: $docs().length, indexHeight: Math.round(index.height), viewport: innerHeight }; })()`,
  );
  check(
    'the dropdown lists every collection and the index fits the window, one doc at a time',
    offered >= 9 &&
      JSON.stringify(layout.entries) === JSON.stringify(['Overview', 'Plain text', 'Reports']) &&
      layout.docs === 0 &&
      layout.indexHeight <= layout.viewport,
    short({ offered, layout }),
  );
  await run(`document.querySelector('main .collection details summary').click()`);
  await sleep(200);
  const variables = await run(
    `[...document.querySelectorAll('main .collection details tbody tr')].map((tr) => [...tr.querySelectorAll('td')].map((td) => td.textContent.trim()))`,
  );
  check(
    'variables fold into a table and a token-like value is masked',
    JSON.stringify(variables) ===
      JSON.stringify([
        ['{{botToken}}', '••••••••'],
        ['{{host}}', 'https://api.example.test'],
      ]),
    short(variables),
  );
  await run(`document.querySelector('main h1').click()`);
  await ctrlF();
  const findFocused = await run(`document.activeElement?.getAttribute('aria-label') ?? null`);
  check(
    'Ctrl+F on the Docs page goes to its search',
    findFocused === `Search the docs of ${NAME}`,
    short(findFocused),
  );
  await type('zebracorn');
  const narrowed = await run(
    `({ entries: $entries(), result: document.getElementById('docs-find-result')?.textContent.trim() })`,
  );
  check(
    'the docs search keeps the requests that match, description included',
    JSON.stringify(narrowed.entries) === JSON.stringify(['Overview', 'Plain text']) &&
      narrowed.result === '1 of 3 requests',
    short(narrowed),
  );
  await run(
    `(() => { const i = document.activeElement; i.value = 'reports'; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(250);
  const section = await run(`$entries()`);
  check(
    'a matching section keeps all of its requests',
    JSON.stringify(section) ===
      JSON.stringify(['Overview', 'Reports', 'Daily report', 'Weekly report']),
    short(section),
  );
  await run(
    `(() => { const i = document.activeElement; i.value = 'nothing like this'; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(250);
  const none = await run(
    `document.querySelector('main .no-match')?.textContent.replace(/\\s+/g, ' ').trim() ?? null`,
  );
  await run(`$btn('Clear search').click()`);
  await sleep(250);
  const cleared = await run(
    `({ entries: $entries().length, value: document.activeElement?.value })`,
  );
  check(
    'no match says so, and Clear search brings the whole index back',
    none?.startsWith('Nothing in these docs matches') &&
      cleared.entries === 3 &&
      cleared.value === '',
    short({ none, cleared }),
  );

  // ---------- Docs in quick search ----------
  await run(`$sideItem('Home').click()`);
  await sleep(500);
  await searchFor('zebracorn');
  const docGroups = await run('$groups()');
  const docsGroup = docGroups.find((group) => group.label === 'Docs');
  check(
    'quick search finds words written in Docs',
    docsGroup?.items[0]?.title === 'Plain text' && docsGroup.items[0].detail.includes('zebracorn'),
    short(docGroups),
  );
  await key('Enter', 'Enter', 13);
  const landed = await waitFor(
    `$h1() === 'Docs' && !!document.querySelector('main article.request')`,
    8000,
  );
  await sleep(900);
  const target = await run(
    `(() => { const doc = [...document.querySelectorAll('main article.request')].find((a) => a.querySelector('h4')?.textContent.trim() === 'Plain text'); const r = doc?.getBoundingClientRect(); const on = document.querySelector('nav[aria-label="Docs index"] button.entry[aria-current="page"] .name')?.textContent.trim(); return r ? { top: Math.round(r.top), viewport: innerHeight, focused: document.activeElement === doc, indexed: on } : null; })()`,
  );
  check(
    'picking it opens Docs on that request, selected in the index and in view',
    landed &&
      target !== null &&
      target.top >= TITLEBAR &&
      target.top < target.viewport - 60 &&
      target.focused &&
      target.indexed === 'Plain text',
    short(target),
  );
  await searchFor('report');
  const onDocs = await run(`$groups().map((g) => g.label)`);
  await key('Escape', 'Escape', 27);
  await run(`$sideItem(${JSON.stringify(NAME)}).click()`);
  await sleep(600);
  await searchFor('report');
  const offDocs = await run(`$groups().map((g) => g.label)`);
  await key('Escape', 'Escape', 27);
  check(
    'Docs results lead while Docs is open, requests lead elsewhere',
    onDocs[0] === 'Docs' && offDocs[0] === 'Requests' && offDocs.includes('Docs'),
    short({ onDocs, offDocs }),
  );
} catch (error) {
  check('the suite ran to the end', false, error.message.slice(0, 300));
} finally {
  for (const id of created) await run(`$invoke('delete_collection', { id: '${id}' })`);
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
  const leftovers = await run(`$invoke('list_collections')`);
  check(
    'test data cleaned up',
    !leftovers.some((collection) => collection.name === NAME || collection.name.startsWith(LIST)),
  );
  server.close();
  socket.close();
}

console.log(results.join('\n'));
process.exit(results.some((line) => line.startsWith('FAIL')) ? 1 : 0);
