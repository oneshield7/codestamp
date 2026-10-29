// End-to-end check of the collections flow through the real Codestamp window (WebView2 CDP).
import http from 'node:http';

const CDP = 'http://127.0.0.1:9333';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ ok: true, path: req.url }));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const target = `http://127.0.0.1:${server.address().port}/e2e`;

const pages = await (await fetch(`${CDP}/json`)).json();
const page = pages.find((p) => p.type === 'page');
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve) => socket.addEventListener('open', resolve));
let nextId = 1;
const pending = new Map();
socket.addEventListener('message', (event) => {
  const message = JSON.parse(event.data);
  pending.get(message.id)?.(message);
});

async function run(expression) {
  const id = nextId++;
  socket.send(
    JSON.stringify({
      id,
      method: 'Runtime.evaluate',
      params: { expression, awaitPromise: true, returnByValue: true },
    }),
  );
  const reply = await new Promise((resolve) => pending.set(id, resolve));
  if (reply.result.exceptionDetails)
    throw new Error(reply.result.exceptionDetails.exception?.description ?? 'eval failed');
  return reply.result.result.value;
}

const helpers = `
  window.$btn = (text) => [...document.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);

await run(helpers);
const before = await run(`document.querySelectorAll('nav.sidebar .item:not(.new)').length`);

await run(`(async () => {
    const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    document.querySelector('nav button.new').click();
    await wait(400);
    const input = document.getElementById('new-collection-name');
    input.value = 'E2E temp';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await wait(300);
    [...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Create collection').click();
    await wait(900);
  })()`);
await sleep(500);
await run(
  `(() => { const t = document.querySelector('input[aria-label="Collection name"]'); $set(t, 'E2E test'); t.dispatchEvent(new Event('blur')); })()`,
);
await sleep(400);
check(
  'collection created and renamed',
  await run(
    `[...document.querySelectorAll('nav .item')].some((b) => b.textContent.includes('E2E test'))`,
  ),
);

await run(`(document.querySelector('.adder') ?? document.querySelector('.start .tile')).click()`);
await sleep(500);
check('new request opens as a card', await run(`!!document.querySelector('article.card')`));

await run(`$set(document.querySelector('article.card .name input'), 'Ping local server')`);
await run(`$set(document.querySelector('input[id^="url-"]'), '${target}')`);
await sleep(1200);
check(
  'autosave finishes',
  (await run(`document.querySelector('.save').textContent.trim()`)) === 'Saved',
);

await run(`document.querySelector('.send').click()`);
await sleep(1000);
const chip = await run(`document.querySelector('.response .chip')?.textContent.trim()`);
check('send returns 200 from local server', chip?.startsWith('200'), chip);
const bodyText = await run(`document.querySelector('.response .cm-content')?.innerText ?? ''`);
check('response body shown and formatted', bodyText.includes('"path": "/e2e"'));

await run(`document.querySelector('button[aria-label="Collapse request"]').click()`);
await sleep(400);
const rowChip = await run(`document.querySelector('button.row .chip')?.textContent.trim()`);
check('collapsed row keeps last status', rowChip?.startsWith('200'), rowChip);

await run(`location.reload()`);
await sleep(2000);
await run(helpers);
await run(
  `[...document.querySelectorAll('nav .item')].find((b) => b.textContent.includes('E2E test')).click()`,
);
await sleep(500);
await run(`document.querySelector('button.row').click()`);
await sleep(500);
const reloaded = await run(
  `({ name: document.querySelector('article.card .name input').value, url: document.querySelector('input[id^="url-"]').value })`,
);
check(
  'request persists after reload',
  reloaded.name === 'Ping local server' && reloaded.url === target,
  JSON.stringify(reloaded),
);

await run(`$btn('Delete collection').click()`);
await sleep(300);
await run(
  `[...document.querySelectorAll('dialog button')].find((b) => b.textContent.trim() === 'Delete').click()`,
);
await sleep(600);
const after = await run(`document.querySelectorAll('nav.sidebar .item:not(.new)').length`);
check('test collection cleaned up', after === before, `${before} -> ${after}`);

console.log(results.join('\n'));
socket.close();
server.close();
