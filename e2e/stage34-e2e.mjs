// End-to-end check of in-app updates against a local update server. `node updater/prepare.mjs`
// makes .work/updater: test.conf.json (a throwaway public key, http://127.0.0.1:9455, plain http
// allowed), fake-installer.exe, which only writes down its arguments, signed for the next version,
// and release.json with the versions this suite announces. The test copy is built with that
// config.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const fake = path.join(here, '.work', 'updater');
const installerLog = path.join(fake, 'installer-log.txt');
const installer = fs.readFileSync(path.join(fake, 'fake-installer.exe'));
const signature = fs.readFileSync(path.join(fake, 'fake-installer.exe.sig'), 'utf8').trim();
// `current` is the app's version, `next` the one the installer is signed for, `other` a third.
const versions = JSON.parse(fs.readFileSync(path.join(fake, 'release.json'), 'utf8'));
const literal = (text) => text.replaceAll('.', '\\.');
const CDP = 'http://127.0.0.1:9333';
const PORT = 9455;
const NOTES = 'Faster sync.\nA new updater.';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// What the server hands out; the test changes it between steps.
const release = { version: versions.next, tamper: false };
const seen = [];
function serve(request, response) {
  seen.push({ url: request.url, agent: request.headers['user-agent'] ?? '' });
  if (request.url === '/latest.json') {
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end(
      JSON.stringify({
        version: release.version,
        notes: NOTES,
        pub_date: '2026-10-27T09:00:00Z',
        platforms: {
          'windows-x86_64': { signature, url: `http://127.0.0.1:${PORT}/Codestamp_x64-setup.exe` },
        },
      }),
    );
    return;
  }
  if (request.url === '/Codestamp_x64-setup.exe') {
    const body = Buffer.from(installer);
    if (release.tamper) body[body.length - 10] ^= 0xff;
    response.writeHead(200, {
      'content-type': 'application/octet-stream',
      'content-length': body.length,
    });
    // Sent in slices so the download progress can be seen.
    let offset = 0;
    const timer = setInterval(() => {
      response.write(body.subarray(offset, offset + 16384));
      offset += 16384;
      if (offset >= body.length) {
        clearInterval(timer);
        response.end();
      }
    }, 120);
    return;
  }
  response.writeHead(404).end();
}
let server;
const startServer = () =>
  new Promise((resolve) => {
    server = http.createServer(serve).listen(PORT, '127.0.0.1', resolve);
  });
const stopServer = () =>
  new Promise((resolve) => {
    server.closeAllConnections();
    server.close(resolve);
  });

let socket;
let nextId = 1;
const pending = new Map();
let closed = false;
async function connect() {
  const pages = await (await fetch(`${CDP}/json`)).json();
  socket = new WebSocket(pages.find((p) => p.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve) => socket.addEventListener('open', resolve));
  socket.addEventListener('close', () => {
    closed = true;
  });
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
async function waitFor(expression, timeout = 15000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(200);
  }
  return false;
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim() === text);
  window.$banner = () => document.querySelector('main .update');
  window.$updates = () => [...document.querySelectorAll('section.panel')].find((s) => s.querySelector('h2')?.textContent.trim() === 'Updates');
  window.$text = (el) => el?.textContent.replace(/\\s+/g, ' ').trim() ?? null;
  window.$goTo = async (text) => { document.querySelector('nav.sidebar button.search').click(); await new Promise((r) => setTimeout(r, 150)); [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); };
  window.$section = (label) => [...document.querySelectorAll('nav[aria-label="Settings sections"] button')].find((b) => b.textContent.trim().startsWith(label));
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 260);
const logLines = () =>
  fs.existsSync(installerLog)
    ? fs.readFileSync(installerLog, 'utf8').trim().split('\n').filter(Boolean)
    : [];
async function reload() {
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
}
async function openAbout() {
  await run(`$goTo('Settings')`);
  await waitFor(`!!$section('About')`);
  await run(`$section('About').click()`);
  await waitFor(`!!$updates()`);
}

fs.rmSync(installerLog, { force: true });
await startServer();
execFileSync(process.execPath, [path.join(here, 'launch.mjs')], {
  stdio: 'inherit',
  env: { ...process.env, CODESTAMP_FAKE_INSTALLER_LOG: installerLog },
});
await connect();
try {
  // ---------- the check at start-up ----------
  const shown = await waitFor(
    `($text($banner()) ?? '').includes('Codestamp ${versions.next} is available. You have ${versions.current}.')`,
    15000,
  );
  check(
    'a newer version shows a banner a few seconds after start',
    shown,
    await run(`$text($banner())`),
  );
  check(
    'the check reads latest.json as the Codestamp updater',
    seen.some((s) => s.url === '/latest.json' && /tauri-plugin-updater/.test(s.agent)),
    short(seen),
  );
  check(
    "the banner offers What's new, Update and restart, and Later",
    short(
      await run(`[...$banner().querySelectorAll('button')].map((b) => b.textContent.trim())`),
    ) === short(["What's new", 'Update and restart', 'Later']),
  );

  // ---------- release notes in Settings › About ----------
  await run(`$btn("What's new", $banner()).click()`);
  await waitFor(`!!$updates()`);
  const about = await run(
    `({ status: $text($updates().querySelector('.status, .news')), notes: $updates().querySelector('.notes')?.textContent, buttons: [...$updates().querySelectorAll('button')].map((b) => b.textContent.trim()) })`,
  );
  check(
    "What's new opens Settings › About with the notes as plain text",
    about.notes === NOTES &&
      about.status.startsWith(
        `Codestamp ${versions.next} is available. You have ${versions.current}. Published `,
      ) &&
      short(about.buttons) === short(['Update and restart', 'Check for updates']),
    short(about),
  );

  // ---------- Later ----------
  await run(`$btn('Later', $banner()).click()`);
  await sleep(200);
  check(
    'Later hides the banner but Settings still offers the update',
    !(await run(`!!$banner()`)) && !!(await run(`!!$btn('Update and restart', $updates())`)),
  );

  // ---------- unsaved environment edits stop the update ----------
  await run(`document.querySelector('footer.statusbar button[aria-label="Environments"]').click()`);
  await waitFor(
    `!!document.querySelector('.layout section.panel input[aria-label="Variable name"]')`,
  );
  await run(
    `(() => { const i = document.querySelector('.layout section.panel input[aria-label="Variable name"]'); i.value = 'unsavedForUpdate'; i.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(200);
  await openAbout();
  await run(`$btn('Update and restart', $updates()).click()`);
  await sleep(800);
  const blocked = await run(`$text($updates().querySelector('.error'))`);
  check(
    'unsaved environment changes stop the update with a reason',
    blocked === 'Save or discard your environment changes first, then update.' &&
      logLines().length === 0,
    String(blocked),
  );
  await reload();

  // ---------- a tampered download is refused ----------
  await waitFor(`!!$banner()`, 15000);
  release.tamper = true;
  await run(`$btn('Update and restart', $banner()).click()`);
  await waitFor(`$banner()?.getAttribute('role') === 'alert'`, 20000);
  const tampered = await run(`$text($banner())`);
  check(
    'a download that fails the signature check is not installed',
    /failed its signature check, so it was not installed/.test(tampered) &&
      logLines().length === 0 &&
      !closed,
    tampered,
  );
  check('the failed update offers Try again', !!(await run(`!!$btn('Try again', $banner())`)));
  release.tamper = false;

  // ---------- a release file announcing another version than the signed one ----------
  release.version = versions.other;
  await openAbout();
  await run(`$btn('Check for updates', $updates()).click()`);
  await waitFor(
    `($text($updates().querySelector('.status, .news')) ?? '').includes('${versions.other} is available')`,
  );
  await run(`$btn('Update and restart', $updates()).click()`);
  await waitFor(`!!$updates().querySelector('.error')`, 20000);
  const mismatch = await run(`$text($updates().querySelector('.error'))`);
  check(
    'an installer signed for another version is refused',
    /signature check/.test(mismatch) && logLines().length === 0,
    mismatch,
  );

  // ---------- up to date ----------
  release.version = versions.current;
  await run(`$btn('Check for updates', $updates()).click()`);
  await waitFor(`/latest version/.test($text($updates().querySelector('.status, .news')))`);
  check(
    'the same version reads as up to date and clears the old failure',
    (await run(`$text($updates().querySelector('.status, .news'))`)) ===
      `You have the latest version, ${versions.current}.` &&
      !(await run(`!!$banner()`)) &&
      !(await run(`!!$btn('Update and restart', $updates())`)) &&
      !(await run(`!!$updates().querySelector('.error')`)),
  );

  // ---------- server down ----------
  await stopServer();
  await run(`$btn('Check for updates', $updates()).click()`);
  await waitFor(`/^Could not check/.test($text($updates().querySelector('.error')))`, 25000);
  const down = await run(`$text($updates().querySelector('.error'))`);
  check(
    'an unreachable server gives a short reason',
    /^Could not check for updates\. (Could not connect: the update server did not answer|The request timed out)$/.test(
      down,
    ),
    down,
  );
  check(
    'Check for updates is ready again',
    await waitFor(`$btn('Check for updates', $updates())?.disabled === false`, 5000),
  );

  // ---------- update and restart ----------
  release.version = versions.next;
  await startServer();
  await reload();
  await waitFor(`!!$banner() && !!$btn('Update and restart', $banner())`, 15000);
  await run(`$btn('Update and restart', $banner()).click()`);
  let progressText = null;
  for (let i = 0; i < 40 && !closed; i++) {
    const text = await run(`$text($banner())`).catch(() => null);
    if (text && new RegExp(`^Downloading Codestamp ${literal(versions.next)}: \\d+%`).test(text)) {
      progressText = text;
      break;
    }
    await sleep(100);
  }
  check('the banner shows download progress', !!progressText, String(progressText));
  const deadline = Date.now() + 20000;
  while (!closed && Date.now() < deadline) await sleep(250);
  await sleep(1500);
  const lines = logLines();
  check(
    'Codestamp closes and starts the installer in update mode',
    closed && lines.length === 1 && /^\/P \/UPDATE \/R \/ARGS\s*$/.test(lines[0]),
    short({ closed, lines }),
  );
  const running = execFileSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-Command',
      `. '${path.join(here, 'test-process.ps1')}'; @(Get-TestProcess).Count`,
    ],
    { encoding: 'utf8' },
  ).trim();
  check('the test copy is no longer running', running === '0', running);
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  console.log(results.join('\n'));
  if (!closed) socket.close();
  if (server.listening) await stopServer();
  process.exit(0);
}
