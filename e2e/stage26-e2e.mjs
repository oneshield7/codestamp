// End-to-end check of the Tools sidebar and Catalog: the sidebar holds Collections and Tools, the
// other pages stay reachable from the status bar and quick search, and the catalog grades a
// collection from its History, a Run all and its quality rules. Creates "E2E Catalog" plus a flow, removes both.
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Catalog';
const FLOW = 'E2E Catalog flow';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const SHOTS = process.argv.includes('--shots');

const server = http.createServer((req, res) => {
  const json = (status, value) => {
    res.writeHead(status, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(value));
  };
  if (req.url === '/users') return json(200, [{ id: 1 }]);
  if (req.url === '/report') return json(500, { error: 'broken' });
  json(404, {});
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
    await sleep(150);
  }
  return false;
}
async function key(name, code, keyCode) {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', { type, key: name, code, windowsVirtualKeyCode: keyCode });
  }
  await sleep(200);
}
async function shot(name) {
  if (!SHOTS) return;
  const reply = await cdp('Page.captureScreenshot', { format: 'png' });
  fs.mkdirSync(path.join(here, '.work', 'screens'), { recursive: true });
  fs.writeFileSync(
    path.join(here, '.work', 'screens', name),
    Buffer.from(reply.result.data, 'base64'),
  );
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$side = () => [...document.querySelectorAll('nav.sidebar .item .name')].map((n) => n.textContent.trim());
  window.$sideItem = (text) => [...document.querySelectorAll('nav.sidebar .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$h1 = () => document.querySelector('main h1')?.textContent.trim();
  window.$row = (name) => [...document.querySelectorAll('main table tbody tr')].find((tr) => tr.querySelector('button.open')?.textContent.trim() === name);
  window.$cells = (name) => { const tr = $row(name); return tr ? [...tr.querySelectorAll('td')].map((td) => td.textContent.replace(/\\s+/g, ' ').trim()) : null; };
  window.$tab = (label) => [...document.querySelectorAll('main [role="tab"]')].find((t) => t.textContent.trim().startsWith(label));
  window.$tiles = () => Object.fromEntries([...document.querySelectorAll('main .tile')].map((t) => [t.querySelector('.label')?.textContent.trim(), { value: t.querySelector('.value')?.textContent.trim(), note: t.querySelector('.note')?.textContent.trim() ?? null }]));
  window.$rules = () => [...document.querySelectorAll('main .rule')].map((r) => ({ title: r.querySelector('h3')?.childNodes[0]?.textContent.trim(), count: r.querySelector('.count')?.textContent.trim(), findings: [...r.querySelectorAll('.findings li')].map((li) => li.textContent.replace(/\\s+/g, ' ').trim()) }));
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 150)); };
  window.$cellsOf = (tr) => [...tr.querySelectorAll('td')].map((td) => td.textContent.replace(/\\s+/g, ' ').trim()).join(' | ');
  window.$goTo = async (text) => { document.querySelector('nav.sidebar button.search').click(); await new Promise((r) => setTimeout(r, 120)); const option = [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text); option?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); return !!option; };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 500);
const request = (name, method, url, extra = {}) => ({
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
  auth: { type: 'none' },
  mcpOperation: null,
  pathVariables: [],
  graphqlVariables: '',
  grpc: null,
  ...extra,
});
const send = (collectionId, requestId, url) =>
  run(
    `$invoke('send_http_request', { request: { method: 'GET', url: ${JSON.stringify(url)}, params: [], headers: [], body: null, timeoutMs: null, collectionId: '${collectionId}', requestId: '${requestId}', recordHistory: true, pathVariables: [] } })`,
  );
async function openCatalog(title = 'Catalog') {
  await run(`$sideItem('Catalog').click()`);
  await waitFor(
    `$h1() === ${JSON.stringify(title)} && !!document.querySelector('main table, main .empty, main .panel')`,
    6000,
  );
  await sleep(200);
}
async function openApi() {
  await run(`$row(${JSON.stringify(NAME)}).querySelector('button.open').click()`);
  await waitFor(`$h1() === ${JSON.stringify(NAME)}`, 6000);
  await sleep(200);
}

await connect();
await run('location.reload()');
await sleep(1800);
await run(helpers);
for (const stale of await run(`$invoke('list_collections')`)) {
  if (stale.name === NAME) await run(`$invoke('delete_collection', { id: '${stale.id}' })`);
}
for (const stale of await run(`$invoke('list_flows')`)) {
  if (stale.name === FLOW) await run(`$invoke('delete_flow', { id: '${stale.id}' })`);
}
let collectionId = null;
let flowId = null;
try {
  // ---------- the sidebar's Tools group ----------
  const sidebar = await run(
    `({ groups: [...document.querySelectorAll('nav.sidebar h2')].map((h) => h.textContent.trim()), items: $side() })`,
  );
  const section = (title) =>
    run(
      `[...document.querySelectorAll('nav.sidebar [role="group"][aria-label="${title}"] .item .name')].map((n) => n.textContent.trim())`,
    );
  const tools = await section('Tools');
  const settings = await run(
    `[...document.querySelectorAll('footer.statusbar button')].map((b) => b.getAttribute('aria-label'))`,
  );
  check(
    'the sidebar has Tools; the other pages sit in the status bar',
    JSON.stringify(sidebar.groups) === JSON.stringify(['Collections', 'Tools']) &&
      JSON.stringify(tools) ===
        JSON.stringify(['Orchestration', 'Catalog', 'Docs', 'Git changes']) &&
      JSON.stringify(settings) ===
        JSON.stringify(['Settings', 'Recently deleted', 'Cookies', 'Environments']) &&
      !sidebar.items.includes('Settings'),
    short({ sidebar, tools, settings }),
  );

  await run(`document.querySelector('nav.sidebar button.search').click()`);
  await sleep(300);
  const goTo = await run(
    `[...document.querySelectorAll('[role="group"][aria-label="Go to"] .result .title')].map((t) => t.textContent.trim())`,
  );
  await key('Escape', 'Escape', 27);
  await sleep(300);
  check(
    'quick search still lists every page, Catalog included',
    [
      'Home',
      'Orchestration',
      'Catalog',
      'Docs',
      'Environments',
      'Cookies',
      'Import',
      'Recently deleted',
      'Settings',
    ].every((name) => goTo.includes(name)) && !goTo.includes('History'),
    short(goTo),
  );
  const reached = [];
  for (const page of ['Environments', 'Recently deleted', 'Settings']) {
    await run(`$goTo(${JSON.stringify(page)})`);
    await sleep(600);
    reached.push(await run(`$h1()`));
  }
  check(
    'Environments, Recently deleted and Settings still open from quick search',
    JSON.stringify(reached) === JSON.stringify(['Environments', 'Recently deleted', 'Settings']),
    short(reached),
  );

  await run(`$sideItem('Home').click()`);
  await sleep(500);
  await run(
    `[...document.querySelectorAll('.home [role="tab"]')].find((t) => t.textContent.trim() === 'Testing').click()`,
  );
  await sleep(300);
  const tiles = await run(
    `[...document.querySelectorAll('.home .tiles input')].map((i) => i.getAttribute('aria-label'))`,
  );
  await run(
    `[...document.querySelectorAll('.home .tiles input')].find((i) => i.getAttribute('aria-label') === 'Catalog').click()`,
  );
  await sleep(300);
  const stage = await run(
    `({ title: document.querySelector('.home .stage h2, .home .stage h1, .home .title')?.textContent.trim(), primary: document.querySelector('.home .primary')?.textContent.trim() })`,
  );
  await run(`document.querySelector('.home .primary').click()`);
  await sleep(700);
  check(
    'Home › Testing has an Catalog tile that opens the catalog',
    tiles.includes('Catalog') &&
      stage.primary === 'Open Catalog' &&
      (await run(`$h1()`)) === 'Catalog',
    short({ tiles, stage }),
  );

  // ---------- fixture: one API with history, rules to break and a flow ----------
  collectionId = (
    await run(
      `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
    )
  ).id;
  const created = {};
  for (const content of [
    request('List users', 'GET', '{{baseUrl}}/users', {
      testScript: "pm.test('ok', () => pm.response.to.have.status(200));",
    }),
    request('Flaky report', 'GET', '{{baseUrl}}/report', {
      headers: [{ enabled: true, key: 'Authorization', value: 'Bearer plain-secret' }],
    }),
    request('list users', 'GET', '{{baseUrl}}/users'),
    {
      ...request('Live', 'GET', 'ws://feed.example.test/live'),
      kind: 'websocket',
      bodyMode: 'text',
    },
  ]) {
    created[content.name] = await run(
      `$invoke('create_request', { collectionId: '${collectionId}', sectionId: null, content: ${JSON.stringify(content)} })`,
    );
  }
  for (let i = 0; i < 3; i++) await send(collectionId, created['List users'].id, `${base}/users`);
  for (let i = 0; i < 2; i++)
    await send(collectionId, created['Flaky report'].id, `${base}/report`);
  await send(collectionId, created['list users'].id, `${base}/missing`);
  flowId = (await run(`$invoke('create_flow', { name: ${JSON.stringify(FLOW)} })`)).id;
  await run(
    `$invoke('save_flow', { flow: { id: '${flowId}', name: ${JSON.stringify(FLOW)}, steps: [{ id: crypto.randomUUID(), requestId: '${created['List users'].id}', extracts: [], checks: [], onFailure: 'stop' }] } })`,
  );

  // ---------- the list ----------
  await run(`$btn('Refresh').click()`);
  await waitFor(`!!$row(${JSON.stringify(NAME)})`, 6000);
  const listed = await run(
    `({ cells: $cells(${JSON.stringify(NAME)}), tiles: $tiles(), headers: [...document.querySelectorAll('main thead th')].map((th) => th.textContent.trim()) })`,
  );
  check(
    'the catalog lists the collection as an API with its error rate and no run yet',
    listed.cells !== null &&
      listed.cells[0].includes('4 requests · 1 with tests') &&
      listed.cells[2] === 'Not run yet' &&
      listed.cells[3] === '33%',
    short(listed.cells),
  );
  check(
    'the totals sit above the list',
    ['APIs', 'Sent in the last 7 days', 'Errors in the last 7 days', 'Need attention'].every(
      (label) => label in listed.tiles,
    ),
    short(listed.tiles),
  );
  await shot('catalog-list.png');

  await run(`$set(document.querySelector('main input[type="search"]'), 'e2e catal')`);
  await sleep(250);
  const searched = await run(
    `[...document.querySelectorAll('main tbody button.open')].map((b) => b.textContent.trim())`,
  );
  await run(`$set(document.querySelector('main input[type="search"]'), 'no api has this name')`);
  await sleep(250);
  const none = await run(
    `({ text: document.querySelector('main .none p')?.textContent.trim(), clear: !!$btn('Clear filters') })`,
  );
  await run(`$btn('Clear filters').click()`);
  await sleep(250);
  check(
    'search narrows the list and an empty result offers to clear the filters',
    JSON.stringify(searched) === JSON.stringify([NAME]) &&
      none.text === 'No API matches these filters.' &&
      none.clear &&
      !!(await run(`$row(${JSON.stringify(NAME)})`)),
    short({ searched, none }),
  );

  await run(
    `[...document.querySelectorAll('main thead button.sort')].find((b) => b.textContent.trim().startsWith('Error rate')).click()`,
  );
  await sleep(200);
  const sorted = await run(
    `({ sort: [...document.querySelectorAll('main thead th')].find((th) => th.textContent.trim().startsWith('Error rate'))?.getAttribute('aria-sort'), first: document.querySelector('main tbody button.open')?.textContent.trim() })`,
  );
  check(
    'sorting by error rate puts the highest first and says so',
    sorted.sort === 'descending' && sorted.first === NAME,
    short(sorted),
  );

  // ---------- one API: owner and tags ----------
  await openApi();
  const head = await run(
    `({ owner: document.querySelector('main .meta')?.textContent.replace(/\\s+/g, ' ').trim(), tabs: [...document.querySelectorAll('main [role="tab"]')].map((t) => t.textContent.trim()) })`,
  );
  check(
    'the API page has the health chip, no owner yet and four tabs',
    head.owner.includes('No owner yet') &&
      JSON.stringify(head.tabs.map((t) => t.split(' ·')[0])) ===
        JSON.stringify(['Overview', 'Runtime', 'Tests', 'Quality']),
    short(head),
  );
  await run(`$btn('Edit owner and tags').click()`);
  await sleep(200);
  const form = `document.querySelector('main form[aria-label="Owner and tags"]')`;
  await run(`$set(${form}.querySelectorAll('input')[0], 'QA team')`);
  await run(`$set(${form}.querySelectorAll('input')[1], 'e2e, <script>')`);
  await run(`${form}.querySelector('button[type="submit"]').click()`);
  await sleep(400);
  const refused = await run(`${form}?.querySelector('.error')?.textContent.trim() ?? null`);
  check(
    'a tag with characters that are not allowed is refused with the reason',
    refused === 'Tags can have letters, digits, spaces and - _ . /',
    refused,
  );
  await run(`$set(${form}.querySelectorAll('input')[1], 'e2e, public')`);
  await run(`${form}.querySelector('button[type="submit"]').click()`);
  await waitFor(`!${form}`, 3000);
  const meta = await run(
    `document.querySelector('main .meta')?.textContent.replace(/\\s+/g, ' ').trim()`,
  );
  check(
    'owner and tags are saved and shown',
    meta.includes('Owner: QA team') && meta.includes('e2e') && meta.includes('public'),
    meta,
  );
  await shot('catalog-overview.png');

  // ---------- overview ----------
  const overview = await run(
    `({ score: document.querySelector('main .score')?.textContent.trim(), parts: [...document.querySelectorAll('main .parts li')].map((li) => li.textContent.replace(/\\s+/g, ' ').trim()), flows: [...document.querySelectorAll('main .flows li')].map((li) => li.textContent.replace(/\\s+/g, ' ').trim()), assets: document.querySelector('main .assets')?.textContent.replace(/\\s+/g, ' ').trim() })`,
  );
  check(
    'the overview splits the score into Runtime, Tests and Quality',
    /^\d+$/.test(overview.score) &&
      overview.parts.length === 3 &&
      overview.parts[0].startsWith('Runtime 40%') &&
      overview.parts[1].startsWith('Tests 35%') &&
      overview.parts[2].startsWith('Quality 25%'),
    short(overview),
  );
  check(
    'it lists what the API holds and the flows that use it',
    overview.assets.includes('HTTP request 3') &&
      overview.assets.includes('WebSocket 1') &&
      overview.flows.length === 1 &&
      overview.flows[0].startsWith(`${FLOW} 1 of 1 step from this API`),
    short(overview),
  );

  // ---------- runtime ----------
  await run(`$tab('Runtime').click()`);
  await sleep(300);
  const runtime = await run(
    `({ tiles: $tiles(), today: (() => { const slot = [...document.querySelectorAll('main .slot')].at(-1); return { errors: !!slot?.querySelector('.segment.errors'), answered: !!slot?.querySelector('.segment.answered'), day: slot?.querySelector('.day')?.textContent.trim() }; })(), legend: [...document.querySelectorAll('main .legend span')].map((s) => s.textContent.trim()), endpoints: [...document.querySelectorAll('main section[aria-labelledby="endpoints-title"] tbody tr')].map($cellsOf) })`,
  );
  check(
    'Runtime counts 6 sends, 2 errors (33%) and 1 client error from History',
    runtime.tiles['Sent in the last 7 days']?.value === '6' &&
      runtime.tiles['Error rate']?.value === '33%' &&
      runtime.tiles['Client errors (4xx)']?.value === '1',
    short(runtime.tiles),
  );
  check(
    'the daily chart stacks errors on answered sends with a legend',
    runtime.today.day === 'Today' &&
      runtime.today.errors &&
      runtime.today.answered &&
      JSON.stringify(runtime.legend) === JSON.stringify(['Answered', 'Errors']),
    short(runtime),
  );
  check(
    'requests are ranked by error rate',
    runtime.endpoints[0]?.startsWith('GET Flaky report | 2 | 2 | 100%') &&
      runtime.endpoints.length === 3,
    short(runtime.endpoints),
  );
  await shot('catalog-runtime.png');
  await run(`$btn('Show as table').click()`);
  await sleep(200);
  const table = await run(
    `({ rows: document.querySelectorAll('main .chart tbody tr').length, pressed: $btn('Show as chart')?.getAttribute('aria-pressed') })`,
  );
  check(
    'the chart can be read as a table instead',
    table.rows === 7 && table.pressed === 'true',
    short(table),
  );

  // ---------- quality ----------
  await run(`$tab('Quality').click()`);
  await sleep(300);
  const rules = await run(`$rules()`);
  const byTitle = Object.fromEntries(rules.map((rule) => [rule.title, rule]));
  check(
    'a token typed into a header breaks the secrets rule',
    byTitle['Secrets are kept in variables']?.count === '1 of 5 fail' &&
      byTitle['Secrets are kept in variables'].findings[0]?.includes(
        'Flaky report Written out: Authorization header',
      ),
    short(byTitle['Secrets are kept in variables']),
  );
  check(
    'a plain ws:// address and requests without tests are found',
    byTitle['Requests use encrypted connections']?.findings[0]?.includes(
      'Sent over ws:// to feed.example.test',
    ) &&
      byTitle['Requests have tests']?.count === '2 of 3 fail' &&
      byTitle['Request names are unique']?.findings.length === 2,
    short(rules),
  );
  await shot('catalog-quality.png');
  await run(
    `[...document.querySelectorAll('main .rule .findings button.target')].find((b) => b.textContent.trim() === 'Flaky report').click()`,
  );
  const opened = await waitFor(
    `document.querySelector('article.card input[aria-label="Request name"], article.card .name input')?.value === 'Flaky report' || [...document.querySelectorAll('article.card')].some((c) => c.textContent.includes('Flaky report'))`,
    5000,
  );
  check('a finding opens its request', opened);

  // ---------- a Run all shows up under Tests ----------
  // Coming back to the catalog reopens the API that was open.
  const reopened = await (async () => {
    await openCatalog(NAME);
    return run(`$h1()`);
  })();
  check('the catalog comes back to the API that was open', reopened === NAME, reopened);
  await run(`$tab('Tests').click()`);
  await sleep(250);
  const before = await run(`document.querySelector('main .panel h2')?.textContent.trim()`);
  await run(`$btn('Run all').click()`);
  await waitFor(
    `!![...document.querySelectorAll('main button')].find((b) => b.textContent.trim() === 'Run again')`,
    15000,
  );
  await sleep(800);
  await openCatalog(NAME);
  await sleep(400);
  const tests = await run(
    `({ tab: document.querySelector('main [role="tab"][aria-selected="true"]')?.textContent.trim(), tiles: $tiles(), latest: document.querySelector('main section[aria-labelledby="latest-title"] .card-head')?.textContent.replace(/\\s+/g, ' ').trim(), rows: [...document.querySelectorAll('main section[aria-labelledby="latest-title"] tbody tr')].map($cellsOf), runs: document.querySelectorAll('main section[aria-labelledby="runs-title"] tbody tr').length })`,
  );
  check('before any run the Tests tab says so', before === 'No runs yet', before);
  check(
    'after Run all the catalog comes back to Tests with the run kept',
    tests.tab === 'Tests' && tests.latest?.startsWith('Latest run Failed') && tests.runs === 1,
    short(tests),
  );
  check(
    'coverage, pass rate and tests come from the run',
    tests.tiles['Test coverage']?.value === '33%' &&
      tests.tiles['Pass rate, latest run']?.value === '67%' &&
      tests.tiles['Tests, latest run']?.value === '1 of 1',
    short(tests.tiles),
  );
  check(
    'each request of the run has its result',
    tests.rows.length === 4 &&
      tests.rows.some((row) => row.startsWith('WS Live') && row.endsWith('Skipped')) &&
      tests.rows.some(
        (row) => row.startsWith('GET Flaky report | None') && row.endsWith('Failed 500'),
      ) &&
      tests.rows.some(
        (row) => row.startsWith('GET List users | 1 of 1') && row.endsWith('Passed 200'),
      ),
    short(tests.rows),
  );
  await shot('catalog-tests.png');

  // ---------- back to the list: the run, the owner and the tag filter ----------
  await run(`$btn('All APIs').click()`);
  await waitFor(`!!$row(${JSON.stringify(NAME)})`, 6000);
  const after = await run(`$cells(${JSON.stringify(NAME)})`);
  await run(`$pick(document.querySelector('main button[aria-label="Tag"]'), 'e2e')`);
  const tagged = await run(
    `[...document.querySelectorAll('main tbody button.open')].map((b) => b.textContent.trim())`,
  );
  await run(`$pick(document.querySelector('main button[aria-label="Tag"]'), 'All tags')`);
  check(
    'the list shows the last run and owner, and filters by tag',
    after[2].startsWith('Failed') &&
      after.join(' ').includes('QA team') &&
      JSON.stringify(tagged) === JSON.stringify([NAME]),
    short({ after, tagged }),
  );

  // ---------- the flow link ----------
  await openApi();
  await run(
    `[...document.querySelectorAll('main .flows button.target')].find((b) => b.textContent.trim() === ${JSON.stringify(FLOW)}).click()`,
  );
  const flowOpen = await waitFor(
    `$h1() === 'Orchestration' && document.querySelector('.editor .name input')?.value === ${JSON.stringify(FLOW)}`,
    5000,
  );
  check('a flow in "Used in Orchestration" opens in Orchestration', flowOpen);

  // ---------- the columns a narrow window keeps ----------
  await openCatalog(NAME);
  await run(`$btn('All APIs').click()`);
  await waitFor(`!!document.querySelector('main .table-wrap')`, 6000);
  const columns = await run(
    `({ width: document.querySelector('main .table-wrap').clientWidth, shown: [...document.querySelectorAll('main thead th')].filter((th) => getComputedStyle(th).display !== 'none').map((th) => th.textContent.trim()) })`,
  );
  check(
    'a narrow list keeps the ranking columns and moves the owner under the name',
    columns.width > 760
      ? columns.shown.length === 7
      : JSON.stringify(columns.shown) ===
          JSON.stringify(['API', 'Health', 'Last run', 'Error rate']),
    short(columns),
  );
} catch (error) {
  check('the suite ran to the end', false, error.message.slice(0, 300));
} finally {
  if (flowId) await run(`$invoke('delete_flow', { id: '${flowId}' })`);
  if (collectionId) await run(`$invoke('delete_collection', { id: '${collectionId}' })`);
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
  const leftovers = await run(`$invoke('list_collections')`);
  check('test data cleaned up', !leftovers.some((collection) => collection.name === NAME));
  server.close();
  socket.close();
}

console.log(results.join('\n'));
process.exit(results.some((line) => line.startsWith('FAIL')) ? 1 : 0);
