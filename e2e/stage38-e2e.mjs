// End-to-end check of the launch screen (2026-10-28). Each reload runs the launch again: the theme
// must be on the page before the app's code runs, the screen must use the theme's colours, stay
// at least 0.5 s, hand over smoothly, and leave the app with no transform behind. Launches the app.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const shots = path.join(here, '.work', 'screens');
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const BACKGROUNDS = {
  light: [0xf3, 0xf4, 0xf6],
  dark: [0x15, 0x17, 0x1c],
  night: [0x07, 0x0a, 0x12],
};

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
}
function cdp(method, params = {}) {
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
    await sleep(50);
  }
  return false;
}
async function shot(name) {
  const image = await cdp('Page.captureScreenshot', { format: 'png' });
  const file = path.join(shots, name);
  fs.writeFileSync(file, Buffer.from(image.result.data, 'base64'));
  return file;
}
/** The colour at a point of a screenshot, read with Python's PIL. */
function pixel(file, x, y) {
  const out = execFileSync(
    'python',
    [
      '-c',
      `from PIL import Image; print(*Image.open(r'${file}').convert('RGB').getpixel((${x}, ${y})))`,
    ],
    { encoding: 'utf8' },
  );
  return out.trim().split(' ').map(Number);
}
const invoke = (command, args) =>
  run(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)}).then((v) => ({ ok: v }), (e) => ({ error: e?.message ?? String(e) }))`,
  );
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);

// Recorded in every new document: the theme before the app's module runs, when the hand-over
// starts, and the frames drawn during it.
const RECORDER = `
  window.__launch = { frames: [] };
  document.addEventListener('readystatechange', () => {
    if (document.readyState === 'interactive') window.__launch.themeAtParse = document.documentElement.dataset.theme ?? null;
  });
  new MutationObserver(() => {
    const root = document.documentElement;
    if (!root || window.__launch.handedOverAt || !root.classList.contains('launched')) return;
    window.__launch.handedOverAt = performance.now();
    const end = performance.now() + 600;
    const frame = (time) => { window.__launch.frames.push(time); if (time < end) requestAnimationFrame(frame); };
    requestAnimationFrame(frame);
  }).observe(document, { subtree: true, attributes: true, attributeFilter: ['class'], childList: true });
`;

async function relaunch() {
  await cdp('Page.reload', {});
  await waitFor(`!!document.getElementById('launch')`, 4000);
}

execFileSync(process.execPath, [path.join(here, 'launch.mjs')], { stdio: 'inherit' });
await connect();
await cdp('Page.enable');
await cdp('Page.addScriptToEvaluateOnNewDocument', { source: RECORDER });
const original = (await invoke('get_appearance', {})).ok;
const appearance = original.appearance ?? original;
try {
  const settled = await run(
    `(() => { const app = getComputedStyle(document.getElementById('app')); return { screen: !!document.getElementById('launch'), launched: document.documentElement.classList.contains('launched'), opacity: app.opacity, transform: app.transform, filter: app.filter }; })()`,
  );
  // An inline <style> in index.html once put a hash in the CSP, which made the browser drop every
  // style injected later: CodeMirror's editors lost their layout.
  const injected = await run(`(() => {
    const probe = document.createElement('div');
    probe.className = 'csp-probe';
    probe.innerHTML = '<span style="display:inline-block;width:37px"></span>';
    const sheet = document.createElement('style');
    sheet.textContent = '.csp-probe { width: 41px; }';
    document.head.append(sheet);
    document.body.append(probe);
    const result = { sheet: getComputedStyle(probe).width, attribute: getComputedStyle(probe.firstChild).width };
    probe.remove();
    sheet.remove();
    return result;
  })()`);
  check(
    'styles injected at runtime still apply (CodeMirror, style attributes)',
    injected.sheet === '41px' && injected.attribute === '37px',
    short(injected),
  );
  check(
    'after launch the screen is gone and the app is left with no transform',
    !settled.screen &&
      settled.launched &&
      settled.opacity === '1' &&
      settled.transform === 'none' &&
      settled.filter === 'none',
    short(settled),
  );

  for (const theme of ['light', 'dark', 'night']) {
    // As Settings › Appearance does: the theme is saved and kept for a reload of this window.
    await invoke('save_appearance', { appearance: { ...appearance, theme } });
    // The window's light or dark mode follows the saved theme; the app re-applies its own theme on
    // that change, so the test writes after it, as the app itself would have the new theme by then.
    await waitFor(
      `matchMedia('(prefers-color-scheme: dark)').matches === ${theme !== 'light'}`,
      3000,
    );
    await sleep(200);
    await run(`sessionStorage.setItem('codestamp.theme', '${theme}')`);
    await relaunch();
    await sleep(120);
    const early = await shot(`stage38-${theme}-launch.png`);
    const state = await run(
      `({ theme: window.__launch.themeAtParse, screen: !!document.getElementById('launch') })`,
    );
    const corner = pixel(early, 20, 60);
    const centre = await run(
      `(() => { const r = document.querySelector('.launch-mark')?.getBoundingClientRect(); return r && [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2), innerWidth / 2, innerHeight / 2]; })()`,
    );
    check(
      `${theme}: the theme is on the page before the app's code runs`,
      state.theme === theme && state.screen,
      short(state),
    );
    check(
      `${theme}: the launch screen is drawn in the theme's background`,
      JSON.stringify(corner) === JSON.stringify(BACKGROUNDS[theme]),
      short({ corner, want: BACKGROUNDS[theme] }),
    );
    check(
      `${theme}: the logo sits in the middle of the window`,
      centre && Math.abs(centre[0] - centre[2]) <= 2 && Math.abs(centre[1] - centre[3]) <= 2,
      short(centre),
    );
    await waitFor(`!document.getElementById('launch')`, 6000);
    check(
      `${theme}: the app keeps the theme for the next reload`,
      (await run(`sessionStorage.getItem('codestamp.theme')`)) === theme,
    );
    const timing = await run(
      `({ at: Math.round(window.__launch.handedOverAt), frames: window.__launch.frames.length, worst: Math.round(Math.max(...window.__launch.frames.slice(1).map((t, i) => t - window.__launch.frames[i]))) })`,
    );
    check(`${theme}: the screen stays at least 0.5 s`, timing.at >= 500, short(timing));
    check(
      `${theme}: the hand-over draws smoothly`,
      timing.frames >= 20 && timing.worst <= 50,
      short(timing),
    );
  }

  // Mid-transition picture, for the eye.
  await invoke('save_appearance', { appearance: { ...appearance, theme: 'dark' } });
  await relaunch();
  await waitFor(`document.documentElement.classList.contains('launched')`, 4000);
  await sleep(140);
  await shot('stage38-handover.png');
  await waitFor(`!document.getElementById('launch')`, 6000);

  // With reduced motion the trace stands still and the app only fades in.
  await cdp('Emulation.setEmulatedMedia', {
    features: [{ name: 'prefers-reduced-motion', value: 'reduce' }],
  });
  await relaunch();
  await sleep(120);
  const still = await run(
    `getComputedStyle(document.querySelector('.launch-trace')).animationName`,
  );
  await waitFor(`document.documentElement.classList.contains('launched')`, 4000);
  const fade = await run(`getComputedStyle(document.getElementById('app')).animationName`);
  check(
    'reduced motion: the trace stands still and the app only fades in',
    still === 'none' && fade === 'launch-fade-in',
    short({ still, fade }),
  );
  await waitFor(`!document.getElementById('launch')`, 6000);
  await cdp('Emulation.setEmulatedMedia', { features: [] });
} finally {
  await invoke('save_appearance', { appearance });
  socket.close();
}
console.log(results.join('\n'));
console.log(`${results.filter((line) => line.startsWith('PASS')).length}/${results.length} passed`);
