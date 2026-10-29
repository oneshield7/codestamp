// End-to-end check of stage 15h: gRPC requests. Services by server reflection or from a .proto
// file (with imports), unary and streaming calls, metadata, auth, error statuses and runs.
// Creates "E2E gRPC" and deletes it.
import { spawn } from 'node:child_process';
import { cpSync, mkdtempSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { PROTO_DIR, startShop } from './grpc-shop-server.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E gRPC';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const shop = await startShop();
// A fresh copy each run, so the path has never been picked on this device before.
const protoCopy = mkdtempSync(path.join(here, 'protos-'));
cpSync(PROTO_DIR, protoCopy, { recursive: true });
const protoFile = path.join(protoCopy, 'shop.proto');
const address = `grpc://127.0.0.1:${shop.port}`;

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
  window.$row = (text) => [...document.querySelectorAll('.row')].find((r) => r.querySelector('.name')?.textContent.trim() === text);
  window.$panel = () => document.querySelector('section[aria-label="gRPC request"]');
  window.$message = () => $panel()?.querySelector('.cm-content')?.textContent ?? '';
  window.$reply = () => document.querySelector('article.card section.response');
  window.$summary = () => $reply()?.querySelector('.summary')?.textContent.replace(/\\s+/g, ' ').trim() ?? '';
  window.$replyText = () => ($reply()?.querySelector('.viewer .cm-content')?.textContent ?? '') + [...($reply()?.querySelectorAll('.messages pre') ?? [])].map((p) => p.textContent).join('|');
  window.$tabs = () => [...($reply()?.querySelectorAll('[role="tab"]') ?? [])].map((t) => t.textContent.trim());
  window.$rows = () => [...($reply()?.querySelectorAll('.rows dt') ?? [])].map((dt) => dt.textContent + '=' + (dt.nextElementSibling?.textContent ?? ''));
  window.$methodTrigger = () => $panel()?.querySelector('button[aria-label="gRPC method"]');
  window.$options = () => [...document.querySelectorAll('ul[role="listbox"] li[role="option"]')].map((li) => li.textContent.trim());
  window.$pick = (text) => { const li = [...document.querySelectorAll('ul[role="listbox"] li[role="option"]')].find((o) => o.textContent.trim().startsWith(text)); li.dispatchEvent(new MouseEvent('mousedown', { bubbles: true })); };
  window.$error = () => [...document.querySelectorAll('article.card [role="alert"]')].map((e) => e.textContent.trim()).join(' | ');
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const grpcRequest = (name, grpc, body, headers = [], auth = { type: 'inherit' }) => ({
  kind: 'grpc',
  name,
  method: 'POST',
  url: '{{shopUrl}}',
  params: [],
  headers,
  bodyMode: 'json',
  body,
  form: [],
  bodyFile: '',
  preRequestScript: '',
  testScript: '',
  auth,
  mcpOperation: null,
  pathVariables: [],
  graphqlVariables: '',
  grpc,
});
const reflection = (method) => ({ method, source: 'reflection', protoFile: '' });

async function openRow(name) {
  await run(`$card()?.querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(350);
  await run(`$row(${JSON.stringify(name)}).click()`);
  await waitFor(`!!$panel()`, 4000);
  await sleep(300);
}
async function invokeAndWait() {
  await run(`$card().querySelector('.send').click()`);
  await sleep(200);
  await waitFor(`!$summary().includes('Calling')`, 10000);
  await sleep(200);
}
async function showTab(label) {
  await run(
    `[...$reply().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith(${JSON.stringify(label)})).click()`,
  );
  await sleep(150);
}

await connect();
let collectionId;
// A run that stopped early may have left its collection behind.
for (const stale of await run(`$invoke('list_collections')`)) {
  if (stale.name === NAME) await run(`$invoke('delete_collection', { id: '${stale.id}' })`);
}
try {
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: '' })`,
  );
  collectionId = collection.id;
  const setId = await run(
    `$invoke('get_collection_variable_set', { collectionId: '${collectionId}' })`,
  );
  const variables = await run(
    `$invoke('save_environment_variables', { id: '${setId}', variables: [{ id: null, key: 'shopUrl', value: ${JSON.stringify(address)}, isSecret: false, enabled: true }, { id: null, key: 'token', value: 'secret-1', isSecret: true, enabled: true }] }).catch((e) => 'ERR ' + JSON.stringify(e))`,
  );
  const requests = [
    grpcRequest('Browse', reflection(''), ''),
    grpcRequest(
      'Get order',
      reflection('shop.Shop/GetOrder'),
      '{"id": "42"}',
      [{ enabled: true, key: 'X-Trace', value: 't-1' }],
      { type: 'bearer', token: '{{token}}', prefix: 'Bearer' },
    ),
    grpcRequest('List orders', reflection('shop.Shop/ListOrders'), '{"count": 3}'),
    grpcRequest(
      'Add items',
      reflection('shop.Shop/AddItems'),
      '[{"sku": "a", "quantity": 2}, {"sku": "b", "quantity": 3}]',
    ),
    grpcRequest('Chat', reflection('shop.Shop/Chat'), '[{"text": "hi"}, {"text": "yo"}]'),
    grpcRequest('Missing', reflection('shop.Shop/GetOrder'), '{"id": "missing"}'),
    grpcRequest('Wrong field', reflection('shop.Shop/GetOrder'), '{"nope": 1}'),
    grpcRequest('From file', { method: '', source: 'proto', protoFile }, ''),
  ];
  for (const content of requests) {
    await run(
      `$invoke('create_request', { collectionId: '${collectionId}', sectionId: null, content: ${JSON.stringify(content)} })`,
    );
  }
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  check(
    'collection variables are set for the test',
    JSON.stringify(variables).includes('shopUrl'),
    short(variables),
  );

  const stamps = await run(
    `[...document.querySelectorAll('.row')].map((r) => r.textContent.replace(/\\s+/g, ' ').trim()).slice(0, 2)`,
  );
  const runAll = await run(`$btn('Run all')?.disabled ?? null`);
  check(
    'rows are stamped gRPC and Run all is off for gRPC only',
    stamps.every((s) => s.startsWith('gRPC') || s.includes(' gRPC ')) && runAll === true,
    short({ stamps, runAll }),
  );

  // ---------- server reflection ----------
  await openRow('Browse');
  const card = await run(
    `({ stamp: $card().querySelector('.method.grpc')?.textContent.trim(), code: !!$btn('Copy as code', $card()), capture: !!$btn('Capture an image', $card()), invoke: $card().querySelector('.send').textContent.trim(), disabled: $card().querySelector('.send').disabled, picker: $methodTrigger()?.disabled })`,
  );
  check(
    'the card shows gRPC, Invoke, and no code or capture tools',
    card.stamp === 'gRPC' &&
      !card.code &&
      !card.capture &&
      card.invoke.startsWith('Invoke') &&
      card.disabled &&
      card.picker,
    short(card),
  );
  await run(`$btn('Load services', $panel()).click()`);
  await waitFor(`!$methodTrigger().disabled`, 8000);
  await run(`$methodTrigger().click()`);
  await sleep(300);
  const options = await run(`$options()`);
  check(
    'reflection lists every method with its kind of stream',
    options.includes('shop.Shop/GetOrder · unary') &&
      options.includes('shop.Shop/ListOrders · server stream') &&
      options.includes('shop.Shop/AddItems · client stream') &&
      options.includes('shop.Shop/Chat · bidirectional stream') &&
      !options.some((o) => o.includes('ServerReflection')),
    short(options),
  );
  await run(`$pick('shop.Shop/GetOrder')`);
  await sleep(400);
  const unaryTemplate = await run(`$message()`);
  check(
    'choosing a method fills in a message with every field',
    unaryTemplate.includes('"id": ""') && !unaryTemplate.trim().startsWith('['),
    short(unaryTemplate),
  );
  await run(`$methodTrigger().click()`);
  await sleep(250);
  await run(`$pick('shop.Shop/AddItems')`);
  await sleep(400);
  const streamTemplate = await run(`$message()`);
  check(
    'an untouched template follows the method; client streams get an array',
    streamTemplate.trim().startsWith('[') &&
      streamTemplate.includes('"sku": ""') &&
      streamTemplate.includes('"quantity": 0'),
    short(streamTemplate),
  );

  // ---------- unary with metadata and auth ----------
  await openRow('Get order');
  await invokeAndWait();
  const unary = await run(`({ summary: $summary(), text: $replyText(), tabs: $tabs() })`);
  check(
    'a unary call answers OK with the message',
    unary.summary.includes('0 OK') &&
      unary.summary.includes('1 message') &&
      unary.text.includes('"id": "42"') &&
      unary.text.includes('"total": "1234"') &&
      unary.text.includes('"status": "PAID"'),
    short(unary),
  );
  check(
    'metadata and Bearer auth (from a secret variable) reach the server',
    unary.text.includes('"trace": "t-1"') && unary.text.includes('"auth": "Bearer secret-1"'),
    short(unary.text),
  );
  await showTab('Headers');
  const headers = await run(`$rows()`);
  await showTab('Trailers');
  const trailers = await run(`$rows()`);
  check(
    'response headers and trailers are shown apart',
    headers.includes('x-served-by=test-shop') &&
      trailers.includes('x-trailer=done') &&
      !headers.some((h) => h.startsWith('x-trailer') || h.startsWith('grpc-status')),
    short({ headers, trailers }),
  );

  // ---------- streams ----------
  await openRow('List orders');
  await invokeAndWait();
  const serverStream = await run(
    `({ summary: $summary(), text: $replyText(), items: $reply().querySelectorAll('.messages li').length })`,
  );
  check(
    'a server stream shows each message',
    serverStream.summary.includes('0 OK') &&
      serverStream.items === 3 &&
      serverStream.text.includes('"id": "o3"'),
    short(serverStream),
  );
  await openRow('Add items');
  await invokeAndWait();
  const clientStream = await run(`({ summary: $summary(), text: $replyText() })`);
  check(
    'a client stream sends every array element',
    clientStream.summary.includes('0 OK') &&
      clientStream.text.includes('"count": 2') &&
      clientStream.text.includes('"quantity": 5'),
    short(clientStream),
  );
  await openRow('Chat');
  await invokeAndWait();
  const bidi = await run(`({ summary: $summary(), text: $replyText() })`);
  check(
    'a bidirectional stream sends and reads both ways',
    bidi.summary.includes('2 messages') &&
      bidi.text.includes('echo: hi') &&
      bidi.text.includes('echo: yo'),
    short(bidi),
  );

  // ---------- failures ----------
  await openRow('Missing');
  await invokeAndWait();
  const notFound = await run(`({ summary: $summary(), error: $error() })`);
  check(
    'an error status is shown with its message',
    notFound.summary.includes('5 NOT_FOUND') && notFound.error.includes('No order missing'),
    short(notFound),
  );
  await openRow('Wrong field');
  await invokeAndWait();
  const wrong = await run(`({ summary: $summary(), error: $error() })`);
  check(
    'a message that does not fit the type is refused before sending',
    wrong.summary.includes('Failed') && wrong.error.includes('does not fit OrderRequest'),
    short(wrong),
  );

  // ---------- a .proto file ----------
  await openRow('From file');
  await run(`$btn('Load services', $panel()).click()`);
  await waitFor(`$error() !== ''`, 5000);
  const ungranted = await run(`$error()`);
  check(
    'a .proto path not picked on this device is not read',
    ungranted.includes('only files picked on this device'),
    short(ungranted),
  );
  const picking = run(
    `($btn('shop.proto', $panel()) ?? $btn('Choose .proto', $panel())).click()`,
  ).catch((error) => error.message);
  await sleep(800);
  const answered = await answerDialog('Choose a .proto file', protoFile);
  await picking;
  await sleep(600);
  await run(`$btn('Load services', $panel()).click()`);
  await waitFor(`!$methodTrigger().disabled`, 8000);
  await run(`$methodTrigger().click()`);
  await sleep(250);
  const fileOptions = await run(`$options()`);
  await run(`$pick('shop.Shop/GetOrder')`);
  await sleep(400);
  await invokeAndWait();
  const fromFile = await run(
    `({ summary: $summary(), text: $replyText(), file: $panel().querySelector('.file')?.textContent.trim() })`,
  );
  check(
    'a picked .proto file (with an import beside it) describes the service',
    answered === 'answered' &&
      fileOptions.length === 4 &&
      fromFile.file === 'shop.proto' &&
      fromFile.summary.includes('0 OK'),
    short({ answered, fileOptions, fromFile }),
  );

  // ---------- saved state ----------
  await sleep(1500);
  const detail = await run(`$invoke('get_collection', { id: '${collectionId}' })`);
  const browse = detail.requests.find((r) => r.name === 'Browse');
  const loaded = await run(`$invoke('get_request', { id: '${browse.id}' })`);
  check(
    'the chosen method is saved with the request',
    loaded.kind === 'grpc' &&
      loaded.grpc?.method === 'shop.Shop/AddItems' &&
      loaded.grpc.source === 'reflection',
    short(loaded.grpc),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    await run(`$card()?.querySelector('button[aria-label="Collapse request"]')?.click()`);
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
  shop.close();
  rmSync(protoCopy, { recursive: true, force: true });
  process.exit(0);
}
