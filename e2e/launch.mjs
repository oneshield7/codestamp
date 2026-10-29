// Starts a Codestamp build with CDP on port 9333 and waits until the workspace has loaded.
// Usage: node launch.mjs [path-to-exe]; without one, the test build in .work/target, kept apart
// from src-tauri/target, which is locked while a Codestamp started from the project runs.
// A Codestamp that is already open holds the default WebView2 folder, so the test copy gets a
// WebView2 folder of its own in .work.
// A test copy already running is stopped first (found by its path, never the user's). E2E_SIZE
// (minimum or maximize) resizes the new window, so a sweep keeps its size across relaunches.
import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const exe =
  process.argv[2] ?? path.join(here, '.work', 'target', 'release', 'codestamp-desktop.exe');
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const powershell = (args) =>
  execFileSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', ...args], {
    encoding: 'utf8',
  });
for (const folder of ['git-hosts', 'screens'])
  fs.mkdirSync(path.join(here, '.work', folder), { recursive: true });

powershell([
  '-Command',
  `. '${path.join(here, 'test-process.ps1')}'; Get-TestProcess | Stop-Process -Force`,
]);
await sleep(1200);
const gitConfig = (pairs) => ({
  GIT_CONFIG_COUNT: String(pairs.length),
  ...Object.fromEntries(
    pairs.flatMap(([key, value], index) => [
      [`GIT_CONFIG_KEY_${index}`, key],
      [`GIT_CONFIG_VALUE_${index}`, value],
    ]),
  ),
});

const child = spawn(exe, [], {
  detached: true,
  stdio: 'ignore',
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9333',
    WEBVIEW2_USER_DATA_FOLDER: path.join(here, '.work', 'webview2-e2e'),
    // Git in the test copy maps https://git.test/… to bare repositories in .work/git-hosts, so the
    // https-only account path runs without a real host. The global Git config is untouched.
    // E2E_GIT_CONFIG adds more [key, value] pairs for one suite.
    ...gitConfig([
      [
        `url.${path.join(here, '.work', 'git-hosts').replaceAll('\\', '/')}/.insteadOf`,
        'https://git.test/',
      ],
      ...JSON.parse(process.env.E2E_GIT_CONFIG ?? '[]'),
    ]),
  },
});
child.unref();

let page;
for (let i = 0; i < 120 && !page; i++) {
  await sleep(250);
  try {
    page = (await (await fetch('http://127.0.0.1:9333/json')).json()).find(
      (p) => p.type === 'page',
    );
  } catch {
    // CDP is not listening yet.
  }
}
if (!page) throw new Error('CDP did not come up');

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
    JSON.stringify({ id, method: 'Runtime.evaluate', params: { expression, returnByValue: true } }),
  );
  const reply = await new Promise((resolve) => pending.set(id, resolve));
  return reply.result.result?.value;
}
let ready = false;
for (let i = 0; i < 60 && !ready; i++) {
  await sleep(250);
  ready = await run(
    `!!document.querySelector('nav button.new') && !document.getElementById('launch')`,
  );
}
await sleep(1000);
if (process.env.E2E_SIZE) {
  const size =
    process.env.E2E_SIZE === 'maximize' ? ['-Maximize'] : ['-Width', '720', '-Height', '520'];
  powershell(['-File', path.join(here, 'resize-window.ps1'), ...size]);
  await sleep(800);
}
console.log(
  'pid',
  child.pid,
  'ready',
  ready,
  'viewport',
  await run(`[innerWidth, innerHeight].join('x')`),
);
socket.close();
