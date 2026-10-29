// End-to-end check of stage 15b: path variables (`/users/:id`) in the card, when sending, in
// the URL preview, in import and export, and collection runs driven by a CSV data file.
// Creates "E2E Paths" and "E2E Paths Import" and deletes them afterwards.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Paths';
const IMPORTED = 'E2E Paths Import';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const here = path.dirname(fileURLToPath(import.meta.url));
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'pc-e2e-'));
const dataPath = path.join(work, 'users.csv');
const exportPath = path.join(work, 'paths.collection.json');
fs.writeFileSync(
  dataPath,
  'email,name\nada@example.test,Ada\n"bo,b@example.test",Bob\ncy@example.test,"Cy ""C"""\n',
);

const seen = [];
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://local');
  seen.push(req.url);
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(
    JSON.stringify({
      path: url.pathname,
      rawPath: req.url.split('?')[0],
      query: Object.fromEntries(url.searchParams),
    }),
  );
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
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$pathTable = () => document.querySelector('section[aria-label="Path variables"]');
  window.$pathKeys = () => [...($pathTable()?.querySelectorAll('td.key') ?? [])].map((k) => k.textContent.trim());
  window.$pathInput = (name) => document.querySelector('[aria-label="Path variable ' + name + '"]');
  window.$body = () => document.querySelector('.response .cm-content')?.innerText ?? '';
  window.$json = () => { try { return JSON.parse($body()); } catch { return null; } };
  window.$error = () => document.querySelector('.response .error')?.textContent ?? '';
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
  ...extra,
});
async function send() {
  await run(`$card().querySelector('.send').click()`);
  await sleep(300);
  await waitFor(`!$card().querySelector('.send').disabled`, 15000);
  await sleep(300);
}
async function typeInto(selector, text) {
  await run(`(() => { const el = ${selector}; el.focus(); el.select?.(); })()`);
  await cdp('Input.dispatchKeyEvent', {
    type: 'rawKeyDown',
    key: 'a',
    code: 'KeyA',
    windowsVirtualKeyCode: 65,
    modifiers: 2,
  });
  await cdp('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: 'a',
    code: 'KeyA',
    windowsVirtualKeyCode: 65,
    modifiers: 2,
  });
  await cdp('Input.insertText', { text });
  await sleep(250);
}
async function deleteCollection(name) {
  if (!(await run(`!!$nav(${JSON.stringify(name)})`))) return;
  await run(`$nav(${JSON.stringify(name)}).click()`);
  await sleep(800);
  if ((await run(`document.querySelector('input[aria-label="Collection name"]')?.value`)) !== name)
    return;
  await run(`$btn('Delete collection').click()`);
  await sleep(300);
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Delete').click()`,
  );
  await sleep(900);
}

await connect();
try {
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
  );
  await run(
    `$invoke('create_request', { collectionId: '${collection.id}', sectionId: null, content: ${JSON.stringify(content('Get order', 'GET', '{{baseUrl}}/users/:id/orders/:orderId?expand=:id'))} })`,
  );
  const echo = content('Echo email', 'GET', '{{baseUrl}}/echo/:who', {
    params: [{ enabled: true, key: 'email', value: '{{email}}' }],
    pathVariables: [{ key: 'who', value: '{{name}}' }],
    testScript:
      "pm.test('data row', () => { pm.expect(pm.response.json().query.email).to.equal(pm.iterationData.get('email')); pm.expect(pm.info.iterationCount).to.equal(3); });",
  });
  await run(
    `$invoke('create_request', { collectionId: '${collection.id}', sectionId: null, content: ${JSON.stringify(echo)} })`,
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);

  // ---------- the card ----------
  await run(`$row('Get order').click()`);
  await sleep(800);
  const keys = await run(`$pathKeys()`);
  check(
    'the Params tab lists each :name of the path once',
    short(keys) === short([':id', ':orderId']),
    short(keys),
  );
  await send();
  check(
    'an empty path variable stops the send with a clear message',
    (await run(`$error()`)).includes('Fill in the path variable :id'),
    await run(`$error()`),
  );
  await typeInto(`$pathInput('id')`, '42');
  await typeInto(`$pathInput('orderId')`, 'a/b c');
  await sleep(500);
  const preview = await run(`document.querySelector('.preview')?.textContent.trim() ?? ''`);
  check(
    'the URL preview shows the filled path',
    preview.includes(`${base}/users/42/orders/a%2Fb%20c?expand=:id`),
    preview,
  );
  await send();
  const sent = await run(`$json()`);
  check(
    'values go into their segments, encoded, and the query keeps its text',
    sent?.rawPath === '/users/42/orders/a%2Fb%20c' && sent?.query?.expand === ':id',
    short(sent),
  );
  const tabLabel = await run(
    `[...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.includes('Params'))?.textContent.trim()`,
  );
  check('the tab count includes path variables', tabLabel === 'Params & Headers · 2', tabLabel);
  await run(
    `$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/users/:userId/orders/:orderId')`,
  );
  await sleep(300);
  check(
    'renaming a segment updates the table',
    short(await run(`$pathKeys()`)) === short([':userId', ':orderId']),
    short(await run(`$pathKeys()`)),
  );
  await run(
    `$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/users/:id/orders/:orderId')`,
  );
  await sleep(1500);
  const restored = await run(`$pathInput('id').value`);
  const saved = await run(
    `$invoke('get_collection', { id: '${collection.id}' }).then((c) => $invoke('get_request', { id: c.requests[0].id })).then((r) => ({ url: r.url, vars: r.pathVariables }))`,
  );
  check(
    'a value comes back with its name and only names in the URL are saved',
    restored === '42' &&
      short(saved.vars) ===
        short([
          { key: 'id', value: '42' },
          { key: 'orderId', value: 'a/b c' },
        ]),
    short({ restored, saved }),
  );

  // ---------- data-driven run ----------
  await run(`$card().querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(500);
  await run(`$btn('Run all').click()`);
  await waitFor(`!!document.querySelector('.summary') && !$btn('Stop')`, 20000);
  seen.length = 0;
  const picked = answerDialog('Choose a data file', dataPath);
  await run(`$btn('Use a data file').click()`);
  const answer = await picked;
  await waitFor(`!!document.querySelector('.data-name')`, 5000);
  const dataInfo = await run(
    `({ name: document.querySelector('.data-name')?.textContent.trim(), meta: document.querySelector('.data-file .meta')?.textContent.replace(/\\s+/g, ' ').trim() })`,
  );
  check(
    'a CSV data file can be chosen',
    answer === 'answered' &&
      dataInfo.name === 'users.csv' &&
      dataInfo.meta === '3 rows · email, name',
    short({ answer, dataInfo }),
  );
  await run(`$btn('Run again').click()`);
  await sleep(500);
  await waitFor(`!$btn('Stop')`, 30000);
  const passes = await run(
    `[...document.querySelectorAll('h2.iteration-head')].map((h) => h.textContent.replace(/\\s+/g, ' ').trim())`,
  );
  const echoes = seen.filter((url) => url.startsWith('/echo/'));
  const statuses = await run(
    `[...document.querySelectorAll('article.result')].map((a) => (a.classList.contains('passed') ? 'passed' : 'failed'))`,
  );
  check(
    'one pass per row, each titled with its data',
    passes.length === 3 &&
      passes[0].startsWith('Iteration 1 of 3') &&
      passes[1].includes('email=bo,b@example.test'),
    short(passes),
  );
  check(
    'each pass sends its row as variables and path values',
    short(echoes) ===
      short([
        '/echo/Ada?email=ada%40example.test',
        '/echo/Bob?email=bo%2Cb%40example.test',
        '/echo/Cy%20%22C%22?email=cy%40example.test',
      ]),
    short(echoes),
  );
  const echoRows = await run(
    `[...document.querySelectorAll('article.result')].filter((a) => a.textContent.includes('Echo email')).map((a) => (a.classList.contains('passed') ? 'passed' : 'failed'))`,
  );
  check(
    'test scripts read pm.iterationData and pm.info',
    short(echoRows) === short(['passed', 'passed', 'passed']),
    short(statuses),
  );
  await run(`$btn('Remove data file').click()`);
  await sleep(300);
  await run(
    `(() => { const i = document.querySelector('.iterations input'); i.value = '2'; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(200);
  seen.length = 0;
  await run(`$btn('Run again').click()`);
  await sleep(500);
  await waitFor(`!$btn('Stop')`, 30000);
  const twice = await run(`document.querySelectorAll('h2.iteration-head').length`);
  check(
    'without a data file, Iterations repeats the run',
    twice === 2 && seen.filter((url) => url.startsWith('/users/42')).length === 2,
    short({ twice, seen }),
  );

  // ---------- export and import ----------
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(800);
  await run(`$btn('Export collection').click()`);
  await waitFor(`!!document.querySelector('section[aria-label="Export"]')`, 5000);
  await run(
    `(async () => { const panel = document.querySelector('section[aria-label="Export"]'); panel.querySelector('button[aria-label="Export format"]').click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === 'Postman Collection v2.1').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); })()`,
  );
  await sleep(200);
  const exported = answerDialog('Export collection', exportPath);
  await run(`$btn('Export…').click()`);
  await exported;
  await waitFor(`(document.body.innerText ?? '').includes('paths.collection.json')`, 5000);
  const file = JSON.parse(fs.readFileSync(exportPath, 'utf8'));
  const orderUrl = file.item.find((item) => item.name === 'Get order')?.request.url;
  check(
    'export writes url.variable',
    short(orderUrl?.variable) ===
      short([
        { key: 'id', value: '42' },
        { key: 'orderId', value: 'a/b c' },
      ]),
    short(orderUrl),
  );
  file.info.name = IMPORTED;
  file.item.find((item) => item.name === 'Get order').request.url.variable = [
    { key: 'id', value: '7' },
  ];
  await run(`$nav('Import').click()`);
  await sleep(600);
  await run(
    `(() => { const input = document.querySelector('input[aria-label="Collection or environment file"]'); const dt = new DataTransfer(); dt.items.add(new File([${JSON.stringify(JSON.stringify(file))}], 'paths.collection.json', { type: 'application/json' })); input.files = dt.files; input.dispatchEvent(new Event('change', { bubbles: true })); })()`,
  );
  await waitFor(`!!$btn('Import 2 requests')`, 5000);
  await run(`$btn('Import 2 requests').click()`);
  await waitFor(`!!$nav(${JSON.stringify(IMPORTED)})`, 8000);
  const importedVars = await run(
    `$invoke('list_collections').then((list) => list.find((c) => c.name === ${JSON.stringify(IMPORTED)})).then((c) => $invoke('get_collection', { id: c.id })).then((c) => Promise.all(c.requests.map((r) => $invoke('get_request', { id: r.id })))).then((rs) => rs.map((r) => ({ name: r.name, vars: r.pathVariables })))`,
  );
  check(
    'import fills path variables from url.variable and adds the missing ones empty',
    short(importedVars) ===
      short([
        {
          name: 'Get order',
          vars: [
            { key: 'id', value: '7' },
            { key: 'orderId', value: '' },
          ],
        },
        { name: 'Echo email', vars: [{ key: 'who', value: '{{name}}' }] },
      ]),
    short(importedVars),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    await deleteCollection(NAME);
    await deleteCollection(IMPORTED);
    check(
      'test data cleaned up',
      !(await run(`!!$nav(${JSON.stringify(NAME)}) || !!$nav(${JSON.stringify(IMPORTED)})`)),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  server.close();
  fs.rmSync(work, { recursive: true, force: true });
  process.exit(0);
}
