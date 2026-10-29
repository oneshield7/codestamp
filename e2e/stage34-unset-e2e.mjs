// A build without an update address or key (the shipped tauri.conf.json) starts normally,
// never shows the update banner and says so in Settings › About.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const { version } = JSON.parse(
  fs.readFileSync(path.join(here, '..', 'src-tauri', 'tauri.conf.json'), 'utf8'),
);
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
  return reply.result.result?.value;
}
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);

await sleep(5000);
check(
  'no update banner without an update server',
  !(await run(`!!document.querySelector('main .update')`)),
);
const answer = await run(`window.__TAURI_INTERNALS__.invoke('check_for_update')`);
check(
  'the check answers notConfigured without a request',
  answer?.kind === 'notConfigured' && answer.current === version,
  JSON.stringify(answer),
);
await run(`document.querySelector('nav.sidebar button.search').click()`);
await sleep(200);
await run(
  `[...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === 'Settings')?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }))`,
);
await sleep(500);
await run(
  `[...document.querySelectorAll('nav[aria-label="Settings sections"] button')].find((b) => b.textContent.trim().startsWith('About')).click()`,
);
await sleep(500);
const about = await run(
  `(() => { const s = [...document.querySelectorAll('section.panel')].find((x) => x.querySelector('h2')?.textContent.trim() === 'Updates'); const a = [...document.querySelectorAll('section.panel')].find((x) => x.querySelector('h2')?.textContent.trim() === 'About Codestamp'); return { updates: s?.textContent.replace(/\\s+/g, ' ').trim(), buttons: s ? s.querySelectorAll('button').length : -1, hint: a?.querySelector('.hint')?.textContent.replace(/\\s+/g, ' ').trim() }; })()`,
);
check(
  'About says this build is not set up, with no buttons',
  about.updates === 'Updates This build of Codestamp is not set up to receive updates.' &&
    about.buttons === 0,
  JSON.stringify(about),
);
check(
  'the privacy note does not mention an update server',
  about.hint?.startsWith('Everything stays on this device; Codestamp sends no usage data. Fonts:'),
  String(about.hint),
);
console.log(results.join('\n'));
socket.close();
