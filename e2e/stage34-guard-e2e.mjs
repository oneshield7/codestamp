// Unsaved collection variables still stop leaving the collection after the leave-guard change.
const CDP = 'http://127.0.0.1:9333';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const pages = await (await fetch(`${CDP}/json`)).json();
const socket = new WebSocket(pages.find((p) => p.type === 'page').webSocketDebuggerUrl);
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
    throw new Error(
      `${expression.slice(0, 80)} → ${reply.result.exceptionDetails.exception?.description}`,
    );
  return reply.result.result?.value;
}
const invoke = (command, args) =>
  run(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)})`);
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const NAME = 'E2E Guard';
let id = null;
// Collections sit in the sidebar; Cookies and the other data pages sit in the status bar.
const nav = (text) =>
  `(document.querySelector('footer.statusbar button[aria-label=' + ${JSON.stringify(JSON.stringify(text))} + ']') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === ${JSON.stringify(text)}))`;
const panel = `document.querySelector('section[aria-label="Collection variables"]')`;
try {
  id = (await invoke('create_collection', { name: NAME, icon: null, baseUrl: null })).id;
  await run('location.reload()');
  await sleep(1800);
  await run(`${nav(NAME)}.click()`);
  await sleep(800);
  await run(
    `[...document.querySelectorAll('button')].find((b) => (b.getAttribute('aria-label') ?? b.textContent.trim()).startsWith('Variables')).click()`,
  );
  await sleep(500);
  await run(
    `(() => { const i = ${panel}.querySelector('input[aria-label="Variable name"]'); i.value = 'pending'; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(300);
  await run(`${nav('Cookies')}.click()`);
  await sleep(500);
  const state = await run(
    `({ banner: document.querySelector('main .banner')?.textContent.replace(/\\s+/g, ' ').trim(), panel: !!${panel}, title: document.querySelector('input[aria-label="Collection name"]')?.value })`,
  );
  check(
    'unsaved collection variables still stop leaving, with the same message',
    state.panel &&
      state.title === NAME &&
      /^Save or close collection variables before leaving this collection\./.test(
        state.banner ?? '',
      ),
    JSON.stringify(state),
  );
  await run(`document.querySelector('main .banner button')?.click()`);
  await run(`${panel}.querySelector('input[aria-label="Variable name"]').value = ''`);
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  if (id) await invoke('delete_collection', { id });
  await run('location.reload()');
  await sleep(1500);
  const left = await invoke('list_collections', {});
  check('test data cleaned up', !left.some((c) => c.id === id));
  console.log(results.join('\n'));
  socket.close();
}
