// End-to-end check of: the "None" editor indentation (new default), Settings › SSL/TLS, and the
// Docs tool (Postman descriptions imported, Markdown drawn safely, editing, masking, duplicates).
// Imports "E2E Docs" and removes it.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Docs';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const SHOTS = process.argv.includes('--shots');

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
async function waitFor(expression, timeout = 8000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(150);
  }
  return false;
}
async function key(name, code, keyCode, modifiers = 0) {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp('Input.dispatchKeyEvent', {
      type,
      key: name,
      code,
      windowsVirtualKeyCode: keyCode,
      modifiers,
    });
  }
  await sleep(200);
}
async function typeIn(selector, text) {
  await run(`(() => { const el = ${selector}; el.focus(); })()`);
  await key('a', 'KeyA', 65, 2);
  await cdp('Input.insertText', { text });
  await sleep(300);
}
async function shot(name) {
  if (!SHOTS) return;
  const reply = await cdp('Page.captureScreenshot', { format: 'png' });
  fs.mkdirSync(path.join(here, '.work', 'screens'), { recursive: true });
  fs.writeFileSync(
    path.join(here, '.work', 'screens', name),
    Buffer.from(reply.result.data, 'base64'),
  );
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$side = (text) => [...document.querySelectorAll('nav.sidebar .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text);
  window.$h1 = () => document.querySelector('main h1')?.textContent.trim();
  window.$goTo = async (text) => { document.querySelector('nav.sidebar button.search').click(); await new Promise((r) => setTimeout(r, 150)); [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); };
  window.$section = (label) => [...document.querySelectorAll('nav[aria-label="Settings sections"] button')].find((b) => b.textContent.trim().startsWith(label));
  window.$choice = (label) => [...document.querySelectorAll('main .segmented label')].find((l) => l.textContent.trim() === label);
  window.$card = () => document.querySelector('article.card');
  window.$editor = () => $card().querySelector('.body-editor .cm-content');
  window.$editorText = () => [...$editor().querySelectorAll('.cm-line')].map((l) => l.textContent).join('\\n');
  window.$doc = (name) => [...document.querySelectorAll('main article.request')].find((a) => a.querySelector('h4')?.textContent.trim() === name);
  window.$entries = () => [...document.querySelectorAll('nav[aria-label="Docs index"] button.entry')].map((b) => b.querySelector('.name')?.textContent.trim());
  window.$entry = (name) => [...document.querySelectorAll('nav[aria-label="Docs index"] button.entry')].find((b) => b.querySelector('.name')?.textContent.trim() === name);
  window.$show = async (name) => { $entry(name).click(); await new Promise((r) => setTimeout(r, 300)); };
  window.$pickCollection = async (name) => { document.querySelector('nav[aria-label="Docs index"] button[aria-haspopup="listbox"]').click(); await new Promise((r) => setTimeout(r, 250)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim().endsWith(name)); option?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 250)); return !!option; };
  window.$cells = (article, title) => [...(article.querySelector('section[aria-label="' + title + '"]')?.querySelectorAll('tbody tr') ?? [])].map((tr) => [...tr.querySelectorAll('td')].map((td) => td.textContent.trim()));
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 500);
const prefs = () => run(`$invoke('get_preferences')`);

const POSTMAN = JSON.stringify({
  info: {
    name: NAME,
    schema: 'https://schema.getpostman.com/json/collection/v2.1.0/collection.json',
    description: 'Imported **docs** for the users API.',
  },
  item: [
    {
      name: 'Users',
      description: 'User endpoints',
      item: [
        {
          name: 'Create user',
          request: {
            method: 'POST',
            url: '{{baseUrl}}/users',
            header: [
              { key: 'Authorization', value: 'Bearer plain-secret' },
              { key: 'X-Token', value: '{{token}}' },
            ],
            body: {
              mode: 'raw',
              raw: '{"name":"Ana","roles":["admin","dev"]}',
              options: { raw: { language: 'json' } },
            },
            description: 'Creates a user.',
          },
          response: [
            {
              name: 'Created',
              code: 201,
              header: [{ key: 'Content-Type', value: 'application/json' }],
              body: '{"id":1}',
            },
          ],
        },
      ],
    },
    { name: 'Ping', request: { method: 'GET', url: '{{baseUrl}}/ping' } },
  ],
});

await connect();
await run('location.reload()');
await sleep(1800);
await run(helpers);
for (const stale of await run(`$invoke('list_collections')`)) {
  if (stale.name === NAME) await run(`$invoke('delete_collection', { id: '${stale.id}' })`);
}
let collectionId = null;
const before = await prefs();
try {
  // ---------- Settings › Editor: None ----------
  await run(`$goTo('Settings')`);
  await sleep(600);
  await run(`$section('Editor').click()`);
  await sleep(300);
  const indents = await run(
    `[...document.querySelectorAll('main [role="radiogroup"][aria-label="Indentation"] label')].map((l) => l.textContent.trim())`,
  );
  await run(`$choice('None').querySelector('input').click()`);
  await sleep(800);
  check(
    'Editor offers None first and saves it',
    JSON.stringify(indents) === JSON.stringify(['None', '2 spaces', '4 spaces', 'Tab']) &&
      (await prefs()).editor.indent === 'none',
    short(indents),
  );

  // ---------- Settings › SSL/TLS ----------
  const sections = await run(
    `[...document.querySelectorAll('nav[aria-label="Settings sections"] button')].map((b) => b.textContent.trim())`,
  );
  check(
    'Certificates is now SSL/TLS',
    sections.includes('SSL/TLS') && !sections.includes('Certificates'),
    short(sections),
  );
  await run(`$section('Requests').click()`);
  await sleep(300);
  await run(`$btn('Open SSL/TLS').click()`);
  await sleep(400);
  const tls = await run(
    `({ current: document.querySelector('nav[aria-label="Settings sections"] [aria-current="page"]')?.textContent.trim(), headings: [...document.querySelectorAll('main .content h2, main h2')].map((h) => h.textContent.trim()), versions: [...document.querySelectorAll('main [aria-label="Minimum TLS version"] label')].map((l) => ({ text: l.textContent.trim(), on: l.classList.contains('on') })) })`,
  );
  check(
    'Requests links to SSL/TLS, which holds the TLS version and both certificate lists',
    tls.current === 'SSL/TLS' &&
      ['TLS version', 'Certificate checks', 'CA certificates', 'Client certificates'].every((h) =>
        tls.headings.includes(h),
      ) &&
      tls.versions[0]?.text === 'TLS 1.2 or newer' &&
      tls.versions[0]?.on,
    short(tls),
  );
  await run(`$choice('TLS 1.3 only').querySelector('input').click()`);
  await sleep(800);
  const strict = (await prefs()).requests.minTlsVersion;
  await run(`$section('Requests').click()`);
  await sleep(300);
  await run(`$btn('Reset request settings').click()`);
  await sleep(800);
  const afterReset = (await prefs()).requests.minTlsVersion;
  check(
    'TLS 1.3 only is saved, and resetting Requests leaves it alone',
    strict === '1.3' && afterReset === '1.3',
    short({ strict, afterReset }),
  );
  await run(`$section('SSL/TLS').click()`);
  await sleep(300);
  await run(`$choice('TLS 1.2 or newer').querySelector('input').click()`);
  await sleep(800);
  check('back to TLS 1.2 or newer', (await prefs()).requests.minTlsVersion === '1.2');
  await shot('settings-tls.png');

  // ---------- Docs from an imported Postman collection ----------
  collectionId = (
    await run(
      `$invoke('run_import', { content: ${JSON.stringify(POSTMAN)}, name: ${JSON.stringify(NAME)} })`,
    )
  ).id;
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  const tools = await run(
    `[...document.querySelectorAll('nav.sidebar [role="group"][aria-label="Tools"] .item .name')].map((n) => n.textContent.trim()).slice(0, 3)`,
  );
  check(
    'Docs sits in Tools after Orchestration and Catalog',
    JSON.stringify(tools) === JSON.stringify(['Orchestration', 'Catalog', 'Docs']),
    short(tools),
  );
  await run(`$side('Docs').click()`);
  await waitFor(`$h1() === 'Docs'`, 5000);
  const picked = await run(`$pickCollection(${JSON.stringify(NAME)})`);
  await waitFor(
    `document.querySelector('#doc-collection-title')?.textContent.trim() === ${JSON.stringify(NAME)}`,
    5000,
  );
  await sleep(300);
  const overview = await run(`({
    collection: document.querySelector('main .collection .markdown')?.textContent.trim(),
    bold: document.querySelector('main .collection .markdown strong')?.textContent,
    index: $entries(),
    docsShown: document.querySelectorAll('main article.request').length,
  })`);
  await run(`$show('Users')`);
  const section = await run(
    `({ text: document.querySelector('main .section-doc .markdown')?.textContent.trim(), links: [...document.querySelectorAll('main .section-doc .link-row .name')].map((n) => n.textContent.trim()) })`,
  );
  await run(`$show('Create user')`);
  const imported = await run(`(() => {
    const create = $doc('Create user');
    return {
      request: create?.querySelector('.markdown')?.textContent.trim(),
      headers: $cells(create, 'Headers'),
      body: create?.querySelector('section[aria-label="Body"] pre')?.textContent,
      example: create?.querySelector('details summary')?.textContent.replace(/\\s+/g, ' ').trim(),
      docsShown: document.querySelectorAll('main article.request').length,
    };
  })()`);
  check(
    'the collection dropdown opens a collection on its overview',
    picked && overview.docsShown === 0,
    short({ picked, overview }),
  );
  check(
    'Postman descriptions arrive for the collection, the folder and the request',
    overview.collection === 'Imported docs for the users API.' &&
      overview.bold === 'docs' &&
      section.text === 'User endpoints' &&
      imported.request === 'Creates a user.',
    short({ overview, section, imported }),
  );
  check(
    'a typed-in token is masked while a {{variable}} is shown',
    JSON.stringify(imported.headers) ===
      JSON.stringify([
        ['Authorization', 'Bearer ••••••••'],
        ['X-Token', '{{token}}'],
      ]),
    short(imported.headers),
  );
  check(
    'the body and the saved example are listed',
    imported.body === '{"name":"Ana","roles":["admin","dev"]}' &&
      imported.example === '201 Created',
    short(imported),
  );
  check(
    'the index lists the overview, requests outside sections, then each folded section; one doc shows at a time',
    JSON.stringify(overview.index) === JSON.stringify(['Overview', 'Ping', 'Users']) &&
      JSON.stringify(section.links) === JSON.stringify(['Create user']) &&
      imported.docsShown === 1,
    short({ index: overview.index, links: section.links, shown: imported.docsShown }),
  );
  await run(`$entry('Ping').focus()`);
  await key('ArrowDown', 'ArrowDown', 40);
  await key('ArrowDown', 'ArrowDown', 40);
  await sleep(300);
  const arrowed = await run(
    `({ focused: document.activeElement?.querySelector('.name')?.textContent.trim(), shown: document.querySelector('main article.request h4')?.textContent.trim() })`,
  );
  check(
    'Up and Down move through the index and show each doc',
    arrowed.focused === 'Create user' && arrowed.shown === 'Create user',
    short(arrowed),
  );
  await run(
    `[...document.querySelectorAll('main .pager-link')].find((b) => b.textContent.includes('Previous'))?.click()`,
  );
  await sleep(300);
  const paged = await run(
    `({ shown: document.querySelector('main article.request h4')?.textContent.trim(), next: document.querySelector('main .pager-link.next .name')?.textContent.trim() })`,
  );
  check(
    'Previous and Next walk the requests in index order',
    paged.shown === 'Ping' && paged.next === 'Create user',
    short(paged),
  );
  await run(`$show('Overview')`);
  await shot('docs-imported.png');

  // ---------- Markdown is drawn, raw HTML never runs ----------
  const markdown = [
    '## Overview',
    '',
    'A **users** API with `JSON` bodies. See [the guide](https://example.com/docs).',
    '',
    '- list users',
    '- create users',
    '',
    '| Code | Meaning |',
    '| --- | --- |',
    '| 201 | Created |',
    '',
    '<img src=x onerror="window.__pwned=1"><script>window.__pwned=2</script>',
    '',
    '[bad](javascript:window.__pwned=3)',
  ].join('\n');
  await run(
    `[...document.querySelector('main .collection').querySelectorAll('button')].find((b) => b.textContent.trim().startsWith('Edit description')).click()`,
  );
  await sleep(300);
  await run(
    `$set(document.querySelector('main .collection textarea'), ${JSON.stringify(markdown)})`,
  );
  await sleep(150);
  await run(`$btn('Preview', document.querySelector('main .collection')).click()`);
  await sleep(200);
  const preview = await run(
    `!!document.querySelector('main .collection .preview .markdown table')`,
  );
  await run(`document.querySelector('main .collection .actions .primary').click()`);
  await waitFor(
    `!document.querySelector('main .collection textarea') && !document.querySelector('main .collection .preview')`,
    4000,
  );
  const drawn = await run(`(() => {
    const root = document.querySelector('main .collection .markdown');
    return {
      heading: root.querySelector('h4')?.textContent.trim(),
      strong: root.querySelector('strong')?.textContent,
      code: root.querySelector('p code')?.textContent,
      items: root.querySelectorAll('li').length,
      cell: root.querySelector('td')?.textContent.trim(),
      link: root.querySelector('a')?.getAttribute('href'),
      raw: [...root.querySelectorAll('code.raw')].map((c) => c.textContent).join(' '),
      dangerous: root.querySelectorAll('img, script').length,
      jsLinks: [...root.querySelectorAll('a')].filter((a) => a.getAttribute('href')?.startsWith('javascript')).length,
      plain: root.querySelector('.plain-link')?.textContent,
      pwned: window.__pwned ?? null,
    };
  })()`);
  check('the preview draws Markdown before saving', preview);
  check(
    'Markdown becomes headings, bold, code, a list, a table and a web link',
    drawn.heading === 'Overview' &&
      drawn.strong === 'users' &&
      drawn.code === 'JSON' &&
      drawn.items === 2 &&
      drawn.cell === '201' &&
      drawn.link === 'https://example.com/docs',
    short(drawn),
  );
  check(
    'raw HTML is shown as text and a javascript: link is not a link',
    drawn.raw.includes('<img src=x') &&
      drawn.raw.includes('<script>') &&
      drawn.dangerous === 0 &&
      drawn.jsLinks === 0 &&
      drawn.plain === 'bad' &&
      drawn.pwned === null,
    short(drawn),
  );
  const stored = (await run(`$invoke('collection_docs', { collectionId: '${collectionId}' })`))
    .description;
  check('the description is saved as written', stored === markdown, short(stored.slice(0, 80)));
  await shot('docs-markdown.png');

  // ---------- a request description: write, preview, Ctrl+Enter, Escape ----------
  await run(`$show('Ping')`);
  await run(`$btn('Add a description', $doc('Ping')).click()`);
  await sleep(300);
  await typeIn(`$doc('Ping').querySelector('textarea')`, 'Checks the **server**.');
  await key('Enter', 'Enter', 13, 2);
  await waitFor(`!$doc('Ping').querySelector('textarea')`, 4000);
  const ping = await run(
    `({ text: $doc('Ping').querySelector('.markdown')?.textContent.trim(), strong: $doc('Ping').querySelector('.markdown strong')?.textContent })`,
  );
  check(
    'Ctrl+Enter saves a request description',
    ping.text === 'Checks the server.' && ping.strong === 'server',
    short(ping),
  );
  await run(`$btn('Edit description', $doc('Ping')).click()`);
  await sleep(300);
  await typeIn(`$doc('Ping').querySelector('textarea')`, 'Something else');
  await key('Escape', 'Escape', 27);
  await sleep(300);
  const kept = await run(
    `({ editing: !!$doc('Ping').querySelector('textarea'), text: $doc('Ping').querySelector('.markdown')?.textContent.trim() })`,
  );
  check(
    'Escape leaves the description as it was',
    !kept.editing && kept.text === 'Checks the server.',
    short(kept),
  );
  await run(`$btn('Edit description', $doc('Ping')).click()`);
  await sleep(300);
  await run(`$set($doc('Ping').querySelector('textarea'), 'x'.repeat(20001))`);
  await sleep(200);
  const tooLong = await run(
    `({ disabled: $doc('Ping').querySelector('.actions .primary').disabled, alert: $doc('Ping').querySelector('[role="alert"]')?.textContent.trim() })`,
  );
  check(
    'a description over 20,000 characters cannot be saved',
    tooLong.disabled && tooLong.alert === 'A description can be up to 20,000 characters.',
    short(tooLong),
  );
  await run(`$btn('Cancel', $doc('Ping')).click()`);
  await sleep(200);

  // ---------- open request, duplicates ----------
  await run(`$show('Create user')`);
  const docs = await run(`$invoke('collection_docs', { collectionId: '${collectionId}' })`);
  const create = docs.requests.find((request) => request.name === 'Create user');
  await run(`$invoke('duplicate_request', { id: '${create.id}', name: 'Create user copy' })`);
  const copied = (
    await run(`$invoke('collection_docs', { collectionId: '${collectionId}' })`)
  ).requests.find((request) => request.name === 'Create user copy');
  check(
    'a duplicated request keeps its description',
    copied?.description === 'Creates a user.',
    short(copied?.description),
  );
  await run(`$btn('Open request', $doc('Create user')).click()`);
  const opened = await waitFor(
    `!!$card() && [...$card().querySelectorAll('input')].some((input) => input.value === 'Create user')`,
    5000,
  );
  check('Open request opens the card', opened);

  // ---------- None in the editor: no indent on Enter, Beautify at the left edge ----------
  await run(
    `document.querySelector('article.card [role="tab"]') && [...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith('Body'))?.click()`,
  );
  await sleep(400);
  await run(`$btn('Beautify', $card()).click()`);
  await sleep(300);
  const flat = await run(`$editorText()`);
  check(
    'Beautify with None puts each element on its own line at the left edge',
    flat === '{\n"name": "Ana",\n"roles": [\n"admin",\n"dev"\n]\n}',
    short(flat),
  );
  await typeIn(`$editor()`, '{');
  await key('Enter', 'Enter', 13);
  await cdp('Input.insertText', { text: 'x' });
  await sleep(300);
  const typed = await run(`$editorText()`);
  await key('Tab', 'Tab', 9);
  const afterTab = await run(
    `({ text: $editorText(), inEditor: !!document.activeElement?.closest('.cm-editor') })`,
  );
  check(
    'Enter starts the next line at the left edge',
    typed.split('\n')[1]?.startsWith('x') === true,
    short(typed),
  );
  check(
    'Tab moves on instead of indenting',
    afterTab.text === typed && !afterTab.inEditor,
    short(afterTab),
  );
  await shot('editor-none.png');
} catch (error) {
  check('the suite ran to the end', false, error.message.slice(0, 300));
} finally {
  await run(`$invoke('save_preferences', { preferences: ${JSON.stringify(before)} })`);
  if (collectionId) await run(`$invoke('delete_collection', { id: '${collectionId}' })`);
  await run('location.reload()');
  await sleep(1500);
  await run(helpers);
  const left = await run(`$invoke('list_collections')`);
  check('test data cleaned up', !left.some((collection) => collection.name === NAME));
  socket.close();
}

console.log(results.join('\n'));
process.exit(results.some((line) => line.startsWith('FAIL')) ? 1 : 0);
