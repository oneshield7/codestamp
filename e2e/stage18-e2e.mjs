// End-to-end check of stage 15d: importing an OpenAPI 3 spec (YAML file) and a Swagger 2 spec
// (from a URL) into collections, then sending an imported request.
// Creates "E2E Pets" and "E2E Legacy" and deletes them afterwards.
import http from 'node:http';

const CDP = 'http://127.0.0.1:9333';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const seen = [];
let base = '';
const yaml = () => `
openapi: 3.0.3
info:
  title: E2E Pets
servers:
  - url: ${base}/v1
security:
  - key: []
tags:
  - name: pets
paths:
  /pets/{petId}:
    get:
      tags: [pets]
      summary: Get a pet
      parameters:
        - { name: petId, in: path, required: true, schema: { type: integer, example: 42 } }
        - { name: fields, in: query, required: true, schema: { type: string, default: name } }
      responses:
        200: { description: ok }
  /pets:
    post:
      tags: [pets]
      summary: Add a pet
      security: []
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                name: { type: string, example: Rex }
  /health:
    get:
      summary: Health
components:
  securitySchemes:
    key: { type: apiKey, in: header, name: X-API-Key }
`;
const swagger = () =>
  JSON.stringify({
    swagger: '2.0',
    info: { title: 'E2E Legacy' },
    host: base.replace('http://', ''),
    schemes: ['http'],
    paths: {
      '/users/{id}': {
        delete: {
          summary: 'Remove user',
          parameters: [{ name: 'id', in: 'path', type: 'string', required: true }],
        },
      },
    },
  });
const server = http.createServer((req, res) => {
  seen.push({ method: req.method, url: req.url, key: req.headers['x-api-key'] ?? null });
  if (req.url === '/specs/legacy.json') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(swagger());
  }
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ path: req.url }));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
base = `http://127.0.0.1:${server.address().port}`;

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
async function waitFor(expression, timeout = 10000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(200);
  }
  return false;
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$card = () => document.querySelector('article.card');
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$json = () => { try { return JSON.parse(document.querySelector('.response .cm-content')?.innerText ?? ''); } catch { return null; } };
  window.$detail = (name) => $invoke('list_collections').then((list) => list.find((c) => c.name === name)).then((c) => c && $invoke('get_collection', { id: c.id }));
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 320);
async function deleteCollection(name) {
  if (!(await run(`!!$nav(${JSON.stringify(name)})`))) return;
  await run(`$nav(${JSON.stringify(name)}).click()`);
  await sleep(800);
  if ((await run(`document.querySelector('input[aria-label="Collection name"]')?.value`)) !== name)
    return;
  await run(`$btn('Delete collection').click()`);
  await sleep(300);
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Delete').click()`,
  );
  await sleep(900);
}

await connect();
try {
  // ---------- YAML file ----------
  await run(`$nav('Import').click()`);
  await sleep(600);
  await run(
    `(() => { const input = document.querySelector('input[aria-label="Collection or environment file"]'); const dt = new DataTransfer(); dt.items.add(new File([${JSON.stringify(yaml())}], 'pets.yaml', { type: 'application/yaml' })); input.files = dt.files; input.dispatchEvent(new Event('change', { bubbles: true })); })()`,
  );
  await waitFor(`!!$btn('Import 3 requests')`, 6000);
  const preview = await run(
    `({ source: document.querySelector('.file .hint')?.textContent.trim(), stats: [...document.querySelectorAll('.stat')].map((s) => s.textContent.replace(/\\s+/g, ' ').trim()), name: document.querySelector('.field input')?.value })`,
  );
  check(
    'a YAML OpenAPI file previews as a collection',
    preview.source.endsWith('OpenAPI 3.0.3') &&
      preview.stats.includes('3requests') &&
      preview.stats.includes('1sections') &&
      preview.name === 'E2E Pets',
    short(preview),
  );
  await run(`$btn('Import 3 requests').click()`);
  await waitFor(`!!$nav('E2E Pets')`, 8000);
  const pets = await run(`$detail('E2E Pets')`);
  const requests = await run(
    `Promise.all(${JSON.stringify(pets.requests.map((r) => r.id))}.map((id) => $invoke('get_request', { id })))`,
  );
  const setId = await run(`$invoke('get_collection_variable_set', { collectionId: '${pets.id}' })`);
  const views = await run(`$invoke('get_environment_variables', { id: '${setId}' })`);
  const vars = views.map((v) => ({
    key: v.key,
    value: v.isSecret ? '(secret)' : v.value,
    secret: v.isSecret,
  }));
  const getPet = requests.find((r) => r.name === 'Get a pet');
  check(
    'operations become requests grouped by tag, with path variables',
    pets.sections.map((s) => s.name).join() === 'pets' &&
      getPet?.url === '{{baseUrl}}/pets/:petId' &&
      getPet.pathVariables[0]?.value === '42' &&
      getPet.params[0]?.key === 'fields',
    short({
      sections: pets.sections,
      getPet: getPet && { url: getPet.url, pv: getPet.pathVariables, params: getPet.params },
    }),
  );
  check(
    'the server becomes {{baseUrl}} and the API key a secret variable',
    vars.some((v) => v.key === 'baseUrl' && v.value === `${base}/v1`) &&
      vars.some((v) => v.key === 'apiKey' && v.secret),
    short(vars),
  );
  check(
    'global security becomes collection auth; an empty list turns it off',
    pets.auth.type === 'apiKey' &&
      pets.auth.key === 'X-API-Key' &&
      requests.find((r) => r.name === 'Add a pet')?.auth.type === 'none' &&
      JSON.parse(requests.find((r) => r.name === 'Add a pet').body).name === 'Rex',
    short({ auth: pets.auth, add: requests.find((r) => r.name === 'Add a pet')?.auth }),
  );
  // Send the imported request against the local server.
  const updated = views.map((v) => ({
    id: v.id,
    key: v.key,
    value: v.key === 'apiKey' ? 'k-123' : v.isSecret ? null : v.value,
    isSecret: v.isSecret,
    enabled: v.enabled,
  }));
  await run(
    `$invoke('save_environment_variables', { id: '${setId}', variables: ${JSON.stringify(updated)} })`,
  );
  await run(`$nav('E2E Pets').click()`);
  await sleep(900);
  await run(
    `[...document.querySelectorAll('.row')].find((r) => r.textContent.includes('Get a pet')).click()`,
  );
  await sleep(800);
  seen.length = 0;
  await run(`$card().querySelector('.send').click()`);
  await waitFor(`!!$json()`, 10000);
  check(
    'an imported request sends with its path variable, query and API key',
    seen[0]?.url === '/v1/pets/42?fields=name' && seen[0]?.key === 'k-123',
    short({ seen }),
  );

  // ---------- Swagger from a URL ----------
  await run(`$nav('Import').click()`);
  await sleep(600);
  await run(
    `$set(document.querySelector('.from-url input'), ${JSON.stringify(`${base}/specs/legacy.json`)})`,
  );
  await run(`document.querySelector('.from-url button[type="submit"]').click()`);
  await waitFor(`!!$btn('Import 1 request')`, 8000);
  const fromUrl = await run(
    `({ source: document.querySelector('.file .hint')?.textContent.trim(), name: document.querySelector('.file-name')?.textContent.trim() })`,
  );
  check(
    'a Swagger 2 spec can be fetched from a URL',
    fromUrl.source.endsWith('Swagger 2.0') && fromUrl.name === 'legacy.json',
    short(fromUrl),
  );
  await run(`$btn('Import 1 request').click()`);
  await waitFor(`!!$nav('E2E Legacy')`, 8000);
  const legacy = await run(`$detail('E2E Legacy')`);
  check(
    'the Swagger operation is imported with its path variable',
    legacy?.requests[0]?.url === '{{baseUrl}}/users/:id' && legacy.requests[0].method === 'DELETE',
    short(legacy?.requests),
  );
  await run(`$nav('Import').click()`);
  await sleep(500);
  await run(`$set(document.querySelector('.from-url input'), 'http://example.com/spec.json')`);
  await run(`document.querySelector('.from-url button[type="submit"]').click()`);
  await waitFor(`!!document.querySelector('.panel .error')`, 5000);
  check(
    'plain http is refused for other hosts',
    (await run(`document.querySelector('.panel .error')?.textContent ?? ''`)).includes('https://'),
    await run(`document.querySelector('.panel .error')?.textContent ?? ''`),
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    await deleteCollection('E2E Pets');
    await deleteCollection('E2E Legacy');
    check('test data cleaned up', !(await run(`!!$nav('E2E Pets') || !!$nav('E2E Legacy')`)));
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  server.close();
  process.exit(0);
}
