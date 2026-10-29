// End-to-end check of Orchestration: a flow from Start to End over a local shop API (register,
// login, add to cart, checkout), drawn as a step map with the values passed between steps, checks,
// failure handling, saving, deleted requests and deleting the flow. Creates "E2E Shop" and
// removes it.
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Shop';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const SHOTS = process.argv.includes('--shots');

const seen = { auth: [], checkout: [] };
const server = http.createServer((req, res) => {
  let body = '';
  req.on('data', (chunk) => (body += chunk));
  req.on('end', () => {
    const json = (status, value) => {
      res.writeHead(status, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(value));
    };
    if (req.url === '/register') return json(201, { id: 'u-1', email: 'rina@test.dev' });
    if (req.url === '/login') return json(200, { token: 'tok-42' });
    if (req.url === '/cart') {
      seen.auth.push(req.headers.authorization ?? null);
      return req.headers.authorization === 'Bearer tok-42'
        ? json(201, { cartId: 'c-9', total: 49.5 })
        : json(401, { error: 'no token' });
    }
    if (req.url === '/checkout') {
      seen.checkout.push(body);
      return body.includes('c-9') ? json(200, { orderId: 'o-1' }) : json(422, { error: 'no cart' });
    }
    json(404, {});
  });
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
    await sleep(150);
  }
  return false;
}
async function shot(name) {
  if (!SHOTS) return;
  const reply = await cdp('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync(
    path.join(here, '.work', 'screens', name),
    Buffer.from(reply.result.data, 'base64'),
  );
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$map = () => document.querySelector('ol[aria-label="Steps from start to end"]');
  window.$nodes = () => [...document.querySelectorAll('ol.nodes button.node')];
  window.$open = async (n) => { $nodes()[n - 1].click(); await new Promise((r) => setTimeout(r, 120)); };
  window.$card = () => document.querySelector('article.step');
  window.$stepInfo = () => $nodes().map((s) => ({ number: s.querySelector('.number')?.textContent.trim(), stamp: s.querySelector('.stamp')?.textContent.trim(), name: s.querySelector('.name')?.textContent.trim(), result: s.querySelector('.result')?.textContent.trim(), state: ['passed','failed','skipped','running','idle'].find((c) => s.classList.contains(c)), current: s.getAttribute('aria-current') === 'step' }));
  window.$lines = () => [...document.querySelectorAll('.map .line')].map((l) => ({ value: l.querySelector('.chip').textContent.trim() + ':' + ['planned','passed','missing'].find((c) => l.classList.contains(c)), columns: l.style.gridColumn.replace(/\\s+/g, ''), tips: l.querySelectorAll('.tip').length }));
  window.$uses = () => [...($card()?.querySelectorAll('.uses .use') ?? [])].map((u) => u.textContent.replace(/\\s+/g, ' ').trim());
  window.$stopped = () => { const s = document.querySelector('.map .stopped'); return s ? { label: s.textContent.trim(), columns: s.style.gridColumn.replace(/\\s+/g, '') } : null; };
  window.$end = () => { const e = document.querySelector('.map .pill.end'); return { pill: e?.textContent.trim(), state: e?.classList.contains('passed') ? 'passed' : e?.classList.contains('failed') ? 'failed' : null, text: document.querySelector('.map-card .end-text')?.textContent.replace(/\\s+/g, ' ').trim() }; };
  window.$pickRequest = async (name) => { $btn('Add step').click(); await new Promise((r) => setTimeout(r, 300)); const input = document.querySelector('dialog.picker input'); $set(input, name); await new Promise((r) => setTimeout(r, 150)); const target = [...document.querySelectorAll('dialog.picker .results button')].find((b) => b.querySelector('.name')?.textContent.trim() === name); target.click(); await new Promise((r) => setTimeout(r, 250)); };
  window.$tab = (label) => [...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith(label)).click();
  window.$last = () => [...($card()?.querySelectorAll('.last li') ?? [])].map((li) => li.textContent.replace(/\\s+/g, ' ').trim());
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 400);
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

async function runAndWait() {
  await run(`$btn('Run flow').click()`);
  await sleep(200);
  await waitFor(`!!$btn('Run flow')`, 15000);
  await sleep(300);
}
async function saved() {
  await waitFor(`document.querySelector('.editor .save')?.textContent.trim() === 'Saved'`, 4000);
}

await connect();
// A fresh page, so no flow from an earlier run is still held in memory.
await run('location.reload()');
await sleep(1800);
await run(helpers);
for (const stale of await run(`$invoke('list_collections')`)) {
  if (stale.name === NAME) await run(`$invoke('delete_collection', { id: '${stale.id}' })`);
}
let collectionId = null;
const flowsAtStart = new Set((await run(`$invoke('list_flows')`)).map((flow) => flow.id));
try {
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
  );
  collectionId = collection.id;
  const created = {};
  for (const content of [
    request('Register', 'POST', '{{baseUrl}}/register'),
    request('Login', 'POST', '{{baseUrl}}/login'),
    request('Add to cart', 'POST', '{{baseUrl}}/cart', {
      headers: [{ enabled: true, key: 'Authorization', value: 'Bearer {{token}}' }],
    }),
    request('Checkout', 'POST', '{{baseUrl}}/checkout', {
      bodyMode: 'json',
      body: '{"cartId":"{{cartId}}"}',
    }),
    { ...request('Live feed', 'GET', 'ws://127.0.0.1/feed'), kind: 'websocket', bodyMode: 'text' },
  ]) {
    created[content.name] = await run(
      `$invoke('create_request', { collectionId: '${collectionId}', sectionId: null, content: ${JSON.stringify(content)} })`,
    );
  }
  const flowsBefore = await run(`$invoke('list_flows')`);

  // ---------- the page and a new flow ----------
  await run(`$btn('Orchestration').click()`);
  await sleep(700);
  const page = await run(
    `({ title: document.querySelector('h1.page-title')?.textContent.trim(), empty: !!document.querySelector('section[aria-label="No flows yet"]') })`,
  );
  check(
    'the sidebar opens the Orchestration page',
    page.title === 'Orchestration' && page.empty === (flowsBefore.length === 0),
    short(page),
  );
  await run(`$btn('New flow').click()`);
  await waitFor(
    `!!$map() && $nodes().length === 0 && document.querySelector('.editor .name input')?.value.startsWith('New flow')`,
    4000,
  );
  await sleep(200);
  const fresh = await run(
    `({ items: [...$map().children].map((li) => li.querySelector('button')?.getAttribute('aria-label') ?? li.textContent.trim()), end: $end(), card: !!$card(), env: !!document.querySelector('.map-card .bar button[aria-label="Active environment"]') })`,
  );
  check(
    'a new flow shows Start, Add step and End on the map',
    fresh.items.join('|') === 'Start|Add step|End' &&
      fresh.end.pill === 'End' &&
      !fresh.card &&
      fresh.env,
    short(fresh),
  );
  await run(
    `(() => { const input = document.querySelector('.editor .name input'); $set(input, 'Register to checkout'); input.dispatchEvent(new FocusEvent('blur')); })()`,
  );

  // ---------- steps ----------
  await run(`$btn('Add step').click()`);
  await sleep(500);
  const offered = await run(
    `[...document.querySelectorAll('dialog.picker .results .name')].map((n) => n.textContent.trim())`,
  );
  check(
    'the picker offers HTTP requests but not WebSocket ones',
    ['Register', 'Login', 'Add to cart', 'Checkout'].every((n) => offered.includes(n)) &&
      !offered.includes('Live feed'),
    short(offered),
  );
  await run(`document.querySelector('dialog.picker .close').click()`);
  await sleep(200);
  for (const name of ['Register', 'Login', 'Add to cart', 'Checkout']) {
    await run(`$pickRequest(${JSON.stringify(name)})`);
  }
  const steps = await run(
    `({ steps: $stepInfo(), items: [...$map().children].map((li) => li.querySelector('button.add, .add button')?.getAttribute('aria-label') ?? li.querySelector('.pill')?.textContent.trim() ?? 'step'), card: $card()?.getAttribute('aria-label') })`,
  );
  check(
    'steps are numbered in order on the map, Add step sits before End',
    steps.steps.map((s) => `${s.number} ${s.stamp} ${s.name}`).join('|') ===
      '01 POST Register|02 POST Login|03 POST Add to cart|04 POST Checkout' &&
      steps.items.at(0) === 'Start' &&
      steps.items.at(-2) === 'Add step' &&
      steps.items.at(-1) === 'End' &&
      steps.steps.at(-1).current &&
      steps.card === 'Step 4: Checkout',
    short(steps),
  );

  // Login passes the token on; Add to cart passes the cart id and checks for 201.
  await run(`$open(2)`);
  await run(`$tab('Pass on')`);
  await sleep(200);
  await run(`$btn('+ Add value to pass on', $card()).click()`);
  await sleep(150);
  await run(
    `(() => { const inputs = $card().querySelectorAll('.rule input'); $set(inputs[0], 'token'); $set(inputs[1], 'token'); })()`,
  );
  await run(`$open(3)`);
  await run(`$tab('Pass on')`);
  await sleep(200);
  await run(`$btn('+ Add value to pass on', $card()).click()`);
  await sleep(150);
  await run(
    `(() => { const inputs = $card().querySelectorAll('.rule input'); $set(inputs[0], 'cartId'); $set(inputs[1], 'cartId'); })()`,
  );
  await run(`$tab('Checks')`);
  await sleep(150);
  await run(`$btn('+ Add check', $card()).click()`);
  await sleep(150);
  await run(`$set($card().querySelector('.rule input.value'), '201')`);
  await saved();
  await sleep(200);
  const planned = await run(
    `({ lines: $lines(), uses: $uses(), card: $card()?.getAttribute('aria-label') })`,
  );
  check(
    'the selected step shows the values it receives and passes on',
    planned.card === 'Step 3: Add to cart' &&
      JSON.stringify(planned.lines) ===
        JSON.stringify([
          { value: '{{token}}:planned', columns: '3/5', tips: 1 },
          { value: '{{cartId}}:planned', columns: '4/6', tips: 1 },
        ]) &&
      JSON.stringify(planned.uses) === JSON.stringify(['{{token}} from step 2']),
    short(planned),
  );
  await run(`$nodes()[0].focus()`);
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: 'ArrowRight',
      code: 'ArrowRight',
      windowsVirtualKeyCode: 39,
    });
  }
  await sleep(200);
  const byKey = await run(
    `({ current: $stepInfo().findIndex((s) => s.current) + 1, focused: document.activeElement === $nodes()[1], card: $card()?.getAttribute('aria-label'), lines: $lines().map((l) => l.value) })`,
  );
  check(
    'arrow keys move between steps on the map',
    byKey.current === 2 &&
      byKey.focused &&
      byKey.card === 'Step 2: Login' &&
      JSON.stringify(byKey.lines) === JSON.stringify(['{{token}}:planned']),
    short(byKey),
  );
  await run(`$open(3)`);
  await shot('postflow-edit.png');

  // ---------- a passing run ----------
  await runAndWait();
  const passedRun = await run(
    `({ steps: $stepInfo(), end: $end(), lines: $lines().map((l) => l.value), stopped: $stopped() })`,
  );
  check(
    'a run goes from Start to End and every step passes',
    passedRun.steps.every((s) => s.state === 'passed') &&
      passedRun.end.state === 'passed' &&
      passedRun.end.text.startsWith('4 passed · 0 failed') &&
      passedRun.stopped === null,
    short(passedRun),
  );
  check(
    'values reach later steps: the token and the cart id',
    seen.auth.at(-1) === 'Bearer tok-42' &&
      seen.checkout.at(-1) === '{"cartId":"c-9"}' &&
      JSON.stringify(passedRun.lines) === '["{{token}}:passed","{{cartId}}:passed"]',
    short({ seen, lines: passedRun.lines }),
  );
  await shot('postflow-passed.png');

  // ---------- a failing check stops the flow ----------
  await run(`$open(3)`);
  await run(`$tab('Checks')`);
  await sleep(150);
  await run(`$set($card().querySelector('.rule input.value'), '200')`);
  await saved();
  await run(`$open(1)`);
  await runAndWait();
  const failedRun = await run(
    `({ steps: $stepInfo(), end: $end(), card: $card()?.getAttribute('aria-label'), last: $last(), stopped: $stopped() })`,
  );
  check(
    'a failed step stops the flow; later steps are skipped',
    failedRun.steps[2].state === 'failed' &&
      failedRun.steps[3].state === 'skipped' &&
      failedRun.end.state === 'failed' &&
      failedRun.end.text.includes('Stopped at step 3: Add to cart') &&
      JSON.stringify(failedRun.stopped) ===
        JSON.stringify({ label: 'Stopped at step 3', columns: '4/-1' }),
    short(failedRun),
  );
  check(
    'the failed step is selected and says why',
    failedRun.steps[2].current &&
      failedRun.card === 'Step 3: Add to cart' &&
      failedRun.last.some((line) => line.includes('status equals 200: got 201')),
    short({ card: failedRun.card, last: failedRun.last }),
  );
  await shot('postflow-failed.png');

  // ---------- continue after a failure ----------
  await run(`$card().querySelector('button[aria-label="If this step fails"]').click()`);
  await sleep(200);
  await run(
    `[...document.querySelectorAll('ul[role="listbox"] li[role="option"]')].find((o) => o.textContent.trim().startsWith('Continue')).dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))`,
  );
  await saved();
  await runAndWait();
  const continued = await run(`({ steps: $stepInfo(), end: $end(), stopped: $stopped() })`);
  check(
    '“Continue with the next step” keeps going after a failure',
    continued.steps[2].state === 'failed' &&
      continued.steps[3].state === 'passed' &&
      continued.end.text.startsWith('3 passed · 1 failed') &&
      continued.stopped === null,
    short(continued),
  );

  // ---------- reorder, persistence, deleted request ----------
  await run(`$open(4)`);
  await run(`$card().querySelector('button[aria-label="Move left"]').click()`);
  await saved();
  const moved = await run(
    `({ names: $stepInfo().map((s) => s.name), current: $stepInfo().findIndex((s) => s.current) + 1 })`,
  );
  await run(`$card().querySelector('button[aria-label="Move right"]').click()`);
  await saved();
  const flows = await run(`$invoke('list_flows')`);
  const flow = flows.find((f) => f.name === 'Register to checkout');
  const stored = await run(`$invoke('get_flow', { id: '${flow.id}' })`);
  check(
    'steps move, and the flow is saved with its rules',
    moved.names.join('|') === 'Register|Login|Checkout|Add to cart' &&
      moved.current === 3 &&
      stored.steps.map((s) => s.request?.name).join('|') ===
        'Register|Login|Add to cart|Checkout' &&
      stored.steps[1].extracts[0].name === 'token' &&
      stored.steps[2].onFailure === 'continue' &&
      stored.steps[2].checks[0].value === '200',
    short(
      stored.steps.map((s) => ({
        name: s.request?.name,
        extracts: s.extracts,
        checks: s.checks,
        onFailure: s.onFailure,
      })),
    ),
  );
  await run(`$invoke('delete_request', { id: '${created['Checkout'].id}' })`);
  await run(`$btn('Home').click()`);
  await sleep(400);
  await run(`$btn('Orchestration').click()`);
  await sleep(800);
  const gap = await run(`$stepInfo().map((s) => s.name)`);
  check(
    'coming back to Orchestration shows a deleted request as a gap',
    gap.join('|') === 'Register|Login|Add to cart|Request deleted',
    short(gap),
  );

  // ---------- quick search and deleting the flow ----------
  await run(`$btn('Home').click()`);
  await sleep(300);
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: ' ',
      code: 'Space',
      windowsVirtualKeyCode: 32,
      modifiers: 2,
    });
  }
  await sleep(400);
  await cdp('Input.insertText', { text: 'postflow' });
  await sleep(350);
  const offeredPage = await run(
    `document.querySelector('dialog.quick-search [role="option"][aria-selected="true"] .title')?.textContent.trim()`,
  );
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: 'Enter',
      code: 'Enter',
      windowsVirtualKeyCode: 13,
    });
  }
  await sleep(700);
  const viaSearch = await run(`document.querySelector('h1.page-title')?.textContent.trim()`);
  check(
    'quick search finds the Orchestration page',
    offeredPage === 'Orchestration' && viaSearch === 'Orchestration',
    short({ offeredPage, viaSearch }),
  );
  await run(`$btn('Delete flow').click()`);
  await sleep(300);
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Delete').click()`,
  );
  await sleep(600);
  const afterDelete = await run(`$invoke('list_flows')`);
  check('a flow can be deleted', !afterDelete.some((f) => f.id === flow.id), short(afterDelete));
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    for (const leftover of await run(`$invoke('list_flows')`)) {
      if (!flowsAtStart.has(leftover.id))
        await run(`$invoke('delete_flow', { id: '${leftover.id}' })`);
    }
    if (collectionId)
      await run(`$invoke('delete_collection', { id: '${collectionId}' }).catch(() => null)`);
    check(
      'test data cleaned up',
      !(await run(`$invoke('list_collections')`)).some((c) => c.name === NAME),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  server.close();
  process.exit(0);
}
