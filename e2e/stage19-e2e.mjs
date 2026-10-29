// End-to-end check of stage 15e: GraphQL requests. The card, variables with {{variables}},
// fetching the schema, completion and checks in the editor, Copy as code and Run all, against a
// real GraphQL server built with the graphql package. Creates "E2E GraphQL" and deletes it.
import http from 'node:http';
import { buildSchema, graphql } from 'graphql';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E GraphQL';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const schema = buildSchema(`
  type User { id: ID!, name: String!, email: String }
  type Query { user(id: ID!): User, users: [User!]! }
`);
const users = { 7: { id: '7', name: 'Ada', email: 'ada@example.test' } };
const rootValue = { user: ({ id }) => users[id] ?? null, users: () => Object.values(users) };
const received = [];
const server = http.createServer((req, res) => {
  const chunks = [];
  req.on('data', (chunk) => chunks.push(chunk));
  req.on('end', async () => {
    let body;
    try {
      body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
    } catch {
      res.writeHead(400);
      return res.end('not json');
    }
    received.push({ contentType: req.headers['content-type'], body });
    const result = await graphql({
      schema,
      source: body.query,
      rootValue,
      variableValues: body.variables,
    });
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(result));
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
async function typeIn(selector, text) {
  await run(`(() => { const el = ${selector}; el.focus(); })()`);
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
  await sleep(300);
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$card = () => document.querySelector('article.card');
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$query = () => document.querySelector('.graphql .query .cm-content');
  window.$variables = () => document.querySelector('.graphql .variables .cm-content');
  window.$json = () => { try { return JSON.parse(document.querySelector('.response .cm-content')?.innerText ?? ''); } catch { return null; } };
  window.$error = () => document.querySelector('.response .error')?.textContent ?? '';
`;
const results = [];
// The code dialog remembers the last language, so the suite picks cURL itself.
const PICK_CURL = `(async () => { const trigger = document.querySelector('dialog[open]:not(.panel-dialog) button[aria-label="Language"]'); trigger.click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === 'cURL').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 200)); })()`;
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
async function send() {
  await run(`$card().querySelector('.send').click()`);
  await sleep(300);
  await waitFor(`!$card().querySelector('.send').disabled`, 15000);
  await sleep(300);
}

await connect();
try {
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
  );
  const setId = await run(
    `$invoke('get_collection_variable_set', { collectionId: '${collection.id}' })`,
  );
  const views = await run(`$invoke('get_environment_variables', { id: '${setId}' })`);
  const variables = [
    ...views.map((v) => ({
      id: v.id,
      key: v.key,
      value: v.value,
      isSecret: v.isSecret,
      enabled: v.enabled,
    })),
    { id: null, key: 'userId', value: '7', isSecret: false, enabled: true },
  ];
  await run(
    `$invoke('save_environment_variables', { id: '${setId}', variables: ${JSON.stringify(variables)} })`,
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);

  // ---------- the card ----------
  await run(`document.querySelector('.start .tile:nth-child(3), .start .tiles .tile')`);
  await run(
    `[...document.querySelectorAll('.start .tile')].find((t) => t.textContent.includes('GraphQL')).click()`,
  );
  await waitFor(`!!$card()`, 4000);
  await sleep(500);
  const card = await run(
    `({ badge: $card().querySelector('.method.gql')?.textContent.trim(), tab: [...$card().querySelectorAll('[role="tab"]')].map((t) => t.textContent.trim()) })`,
  );
  check(
    'Add GraphQL makes a GQL card with a Query tab',
    card.badge === 'GQL' && card.tab.includes('Query'),
    short(card),
  );
  await run(`$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/graphql')`);
  await run(
    `[...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim() === 'Query').click()`,
  );
  await sleep(400);
  await typeIn(`$query()`, 'query One($id: ID!) { user(id: $id) { name email } }');
  await typeIn(`$variables()`, '{ "id": "{{userId}}" }');
  await sleep(800);
  await send();
  const answer = await run(`$json()`);
  check(
    'variables with {{variables}} go out as GraphQL JSON',
    answer?.data?.user?.name === 'Ada' &&
      received.at(-1)?.body?.variables?.id === '7' &&
      received.at(-1)?.contentType === 'application/json',
    short({ answer, last: received.at(-1) }),
  );

  // ---------- schema, completion, checks ----------
  await run(`$btn('Fetch schema').click()`);
  await waitFor(
    `(document.querySelector('.schema-status')?.textContent ?? '').includes('Schema loaded')`,
    10000,
  );
  const status = await run(`document.querySelector('.schema-status')?.textContent.trim()`);
  check(
    'Fetch schema loads the types',
    /Schema loaded · \d+ types/.test(status) && received.at(-1)?.body?.query?.includes('__schema'),
    status,
  );
  await typeIn(`$query()`, '{ ');
  for (const char of 'us') {
    await cdp('Input.insertText', { text: char });
    await sleep(150);
  }
  await waitFor(`!!document.querySelector('.cm-tooltip-autocomplete')`, 4000);
  const options = await run(
    `[...document.querySelectorAll('.cm-tooltip-autocomplete li')].map((li) => li.textContent.trim())`,
  );
  check(
    'the editor completes fields from the schema',
    options.some((option) => option.startsWith('user')) &&
      options.some((option) => option.startsWith('users')),
    short(options),
  );
  await cdp('Input.dispatchKeyEvent', {
    type: 'rawKeyDown',
    key: 'Escape',
    code: 'Escape',
    windowsVirtualKeyCode: 27,
  });
  await typeIn(`$query()`, '{ users { nope } }');
  await waitFor(`!!document.querySelector('.graphql .query .cm-lintRange-error')`, 5000);
  check(
    'unknown fields are marked as errors',
    await run(`!!document.querySelector('.graphql .query .cm-lintRange-error')`),
  );
  await typeIn(`$query()`, '{ users { id name } }');
  await typeIn(`$variables()`, '{ "id": ');
  await send();
  check(
    'variables that are not JSON are refused',
    (await run(`$error()`)).includes('GraphQL variables are not valid JSON'),
    await run(`$error()`),
  );
  await typeIn(`$variables()`, '');
  await send();
  check(
    'blank variables are left out',
    Array.isArray((await run(`$json()`))?.data?.users) &&
      !('variables' in (received.at(-1)?.body ?? {})),
    short(received.at(-1)?.body),
  );

  // ---------- Copy as code ----------
  await run(`$card().querySelector('button[aria-label="Copy as code"]').click()`);
  await sleep(500);
  await run(PICK_CURL);
  const code = await run(
    `document.querySelector('dialog[open]:not(.panel-dialog) .cm-content')?.innerText ?? ''`,
  );
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => (b.getAttribute('aria-label') ?? b.textContent).trim().startsWith('Close'))?.click()`,
  );
  await sleep(300);
  check(
    'Copy as code sends the query as JSON with POST',
    code.includes('--request POST') && code.includes('"query": "{ users { id name } }"'),
    short(code),
  );

  // ---------- Run all ----------
  await run(`$card().querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(500);
  await sleep(1200);
  const stamp = await run(
    `[...document.querySelectorAll('.row .stamp')].map((s) => s.textContent.trim())`,
  );
  await run(`$btn('Run all').click()`);
  await waitFor(
    `!!document.querySelector('article.result.passed, article.result.failed') && !$btn('Stop')`,
    15000,
  );
  const runDebug = await run(
    `({ view: document.querySelector('main h1')?.textContent.trim(), banner: document.querySelector('.banner')?.textContent.trim() ?? null })`,
  );
  const runRow = await run(
    `[...document.querySelectorAll('article.result')].map((a) => ({ passed: a.classList.contains('passed'), stamp: a.querySelector('.method')?.textContent.trim() }))`,
  );
  check(
    'Run all sends GraphQL requests',
    runRow.length === 1 && runRow[0].passed && runRow[0].stamp === 'GQL' && stamp.includes('GQL'),
    short({ runRow, stamp, runDebug }),
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
  process.exit(0);
}
