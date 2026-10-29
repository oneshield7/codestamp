// End-to-end check of the styled dropdowns (method colours, blue ring, keyboard) and the coloured
// icons in Light and Dark Night. Creates "E2E Dropdowns" and deletes it afterwards.
const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Dropdowns';
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
async function key(name, code, keyCode) {
  for (const type of ['keyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', { type, key: name, code, windowsVirtualKeyCode: keyCode });
  }
  await sleep(150);
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$dialog = () => document.querySelector('dialog[open]:not(.panel-dialog)');
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$method = () => document.querySelector('article.card button[id^="method-"]');
  window.$options = () => [...document.querySelectorAll('[role="listbox"] [role="option"]')].map((o) => ({ text: o.textContent.trim(), color: getComputedStyle(o.querySelector('.label')).color, selected: o.getAttribute('aria-selected') === 'true' }));
  window.$iconColor = (name) => getComputedStyle($nav(name).querySelector('.nav-icon')).color;
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 260);

await connect();
// The checks below expect Light first; the theme the user chose is put back at the end.
const originalTheme = await run(`$invoke('get_appearance').then((s) => s.appearance.theme)`);
// Saved through the Rust core, then the page starts over so it reads the saved theme itself.
async function setTheme(theme) {
  await run(
    `$invoke('get_appearance').then((s) => $invoke('save_appearance', { appearance: { ...s.appearance, theme: '${theme}' } }))`,
  );
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
}
await setTheme('light');
try {
  await run(`document.querySelector('nav button.new').click()`);
  await sleep(500);
  await run(`$set(document.getElementById('new-collection-name'), ${JSON.stringify(NAME)})`);
  await sleep(300);
  await run(`$btn('Create collection', $dialog()).click()`);
  await sleep(1200);
  await run(`document.querySelector('.start .tile').click()`);
  await sleep(900);

  await run(`$method().click()`);
  await sleep(250);
  const open = await run(
    `({ expanded: $method().getAttribute('aria-expanded'), ring: getComputedStyle($method()).boxShadow, options: $options(), focused: document.activeElement?.getAttribute('role') })`,
  );
  const get = open.options.find((o) => o.text === 'GET');
  const del = open.options.find((o) => o.text === 'DELETE');
  check(
    'the method list opens with each method in its colour',
    open.expanded === 'true' &&
      open.options.length === 7 &&
      get?.selected &&
      get?.color === 'rgb(10, 122, 75)' &&
      del?.color === 'rgb(192, 38, 27)' &&
      open.focused === 'listbox',
    short(open.options.map((o) => `${o.text}:${o.color}`)),
  );
  check(
    'an open list has a blue edge',
    /47, 107, 255/.test(open.ring) && /inset/.test(open.ring),
    open.ring,
  );

  await key('ArrowDown', 'ArrowDown', 40);
  await key('Enter', 'Enter', 13);
  const keyed = await run(
    `({ method: $method().querySelector('.text').textContent.trim(), color: getComputedStyle($method().querySelector('.text')).color, closed: $method().getAttribute('aria-expanded') === 'false', focus: document.activeElement === $method() })`,
  );
  check(
    'arrow keys and Enter pick the next method',
    keyed.method === 'POST' && keyed.color === 'rgb(164, 75, 7)' && keyed.closed && keyed.focus,
    short(keyed),
  );

  await run(`$method().click()`);
  await sleep(200);
  await key('d', 'KeyD', 68);
  await key('Escape', 'Escape', 27);
  const escaped = await run(
    `({ method: $method().querySelector('.text').textContent.trim(), list: !!document.querySelector('[role="listbox"]'), card: !!document.querySelector('article.card') })`,
  );
  check(
    'typing jumps to an option and Escape closes without choosing',
    escaped.method === 'POST' && !escaped.list && escaped.card,
    short(escaped),
  );

  await run(`$method().click()`);
  await sleep(200);
  await run(
    `document.querySelector('header.top h1, input[aria-label="Collection name"]').dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))`,
  );
  await sleep(200);
  check(
    'clicking elsewhere closes the list',
    !(await run(`!!document.querySelector('[role="listbox"]')`)),
  );

  const env = await run(
    `(() => { const b = document.querySelector('button[aria-label="Active environment"]'); return { text: b.querySelector('.text').textContent.trim(), dot: getComputedStyle(b.querySelector('.dot')).backgroundColor }; })()`,
  );
  check(
    'the environment picker shows its status dot',
    env.text === 'No environment' && env.dot === 'rgb(140, 138, 132)',
    short(env),
  );

  const light = await run(
    `({ home: $iconColor('Home'), flows: $iconColor('Orchestration'), catalog: $iconColor('Catalog'), tile: getComputedStyle($nav('Home').querySelector('.nav-icon')).backgroundColor })`,
  );
  check(
    'sidebar icons are coloured in Light',
    light.home === 'rgb(36, 89, 214)' &&
      light.flows === 'rgb(181, 71, 13)' &&
      light.catalog === 'rgb(107, 63, 212)' &&
      light.tile === 'rgb(232, 239, 253)',
    short(light),
  );
  await run(`$nav('Settings').click()`);
  await sleep(500);
  const title = await run(
    `(() => { const icon = document.querySelector('h1.page-title .nav-icon'); return { text: document.querySelector('h1.page-title').textContent.trim(), color: icon && getComputedStyle(icon).color }; })()`,
  );
  const sectionColors = await run(
    `[...document.querySelectorAll('.sections .section svg')].map((s) => getComputedStyle(s).color)`,
  );
  check(
    'page titles and Settings sections have coloured icons',
    title.text === 'Settings' &&
      title.color === 'rgb(184, 36, 95)' &&
      new Set(sectionColors).size >= 6,
    short({ title, sectionColors }),
  );
  const font = await run(
    `(() => { const b = document.querySelector('button[aria-label="Interface font family"]'); return b ? b.querySelector('.text').textContent.trim() : null; })()`,
  );
  check('Settings uses the same dropdown', font === 'Geist (default)', String(font));
  await setTheme('night');
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);
  await run(`document.querySelector('.row').click()`);
  await sleep(900);
  const night = await run(`({ home: $iconColor('Home'), catalog: $iconColor('Catalog') })`);
  await run(`$method().click()`);
  await sleep(250);
  const nightList = await run(
    `({ bg: getComputedStyle(document.querySelector('[role="listbox"]')).backgroundColor, get: $options().find((o) => o.text === 'GET').color })`,
  );
  await key('Escape', 'Escape', 27);
  check(
    'icons and the list follow Dark Night',
    night.home === 'rgb(130, 170, 255)' &&
      night.catalog === 'rgb(185, 162, 255)' &&
      nightList.bg === 'rgb(13, 18, 29)' &&
      nightList.get === 'rgb(79, 207, 147)',
    short({ night, nightList }),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    if (await run(`!!document.querySelector('[role="listbox"]')`)) {
      await key('Escape', 'Escape', 27);
    }
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
    await setTheme(originalTheme);
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  process.exit(0);
}
