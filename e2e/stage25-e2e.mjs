// End-to-end check of the custom title bar: window buttons, double-click to maximize, resize
// edges, the sidebar toggle to the icon rail (button, Ctrl+B, remembered, locked in focus mode
// and in narrow windows) and, when the
// desktop is unlocked, the native application menu. Creates "E2E Titlebar" and deletes it.
// Closing the window is checked last, so run this suite on its own.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Titlebar';
const HERE = path.dirname(fileURLToPath(import.meta.url));
const SHOTS = path.join(HERE, '.work', 'screens');
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

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
async function waitFor(expression, timeout = 5000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(150);
  }
  return false;
}
async function key(keyName, code, vk, modifiers = 0) {
  await cdp('Input.dispatchKeyEvent', {
    type: 'rawKeyDown',
    key: keyName,
    code,
    windowsVirtualKeyCode: vk,
    modifiers,
  });
  await cdp('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: keyName,
    code,
    windowsVirtualKeyCode: vk,
    modifiers,
  });
  await sleep(300);
}
async function center(selector) {
  return run(
    `(() => { const r = ${selector}.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`,
  );
}
async function click(selector, clickCount = 1) {
  const point = await center(selector);
  for (let count = 1; count <= clickCount; count++) {
    await cdp('Input.dispatchMouseEvent', {
      type: 'mousePressed',
      x: point.x,
      y: point.y,
      button: 'left',
      clickCount: count,
    });
    await cdp('Input.dispatchMouseEvent', {
      type: 'mouseReleased',
      x: point.x,
      y: point.y,
      button: 'left',
      clickCount: count,
    });
  }
  await sleep(500);
}
function powershell(script, args = []) {
  const result = spawnSync(
    'powershell',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(HERE, script), ...args],
    { encoding: 'utf8' },
  );
  return result.stdout;
}
function windowState() {
  const out = powershell('hit-test.ps1');
  return { zoomed: /zoomed: True/.test(out), minimized: /minimized: True/.test(out), out };
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$bar = (label) => [...document.querySelectorAll('header.titlebar button')].find((b) => b.getAttribute('aria-label').startsWith(label));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav.sidebar .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$card = () => document.querySelector('article.card');
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ': ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);

await connect();
let collectionId = null;
try {
  // ---------- the bar ----------
  const bar = await run(`({
    labels: [...document.querySelectorAll('header.titlebar button')].map((b) => b.getAttribute('aria-label')),
    height: Math.round(document.querySelector('header.titlebar').getBoundingClientRect().height),
    sidebarTop: Math.round(document.getElementById('sidebar').getBoundingClientRect().top),
    drag: document.querySelectorAll('[data-tauri-drag-region]').length,
  })`);
  check(
    'the title bar has the menu, the sidebar toggle and the window buttons',
    short(bar.labels) === short(['Menu', 'Collapse sidebar', 'Minimize', 'Maximize', 'Close']) &&
      bar.height === 36 &&
      bar.sidebarTop === 36 &&
      bar.drag === 2,
    short(bar),
  );

  // ---------- sidebar toggle ----------
  const railState = `({
    rail: document.getElementById('sidebar')?.classList.contains('rail') ?? null,
    width: Math.round(document.getElementById('sidebar')?.getBoundingClientRect().width ?? 0),
    icons: [...document.querySelectorAll('#sidebar .item')].filter((item) => item.getBoundingClientRect().width > 0).length,
    stored: localStorage.getItem('codestamp.sidebarCollapsed'),
    mainLeft: Math.round(document.querySelector('main').getBoundingClientRect().left),
  })`;
  await click(`$bar('Collapse sidebar')`);
  const collapsed = await run(railState);
  const expandLabel = await run(`$bar('Expand sidebar')?.getAttribute('aria-label') ?? null`);
  check(
    'the toggle collapses the sidebar to the icon rail, navigation still in view',
    collapsed.rail === true &&
      collapsed.width === 64 &&
      collapsed.icons >= 3 &&
      collapsed.stored === '1' &&
      collapsed.mainLeft === 64 &&
      expandLabel === 'Expand sidebar',
    short({ collapsed, expandLabel }),
  );
  await run(`$nav('Catalog').click()`);
  await sleep(700);
  check(
    'a rail icon still opens its page',
    (await run(`document.querySelector('nav.sidebar .item.on .name')?.textContent.trim()`)) ===
      'Catalog',
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  check(
    'a collapsed sidebar stays collapsed after a restart',
    (await run(railState)).rail === true,
  );
  await run(`document.activeElement?.blur()`);
  await key('b', 'KeyB', 66, 2);
  const expanded = await run(railState);
  check(
    'Ctrl+B expands it again',
    expanded.rail === false && expanded.width === 264 && expanded.stored === '0',
    short(expanded),
  );
  await run(`document.querySelector('nav.sidebar button.search').click()`);
  await sleep(400);
  await key('b', 'KeyB', 66, 2);
  const withDialog = (await run(railState)).rail === false;
  await key('Escape', 'Escape', 27);
  check('Ctrl+B leaves the sidebar alone while a dialog is open', withDialog);

  // ---------- focus mode locks the toggle ----------
  collectionId = (
    await run(
      `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: null })`,
    )
  ).id;
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(800);
  await run(
    `[...document.querySelectorAll('.start .tile')].find((t) => t.textContent.includes('HTTP request')).click()`,
  );
  await waitFor(`!!$card()`, 4000);
  await run(`$card().querySelector('button[aria-label="Focus mode"]').click()`);
  await sleep(500);
  const locked = await run(
    `({ sidebar: !!document.getElementById('sidebar'), disabled: $bar('Collapse sidebar')?.disabled })`,
  );
  await key('Escape', 'Escape', 27);
  const unlocked = await run(
    `({ sidebar: !!document.getElementById('sidebar'), disabled: $bar('Collapse sidebar')?.disabled })`,
  );
  check(
    'focus mode hides the sidebar and locks the toggle until it ends',
    !locked.sidebar && locked.disabled === true && unlocked.sidebar && unlocked.disabled === false,
    short({ locked, unlocked }),
  );

  // ---------- narrow window ----------
  powershell('resize-window.ps1', ['-Width', '720', '-Height', '520']);
  await sleep(800);
  const narrow = await run(
    `({ ...${railState}, label: $bar('Widen')?.getAttribute('aria-label') ?? null, disabled: $bar('Widen')?.disabled ?? null })`,
  );
  await key('b', 'KeyB', 66, 2);
  const narrowAfterKey = await run(railState);
  powershell('resize-window.ps1', ['-Width', '1280', '-Height', '820']);
  await sleep(800);
  check(
    'a narrow window shows the rail and the toggle says why it waits',
    narrow.rail === true &&
      narrow.disabled === true &&
      narrow.label === 'Widen the window to expand the sidebar' &&
      narrowAfterKey.stored === '0' &&
      (await run(railState)).rail === false,
    short({ narrow, narrowAfterKey }),
  );

  // ---------- window buttons ----------
  await click(`$bar('Maximize')`);
  await waitFor(`!!$bar('Restore')`, 4000);
  const afterMax = windowState();
  const restoreLabel = await run(`$bar('Restore')?.getAttribute('aria-label') ?? null`);
  await click(`$bar('Restore')`);
  await waitFor(`!!$bar('Maximize')`, 4000);
  const afterRestore = windowState();
  check(
    'Maximize and Restore resize the window and swap the button',
    afterMax.zoomed && restoreLabel === 'Restore' && !afterRestore.zoomed,
    short({ afterMax: afterMax.zoomed, afterRestore: afterRestore.zoomed }),
  );
  await click(`document.querySelector('header.titlebar .drag')`, 2);
  await waitFor(`!!$bar('Restore')`, 4000);
  const doubleClicked = windowState().zoomed;
  await click(`document.querySelector('header.titlebar .drag')`, 2);
  await waitFor(`!!$bar('Maximize')`, 4000);
  check(
    'a double-click on the empty bar maximizes and restores',
    doubleClicked && !windowState().zoomed,
  );
  const edges = windowState().out;
  check(
    'the window can still be resized from its edges',
    [
      'left edge:   LEFT',
      'right edge:  RIGHT',
      'top edge:    TOP',
      'bottom edge: BOTTOM',
      'center:      CLIENT',
    ].every((line) => edges.includes(line)),
    edges.split('\n').slice(2).join(' | '),
  );
  await click(`$bar('Minimize')`);
  await sleep(600);
  const minimized = windowState().minimized;
  powershell('restore-window.ps1');
  await sleep(800);
  check('Minimize sends the window to the taskbar', minimized && !windowState().minimized);

  // ---------- the native menu (needs an unlocked desktop) ----------
  // It presses real keys in the foreground window, so it only runs when asked for, with nobody
  // typing on this PC meanwhile: CODESTAMP_MENU_TEST=1.
  if (process.env.CODESTAMP_MENU_TEST !== '1') {
    results.push(
      'SKIP  the native menu opens and its items work: run with CODESTAMP_MENU_TEST=1 while nobody uses the keyboard',
    );
  } else if (powershell('desktop-locked.ps1').trim() === 'locked') {
    results.push(
      'SKIP  the native menu opens and its items work: the desktop is locked, so Windows shows no menus',
    );
  } else {
    fs.mkdirSync(SHOTS, { recursive: true });
    powershell('activate-window.ps1');
    await run(`$nav('Home').click()`);
    await sleep(500);
    const opening = click(`$bar('Menu')`);
    await sleep(900);
    powershell('menu-keys.ps1', [
      '-Shot',
      path.join(SHOTS, 'menu-go.png'),
      '-Keys',
      '{DOWN 4}{RIGHT}{DOWN 5}{ENTER}',
    ]);
    await opening;
    const went = await waitFor(
      `document.querySelector('main h1')?.textContent.trim() === 'Import'`,
      4000,
    );
    check('Go › Import opens Import', went);
    const reopening = click(`$bar('Menu')`);
    await sleep(900);
    powershell('menu-keys.ps1', ['-Keys', '{DOWN 3}{RIGHT}{ENTER}']);
    await reopening;
    const viaMenu = await waitFor(
      `document.getElementById('sidebar')?.classList.contains('rail')`,
      3000,
    );
    check('View › Collapse sidebar collapses it', viaMenu);
    await run(`document.activeElement?.blur()`);
    await key('b', 'KeyB', 66, 2);
  }
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    if (collectionId) {
      await run(`$invoke('delete_collection', { id: '${collectionId}' })`);
    }
    await run(`localStorage.removeItem('codestamp.sidebarCollapsed')`);
    check('test data cleaned up', true);
    // Last: Close ends the app the same way the old Windows button did.
    await click(`$bar('Close')`);
    await sleep(2500);
    const still = spawnSync(
      'powershell',
      [
        '-NoProfile',
        '-Command',
        `. '${path.join(HERE, 'test-process.ps1')}'; @(Get-TestProcess).Count`,
      ],
      { encoding: 'utf8' },
    ).stdout.trim();
    check('Close quits Codestamp', still === '0' || still === '', `processes left: ${still}`);
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  process.exit(0);
}
