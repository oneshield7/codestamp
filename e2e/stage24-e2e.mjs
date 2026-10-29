// End-to-end check of stage 18: XML, HTML and Binary bodies, formatted XML/HTML responses, SOAP
// requests from a WSDL (imports, SOAP 1.1 and 1.2, faults, xml2Json in tests), importing a WSDL
// as a collection, and the Home tile tooltip fix. Creates "E2E SOAP" and "Calculator", then
// deletes both.
import { startSoapServer } from './soap-server.mjs';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E SOAP';
const IMPORTED = 'Calculator';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const { server, received, base } = await startSoapServer();
let other = null;

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
async function waitFor(expression, timeout = 10000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await run(expression)) return true;
    await sleep(200);
  }
  return false;
}
async function typeIn(selector, text) {
  await run(`(() => { const el = ${selector}; el.focus(); })()`);
  await cdp('Input.dispatchKeyEvent', {
    type: 'rawKeyDown',
    key: 'a',
    code: 'KeyA',
    windowsVirtualKeyCode: 65,
    modifiers: 2,
  });
  await cdp('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: 'a',
    code: 'KeyA',
    windowsVirtualKeyCode: 65,
    modifiers: 2,
  });
  await cdp('Input.insertText', { text });
  await sleep(400);
}
async function hover(selector) {
  const point = await run(
    `(() => { const r = ${selector}.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`,
  );
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: point.x, y: point.y });
  await sleep(250);
}
const helpers = `
  window.$btn = (text, root = document) => [...root.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(text) || (b.getAttribute('aria-label') ?? '').startsWith(text));
  window.$nav = (text) => document.querySelector('footer.statusbar button[aria-label="' + text + '"]') ?? [...document.querySelectorAll('nav.sidebar .item')].find((b) => b.querySelector('.name')?.textContent.trim() === text) ?? window.$goTo(text);
  window.$goTo = (text) => ['Environments', 'Cookies', 'History', 'Import', 'Settings'].includes(text) ? { click: () => { document.querySelector('nav.sidebar button.search').click(); setTimeout(() => [...document.querySelectorAll('[role="group"][aria-label="Go to"] .result')].find((r) => r.querySelector('.title')?.textContent.trim() === text)?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })), 80); } } : undefined;
  window.$invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  window.$card = () => document.querySelector('article.card');
  window.$tab = (label) => [...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith(label));
  window.$set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  window.$editor = () => $card().querySelector('.body-editor .cm-content');
  window.$editorText = () => $editor().cmView?.view?.state.doc.toString() ?? [...$editor().querySelectorAll('.cm-line')].map((l) => l.textContent).join('\\n');
  window.$response = () => [...($card().querySelector('.response .cm-content')?.querySelectorAll('.cm-line') ?? [])].map((l) => l.textContent).join('\\n');
  window.$segment = (label) => [...$card().querySelectorAll('.segmented button')].find((b) => b.textContent.trim() === label);
  window.$pick = async (trigger, label) => { trigger.click(); await new Promise((r) => setTimeout(r, 150)); const option = [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === label); option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 150)); };
  window.$soapAction = () => [...$card().querySelectorAll('label')].find((l) => l.textContent.trim().startsWith('SOAPAction'))?.querySelector('input');
  window.$hub = (label) => [...document.querySelectorAll('.home [role="tab"]')].find((t) => t.textContent.trim() === label);
  window.$homeTile = (name) => [...document.querySelectorAll('.home .tiles input')].find((i) => i.getAttribute('aria-label') === name)?.closest('label');
`;
const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 400);
async function send() {
  await run(`$card().querySelector('.send').click()`);
  await sleep(300);
  await waitFor(`!$card().querySelector('.send').disabled`, 15000);
  await sleep(400);
}
async function openCollection(name) {
  await run(`$nav(${JSON.stringify(name)}).click()`);
  await sleep(900);
}
async function deleteCollection(name) {
  if (!(await run(`!!$nav(${JSON.stringify(name)})`))) return;
  await openCollection(name);
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
  await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await openCollection(NAME);

  // ---------- XML, HTML and Binary bodies ----------
  await run(
    `[...document.querySelectorAll('.start .tile')].find((t) => t.textContent.includes('HTTP request')).click()`,
  );
  await waitFor(`!!$card()`, 4000);
  await run(`$tab('Body').click()`);
  await sleep(300);
  const modes = await run(
    `[...$card().querySelectorAll('.body .segmented button')].map((b) => b.textContent.trim())`,
  );
  check(
    'the body offers XML, HTML and Binary',
    ['XML', 'HTML', 'Binary'].every((mode) => modes.includes(mode)) && !modes.includes('File'),
    short(modes),
  );
  await run(`$segment('XML').click()`);
  await sleep(300);
  await typeIn(`$editor()`, '<a><b>1</b><c x="1"/></a>');
  await run(`$btn('Beautify', $card()).click()`);
  await sleep(300);
  const xmlText = await run(`$editorText()`);
  const sent = await run(`(() => { const tab = $tab('Body').textContent.trim(); return tab; })()`);
  check(
    'Beautify indents XML',
    xmlText === '<a>\n  <b>1</b>\n  <c x="1"/>\n</a>' && sent === 'Body · XML',
    short({ xmlText, sent }),
  );
  await typeIn(`$editor()`, '<a><b></a>');
  await sleep(400);
  const problem = await run(`$card().querySelector('.body .problem')?.textContent.trim() ?? ''`);
  check('broken XML says where', /^Line \d+, column \d+:/.test(problem), problem);
  await run(`$segment('HTML').click()`);
  await sleep(300);
  await typeIn(`$editor()`, '<div><p>Hi</p><br><script>if (a<b) {}</script></div>');
  await run(`$btn('Beautify', $card()).click()`);
  await sleep(300);
  const htmlText = await run(`$editorText()`);
  check(
    'Beautify indents HTML and keeps scripts as written',
    htmlText === '<div>\n  <p>Hi</p>\n  <br>\n  <script>if (a<b) {}</script>\n</div>',
    short(htmlText),
  );
  await run(`$segment('None').click()`);
  await sleep(200);

  // ---------- formatted XML and HTML responses ----------
  await run(`$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/plain.xml')`);
  await sleep(300);
  await send();
  const xmlResponse = await run(`$response()`);
  check(
    'an XML response is shown indented',
    xmlResponse === '<?xml version="1.0"?>\n<root>\n  <a x="1">t</a>\n  <b/>\n</root>',
    short(xmlResponse),
  );
  await run(`$set(document.querySelector('input[id^="url-"]'), '{{baseUrl}}/page.html')`);
  await sleep(300);
  await send();
  const htmlResponse = await run(`$response()`);
  check(
    'an HTML response is shown indented',
    htmlResponse.startsWith('<!DOCTYPE html>\n<html>\n  <head>\n    <title>T</title>') &&
      htmlResponse.includes('\n    <p>Hi</p>\n'),
    short(htmlResponse),
  );
  await run(`$card().querySelector('button[aria-label="Collapse request"]').click()`);
  await sleep(500);

  // ---------- a SOAP request from a WSDL ----------
  await run(
    `[...document.querySelectorAll('button.adder')].find((b) => b.textContent.trim() === 'Add SOAP').click()`,
  );
  await waitFor(`!!$card()?.querySelector('.method.soap')`, 4000);
  const soapCard = await run(
    `({ stamp: $card().querySelector('.method.soap')?.textContent.trim(), tabs: [...$card().querySelectorAll('[role="tab"]')].map((t) => t.textContent.trim()) })`,
  );
  check(
    'Add SOAP makes a SOAP card with an Envelope tab',
    soapCard.stamp === 'SOAP' && soapCard.tabs.includes('Envelope'),
    short(soapCard),
  );
  await run(`$tab('Envelope').click()`);
  await sleep(300);
  await run(
    `$set($card().querySelector('input[placeholder^="WSDL URL"]'), ${JSON.stringify(`${base}/calculator.asmx?wsdl`)})`,
  );
  await run(`$btn('Load WSDL', $card()).click()`);
  await waitFor(
    `$card().querySelector('button[aria-label="Operation"] .text')?.textContent.includes('CalculatorSoap')`,
    10000,
  );
  await sleep(400);
  const loaded = await run(`({
    operation: $card().querySelector('button[aria-label="Operation"] .text')?.textContent.trim(),
    name: $card().querySelector('.name input')?.value,
    url: document.querySelector('input[id^="url-"]')?.value,
    action: $soapAction()?.value,
    version: [...$card().querySelectorAll('[aria-label="SOAP version"] button')].find((b) => b.getAttribute('aria-pressed') === 'true')?.textContent.trim(),
    doc: $card().querySelector('.soap .hint')?.textContent.trim(),
    envelope: $editorText(),
  })`);
  check(
    'Load WSDL picks the first operation with its sample, endpoint and action',
    loaded.operation === 'Add · CalculatorSoap · SOAP 1.1' &&
      loaded.name === 'Add' &&
      loaded.url === `${base}/calculator.asmx` &&
      loaded.action === 'http://tempuri.org/Add' &&
      loaded.version === '1.1' &&
      loaded.doc === 'Adds intA and intB.',
    short(loaded),
  );
  check(
    'the sample envelope reads the types from the imported XSD',
    loaded.envelope.includes(
      '<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"',
    ) &&
      loaded.envelope.includes('<tem:intA>?</tem:intA>') &&
      loaded.envelope.includes('<tem:intB>?</tem:intB>'),
    short(loaded.envelope),
  );

  await send();
  const faulted = await run(
    `({ chip: [...$card().querySelectorAll('.response .chip')].map((c) => c.textContent.trim()), alert: $card().querySelector('.response .error')?.textContent.trim() })`,
  );
  check(
    'a SOAP Fault is pointed out',
    faulted.chip.includes('SOAP Fault') &&
      faulted.chip.some((c) => c.startsWith('500')) &&
      faulted.alert?.startsWith('soap:Client: Server was unable'),
    short(faulted),
  );
  check(
    'SOAP 1.1 sends text/xml with a quoted SOAPAction',
    received.at(-1)?.contentType === 'text/xml; charset=utf-8' &&
      received.at(-1)?.soapAction === '"http://tempuri.org/Add"',
    short(received.at(-1)),
  );

  const filled = loaded.envelope
    .replace('<tem:intA>?</tem:intA>', '<tem:intA>2</tem:intA>')
    .replace('<tem:intB>?</tem:intB>', '<tem:intB>3</tem:intB>');
  await typeIn(`$editor()`, filled);
  await run(`$tab('Scripts').click()`);
  await sleep(300);
  await run(`$segment('Tests · after response').click()`);
  await sleep(300);
  await typeIn(
    `$card().querySelector('.scripts .cm-content')`,
    "pm.test('sum', () => pm.expect(xml2Json(pm.response.text())['soap:Envelope']['soap:Body'].AddResponse.AddResult).to.equal('5'));",
  );
  await send();
  const answered = await run(
    `({ chips: [...$card().querySelectorAll('.response .chip')].map((c) => c.textContent.trim()), body: $response() })`,
  );
  check(
    'a filled envelope is answered and xml2Json reads it in tests',
    answered.chips.some((c) => c.startsWith('200')) &&
      answered.chips.includes('Tests 1/1') &&
      !answered.chips.includes('SOAP Fault') &&
      answered.body.includes('\n      <AddResult>5</AddResult>'),
    short(answered),
  );

  await run(`$tab('Envelope').click()`);
  await sleep(300);
  await run(
    `$pick($card().querySelector('button[aria-label="Operation"]'), 'Add · CalculatorSoap12 · SOAP 1.2')`,
  );
  await sleep(300);
  const kept = await run(`$editorText()`);
  await run(`$btn('Reset the envelope', $card()).click()`);
  await sleep(300);
  const reset = await run(
    `({ envelope: $editorText(), version: [...$card().querySelectorAll('[aria-label="SOAP version"] button')].find((b) => b.getAttribute('aria-pressed') === 'true')?.textContent.trim() })`,
  );
  check(
    'switching operation keeps an edited envelope until Reset',
    kept === filled &&
      reset.version === '1.2' &&
      reset.envelope.includes('http://www.w3.org/2003/05/soap-envelope'),
    short({ reset }),
  );
  await typeIn(
    `$editor()`,
    reset.envelope
      .replace('<tem:intA>?</tem:intA>', '<tem:intA>9</tem:intA>')
      .replace('<tem:intB>?</tem:intB>', '<tem:intB>4</tem:intB>'),
  );
  await send();
  const v12 = received.at(-1);
  check(
    'SOAP 1.2 puts the action in the Content-Type',
    v12?.contentType === 'application/soap+xml; charset=utf-8; action="http://tempuri.org/Add"' &&
      v12?.soapAction === null &&
      (await run(`$response()`)).includes('<AddResult>13</AddResult>'),
    short(v12),
  );

  // ---------- the section head offers SOAP too ----------
  await run(`$card().querySelector('button[aria-label="Collapse request"]').click()`);
  await sleep(400);
  await run(
    `[...document.querySelectorAll('button.adder')].find((b) => b.textContent.trim() === 'Add section').click()`,
  );
  await sleep(700);
  const tools = await run(
    `[...document.querySelectorAll('.section-tools button')].map((b) => b.getAttribute('aria-label'))`,
  );
  check('the section head has an Add SOAP button', tools.includes('Add SOAP'), short(tools));

  // ---------- importing a WSDL as a collection ----------
  await run(`$nav('Import').click()`);
  await sleep(600);
  await run(
    `$set(document.querySelector('.from-url input'), ${JSON.stringify(`${base}/calculator.asmx?wsdl`)})`,
  );
  await run(`$btn('Fetch').click()`);
  await waitFor(
    `(document.querySelector('.file .hint')?.textContent ?? '').includes('WSDL')`,
    10000,
  );
  const preview = await run(
    `({ source: document.querySelector('.file .hint')?.textContent.trim(), stats: [...document.querySelectorAll('.stats .stat')].map((s) => s.textContent.replace(/\\s+/g, ' ').trim()) })`,
  );
  check(
    'the Import page previews a WSDL',
    preview.source?.endsWith('WSDL 1.1') &&
      preview.stats.includes('4requests') &&
      preview.stats.includes('2sections'),
    short(preview),
  );
  await run(`$btn('Import 4 requests').click()`);
  await waitFor(`!!$nav(${JSON.stringify(IMPORTED)})`, 8000);
  await sleep(900);
  const imported = await run(
    `({ sections: [...document.querySelectorAll('.section-head .name, .section-head input')].map((n) => (n.value ?? n.textContent).trim()), rows: [...document.querySelectorAll('.row')].map((r) => r.textContent.replace(/\\s+/g, ' ').trim()) })`,
  );
  check(
    'the import makes a section per binding and a SOAP request per operation',
    imported.sections.includes('CalculatorSoap · SOAP 1.1') &&
      imported.sections.includes('CalculatorSoap12 · SOAP 1.2') &&
      imported.rows.filter((r) => r.includes('SOAP')).length === 4,
    short(imported),
  );

  // ---------- a WSDL over plain http on another host ----------
  other = await startSoapServer('127.0.0.2');
  await run(`$nav('Import').click()`);
  await sleep(600);
  await run(
    `$set(document.querySelector('.from-url input'), ${JSON.stringify(`${other.base}/calculator.asmx?wsdl`)})`,
  );
  await run(`$btn('Fetch').click()`);
  await waitFor(
    `(document.querySelector('.file .hint')?.textContent ?? '').includes('WSDL') || !!document.querySelector('.panel .error')`,
    10000,
  );
  const plain = await run(
    `({ source: document.querySelector('.file .hint')?.textContent.trim() ?? null, error: document.querySelector('.panel .error')?.textContent.trim() ?? null })`,
  );
  check(
    'a WSDL over plain http on another host is read with its imports',
    plain.source === 'WSDL 1.1' &&
      plain.error === null &&
      other.fetched.includes('/calculator.asmx') &&
      other.fetched.includes('/calculator.xsd'),
    short({ plain, fetched: other.fetched }),
  );
  await run(`$btn('Choose another file').click()`);
  await sleep(300);

  // ---------- Home: the chosen tile hides its tooltip ----------
  await run(`$nav('Home').click()`);
  await sleep(600);
  await run(`$hub('Requests').click()`);
  await sleep(500);
  await hover(`$homeTile('SOAP')`);
  const before = await run(`getComputedStyle($homeTile('SOAP').querySelector('.tip')).visibility`);
  await run(`$homeTile('SOAP').click()`);
  await sleep(500);
  const after = await run(
    `({ tip: getComputedStyle($homeTile('SOAP').querySelector('.tip')).visibility, title: document.querySelector('.home .stage h2')?.textContent.trim(), primary: document.querySelector('.home .primary')?.textContent.trim() })`,
  );
  check(
    'a tile shows its name on hover, but not once chosen',
    before === 'visible' &&
      after.tip === 'hidden' &&
      after.title === 'SOAP' &&
      after.primary === `Add SOAP to ${IMPORTED}`,
    short({ before, after }),
  );
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 5, y: 5 });
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
    await deleteCollection(NAME);
    await deleteCollection(IMPORTED);
    check(
      'test data cleaned up',
      !(await run(`!!$nav(${JSON.stringify(NAME)}) || !!$nav(${JSON.stringify(IMPORTED)})`)),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  server.close();
  other?.server.close();
  process.exit(0);
}
