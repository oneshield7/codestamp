// End-to-end check of stage 15g: Server-Sent Events. A text/event-stream response is shown
// event by event, outlives the request timeout, can be stopped, and does not stall Run all.
// Creates "E2E Events" and deletes it; puts the request timeout back as it was.
import http from 'node:http';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Events';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

let foreverClosed = false;
const server = http.createServer((req, res) => {
  if (req.url === '/events') {
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' });
    res.write(': welcome\n\n');
    let n = 0;
    const timer = setInterval(() => {
      n += 1;
      res.write(`event: tick\nid: ${n}\ndata: {"n":${n},"ok":true}\n\n`);
      if (n === 5) {
        clearInterval(timer);
        res.end();
      }
    }, 150);
    return;
  }
  if (req.url === '/forever') {
    res.writeHead(200, { 'Content-Type': 'text/event-stream' });
    let n = 0;
    const timer = setInterval(() => res.write(`data: beat ${++n}\n\n`), 100);
    req.on('close', () => {
      clearInterval(timer);
      foreverClosed = true;
    });
    return;
  }
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end('{"ok":true}');
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
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$card = () => document.querySelector('article.card');
  window.$row = (text) => [...document.querySelectorAll('.row')].find((r) => r.textContent.includes(text));
  window.$streamHead = () => document.querySelector('.response .stream-head')?.textContent.replace(/\\s+/g, ' ').trim() ?? '';
  window.$events = () => [...document.querySelectorAll('.response .events li:not(.empty)')].map((li) => ({ name: li.querySelector('.event-name')?.textContent ?? null, id: li.querySelector('.event-id')?.textContent ?? null, data: li.querySelector('.event-data')?.textContent ?? '' }));
  window.$bodyTab = () => [...document.querySelectorAll('.response [role="tab"]')][0]?.textContent.trim();
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const content = (name, url) => ({
  kind: 'http',
  name,
  method: 'GET',
  url,
  params: [],
  headers: [],
  bodyMode: 'none',
  body: '',
  form: [],
  bodyFile: '',
  preRequestScript: '',
  testScript: "pm.test('t', () => {});",
  auth: { type: 'inherit' },
  mcpOperation: null,
  pathVariables: [],
  graphqlVariables: '',
});

await connect();
const preferences = await run(`$invoke('get_preferences')`);
try {
  // A one-second timeout: the endless stream must still run past it.
  await run(
    `$invoke('save_preferences', { preferences: ${JSON.stringify({ ...preferences, requests: { ...preferences.requests, timeoutSeconds: 1 } })} })`,
  );
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
  );
  for (const [name, path] of [
    ['Ticks', '/events'],
    ['Forever', '/forever'],
  ]) {
    await run(
      `$invoke('create_request', { collectionId: '${collection.id}', sectionId: null, content: ${JSON.stringify(content(name, `{{baseUrl}}${path}`))} })`,
    );
  }
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);

  // ---------- a finite stream ----------
  await run(`$row('Ticks').click()`);
  await sleep(700);
  await run(`$card().querySelector('.send').click()`);
  await waitFor(`$streamHead().startsWith('Streaming')`, 5000);
  const live = await run(`({ head: $streamHead(), tab: $bodyTab() })`);
  await waitFor(`$streamHead().startsWith('Stream ended')`, 8000);
  const ended = await run(
    `({ head: $streamHead(), tab: $bodyTab(), events: $events(), console: [...document.querySelectorAll('.response [role="tab"]')].map((t) => t.textContent.trim()) })`,
  );
  check(
    'events show as they arrive, under an Events tab',
    live.head.startsWith('Streaming') &&
      ended.tab === 'Events · 5' &&
      ended.head === 'Stream ended · 5 events',
    short({ live, head: ended.head, tab: ended.tab }),
  );
  check(
    'named events, ids and JSON data are shown',
    ended.events.length === 5 &&
      ended.events[0].name === 'tick' &&
      ended.events[4].id === 'id 5' &&
      ended.events[0].data.includes('"n": 1'),
    short(ended.events.slice(0, 2)),
  );
  check(
    'the test script waits out event streams',
    ended.console.some((t) => t.startsWith('Console · 1')),
    short(ended.console),
  );

  // ---------- an endless stream, past the timeout, stopped ----------
  await run(`$card().querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(400);
  await run(`$row('Forever').click()`);
  await sleep(700);
  await run(`$card().querySelector('.send').click()`);
  await sleep(2500);
  const running = await run(`({ head: $streamHead(), count: $events().length })`);
  check(
    'a stream outlives the request timeout',
    running.head.startsWith('Streaming') && running.count >= 15,
    short(running),
  );
  await run(`$btn('Stop', document.querySelector('.response .stream-head')).click()`);
  await waitFor(`$streamHead().startsWith('Stream ended')`, 3000);
  await sleep(500);
  check(
    'Stop ends the stream and closes the connection',
    (await run(`$streamHead()`)).startsWith('Stream ended') && foreverClosed,
    short({ head: await run(`$streamHead()`), foreverClosed }),
  );
  await run(`$card().querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(400);

  // ---------- Run all ----------
  foreverClosed = false;
  await run(`$btn('Run all').click()`);
  await waitFor(
    `document.querySelectorAll('article.result.passed, article.result.failed').length === 2 && !$btn('Stop')`,
    15000,
  );
  await sleep(500);
  const rows = await run(
    `[...document.querySelectorAll('article.result')].map((a) => ({ name: a.querySelector('.name')?.textContent.trim(), note: a.querySelector('.meta')?.textContent.trim() }))`,
  );
  check(
    'Run all does not wait for streams and closes them',
    rows.length === 2 &&
      rows.every((row) => row.note === 'Event stream: stopped once its headers arrived') &&
      foreverClosed,
    short({ rows, foreverClosed }),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    await run(`$invoke('save_preferences', { preferences: ${JSON.stringify(preferences)} })`);
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
    const restored = await run(`$invoke('get_preferences').then((p) => p.requests.timeoutSeconds)`);
    check(
      'test data cleaned up and the timeout put back',
      !(await run(`!!$nav(${JSON.stringify(NAME)})`)) &&
        restored === preferences.requests.timeoutSeconds,
      String(restored),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  server.close();
  process.exit(0);
}
