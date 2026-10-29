// End-to-end check of stage 15f: saving responses as examples, managing them in the Examples
// tab, the local mock server answering with them, and examples in the exported collection.
// Creates "E2E Mock" and deletes it afterwards.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Mock';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const here = path.dirname(fileURLToPath(import.meta.url));
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'pc-e2e-'));
const exportPath = path.join(work, 'mock.collection.json');

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'application/json', 'X-Real': '1' });
  res.end(JSON.stringify({ id: 7, name: 'Ada', path: req.url }));
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
  window.$card = () => document.querySelector('article.card');
  window.$row = (text) => [...document.querySelectorAll('.row')].find((r) => r.textContent.includes(text));
  window.$tab = (text) => [...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith(text));
  window.$examples = () => [...document.querySelectorAll('.examples li')].map((li) => ({ status: li.querySelector('.chip')?.textContent.trim(), name: li.querySelector('.name input')?.value }));
  window.$json = () => { try { return JSON.parse(document.querySelector('.response .cm-content')?.innerText ?? ''); } catch { return null; } };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const content = (name, method, url, extra = {}) => ({
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
  pathVariables: [],
  graphqlVariables: '',
  ...extra,
});
async function send() {
  await run(`$card().querySelector('.send').click()`);
  await sleep(300);
  await waitFor(`!$card().querySelector('.send').disabled`, 15000);
  await sleep(300);
}

await connect();
let mockUrl = null;
try {
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
  );
  const getUser = await run(
    `$invoke('create_request', { collectionId: '${collection.id}', sectionId: null, content: ${JSON.stringify(content('Get user', 'GET', '{{baseUrl}}/users/:id', { pathVariables: [{ key: 'id', value: '7' }] }))} })`,
  );
  await run(
    `$invoke('create_request', { collectionId: '${collection.id}', sectionId: null, content: ${JSON.stringify(content('Create user', 'POST', '{{baseUrl}}/users'))} })`,
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);

  // ---------- save as example ----------
  await run(`$row('Get user').click()`);
  await sleep(800);
  await send();
  await run(`$btn('Save as example').click()`);
  await waitFor(`($tab('Examples')?.textContent ?? '').includes('· 1')`, 5000);
  await run(`$tab('Examples').click()`);
  await sleep(400);
  const saved = await run(`$examples()`);
  check(
    'a response is saved as an example and listed',
    short(saved) === short([{ status: '200', name: '200 OK' }]),
    short(saved),
  );
  await run(
    `(() => { const input = document.querySelector('.examples .name input'); input.focus(); input.select(); })()`,
  );
  await cdp('Input.insertText', { text: 'Ada' });
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: 'Enter',
      code: 'Enter',
      windowsVirtualKeyCode: 13,
    });
  }
  await sleep(600);
  await run(
    `$invoke('save_example', { requestId: '${getUser.id}', example: { name: 'Missing', status: 404, headers: [{ name: 'Content-Type', value: 'application/json' }], body: '{"error":"nope"}' } })`,
  );
  const stored = await run(`$invoke('list_examples', { requestId: '${getUser.id}' })`);
  check(
    'examples can be renamed',
    stored.map((e) => e.name).join() === 'Ada,Missing',
    short(stored),
  );
  await run(`document.querySelector('.examples .toggle').click()`);
  await sleep(500);
  const opened = await run(
    `({ headers: [...document.querySelectorAll('.examples .headers dt')].map((d) => d.textContent), body: document.querySelector('.examples .body .cm-content')?.innerText ?? '' })`,
  );
  check(
    'an example opens with its headers and body',
    opened.headers.some((h) => h.toLowerCase() === 'x-real') &&
      opened.body.includes('"name": "Ada"'),
    short(opened),
  );

  // ---------- mock server ----------
  await run(`$card().querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(400);
  await run(`$btn('Mock server').click()`);
  await sleep(400);
  await run(`$btn('Start mock server').click()`);
  await waitFor(`!!document.querySelector('.panel .running .url')`, 5000);
  mockUrl = await run(`document.querySelector('.panel .running .url').textContent.trim()`);
  const headerLabel = await run(
    `document.querySelector('header.top button[aria-label^="Mock server"]')?.getAttribute('aria-label')`,
  );
  check(
    'the mock server starts on 127.0.0.1',
    /^http:\/\/127\.0\.0\.1:\d+$/.test(mockUrl) &&
      headerLabel === `Mock server · running at ${mockUrl}`,
    short({ mockUrl, headerLabel }),
  );
  const first = await fetch(`${mockUrl}/users/99?x=1`);
  const firstBody = await first.json();
  check(
    'a matching path answers with the first successful example',
    first.status === 200 &&
      firstBody.name === 'Ada' &&
      first.headers.get('x-real') === '1' &&
      first.headers.get('x-codestamp-mock') === '1',
    short({ status: first.status, firstBody }),
  );
  const picked = await fetch(`${mockUrl}/users/99`, { headers: { 'x-mock-response-code': '404' } });
  check(
    'x-mock-response-code picks another example',
    picked.status === 404 && (await picked.json()).error === 'nope',
  );
  const named = await fetch(`${mockUrl}/users/1`, {
    headers: { 'x-mock-response-name': 'missing' },
  });
  check('x-mock-response-name picks by name', named.status === 404);
  const none = await fetch(`${mockUrl}/nope`, { method: 'DELETE' });
  const noneBody = await none.json();
  check(
    'an unmatched request gets a clear 404',
    none.status === 404 && noneBody.error === 'No saved example matches DELETE /nope',
    short(noneBody),
  );
  await waitFor(`document.querySelectorAll('.hits li').length >= 4`, 4000);
  const hits = await run(
    `[...document.querySelectorAll('.hits li')].map((li) => li.querySelector('.request').textContent + ' ' + li.querySelector('.example').textContent)`,
  );
  check(
    'the panel logs what the mock answered',
    hits.length === 4 && hits[0].startsWith('DELETE /nope') && hits.at(-1) === 'GET /users/99 Ada',
    short(hits),
  );

  // ---------- send through the mock from the app ----------
  const setId = await run(
    `$invoke('get_collection_variable_set', { collectionId: '${collection.id}' })`,
  );
  const views = await run(`$invoke('get_environment_variables', { id: '${setId}' })`);
  await run(
    `$invoke('save_environment_variables', { id: '${setId}', variables: ${JSON.stringify(views.map((v) => ({ id: v.id, key: v.key, value: v.key === 'baseUrl' ? mockUrl : v.value, isSecret: v.isSecret, enabled: v.enabled })))} })`,
  );
  await run(`$row('Get user').click()`);
  await sleep(800);
  await send();
  const body = await run(`$json()`);
  await run(
    `[...document.querySelectorAll('.response [role="tab"]')].find((t) => t.textContent.startsWith('Headers')).click()`,
  );
  await sleep(300);
  const throughMock = {
    body,
    headers: await run(
      `[...document.querySelectorAll('.response .headers dt')].map((d) => d.textContent.toLowerCase())`,
    ),
  };
  check(
    'requests sent from Codestamp reach the mock',
    throughMock.body?.name === 'Ada' && throughMock.headers.includes('x-codestamp-mock'),
    short(throughMock),
  );
  await run(`$card().querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(400);

  // ---------- export ----------
  await run(`$btn('Export collection').click()`);
  await waitFor(`!!document.querySelector('section[aria-label="Export"]')`, 5000);
  await run(
    `(async () => { const panel = document.querySelector('section[aria-label="Export"]'); panel.querySelector('button[aria-label="Export format"]').click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === 'Postman Collection v2.1').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); })()`,
  );
  await sleep(200);
  const exported = answerDialog('Export collection', exportPath);
  await run(`$btn('Export…').click()`);
  await exported;
  await waitFor(`(document.body.innerText ?? '').includes('mock.collection.json')`, 5000);
  const file = JSON.parse(fs.readFileSync(exportPath, 'utf8'));
  const responses = file.item.find((item) => item.name === 'Get user')?.response ?? [];
  check(
    'examples are exported as Collection v2.1 responses',
    responses.map((r) => `${r.name}:${r.code}`).join() === 'Ada:200,Missing:404' &&
      responses[0].header.some((h) => h.key.toLowerCase() === 'x-real'),
    short(responses.map((r) => ({ name: r.name, code: r.code }))),
  );

  // ---------- stop and delete ----------
  await run(`$btn('Mock server').click()`);
  await waitFor(`!!$btn('Stop')`, 4000);
  await run(`$btn('Stop').click()`);
  await waitFor(`!document.querySelector('.panel .running')`, 4000);
  let stopped = false;
  try {
    await fetch(`${mockUrl}/users/1`);
  } catch {
    stopped = true;
  }
  check('Stop closes the mock server', stopped);
  await run(`$btn('Close', document.querySelector('section[aria-label="Mock server"]')).click()`);
  await run(`$row('Get user').click()`);
  await sleep(800);
  await run(`$tab('Examples').click()`);
  await sleep(300);
  await run(`$btn('Delete example Missing').click()`);
  await sleep(500);
  check(
    'an example can be deleted',
    short(await run(`$examples()`)) === short([{ status: '200', name: 'Ada' }]),
    short(await run(`$examples()`)),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
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
