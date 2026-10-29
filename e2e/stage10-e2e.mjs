// End-to-end check of stage 10: form-data / URL-encoded / binary bodies, cookie jar, dynamic
// variables, duplicates, Copy as code and Postman export, through the real release build.
import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CDP = 'http://127.0.0.1:9333';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const here = path.dirname(fileURLToPath(import.meta.url));
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'pc-e2e-'));
const uploadPath = path.join(work, 'quarterly-report.txt');
const binaryPath = path.join(work, 'payload.bin');
const exportPath = path.join(work, 'stage10.collection.json');
const globalsPath = path.join(work, 'globals.json');
fs.writeFileSync(uploadPath, 'hello upload from codestamp');
fs.writeFileSync(binaryPath, Buffer.from('BINARY-0123456789'));
const SECRET = 'col-secret-42';

// ---------- local server ----------
const server = http.createServer((req, res) => {
  const chunks = [];
  req.on('data', (chunk) => chunks.push(chunk));
  req.on('end', () => {
    const url = new URL(req.url, 'http://local');
    const body = Buffer.concat(chunks);
    if (url.pathname === '/login') {
      res.writeHead(200, {
        'Content-Type': 'application/json',
        'Set-Cookie': 'sid=abc123; Path=/; HttpOnly',
      });
      return res.end('{"ok":true}');
    }
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(
      JSON.stringify({
        method: req.method,
        path: url.pathname,
        query: url.search,
        contentType: req.headers['content-type'] ?? null,
        contentLength: req.headers['content-length'] ?? null,
        transferEncoding: req.headers['transfer-encoding'] ?? null,
        cookie: req.headers.cookie ?? null,
        bodyLength: body.length,
        bodyText: body.toString('utf8'),
      }),
    );
  });
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;

// ---------- CDP ----------
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
async function realClick(expression) {
  const box = await run(
    `(() => { const r = (${expression}).getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`,
  );
  for (const type of ['mousePressed', 'mouseReleased']) {
    await cdp('Input.dispatchMouseEvent', {
      type,
      x: box.x,
      y: box.y,
      button: 'left',
      clickCount: 1,
    });
  }
}
function answerDialog(title, file) {
  return new Promise((resolve) => {
    const child = spawn('powershell', [
      '-NoProfile',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      path.join(here, 'answer-dialog.ps1'),
      '-Title',
      title,
      '-Path',
      file,
    ]);
    let output = '';
    child.stdout.on('data', (chunk) => (output += chunk));
    child.on('close', () => resolve(output.trim()));
  });
}
function powershell(command) {
  return execFileSync('powershell', ['-NoProfile', '-Command', command], {
    encoding: 'utf8',
  }).trim();
}
const helpers = `
  window.$pick = async (trigger, label) => { trigger.click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 100)); };
  window.$picked = (trigger) => trigger.querySelector('.text').textContent.trim();
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$set = (el, value, type = 'input') => { el.value = value; el.dispatchEvent(new Event(type, { bubbles: true })); };
  window.$card = () => document.querySelector('article.card');
  window.$dialog = () => document.querySelector('dialog[open]:not(.panel-dialog)');
  window.$body = () => document.querySelector('.response .cm-content')?.innerText ?? '';
  window.$json = () => { try { return JSON.parse($body()); } catch { return null; } };
  window.$error = () => document.querySelector('.response .error')?.textContent ?? '';
  window.$tab = (text) => [...$card().querySelectorAll('.request [role=tab]')].find((t) => t.textContent.trim().startsWith(text));
  window.$responseTab = (text) => [...$card().querySelectorAll('.response [role=tab]')].find((t) => t.textContent.trim().startsWith(text));
  window.$mode = (text) => [...document.querySelectorAll('[aria-label="Body type"] button')].find((b) => b.textContent.trim() === text);
  window.$formRows = () => [...document.querySelectorAll('section[aria-label="Form fields"] tbody tr')];
  window.$row = (name) => [...document.querySelectorAll('.row')].find((r) => r.querySelector('.name')?.textContent.trim() === name);
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav .item')].find((b) => (b.querySelector('.name')?.textContent ?? b.textContent).trim().startsWith(text)) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$banner = () => document.querySelector('.banner')?.textContent ?? '';
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 180);

async function addRequest(name, url) {
  await run(`(document.querySelector('.adder') ?? document.querySelector('.start .tile')).click()`);
  await sleep(500);
  await run(`$set($card().querySelector('.name input'), ${JSON.stringify(name)})`);
  await run(`$set(document.querySelector('input[id^="url-"]'), ${JSON.stringify(url)})`);
  await sleep(200);
}
async function send() {
  await run(`$card().querySelector('.send').click()`);
  await sleep(300);
  await waitFor(`!$card().querySelector('.send').disabled`, 15000);
  await sleep(300);
}
async function collapse() {
  await run(`$card().querySelector('button[aria-label="Collapse request"]').click()`);
  await sleep(700);
}
async function openRow(name) {
  await run(`$row(${JSON.stringify(name)}).click()`);
  await sleep(700);
}
async function openCollection(name) {
  await run(`$nav(${JSON.stringify(name)}).click()`);
  await sleep(900);
}
async function deleteCollection(name) {
  const exists = await run(`!!$nav(${JSON.stringify(name)})`);
  if (!exists) return;
  await openCollection(name);
  if ((await run(`document.querySelector('input[aria-label="Collection name"]')?.value`)) !== name)
    return;
  await run(`$btn('Delete collection').click()`);
  // Under load the confirmation can take a moment to open.
  await waitFor(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].some((b) => b.textContent.trim() === 'Delete')`,
    5000,
  );
  await run(
    `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Delete').click()`,
  );
  await waitFor(`!$nav(${JSON.stringify(name)})`, 5000);
}
async function invokeSend(body) {
  return run(
    `window.__TAURI_INTERNALS__.invoke('send_http_request', { request: { method: 'POST', url: '${base}/echo', params: [], headers: [], body: ${JSON.stringify(body)}, timeoutMs: null, collectionId: null, requestId: null, name: null, preRequestScript: null, testScript: null, auth: null, recordHistory: false } }).then(() => 'sent', (e) => e.message)`,
  );
}

await connect();
try {
  await run(`(async () => {
    const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    document.querySelector('nav button.new').click();
    await wait(400);
    const input = document.getElementById('new-collection-name');
    input.value = 'E2E temp';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await wait(300);
    [...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Create collection').click();
    await wait(900);
  })()`);
  await sleep(700);
  await run(
    `(() => { const t = document.querySelector('input[aria-label="Collection name"]'); $set(t, 'E2E Stage10'); t.dispatchEvent(new Event('blur')); })()`,
  );
  await sleep(400);

  // collection variables: baseUrl + a secret (for the duplicate check)
  await run(`$btn('Variables').click()`);
  await sleep(600);
  const vars = `document.querySelectorAll('section[aria-label="Collection variables"] tbody tr')`;
  await run(`$set(${vars}[0].querySelector('input[aria-label="Variable name"]'), 'baseUrl')`);
  await run(`$set(${vars}[0].querySelector('input[aria-label="Variable value"]'), '${base}')`);
  await sleep(200);
  await run(`$set(${vars}[1].querySelector('input[aria-label="Variable name"]'), 'apiKey')`);
  await sleep(200);
  await run(`$pick(${vars}[1].querySelector('button[aria-label="Variable type"]'), 'Secret')`);
  await sleep(200);
  await run(`$set(${vars}[1].querySelector('input[aria-label="Variable value"]'), '${SECRET}')`);
  await sleep(200);
  await run(
    `$btn('Save', document.querySelector('section[aria-label="Collection variables"]')).click()`,
  );
  await sleep(700);
  await run(
    `$btn('Close', document.querySelector('section[aria-label="Collection variables"]')).click()`,
  );
  await sleep(300);

  // ---------- form data with a picked file ----------
  await addRequest('Form upload', '{{baseUrl}}/echo');
  await run(`$pick($card().querySelector('button[id^="method-"]'), 'POST')`);
  await run(`$tab('Body').click()`);
  await sleep(200);
  await run(`$mode('Form data').click()`);
  await sleep(300);
  await run(`$set($formRows()[0].querySelector('input[aria-label="Form fields key"]'), 'title')`);
  await sleep(150);
  await run(
    `$set($formRows()[0].querySelector('input[aria-label="Form fields value"]'), 'Quarterly')`,
  );
  await sleep(150);
  await run(`$set($formRows()[1].querySelector('input[aria-label="Form fields key"]'), 'doc')`);
  await sleep(150);
  await run(`$pick($formRows()[1].querySelector('button[aria-label="Form fields type"]'), 'File')`);
  await sleep(300);
  const picked = answerDialog('Choose a file to upload', uploadPath);
  await run(`$formRows()[1].querySelector('button[aria-label="Form fields file"]').click()`);
  const pickAnswer = await picked;
  await waitFor(
    `$formRows()[1].querySelector('button[aria-label="Form fields file"]').textContent.includes('quarterly-report.txt')`,
  );
  const fileLabel = await run(
    `$formRows()[1].querySelector('button[aria-label="Form fields file"]').textContent.trim()`,
  );
  check(
    'form file is picked through the system dialog',
    pickAnswer === 'answered' && fileLabel === 'quarterly-report.txt',
    `${pickAnswer} · ${fileLabel}`,
  );
  await send();
  let echo = await run('$json()');
  check(
    'form data is sent as multipart with the file',
    echo?.contentType?.startsWith('multipart/form-data; boundary=') &&
      echo.bodyText.includes('name="title"') &&
      echo.bodyText.includes('Quarterly') &&
      echo.bodyText.includes('filename="quarterly-report.txt"') &&
      echo.bodyText.includes('hello upload from codestamp') &&
      echo.transferEncoding === null,
    short({
      contentType: echo?.contentType,
      length: echo?.contentLength,
      te: echo?.transferEncoding,
    }),
  );

  const stolen = await invokeSend({ mode: 'binary', path: 'C:\\Windows\\win.ini' });
  check(
    'a file that was not picked cannot be uploaded',
    String(stolen).includes('only files picked on this device'),
    stolen,
  );
  const stolenForm = await invokeSend({
    mode: 'formData',
    parts: [{ name: 'x', value: 'C:\\Windows\\win.ini', file: true }],
  });
  check(
    'form parts cannot point at unpicked files either',
    String(stolenForm).includes('only files picked'),
    stolenForm,
  );
  const injected = await invokeSend({
    mode: 'formData',
    parts: [{ name: 'a\r\nX-Evil: 1', value: 'v', file: false }],
  });
  check(
    'form field names with line breaks are refused',
    String(injected).includes('line breaks'),
    injected,
  );

  // ---------- URL-encoded shares the rows; file rows are left out ----------
  await run(`$mode('URL-encoded').click()`);
  await sleep(400);
  const note = await run(
    `document.querySelector('section[aria-label="Form fields"] .note')?.textContent ?? ''`,
  );
  check(
    'URL-encoded mode explains that file rows are skipped',
    note.includes('only sent as form data'),
    note,
  );
  await send();
  echo = await run('$json()');
  check(
    'URL-encoded body is sent',
    echo?.contentType === 'application/x-www-form-urlencoded' &&
      echo.bodyText === 'title=Quarterly',
    short({ ct: echo?.contentType, body: echo?.bodyText }),
  );

  // ---------- binary file body ----------
  await run(`$mode('Binary').click()`);
  await sleep(300);
  const pickedBinary = answerDialog('Choose a file to upload', binaryPath);
  await run(`$btn('Choose file', $card()).click()`);
  await pickedBinary;
  await waitFor(`$card().innerText.includes('payload.bin')`);
  await send();
  echo = await run('$json()');
  check(
    'binary file is streamed with its length',
    echo?.contentType === 'application/octet-stream' &&
      echo.contentLength === '17' &&
      echo.bodyText === 'BINARY-0123456789' &&
      echo.transferEncoding === null,
    short({ ct: echo?.contentType, len: echo?.contentLength, body: echo?.bodyText }),
  );
  await run(`$mode('Form data').click()`);
  await sleep(900);
  await collapse();
  await openRow('Form upload');
  await run(`$tab('Body').click()`);
  await sleep(200);
  const reopened = await run(
    `({ mode: [...document.querySelectorAll('[aria-label="Body type"] button')].find((b) => b.classList.contains('on'))?.textContent.trim(), file: $formRows()[1]?.querySelector('button[aria-label="Form fields file"]')?.textContent.trim() })`,
  );
  check(
    'form rows and the picked file are saved',
    reopened.mode === 'Form data' && reopened.file === 'quarterly-report.txt',
    short(reopened),
  );

  // ---------- duplicate request ----------
  await run(`$card().querySelector('button[aria-label="Duplicate request"]').click()`);
  await sleep(400);
  await run(`$btn('Duplicate', $dialog()).click()`);
  await sleep(1200);
  const afterDuplicate = await run(
    `({ card: $card()?.querySelector('.name input').value, rows: [...document.querySelectorAll('.row .name')].map((n) => n.textContent.trim()) })`,
  );
  check(
    'Duplicate opens the copy right after the original',
    afterDuplicate.card === 'Form upload copy',
    short(afterDuplicate),
  );
  await collapse();
  const order = await run(
    `[...document.querySelectorAll('.row .name')].map((n) => n.textContent.trim())`,
  );
  check(
    'copy sits right after the original',
    order.join('|') === 'Form upload|Form upload copy',
    order.join(' | '),
  );

  // ---------- dynamic variables ----------
  await addRequest('Dynamic', '{{baseUrl}}/echo?id={{$guid}}&n={{$randomInt}}');
  await sleep(700);
  const preview = await run(`$card().querySelector('.preview')?.textContent.trim() ?? ''`);
  check(
    'preview keeps dynamic variables and does not flag them',
    preview.startsWith('Resolves to') && preview.includes('{{$guid}}'),
    preview,
  );
  await send();
  echo = await run('$json()');
  const id = new URLSearchParams(echo?.query ?? '').get('id') ?? '';
  const n = Number(new URLSearchParams(echo?.query ?? '').get('n'));
  check(
    '{{$guid}} and {{$randomInt}} are filled in when sending',
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id) &&
      n >= 0 &&
      n <= 1000,
    echo?.query,
  );
  await run(
    `(() => { const u = document.querySelector('input[id^="url-"]'); u.focus(); $set(u, '{{$gu'); u.setSelectionRange(5, 5); u.dispatchEvent(new Event('input', { bubbles: true })); })()`,
  );
  await sleep(500);
  const suggestions = await run(
    `[...document.querySelectorAll('[role="option"]')].map((o) => o.textContent.replace(/\\s+/g, ' ').trim())`,
  );
  check(
    'autocomplete offers dynamic variables',
    suggestions.some((s) => s.includes('$guid') && s.includes('Dynamic')),
    short(suggestions),
  );
  await run(
    `$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/echo?who={{$randomFirstName}}')`,
  );
  await sleep(300);
  await send();
  check(
    'unsupported dynamic variables are reported',
    (await run('$error()')).includes('not a dynamic variable Codestamp supports'),
    await run('$error()'),
  );
  await run(`$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/echo?id={{$guid}}')`);
  await sleep(900);

  // ---------- Copy as code ----------
  await collapse();
  await openRow('Form upload');
  const openCode = async () => {
    await run(`$card().querySelector('button[aria-label="Copy as code"]').click()`);
    await waitFor(`!!$dialog()?.querySelector('.cm-content')`);
    await sleep(300);
  };
  const codeText = () => run(`$dialog().querySelector('.cm-content').innerText`);
  const pickIn = (picker, label) =>
    run(
      `$pick($dialog().querySelector('button[aria-label="${picker}"]'), ${JSON.stringify(label)})`,
    );
  const pickedIn = (picker) =>
    run(`$picked($dialog().querySelector('button[aria-label="${picker}"]'))`);
  await openCode();
  const languages = await run(
    `(async () => { const trigger = $dialog().querySelector('button[aria-label="Language"]'); trigger.click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); const options = [...document.querySelectorAll('[role="listbox"] [role="option"]')]; const labels = options.map((o) => o.textContent.trim()); options.find((o) => o.textContent.trim() === 'cURL').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); return labels; })()`,
  );
  await sleep(300);
  const controls = await run(
    `({ language: $picked($dialog().querySelector('button[aria-label="Language"]')), values: $picked($dialog().querySelector('button[aria-label="Values"]')), hint: $dialog().querySelector('.hint')?.textContent.trim() ?? '', buttons: $dialog().querySelectorAll('.segmented').length })`,
  );
  check(
    'language and values are dropdowns, with fifteen languages and environment variables first',
    JSON.stringify(languages) ===
      JSON.stringify([
        'cURL',
        'HTTPie',
        'PowerShell',
        'JavaScript fetch',
        'JavaScript axios',
        'Python requests',
        'PHP Guzzle',
        'Go net/http',
        'Java HttpClient',
        'Kotlin OkHttp',
        'C# HttpClient',
        'Dart http',
        'Swift URLSession',
        'Ruby Net::HTTP',
        'Rust reqwest',
      ]) &&
      controls.language === 'cURL' &&
      controls.values === 'Environment variables' &&
      controls.buttons === 0 &&
      controls.hint.startsWith('Reads BASE_URL from environment variables'),
    JSON.stringify({ languages, controls }).slice(0, 300),
  );
  const curlCode = await codeText();
  check(
    'cURL reads variables from the environment and uses --form for files',
    curlCode.startsWith('# Environment variables: BASE_URL') &&
      curlCode.includes('curl --request POST "${BASE_URL}/echo"') &&
      curlCode.includes("--form-string 'title=Quarterly'") &&
      curlCode.includes(`--form 'doc=@"`) &&
      !curlCode.toLowerCase().includes('content-type: multipart'),
    curlCode.replace(/\s+/g, ' ').slice(0, 200),
  );
  await pickIn('Language', 'JavaScript fetch');
  await sleep(300);
  const fetchCode = await codeText();
  check(
    'fetch code builds a FormData and reads the file in Node',
    fetchCode.includes('import { readFile } from "node:fs/promises";') &&
      fetchCode.includes('const baseUrl = process.env.BASE_URL;') &&
      fetchCode.includes('const form = new FormData();') &&
      fetchCode.includes('form.append("title", "Quarterly");'),
    fetchCode.replace(/\s+/g, ' ').slice(0, 200),
  );
  await pickIn('Language', 'Python requests');
  await sleep(300);
  const pythonCode = await codeText();
  check(
    'Python code sends files with requests',
    pythonCode.includes('import requests') &&
      pythonCode.includes('base_url = os.environ["BASE_URL"]') &&
      pythonCode.includes('files = [') &&
      pythonCode.includes('("title", (None, "Quarterly"))'),
    pythonCode.replace(/\s+/g, ' ').slice(0, 200),
  );
  await pickIn('Language', 'Go net/http');
  await pickIn('Values', 'As written, {{name}}');
  await sleep(300);
  const goCode = await codeText();
  await run(
    `[...$dialog().querySelectorAll('button')].find((b) => (b.getAttribute('aria-label') ?? '') === 'Close').click()`,
  );
  await sleep(300);
  await openCode();
  const remembered = { language: await pickedIn('Language'), values: await pickedIn('Values') };
  check(
    'the language is remembered; values start on environment variables again',
    goCode.includes('package main') &&
      goCode.includes('"{{baseUrl}}/echo"') &&
      remembered.language === 'Go net/http' &&
      remembered.values === 'Environment variables',
    JSON.stringify({ remembered, go: goCode.replace(/\s+/g, ' ').slice(0, 120) }),
  );
  await pickIn('Language', 'cURL');
  await pickIn('Values', 'Filled in (shows secrets)');
  await waitFor(`$dialog().querySelector('.cm-content').innerText.includes('${base}')`);
  const resolvedCode = await codeText();
  check(
    '"Filled in" resolves through the Rust core',
    resolvedCode.includes(`'${base}/echo'`) && !resolvedCode.includes('{{baseUrl}}'),
    resolvedCode.replace(/\s+/g, ' ').slice(0, 160),
  );
  // Windows refuses the clipboard to every program while the screen is locked.
  let clipboardReady = true;
  try {
    powershell('Set-Clipboard -Value "before"');
  } catch {
    clipboardReady = false;
  }
  if (clipboardReady) {
    await realClick(
      `[...$dialog().querySelectorAll('button')].find((x) => x.textContent.trim() === 'Copy')`,
    );
    await sleep(800);
    const clipboard = powershell('Get-Clipboard -Raw');
    const copiedStatus = await run(`$dialog().querySelector('.status')?.textContent.trim() ?? ''`);
    check(
      'Copy puts the code on the clipboard',
      clipboard.includes(`curl --request POST '${base}/echo'`) && copiedStatus.startsWith('Copied'),
      `${copiedStatus} · ${clipboard.slice(0, 60)}`,
    );
  } else {
    results.push(
      'SKIP  Copy puts the code on the clipboard: Windows denies the clipboard (screen locked?)',
    );
  }
  await run(`$dialog().querySelector('button[aria-label="Close"]').click()`);
  await sleep(300);
  await collapse();

  // ---------- cookie jar ----------
  await addRequest('Login', '{{baseUrl}}/login');
  await send();
  const cookieTab = await run(`$responseTab('Cookies')?.textContent.trim() ?? ''`);
  await run(`$responseTab('Cookies')?.click()`);
  await sleep(200);
  const cookieRow = await run(
    `$card().querySelector('.response .cookies tbody tr')?.innerText.replace(/\\s+/g, ' ') ?? ''`,
  );
  check(
    'response Cookies tab shows the cookie it set',
    cookieTab === 'Cookies · 1' &&
      cookieRow.includes('sid') &&
      cookieRow.includes('abc123') &&
      cookieRow.includes('HttpOnly'),
    `${cookieTab} | ${cookieRow}`,
  );
  await collapse();
  await addRequest('Me', '{{baseUrl}}/me');
  await send();
  echo = await run('$json()');
  check('the jar sends the cookie back', echo?.cookie === 'sid=abc123', short(echo?.cookie));
  await run(`$tab('Params').click()`);
  await sleep(150);
  await run(`$set($card().querySelector('input[aria-label="Headers key"]'), 'Cookie')`);
  await run(`$set($card().querySelector('input[aria-label="Headers value"]'), 'sid=typed')`);
  await sleep(200);
  await send();
  echo = await run('$json()');
  check(
    'a typed Cookie header wins over the jar',
    echo?.cookie === 'sid=typed',
    short(echo?.cookie),
  );
  await run(`$set($card().querySelector('input[aria-label="Headers key"]'), '')`);
  await run(`$set($card().querySelector('input[aria-label="Headers value"]'), '')`);
  await sleep(900);
  await collapse();

  await run(`$nav('Cookies').click()`);
  await sleep(800);
  const group = `document.querySelector('section[aria-label="Cookies for 127.0.0.1"]')`;
  const listed = await run(`${group}?.innerText.replace(/\\s+/g, ' ') ?? ''`);
  check(
    'Cookies view lists the cookie with its value hidden',
    listed.includes('sid') && listed.includes('••••') && !listed.includes('abc123'),
    listed.slice(0, 160),
  );
  await run(`$btn('Show', ${group}).click()`);
  await sleep(400);
  check('Show reveals the value', (await run(`${group}.innerText`)).includes('abc123'));

  // ---------- duplicate section + collection, export ----------
  await openCollection('E2E Stage10');
  await run(
    `[...document.querySelectorAll('.adder')].find((b) => b.textContent.includes('Add section')).click()`,
  );
  await sleep(600);
  await run(`$btn('Add request', document.querySelector('.section-head')).click()`);
  await sleep(600);
  await run(`$set($card().querySelector('.name input'), 'Inside')`);
  await run(`$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/echo')`);
  await sleep(900);
  await collapse();
  await run(`$btn('Duplicate', document.querySelector('.section-head')).click()`);
  await sleep(400);
  await run(`$btn('Duplicate', $dialog()).click()`);
  await sleep(1200);
  const sections = await run(
    `[...document.querySelectorAll('.section-head input[aria-label="Section name"], .section-head .editable')].map((e) => e.value ?? e.textContent.trim())`,
  );
  const insideCount = await run(
    `[...document.querySelectorAll('.row .name')].filter((n) => n.textContent.trim() === 'Inside').length`,
  );
  check(
    'Duplicate section copies it with its requests',
    sections.some((s) => String(s).includes('New section copy')) && insideCount === 2,
    `${short(sections)} · Inside ×${insideCount}`,
  );
  const totalRequests = await run(`document.querySelectorAll('.row').length`);

  await run(`$btn('Export collection').click()`);
  await waitFor(`!!document.querySelector('section[aria-label="Export"]')`, 5000);
  await run(
    `(async () => { const panel = document.querySelector('section[aria-label="Export"]'); panel.querySelector('button[aria-label="Export format"]').click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === 'Postman Collection v2.1').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); })()`,
  );
  await sleep(200);
  const exportAnswer = answerDialog('Export collection', exportPath);
  await run(`$btn('Export…').click()`);
  const exportResult = await exportAnswer;
  await waitFor(`!!document.querySelector('.exported')`);
  const exportedNote = await run(
    `document.querySelector('.exported')?.innerText.replace(/\\s+/g, ' ') ?? ''`,
  );
  let exportedFile = null;
  if (fs.existsSync(exportPath)) exportedFile = JSON.parse(fs.readFileSync(exportPath, 'utf8'));
  const exportedText = exportedFile ? JSON.stringify(exportedFile) : '';
  check(
    'Export writes a Postman v2.1 collection',
    exportedFile?.info?.schema ===
      'https://schema.getpostman.com/json/collection/v2.1.0/collection.json' &&
      exportedText.includes('"mode":"formdata"') &&
      exportedText.includes('"src":"quarterly-report.txt"'),
    `${exportResult} · items ${exportedFile?.item?.length}`,
  );
  check(
    'export leaves out local folders and secret values',
    !exportedText.includes(work.replaceAll('\\', '\\\\')) &&
      !exportedText.includes(SECRET) &&
      exportedNote.includes('secret value'),
    exportedNote.slice(0, 200),
  );

  await run(`$nav('Import').click()`);
  await sleep(600);
  const handle = await cdp('Runtime.evaluate', {
    expression: `document.querySelector('input[type="file"]')`,
  });
  await cdp('DOM.getDocument', {});
  await cdp('DOM.setFileInputFiles', {
    files: [exportPath],
    objectId: handle.result.result.objectId,
  });
  await waitFor(`!!$btn('Import ')`, 10000);
  const stats = await run(
    `[...document.querySelectorAll('.stat')].map((s) => s.innerText.replace(/\\n/g, ' ')).join(', ')`,
  );
  await run(`$set(document.querySelector('.field input'), 'E2E Stage10 (imported)')`);
  await run(
    `[...document.querySelectorAll('button')].find((b) => /^Import \\d+ requests$/.test(b.textContent.trim())).click()`,
  );
  await sleep(1500);
  const importedRows = await run(`document.querySelectorAll('.row').length`);
  check(
    'the export imports back into Codestamp',
    importedRows === totalRequests,
    `${stats} · rows ${importedRows}/${totalRequests}`,
  );
  await deleteCollection('E2E Stage10 (imported)');

  await openCollection('E2E Stage10');
  await run(`$btn('Duplicate', document.querySelector('header.top')).click()`);
  await sleep(400);
  await run(`$btn('Duplicate', $dialog()).click()`);
  await sleep(1500);
  const copyTitle = await run(
    `document.querySelector('input[aria-label="Collection name"]')?.value`,
  );
  const copyRows = await run(`document.querySelectorAll('.row').length`);
  await run(`$btn('Variables').click()`);
  await sleep(700);
  const copiedVars = `document.querySelectorAll('section[aria-label="Collection variables"] tbody tr')`;
  await run(`${copiedVars}[1].querySelector('button[aria-label="Show value"]')?.click()`);
  await sleep(500);
  const copiedSecret = await run(
    `${copiedVars}[1].querySelector('input[aria-label="Variable value"]').value`,
  );
  check(
    'Duplicate collection copies requests and secret variables',
    copyTitle === 'E2E Stage10 copy' && copyRows === totalRequests && copiedSecret === SECRET,
    `${copyTitle} · ${copyRows} rows · secret ${copiedSecret === SECRET ? 'ok' : copiedSecret}`,
  );
  await run(
    `$btn('Close', document.querySelector('section[aria-label="Collection variables"]')).click()`,
  );
  await sleep(300);

  await run(`$nav('Environments').click()`);
  await sleep(600);
  await run(
    `[...document.querySelectorAll('.env')].find((b) => b.textContent.includes('Globals')).click()`,
  );
  await sleep(400);
  const globalsAnswer = answerDialog('Export environment', globalsPath);
  await run(`$btn('Export', document.querySelector('.panel-head')).click()`);
  await globalsAnswer;
  await waitFor(`!!document.querySelector('.exported')`);
  const globalsFile = fs.existsSync(globalsPath)
    ? JSON.parse(fs.readFileSync(globalsPath, 'utf8'))
    : null;
  check(
    'Globals export as a Postman globals file',
    globalsFile?._postman_variable_scope === 'globals' && Array.isArray(globalsFile.values),
    short(globalsFile && { name: globalsFile.name, scope: globalsFile._postman_variable_scope }),
  );

  // ---------- the jar survives a restart (stored encrypted) ----------
  socket.close();
  // Found by path: the user's own Codestamp may run under the same process name.
  const pid = Number(
    powershell(
      `. '${path.join(here, 'test-process.ps1')}'; (Get-TestProcess | Select-Object -First 1).Id`,
    ),
  );
  if (!pid) throw new Error('the test copy is not running');
  process.kill(pid);
  await sleep(2000);
  execFileSync('node', [path.join(here, 'launch.mjs')], { stdio: 'ignore' });
  await connect();
  await run(`$nav('Cookies').click()`);
  await sleep(800);
  check(
    'cookies are still there after a restart',
    (await run(`${group}?.innerText ?? ''`)).includes('sid'),
  );
  const dbFiles = fs
    .readdirSync(path.join(process.env.APPDATA, 'dev.codestamp.desktop'))
    .filter((f) => f.startsWith('codestamp.db'));
  const leaked = dbFiles.some((f) =>
    fs.readFileSync(path.join(process.env.APPDATA, 'dev.codestamp.desktop', f)).includes('abc123'),
  );
  check('the cookie value is not stored in plain text', !leaked, dbFiles.join(', '));
  await run(`${group}.querySelector('button[aria-label="Delete cookie sid"]').click()`);
  await sleep(700);
  check('a cookie can be deleted', !(await run(`!!${group}`)));
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(
      `document.querySelector('dialog[open]:not(.panel-dialog) button[aria-label="Close"]')?.click()`,
    );
    await deleteCollection('E2E Stage10 copy');
    await deleteCollection('E2E Stage10 (imported)');
    await deleteCollection('E2E Stage10');
    await run(`$nav('Cookies').click()`);
    await sleep(600);
    if (await run(`!!$btn('Clear all') && !$btn('Clear all').disabled`)) {
      await run(`$btn('Clear all').click()`);
      await sleep(300);
      await run(
        `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => b.textContent.trim() === 'Delete').click()`,
      );
      await sleep(600);
    }
    const left = await run(
      `[...document.querySelectorAll('nav .item')].some((b) => b.textContent.includes('E2E Stage10'))`,
    );
    check('test data cleaned up', !left);
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  console.log(`\nhistory search term for cleanup: 127.0.0.1:${server.address().port}`);
  fs.rmSync(work, { recursive: true, force: true });
  socket.close();
  server.close();
  process.exit(0);
}
