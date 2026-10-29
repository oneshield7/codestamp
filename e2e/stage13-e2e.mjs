// End-to-end check of stage 13 (themes, Settings, coloured {{variables}}, MCP client) through the
// real release build. Runs the MCP test server over STDIO (started by Codestamp) and over HTTP.
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E MCP';
const SERVER = path.join(path.dirname(fileURLToPath(import.meta.url)), 'mcp-server.mjs');
// Approvals are remembered on purpose; a value new to every run makes this run ask again.
const RUN_ID = Date.now().toString(36);
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
async function waitFor(expression, timeout = 6000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(150);
  }
  return false;
}
/** Replaces the text of a CodeMirror editor the way a user would: select all, then type. */
async function typeInEditor(label, text) {
  await run(`document.querySelector('.cm-content[aria-label="${label}"]').focus()`);
  await sleep(100);
  for (const type of ['keyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: 'a',
      code: 'KeyA',
      windowsVirtualKeyCode: 65,
      modifiers: 2,
    });
  }
  await cdp('Input.insertText', { text });
  await sleep(200);
}
const helpers = `
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 100)); };
  window.$picked = (trigger) => trigger.querySelector('.text').textContent.trim();
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$dialog = () => document.querySelector('dialog[open]:not(.panel-dialog)');
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$card = () => document.querySelector('article.card');
  window.$tab = (text) => [...$card().querySelectorAll('[role="tab"]')].find((b) => b.textContent.trim().startsWith(text));
  window.$item = (title) => [...$card().querySelectorAll('.items .item')].find((b) => b.querySelector('.item-name').textContent.trim() === title);
  window.$section = (text) => [...document.querySelectorAll('.sections .section')].find((b) => b.textContent.trim() === text);
  window.$theme = (label) => [...document.querySelectorAll('.themes .option')].find((o) => o.querySelector('.label').textContent.trim() === label).querySelector('input');
  window.$result = () => [...$card().querySelectorAll('section.response .block')].map((b) => b.textContent.trim()).join(' | ');
  window.$chip = () => $card().querySelector('section.response .chip')?.textContent.trim();
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 240);

async function runOperation(label) {
  await run(`$btn(${JSON.stringify(label)}, $card()).click()`);
  await sleep(250);
  await waitFor(
    `!$card().querySelector('section.response .chip') ? false : !$btn('Running', $card())`,
    8000,
  );
  await sleep(300);
}

let httpServer;
await connect();
try {
  // ---------- themes and Settings ----------
  await run(`$nav('Settings').click()`);
  await sleep(500);
  const sections = await run(
    `[...document.querySelectorAll('.sections .section')].map((b) => b.textContent.trim())`,
  );
  check(
    'Settings has eight sections',
    sections.length === 8 &&
      sections.includes('Proxy') &&
      sections.includes('SSL/TLS') &&
      sections.includes('Git accounts'),
    short(sections),
  );
  await run(`$section('Appearance').click()`);
  await sleep(300);
  await run(`$theme('Light').click()`);
  await sleep(400);
  await run(`$theme('Dark Night').click()`);
  await sleep(600);
  const night = await run(
    `({ theme: document.documentElement.dataset.theme, bg: getComputedStyle(document.body).backgroundColor })`,
  );
  check(
    'Dark Night paints the window',
    night.theme === 'night' && night.bg === 'rgb(7, 10, 18)',
    short(night),
  );
  await run('location.reload()');
  await sleep(1500);
  await waitFor(`!!document.querySelector('nav button.new')`, 15000);
  await run(helpers);
  const kept = await run(
    `$invoke('get_appearance').then((s) => ({ stored: s.appearance.theme, painted: document.documentElement.dataset.theme }))`,
  );
  check(
    'the theme is saved and back after a reload',
    kept.stored === 'night' && kept.painted === 'night',
    short(kept),
  );

  await run(`$nav('Settings').click()`);
  await sleep(400);
  await run(`$section('Editor').click()`);
  await sleep(300);
  await run(
    `[...document.querySelectorAll('.segmented label')].find((l) => l.textContent.trim() === '4 spaces').querySelector('input').click()`,
  );
  await sleep(900);
  await run(`$section('Requests').click()`);
  await sleep(300);
  await run(`$set(document.querySelector('.content input.mono'), 'Bad\\u0007Agent')`);
  await sleep(900);
  const refused = await run(`document.querySelector('.content .error')?.textContent.trim() ?? ''`);
  await run(`$set(document.querySelector('.content input.mono'), 'CodestampE2E/1.0')`);
  await sleep(900);
  const prefs = await run(`$invoke('get_preferences')`);
  check(
    'Settings refuse a bad User-Agent and save good values',
    /User-Agent/.test(refused) &&
      prefs.editor.indent === 'four' &&
      prefs.requests.userAgent === 'CodestampE2E/1.0',
    short({ refused, editor: prefs.editor.indent, agent: prefs.requests.userAgent }),
  );

  await run(`$section('Proxy').click()`);
  await sleep(500);
  await run(
    `[...document.querySelectorAll('.mode')].find((m) => m.textContent.includes('Custom proxy')).querySelector('input').click()`,
  );
  await sleep(200);
  await run(`$set(document.getElementById('proxy-host'), 'http://proxy/path')`);
  await run(`$btn('Save proxy').click()`);
  await sleep(700);
  const proxyError = await run(
    `document.querySelector('.content .error')?.textContent.trim() ?? ''`,
  );
  await run(`$set(document.getElementById('proxy-host'), '')`);
  await run(
    `[...document.querySelectorAll('.mode')].find((m) => m.textContent.includes('Use the system proxy')).querySelector('input').click()`,
  );
  await sleep(100);
  await run(`$btn('Save proxy').click()`);
  await sleep(700);
  const proxySaved = await run(
    `(async () => ({ status: document.querySelector('.content .status')?.textContent.trim(), stored: await $invoke('get_proxy') }))()`,
  );
  check(
    'the proxy refuses an address with a path and saves the system proxy',
    /host/.test(proxyError) &&
      proxySaved.stored.settings.mode === 'system' &&
      /Saved/.test(proxySaved.status ?? ''),
    short({ proxyError, proxySaved }),
  );

  // ---------- an MCP request ----------
  await run(`document.querySelector('nav button.new').click()`);
  await sleep(500);
  await run(`$set(document.getElementById('new-collection-name'), ${JSON.stringify(NAME)})`);
  await run(`$set(document.getElementById('new-collection-base-url'), 'https://mcp.test')`);
  await sleep(300);
  await run(`$btn('Create collection', $dialog()).click()`);
  await waitFor(
    `document.querySelector('input[aria-label="Collection name"]')?.value === ${JSON.stringify(NAME)}`,
  );
  const tile = await run(
    `!![...document.querySelectorAll('.start .tile')].find((t) => t.textContent.includes('MCP server'))`,
  );
  await run(
    `[...document.querySelectorAll('.start .tile')].find((t) => t.textContent.includes('MCP server')).click()`,
  );
  await waitFor(`!!$card()?.querySelector('section[aria-label="MCP server"]')`);
  const fresh = await run(
    `({ name: $card().querySelector('.name input').value, transport: $picked($card().querySelector('button[id^="method-"]')), connect: $card().querySelector('.send').textContent.trim(), disabled: $card().querySelector('.send').disabled, sendBg: getComputedStyle($card().querySelector('.send')).backgroundColor })`,
  );
  check(
    'the MCP tile opens an MCP card',
    tile &&
      fresh.name === 'New MCP server' &&
      fresh.transport === 'STDIO' &&
      fresh.connect === 'Connect' &&
      fresh.disabled &&
      fresh.sendBg === 'rgb(232, 197, 94)',
    short(fresh),
  );

  const command = `node "${SERVER}" stdio`;
  await run(`$set($card().querySelector('input[id^="url-"]'), ${JSON.stringify(command)})`);
  await run(`$tab('Environment').click()`);
  await sleep(300);
  await run(
    `$set($card().querySelector('input[aria-label="Environment variables key"]'), 'CODESTAMP_TEST_TOKEN')`,
  );
  await run(
    `$set($card().querySelector('input[aria-label="Environment variables value"]'), 'token {{baseUrl}} ${RUN_ID}')`,
  );
  await sleep(300);
  const painted = await run(
    `[...$card().querySelectorAll('.mirror .var')].map((v) => ({ text: v.textContent, missing: v.classList.contains('missing') }))`,
  );
  check(
    'variables in a field are coloured',
    painted.some((v) => v.text === '{{baseUrl}}' && !v.missing),
    short(painted),
  );
  await sleep(1200);

  await run(`$card().querySelector('.send').click()`);
  await waitFor(`!!$dialog()?.querySelector('#mcp-approval-title')`);
  const approval = await run(
    `({ title: $dialog().querySelector('h2').textContent.trim(), text: $dialog().textContent.replace(/\\s+/g, ' ') })`,
  );
  check(
    'a program asks for approval before it starts',
    approval.title === 'Start this program?' &&
      approval.text.includes('mcp-server.mjs') &&
      approval.text.includes('CODESTAMP_TEST_TOKEN') &&
      !approval.text.includes('mcp.test'),
    short(approval.text),
  );
  await run(`$btn('Start program', $dialog()).click()`);
  const connected = await waitFor(
    `$card().querySelector('.send').textContent.trim() === 'Disconnect' && !!$tab('Tools · 4')`,
    10000,
  );
  const server = await run(
    `({ name: $card().querySelector('.server-name')?.textContent.trim(), tabs: [...$card().querySelectorAll('[aria-label="MCP sections"] [role="tab"]')].map((t) => t.textContent.trim()) })`,
  );
  check(
    'the STDIO server connects and lists what it offers',
    connected &&
      server.name === 'codestamp-test' &&
      server.tabs.includes('Prompts · 1') &&
      server.tabs.includes('Resources · 1'),
    short(server),
  );

  check(
    'connecting shows the tools list',
    !!(await run(`$item('echo')`)),
    await run(
      `$card().querySelector('[aria-label="MCP sections"] [aria-selected="true"]')?.textContent.trim()`,
    ),
  );
  await run(`$tab('Tools').click()`);
  await sleep(200);
  await run(`$item('echo').click()`);
  await sleep(400);
  const template = await run(
    `$card().querySelector('.cm-content[aria-label="Arguments"]').textContent`,
  );
  await typeInEditor('Arguments', '{"text": "hi {{baseUrl}}"}');
  await runOperation('Run tool');
  const echoed = await run(`({ chip: $chip(), result: $result() })`);
  check(
    'a tool runs with variables filled in',
    template.replace(/\s/g, '') === '{"text":""}' &&
      echoed.chip === 'Result' &&
      echoed.result.includes('hi https://mcp.test'),
    short({ template, echoed }),
  );

  await run(`$item('whoami').click()`);
  await sleep(300);
  await runOperation('Run tool');
  check(
    'the program got its environment with variables filled in',
    (await run(`$result()`)).includes('token https://mcp.test'),
    await run(`$result()`),
  );

  await run(`$item('add').click()`);
  await sleep(300);
  const addTemplate = await run(
    `$card().querySelector('.cm-content[aria-label="Arguments"]').textContent.replace(/\\s/g, '')`,
  );
  await typeInEditor('Arguments', '{"a": 2, "b": 3}');
  await runOperation('Run tool');
  const sum = await run(`$result()`);
  check(
    'templates follow the schema and structured content shows',
    addTemplate === '{"a":0,"b":2}' && sum.includes('5') && sum.includes('"sum": 5'),
    short({ addTemplate, sum }),
  );

  await run(`$item('fail').click()`);
  await sleep(300);
  await runOperation('Run tool');
  check(
    'a tool error is shown as one',
    (await run(`$chip()`)) === 'Tool error',
    await run(`$chip()`),
  );

  await typeInEditor('Arguments', '[1, 2]');
  const notObject = await run(
    `({ problem: $card().querySelector('section[aria-label="MCP server"] .problem')?.textContent.trim(), disabled: $btn('Run tool', $card()).disabled })`,
  );
  check(
    'arguments must be a JSON object',
    /JSON object/.test(notObject.problem ?? '') && notObject.disabled,
    short(notObject),
  );
  await typeInEditor('Arguments', '{}');

  await run(`$tab('Prompts').click()`);
  await sleep(300);
  await run(`$item('greet').click()`);
  await sleep(300);
  await typeInEditor('Arguments', '{"name": "Rina"}');
  await runOperation('Get prompt');
  check(
    'a prompt is fetched with its arguments',
    (await run(`$result()`)).includes('Hello, Rina!'),
    await run(`$result()`),
  );

  await run(`$tab('Resources').click()`);
  await sleep(300);
  await run(`$item('Read me').click()`);
  await sleep(300);
  await runOperation('Read resource');
  check(
    'a resource is read',
    (await run(`$result()`)).includes('Codestamp reads resources.'),
    await run(`$result()`),
  );

  await run(`$tab('Messages').click()`);
  await sleep(300);
  const log = await run(
    `[...$card().querySelectorAll('.messages li')].map((li) => li.className.split(' ')[0] + ':' + li.querySelector('.text').textContent.slice(0, 60))`,
  );
  check(
    'messages show traffic, stderr and logging',
    log.some((l) => l.startsWith('sent:') && l.includes('tools/call')) &&
      log.some((l) => l.startsWith('received:')) &&
      log.some((l) => l.includes('codestamp-test ready')) &&
      log.some((l) => l.includes('info: calling echo')),
    short(log.slice(-6)),
  );
  await run(`$tab('Result').click()`);

  await run(`$card().querySelector('.send').click()`);
  await waitFor(`$card().querySelector('.send').textContent.trim() === 'Connect'`);
  await run(`$card().querySelector('button[aria-label="Collapse request"]').click()`);
  await sleep(900);
  const row = await run(
    `(() => { const s = document.querySelector('.row .stamp'); return { stamp: s?.textContent.trim(), color: s && getComputedStyle(s).color }; })()`,
  );
  check('the row shows an MCP stamp', row.stamp === 'MCP', short(row));
  await run(`document.querySelector('.row').click()`);
  await waitFor(`!!$card()?.querySelector('section[aria-label="MCP server"]')`);
  await sleep(500);
  const saved = await run(
    `$card().querySelector('section[aria-label="MCP server"] .hint')?.textContent.trim()`,
  );
  check(
    'the chosen resource is saved with the request',
    /resource “note:\/\/readme”/.test(saved ?? ''),
    saved,
  );

  await run(`$card().querySelector('.send').click()`);
  const again = await waitFor(
    `$card().querySelector('.send').textContent.trim() === 'Disconnect'`,
    10000,
  );
  const askedAgain = await run(`!!$dialog()`);
  check('an approved program starts without asking again', again && !askedAgain);
  await run(`$card().querySelector('.send').click()`);
  await waitFor(`$card().querySelector('.send').textContent.trim() === 'Connect'`);

  await run(`$tab('Environment').click()`);
  await sleep(200);
  await run(
    `$set($card().querySelector('input[aria-label="Environment variables value"]'), 'changed')`,
  );
  await sleep(400);
  await run(`$card().querySelector('.send').click()`);
  const changedAsks = await waitFor(`!!$dialog()?.querySelector('#mcp-approval-title')`);
  await run(`$btn('Cancel', $dialog()).click()`);
  await sleep(400);
  const cancelled = await run(
    `({ dialog: !!$dialog(), button: $card().querySelector('.send').textContent.trim() })`,
  );
  check(
    'a changed environment asks again, and Cancel starts nothing',
    changedAsks && !cancelled.dialog && cancelled.button === 'Connect',
    short(cancelled),
  );

  // ---------- the same server over Streamable HTTP ----------
  httpServer = spawn(process.execPath, [SERVER, 'http', '0'], {
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const httpOutput = [];
  const port = await new Promise((resolve) => {
    httpServer.stdout.on('data', (chunk) => {
      httpOutput.push(String(chunk));
      const match = String(chunk).match(/listening (\d+)/);
      if (match) resolve(Number(match[1]));
    });
  });
  await run(`$pick($card().querySelector('button[id^="method-"]'), 'HTTP')`);
  await run(`$set($card().querySelector('input[id^="url-"]'), 'http://127.0.0.1:${port}/mcp')`);
  await run(`$tab('Headers').click()`);
  await sleep(300);
  await run(
    `(() => { const rows = $card().querySelectorAll('input[aria-label="Headers key"]'); const values = $card().querySelectorAll('input[aria-label="Headers value"]'); $set(rows[0], 'X-Test'); $set(values[0], 'abc'); })()`,
  );
  await sleep(300);
  await run(`$card().querySelector('.send').click()`);
  const httpConnected = await waitFor(
    `$card().querySelector('.send').textContent.trim() === 'Disconnect' && !!$tab('Tools · 4')`,
    8000,
  );
  check(
    'the HTTP server connects without asking to start a program',
    httpConnected && !(await run(`!!$dialog()`)),
  );
  await run(`$tab('Tools').click()`);
  await sleep(200);
  await run(`$item('echo').click()`);
  await sleep(300);
  await typeInEditor('Arguments', '{"text": "over http"}');
  await runOperation('Run tool');
  check(
    'a tool answered as an event stream shows its result',
    (await run(`$result()`)).includes('over http'),
    await run(`$result()`),
  );
  await run(`$card().querySelector('.send').click()`);
  await waitFor(`$card().querySelector('.send').textContent.trim() === 'Connect'`);
  await sleep(500);
  check(
    'disconnecting ends the HTTP session on the server',
    httpOutput.join('').includes('deleted '),
    short(httpOutput.join('')),
  );

  await run(`$tab('Headers').click()`);
  await sleep(200);
  await run(`$set($card().querySelector('input[aria-label="Headers value"]'), 'wrong')`);
  await sleep(300);
  await run(`$card().querySelector('.send').click()`);
  await waitFor(
    `$card().querySelector('.send').textContent.trim() === 'Connect' && !!$card().querySelector('section.response .problem')`,
    8000,
  );
  const refusedHttp = await run(
    `$card().querySelector('section.response .problem')?.textContent.trim()`,
  );
  check(
    'a refused connection explains why',
    /401/.test(refusedHttp ?? '') && /Missing X-Test/.test(refusedHttp ?? ''),
    refusedHttp,
  );

  const stored = await run(
    `(async () => { const detail = await $invoke('get_collection', { id: await $invoke('list_collections').then((l) => l.find((c) => c.name === ${JSON.stringify(NAME)}).id) }); const request = detail.requests[0]; return $invoke('get_request', { id: request.id }); })()`,
  );
  check(
    'the request is stored as MCP over HTTP with its operation',
    stored.kind === 'mcp' && stored.method === 'HTTP' && stored.mcpOperation?.name === 'echo',
    short({ kind: stored.kind, method: stored.method, op: stored.mcpOperation }),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    httpServer?.kill();
    await run(helpers);
    if (await run(`!!$dialog()`)) {
      await run(`$btn('Cancel', $dialog())?.click()`);
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
    await run(
      `$invoke('save_preferences', { preferences: { editor: { indent: 'two', closeBrackets: true, wrapLines: false, layout: 'stacked', variableHints: true }, requests: { timeoutSeconds: 30, maxResponseMb: 0, followRedirects: true, cookies: true, noCache: false, userAgent: '' }, history: { maxEntries: 1000, keepDays: 30 } } })`,
    );
    await run(
      `$invoke('get_appearance').then((s) => $invoke('save_appearance', { appearance: { ...s.appearance, theme: 'light' } }))`,
    );
    check('test data cleaned up', !(await run(`!!$nav(${JSON.stringify(NAME)})`)));
    // The page keeps its own copy of the settings, so it starts over from what is saved now.
    await run('location.reload()');
    await sleep(1500);
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  process.exit(0);
}
