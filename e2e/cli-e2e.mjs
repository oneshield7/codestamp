// End-to-end check of step 9: codestamp-cli runs a collection file or a Collaboration folder
// against a local server, with an environment, --var, a data file, --bail and reports.
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import http from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const CLI =
  process.argv[2] ?? path.join(here, '..', 'src-tauri', 'target', 'debug', 'codestamp-cli.exe');
const work = mkdtempSync(path.join(tmpdir(), 'cs-e2e-cli-'));

const created = [];
const server = http.createServer((req, res) => {
  const json = (status, body) => {
    res.writeHead(status, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(body));
  };
  if (req.url === '/users' && req.method === 'GET') return json(200, [{ id: 7, name: 'Rina' }]);
  if (req.url === '/users/7') return json(200, { id: 7, name: 'Rina' });
  if (req.url === '/users' && req.method === 'POST') {
    let body = '';
    req.on('data', (chunk) => (body += chunk));
    req.on('end', () => {
      created.push(body);
      json(201, JSON.parse(body));
    });
    return;
  }
  if (req.url === '/echo') return json(200, { authorization: req.headers.authorization ?? null });
  if (req.url === '/fail') return json(500, { error: 'boom' });
  if (req.url === '/events') {
    res.writeHead(200, { 'Content-Type': 'text/event-stream' });
    res.write('data: hello\n\n');
    return;
  }
  json(404, {});
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;

function cli(args, env = {}) {
  return new Promise((resolve) => {
    const child = spawn(CLI, args, { env: { ...process.env, NO_COLOR: '1', ...env } });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => (stdout += chunk));
    child.stderr.on('data', (chunk) => (stderr += chunk));
    child.on('close', (code) => resolve({ code, stdout, stderr }));
  });
}

const results = [];
const check = (name, ok, detail = '') =>
  results.push(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);
const short = (value) => JSON.stringify(value)?.slice(0, 400);
const test = (lines) => [{ listen: 'test', script: { type: 'text/javascript', exec: lines } }];

// ---------- a Collection v2.1 file and an environment file ----------
const collection = {
  info: {
    name: 'CLI Users',
    schema: 'https://schema.getpostman.com/json/collection/v2.1.0/collection.json',
  },
  item: [
    {
      name: 'List users',
      request: { method: 'GET', url: '{{baseUrl}}/users' },
      event: test([
        "pm.test('status 200', () => pm.response.to.have.status(200));",
        "pm.environment.set('firstId', String(pm.response.json()[0].id));",
      ]),
    },
    {
      name: 'Get first',
      request: { method: 'GET', url: '{{baseUrl}}/users/{{firstId}}' },
      event: test([
        "pm.test('same user', () => pm.expect(pm.response.json().id).to.eql(Number(pm.environment.get('firstId'))));",
      ]),
    },
    {
      name: 'Auth echo',
      request: {
        method: 'GET',
        url: '{{baseUrl}}/echo',
        auth: { type: 'bearer', bearer: [{ key: 'token', value: '{{token}}', type: 'string' }] },
      },
      event: test([
        "console.log('checking auth');",
        "pm.test('bearer sent', () => pm.expect(pm.response.json().authorization).to.eql('Bearer s3cret'));",
      ]),
    },
    { name: 'Broken', request: { method: 'GET', url: '{{baseUrl}}/fail' } },
    {
      name: 'Admin',
      item: [
        {
          name: 'Create user',
          request: {
            method: 'POST',
            url: '{{baseUrl}}/users',
            header: [{ key: 'Content-Type', value: 'application/json' }],
            body: { mode: 'raw', raw: '{"name":"{{name}}"}' },
          },
          event: test([
            "pm.test('echoes the row', () => pm.expect(pm.response.json().name).to.eql(pm.iterationData.get('name') ?? pm.environment.get('name')));",
          ]),
        },
      ],
    },
  ],
};
const collectionFile = path.join(work, 'users.postman_collection.json');
writeFileSync(collectionFile, JSON.stringify(collection));
const environmentFile = path.join(work, 'local.postman_environment.json');
writeFileSync(
  environmentFile,
  JSON.stringify({
    name: 'Local',
    values: [
      { key: 'baseUrl', value: base, enabled: true },
      { key: 'name', value: 'Budi', enabled: true },
    ],
    _postman_variable_scope: 'environment',
  }),
);
const junitFile = path.join(work, 'out.xml');
const jsonFile = path.join(work, 'out.json');

const full = await cli([
  'run',
  collectionFile,
  '-e',
  environmentFile,
  '--var',
  'token=s3cret',
  '--junit',
  junitFile,
  '--json',
  jsonFile,
]);
check(
  'a failing request makes the run exit 1',
  full.code === 1,
  short({ code: full.code, stderr: full.stderr }),
);
check(
  'each request and test is printed in notebook order',
  /✓ GET\s+List users · 200 OK/.test(full.stdout) &&
    /✓ GET\s+Get first/.test(full.stdout) &&
    /✗ GET\s+Broken · 500/.test(full.stdout) &&
    full.stdout.indexOf('Broken') < full.stdout.indexOf('Create user') &&
    full.stdout.includes('✓ bearer sent') &&
    full.stdout.includes('│ log: checking auth'),
  short(full.stdout),
);
check(
  'variables flow between requests; --var feeds auth',
  full.stdout.includes('✓ same user') && full.stdout.includes('✓ bearer sent'),
  '',
);
check(
  'the summary counts requests and tests',
  full.stdout.includes('Requests  5 · 4 passed · 1 failed · 0 skipped') &&
    full.stdout.includes('Tests     4 · 0 failed'),
  short(full.stdout.split('\n').slice(-6)),
);
const report = JSON.parse(readFileSync(jsonFile, 'utf8'));
check(
  'the JSON report has the app report shape',
  report.app === 'Codestamp' &&
    report.collection === 'CLI Users' &&
    report.environment === 'Local' &&
    report.summary.failed === 1 &&
    report.results.length === 5 &&
    report.results[3].status === 'failed' &&
    report.results[3].code === 500,
  short(report.summary),
);
const xml = readFileSync(junitFile, 'utf8');
check(
  'the JUnit report lists suites, cases and the failure',
  xml.startsWith('<?xml') &&
    xml.includes('<testsuites name="CLI Users" tests="5" failures="1"') &&
    xml.includes('<testsuite name="Broken"') &&
    xml.includes('<failure message="The server answered 500"/>'),
  short(xml.slice(0, 300)),
);

const bail = await cli([
  'run',
  collectionFile,
  '-e',
  environmentFile,
  '--var',
  'token=s3cret',
  '--bail',
]);
check(
  '--bail skips what comes after a failure',
  bail.code === 1 && /- POST\s+Create user · skipped: Skipped after a failure/.test(bail.stdout),
  short(bail.stdout.split('\n').slice(-8)),
);

const dataFile = path.join(work, 'people.csv');
writeFileSync(dataFile, 'name\nAni\nDewi\n');
created.length = 0;
const data = await cli(
  ['run', collectionFile, '-e', environmentFile, '--var', 'TOKEN_FROM_CI', '--data', dataFile],
  { TOKEN_FROM_CI: 'unused' },
);
const fromEnv = await cli(
  ['run', collectionFile, '-e', environmentFile, '--var', 'token', '-n', '1'],
  { token: 's3cret' },
);
check(
  'a data file runs once per row with its values',
  data.stdout.includes('Iteration 1/2') &&
    data.stdout.includes('Iteration 2/2') &&
    created.includes('{"name":"Ani"}') &&
    created.includes('{"name":"Dewi"}'),
  short(created),
);
check(
  '--var <name> reads the process environment',
  fromEnv.stdout.includes('✓ bearer sent'),
  short(fromEnv.stdout.split('\n').filter((l) => l.includes('bearer'))),
);

// ---------- a Collaboration folder that passes ----------
const repo = path.join(work, 'repo');
mkdirSync(path.join(repo, 'requests'), { recursive: true });
const ids = [randomUUID(), randomUUID(), randomUUID()];
writeFileSync(path.join(repo, 'codestamp.json'), JSON.stringify({ format: 4, app: 'Codestamp' }));
writeFileSync(
  path.join(repo, 'collection.json'),
  JSON.stringify({
    id: randomUUID(),
    name: 'Shared API',
    auth: { type: 'none' },
    sections: [],
    order: ids,
    variables: [{ key: 'baseUrl', value: base, secret: false, enabled: true }],
  }),
);
const request = (id, name, kind, url, testScript) => ({
  id,
  name,
  kind,
  method: 'GET',
  url,
  params: [],
  headers: [],
  bodyMode: 'none',
  body: '',
  form: [],
  preRequestScript: '',
  testScript,
  auth: { type: 'inherit' },
});
writeFileSync(
  path.join(repo, 'requests', `${ids[0]}.json`),
  JSON.stringify(
    request(
      ids[0],
      'Users',
      'http',
      '{{baseUrl}}/users',
      "pm.test('ok', () => pm.response.to.be.ok);",
    ),
  ),
);
writeFileSync(
  path.join(repo, 'requests', `${ids[1]}.json`),
  JSON.stringify(request(ids[1], 'Live feed', 'websocket', 'ws://127.0.0.1/feed', '')),
);
writeFileSync(
  path.join(repo, 'requests', `${ids[2]}.json`),
  JSON.stringify(request(ids[2], 'Events', 'http', '{{baseUrl}}/events', '')),
);
const shared = await cli(['run', repo]);
check(
  'a Collaboration folder runs; WebSocket is skipped; a stream is cut short',
  shared.code === 0 &&
    /✓ GET\s+Users/.test(shared.stdout) &&
    /- WS\s+Live feed · skipped: WebSocket requests are not part of a run/.test(shared.stdout) &&
    shared.stdout.includes('Event stream: stopped once its headers arrived'),
  short({ code: shared.code, out: shared.stdout, err: shared.stderr }),
);

// ---------- the run cannot start ----------
const help = await cli([]);
const missing = await cli(['run', path.join(work, 'nope.json')]);
const wrongKind = await cli(['run', environmentFile]);
const badOption = await cli(['run', repo, '--iterations', '0']);
check(
  'help exits 0; a run that cannot start exits 2 with the reason',
  help.code === 0 &&
    help.stdout.includes('Usage: codestamp-cli run') &&
    missing.code === 2 &&
    missing.stderr.includes('cannot be read') &&
    wrongKind.code === 2 &&
    wrongKind.stderr.includes('pass it with --environment') &&
    badOption.code === 2 &&
    badOption.stderr.includes('--iterations must be a number'),
  short({
    help: help.code,
    missing: missing.stderr,
    wrongKind: wrongKind.stderr,
    badOption: badOption.stderr.split('\n')[0],
  }),
);

console.log(results.join('\n'));
server.close();
rmSync(work, { recursive: true, force: true });
