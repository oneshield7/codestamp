// End-to-end check of stage 15c: Digest, AWS Signature V4 and OAuth 2.0 (client credentials,
// password with refresh, authorization code with PKCE in its sign-in window, inherited from the
// collection), checked by local servers that verify what they receive.
// Creates "E2E Auth" and deletes it afterwards.
import crypto from 'node:crypto';
import http from 'node:http';

const CDP = 'http://127.0.0.1:9333';
const NAME = 'E2E Auth';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const md5 = (text) => crypto.createHash('md5').update(text).digest('hex');
const sha256 = (text) => crypto.createHash('sha256').update(text).digest('hex');
const hmac = (key, text) => crypto.createHmac('sha256', key).update(text).digest();

// ---------- the servers ----------
const REALM = 'codestamp-test';
const NONCE = 'n0nce-42';
const OPAQUE = 'op4que';
const challenges = new Map();
const tokenLog = [];
let ccCount = 0;

function digestParams(header) {
  const params = {};
  for (const match of header.slice(7).matchAll(/(\w+)=(?:"([^"]*)"|([^,\s]+))/g)) {
    params[match[1]] = match[2] ?? match[3];
  }
  return params;
}

function awsCheck(req, body) {
  const header = req.headers.authorization ?? '';
  const signature = /Signature=([0-9a-f]+)/.exec(header)?.[1];
  const signed = /SignedHeaders=([^,]+)/.exec(header)?.[1]?.split(';') ?? [];
  const amzDate = req.headers['x-amz-date'];
  const url = new URL(req.url, 'http://local');
  const encode = (text) =>
    encodeURIComponent(text).replace(
      /[!'()*]/g,
      (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
    );
  const path = url.pathname
    .split('/')
    .map((segment) => encode(segment))
    .join('/');
  const query = [...url.searchParams]
    .map(([k, v]) => [encode(k), encode(v)])
    .sort()
    .map(([k, v]) => `${k}=${v}`)
    .join('&');
  const headers = signed
    .map((name) => `${name}:${(name === 'host' ? req.headers.host : req.headers[name]).trim()}\n`)
    .join('');
  const canonical = [req.method, path, query, headers, signed.join(';'), sha256(body)].join('\n');
  const scope = `${amzDate.slice(0, 8)}/us-east-1/execute-api/aws4_request`;
  const toSign = ['AWS4-HMAC-SHA256', amzDate, scope, sha256(canonical)].join('\n');
  let key = hmac('AWS4wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY', amzDate.slice(0, 8));
  for (const part of ['us-east-1', 'execute-api', 'aws4_request']) key = hmac(key, part);
  const expected = crypto.createHmac('sha256', key).update(toSign).digest('hex');
  return { ok: signature === expected, signed: signed.join(';') };
}

const server = http.createServer((req, res) => {
  const chunks = [];
  req.on('data', (chunk) => chunks.push(chunk));
  req.on('end', () => {
    const body = Buffer.concat(chunks).toString('utf8');
    const url = new URL(req.url, 'http://local');
    const json = (status, value) => {
      res.writeHead(status, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(value));
    };
    if (url.pathname === '/digest') {
      const header = req.headers.authorization;
      if (!header?.startsWith('Digest ')) {
        res.writeHead(401, {
          'WWW-Authenticate': `Digest realm="${REALM}", qop="auth", nonce="${NONCE}", opaque="${OPAQUE}", algorithm=MD5`,
        });
        return res.end('needs auth');
      }
      const p = digestParams(header);
      const ha1 = md5(`ada:${REALM}:s3cret`);
      const ha2 = md5(`${req.method}:${p.uri}`);
      const ok =
        p.response === md5(`${ha1}:${NONCE}:${p.nc}:${p.cnonce}:auth:${ha2}`) &&
        p.uri === req.url &&
        p.opaque === OPAQUE;
      return ok ? json(200, { digestOk: true, uri: p.uri }) : json(401, { digestOk: false });
    }
    if (url.pathname === '/aws') {
      return json(200, { aws: awsCheck(req, body) });
    }
    if (url.pathname === '/authorize') {
      const state = url.searchParams.get('state');
      challenges.set(state, url.searchParams.get('code_challenge'));
      res.writeHead(302, {
        Location: `${url.searchParams.get('redirect_uri')}?code=code-1&state=${encodeURIComponent(state)}`,
      });
      return res.end();
    }
    if (url.pathname === '/token') {
      const form = Object.fromEntries(new URLSearchParams(body));
      const basic = req.headers.authorization?.startsWith('Basic ')
        ? Buffer.from(req.headers.authorization.slice(6), 'base64').toString()
        : null;
      tokenLog.push({
        grant: form.grant_type,
        basic,
        clientId: form.client_id ?? null,
        secret: form.client_secret ?? null,
      });
      if (form.grant_type === 'client_credentials') {
        if (basic !== 'cid:csecret')
          return json(401, { error: 'invalid_client', error_description: 'Bad credentials' });
        ccCount += 1;
        return json(200, {
          access_token: `cc-token-${ccCount}`,
          token_type: 'Bearer',
          expires_in: 1,
          scope: form.scope ?? '',
        });
      }
      if (form.grant_type === 'password') {
        const ok =
          form.username === 'ada' &&
          form.password === 'pw' &&
          form.client_id === 'cid' &&
          form.client_secret === 'csecret';
        return ok
          ? json(200, { access_token: 'pw-token-1', expires_in: 1, refresh_token: 'r-pw' })
          : json(400, { error: 'invalid_grant' });
      }
      if (form.grant_type === 'refresh_token') {
        return form.refresh_token === 'r-pw'
          ? json(200, { access_token: 'pw-token-2', expires_in: 3600 })
          : json(400, { error: 'invalid_grant' });
      }
      if (form.grant_type === 'authorization_code') {
        const challenge = crypto
          .createHash('sha256')
          .update(form.code_verifier ?? '')
          .digest('base64url');
        const known = [...challenges.values()].includes(challenge);
        return form.code === 'code-1' && known && form.client_id === 'public-app'
          ? json(200, {
              access_token: 'code-token',
              expires_in: 3600,
              refresh_token: 'r-code',
              scope: 'read',
            })
          : json(400, { error: 'invalid_grant', error_description: `pkce ${known}` });
      }
      return json(400, { error: 'unsupported_grant_type' });
    }
    if (url.pathname === '/resource') {
      return json(200, { authorization: req.headers.authorization ?? null });
    }
    json(404, {});
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
  socket = new WebSocket(
    pages.find((p) => p.type === 'page' && !p.url.includes('/authorize')).webSocketDebuggerUrl,
  );
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
  window.$row = (text) => [...document.querySelectorAll('.row')].find((r) => r.querySelector('.name, .request-name')?.textContent.trim() === text || r.textContent.includes(text));
  window.$body = () => document.querySelector('.response .cm-content')?.innerText ?? '';
  window.$json = () => { try { return JSON.parse($body()); } catch { return null; } };
  window.$status = () => document.querySelector('.response .summary .chip')?.textContent.trim() ?? '';
  window.$tab = (text) => [...$card().querySelectorAll('[role="tab"]')].find((t) => t.textContent.trim().startsWith(text));
  window.$tokenStatus = () => document.querySelector('section[aria-label="Access token"] .token-status')?.textContent.trim() ?? '';
`;
const results = [];
// The code dialog remembers the last language, so the suite picks cURL itself.
const PICK_CURL = `(async () => { const trigger = document.querySelector('dialog[open]:not(.panel-dialog) button[aria-label="Language"]'); trigger.click(); for (let i = 0; i < 40 && !document.querySelector('[role="listbox"] [role="option"]'); i++) await new Promise((r) => setTimeout(r, 50)); [...document.querySelectorAll('[role="listbox"] [role="option"]')].find((o) => o.textContent.trim() === 'cURL').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true })); await new Promise((r) => setTimeout(r, 200)); })()`;
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 300);
const content = (name, method, url, auth, extra = {}) => ({
  kind: 'http',
  name,
  method,
  url,
  params: [],
  headers: [],
  bodyMode: 'none',
  body: '',
  form: [],
  bodyFile: '',
  preRequestScript: '',
  testScript: '',
  auth,
  mcpOperation: null,
  pathVariables: [],
  ...extra,
});
const oauth = (overrides) => ({
  type: 'oauth2',
  grant: 'clientCredentials',
  authUrl: '',
  tokenUrl: '{{baseUrl}}/token',
  clientId: 'cid',
  clientSecret: 'csecret',
  scope: 'read',
  redirectUri: '',
  usePkce: true,
  username: '',
  password: '',
  clientAuth: 'header',
  headerPrefix: '',
  ...overrides,
});
async function open(name) {
  await run(`$row(${JSON.stringify(name)}).click()`);
  await waitFor(`$card()?.querySelector('.name input')?.value === ${JSON.stringify(name)}`, 4000);
  await sleep(300);
}
async function send() {
  await run(`$card().querySelector('.send').click()`);
  await sleep(300);
  await waitFor(`!$card().querySelector('.send').disabled`, 15000);
  await sleep(300);
}
async function collapse() {
  await run(`$card()?.querySelector('button[aria-label="Collapse request"]')?.click()`);
  await sleep(400);
}

await connect();
const ids = {};
try {
  const collection = await run(
    `$invoke('create_collection', { name: ${JSON.stringify(NAME)}, icon: null, baseUrl: ${JSON.stringify(base)} })`,
  );
  ids.collection = collection.id;
  const requests = [
    content('Digest', 'GET', '{{baseUrl}}/digest?x=1', {
      type: 'digest',
      username: 'ada',
      password: 's3cret',
    }),
    content('Digest wrong', 'GET', '{{baseUrl}}/digest', {
      type: 'digest',
      username: 'ada',
      password: 'nope',
    }),
    content('AWS get', 'GET', '{{baseUrl}}/aws?b=2&a=hello world', {
      type: 'awsSigV4',
      accessKey: 'AKIDEXAMPLE',
      secretKey: 'wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY',
      sessionToken: '',
      region: 'us-east-1',
      service: 'execute-api',
    }),
    content(
      'AWS post',
      'POST',
      '{{baseUrl}}/aws',
      {
        type: 'awsSigV4',
        accessKey: 'AKIDEXAMPLE',
        secretKey: 'wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY',
        sessionToken: '',
        region: 'us-east-1',
        service: 'execute-api',
      },
      {
        bodyMode: 'json',
        body: '{"hello":"world"}',
        headers: [{ enabled: true, key: 'Content-Type', value: 'application/json' }],
      },
    ),
    content('Client credentials', 'GET', '{{baseUrl}}/resource', oauth({})),
    content(
      'Password grant',
      'GET',
      '{{baseUrl}}/resource',
      oauth({ grant: 'password', username: 'ada', password: 'pw', clientAuth: 'body' }),
    ),
    content(
      'Code grant',
      'GET',
      '{{baseUrl}}/resource',
      oauth({
        grant: 'authorizationCode',
        authUrl: '{{baseUrl}}/authorize',
        clientId: 'public-app',
        clientSecret: '',
        redirectUri: `${base}/callback`,
        headerPrefix: 'Token',
      }),
    ),
    content('Inherited', 'GET', '{{baseUrl}}/resource', { type: 'inherit' }),
  ];
  for (const request of requests) {
    const saved = await run(
      `$invoke('create_request', { collectionId: '${collection.id}', sectionId: null, content: ${JSON.stringify(request)} })`,
    );
    ids[request.name] = saved.id;
  }
  await run(
    `$invoke('set_collection_auth', { id: '${collection.id}', auth: ${JSON.stringify(oauth({ scope: 'shared' }))} })`,
  );
  await run('location.reload()');
  await sleep(1800);
  await run(helpers);
  await run(`$nav(${JSON.stringify(NAME)}).click()`);
  await sleep(900);

  // ---------- Digest ----------
  await open('Digest');
  await send();
  check(
    'Digest answers the 401 challenge',
    (await run(`$status()`)).startsWith('200') &&
      (await run(`$json()`))?.digestOk === true &&
      (await run(`$json()`))?.uri === '/digest?x=1',
    short({ status: await run(`$status()`), body: await run(`$json()`) }),
  );
  const digestCode = await (async () => {
    await run(`$card().querySelector('button[aria-label="Copy as code"]').click()`);
    await sleep(500);
    await run(PICK_CURL);
    const text = await run(
      `document.querySelector('dialog[open]:not(.panel-dialog) .cm-content')?.innerText ?? ''`,
    );
    await run(
      `[...document.querySelectorAll('dialog[open]:not(.panel-dialog) button')].find((b) => (b.getAttribute('aria-label') ?? b.textContent).trim().startsWith('Close'))?.click()`,
    );
    await sleep(300);
    return text;
  })();
  check(
    'Copy as code uses curl --digest',
    digestCode.includes("--digest --user 'ada:s3cret'"),
    short(digestCode),
  );
  await collapse();
  await open('Digest wrong');
  await send();
  check(
    'a wrong Digest password shows the 401',
    (await run(`$status()`)).startsWith('401'),
    await run(`$status()`),
  );
  await collapse();

  // ---------- AWS ----------
  await open('AWS get');
  await send();
  const awsGet = await run(`$json()`);
  check(
    'AWS Signature V4 signs path, sorted query and headers',
    awsGet?.aws?.ok === true && awsGet.aws.signed === 'host;x-amz-date',
    short(awsGet),
  );
  await collapse();
  await open('AWS post');
  await send();
  const awsPost = await run(`$json()`);
  check(
    'AWS Signature V4 signs a JSON body',
    awsPost?.aws?.ok === true && awsPost.aws.signed === 'content-type;host;x-amz-date',
    short(awsPost),
  );
  await collapse();

  // ---------- OAuth: client credentials ----------
  await open('Client credentials');
  await run(`$tab('Auth').click()`);
  await sleep(300);
  check(
    'the Auth tab shows OAuth 2.0 with no token yet',
    (await run(`$tokenStatus()`)) === 'No access token yet.',
    await run(`$tokenStatus()`),
  );
  await run(`$btn('Get new access token').click()`);
  await waitFor(`$tokenStatus().startsWith('Got')`, 8000);
  check(
    'client credentials get a token with the Basic header',
    (await run(`$tokenStatus()`)).includes('renews on the next send') &&
      tokenLog.at(-1)?.basic === 'cid:csecret',
    short({ status: await run(`$tokenStatus()`), log: tokenLog.at(-1) }),
  );
  await send();
  check(
    'an expired client-credentials token renews by itself',
    (await run(`$json()`))?.authorization === 'Bearer cc-token-2',
    short(await run(`$json()`)),
  );
  await run(`$btn('Clear').click()`);
  await sleep(400);
  check(
    'Clear forgets the token',
    (await run(`$tokenStatus()`)) === 'No access token yet.' &&
      (await run(`$invoke('oauth_token_status', { ownerId: '${ids['Client credentials']}' })`)) ===
        null,
    await run(`$tokenStatus()`),
  );
  await collapse();

  // ---------- OAuth: password + refresh ----------
  await open('Password grant');
  await run(`$tab('Auth').click()`);
  await sleep(300);
  await run(`$btn('Get new access token').click()`);
  await waitFor(`$tokenStatus().startsWith('Got')`, 8000);
  const pwLog = tokenLog.at(-1);
  await send();
  check(
    'the password grant sends credentials in the body, then refreshes',
    pwLog?.grant === 'password' &&
      pwLog.secret === 'csecret' &&
      pwLog.basic === null &&
      tokenLog.at(-1)?.grant === 'refresh_token' &&
      (await run(`$json()`))?.authorization === 'Bearer pw-token-2',
    short({ pwLog, last: tokenLog.at(-1), body: await run(`$json()`) }),
  );
  await collapse();

  // ---------- OAuth: authorization code + PKCE ----------
  await open('Code grant');
  await run(`$tab('Auth').click()`);
  await sleep(300);
  await run(`$btn('Get new access token').click()`);
  await waitFor(`$tokenStatus().includes('expires in')`, 15000);
  const codeStatus = await run(`$tokenStatus()`);
  const targets = await (await fetch(`${CDP}/json`)).json();
  const signInLeft = targets.some(
    (target) => target.url.includes('/authorize') || target.url.includes('/callback'),
  );
  check(
    'authorization code with PKCE signs in through its own window',
    codeStatus.includes('expires in 1 h') &&
      tokenLog.at(-1)?.grant === 'authorization_code' &&
      !signInLeft,
    short({ codeStatus, last: tokenLog.at(-1), signInLeft }),
  );
  await send();
  check(
    'the token goes out with the chosen header prefix',
    (await run(`$json()`))?.authorization === 'Token code-token',
    short(await run(`$json()`)),
  );
  await collapse();

  // ---------- OAuth: inherited from the collection ----------
  await open('Inherited');
  await send();
  const inherited = await run(`$json()`);
  const collectionToken = await run(
    `$invoke('oauth_token_status', { ownerId: '${ids.collection}' })`,
  );
  check(
    'an inheriting request uses the collection’s token',
    /^Bearer cc-token-\d+$/.test(inherited?.authorization ?? '') &&
      collectionToken?.scope === 'shared',
    short({ inherited, collectionToken }),
  );
  await collapse();

  // ---------- tokens go with their request ----------
  await run(`$invoke('delete_request', { id: '${ids['Code grant']}' })`);
  check(
    'deleting a request removes its token',
    (await run(`$invoke('oauth_token_status', { ownerId: '${ids['Code grant']}' })`)) === null,
  );
} catch (error) {
  results.push(`FAIL  script stopped: ${error.message}`);
} finally {
  try {
    await run(helpers);
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
    const leftover = ids.collection
      ? await run(`$invoke('oauth_token_status', { ownerId: '${ids.collection}' })`)
      : null;
    check(
      'test data and its tokens cleaned up',
      !(await run(`!!$nav(${JSON.stringify(NAME)})`)) && leftover === null,
      short(leftover),
    );
  } catch (error) {
    results.push(`FAIL  cleanup: ${error.message}`);
  }
  console.log(results.join('\n'));
  socket.close();
  server.close();
  process.exit(0);
}
