// Runs the code Copy as code writes, in every language with a toolchain on this machine, against
// a local server, and checks that the server received the same request from each of them.
// Go and PHP run in Docker (golang and composer images) when Docker is running.
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, '..');
const work = path.join(here, '.work', 'codegen');
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });

const { rolldown } = await import('rolldown');
const bundle = await rolldown({
  input: path.join(root, 'src', 'lib', 'request', 'codegen', 'index.ts'),
  platform: 'node',
});
await bundle.write({ file: path.join(work, 'codegen.mjs'), format: 'esm' });
const { environmentVariables, generateCode } = await import(
  pathToFileURL(path.join(work, 'codegen.mjs')).href
);

const received = [];
const server = http.createServer((req, res) => {
  const chunks = [];
  req.on('data', (chunk) => chunks.push(chunk));
  req.on('end', () => {
    received.push({
      method: req.method,
      url: req.url,
      headers: req.headers,
      body: Buffer.concat(chunks),
    });
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end('{"ok":true}');
  });
});
// Docker Desktop forwards host.docker.internal to the host's loopback, so containers reach it too.
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;

const BOM = String.fromCharCode(0xfeff);
const fileText = 'hello file ✓ \'@ "@ `tick`';
fs.writeFileSync(path.join(work, 'doc.txt'), fileText);

/** Each case: the request Copy as code gets, the values it reads, and what must arrive. */
const cases = [
  {
    name: 'JSON body, query and headers from environment variables',
    source: 'environment',
    env: { search: 'kopi & teh', token: 'tok-42', name: 'Rina "R" O\'Neil' },
    request: (base) => ({
      method: 'POST',
      url: '{{baseUrl}}/orders',
      query: [
        { name: 'page', value: '2' },
        { name: 'q', value: '{{search}}' },
      ],
      headers: [
        { name: 'Authorization', value: 'Bearer {{token}}' },
        { name: 'X-Trace', value: 'a b/c' },
        { name: 'Content-Type', value: 'application/json' },
      ],
      body: {
        mode: 'raw',
        text: '{\n  "name": "{{name}}",\n  "note": "quote \\" backslash \\\\ $HOME 100% `tick` {brace}",\n  "city": "Bandung é"\n}',
      },
      baseUrl: base,
    }),
    expect: {
      method: 'POST',
      path: '/orders',
      query: [
        ['page', '2'],
        ['q', 'kopi & teh'],
      ],
      headers: {
        authorization: 'Bearer tok-42',
        'x-trace': 'a b/c',
        'content-type': /^application\/json/,
      },
      body: '{\n  "name": "Rina "R" O\'Neil",\n  "note": "quote \\" backslash \\\\ $HOME 100% `tick` {brace}",\n  "city": "Bandung é"\n}',
    },
  },
  {
    name: 'URL-encoded form',
    source: 'environment',
    env: { note: '50% off + gratis' },
    request: (base) => ({
      method: 'POST',
      url: '{{baseUrl}}/form',
      query: [],
      headers: [{ name: 'Content-Type', value: 'application/x-www-form-urlencoded' }],
      body: {
        mode: 'urlencoded',
        fields: [
          { name: 'title', value: 'Kopi & Teh' },
          { name: 'note', value: '{{note}}' },
        ],
      },
      baseUrl: base,
    }),
    expect: {
      method: 'POST',
      path: '/form',
      form: [
        ['title', 'Kopi & Teh'],
        ['note', '50% off + gratis'],
      ],
    },
  },
  {
    name: 'Basic auth encoded when the code runs',
    source: 'environment',
    env: { user: 'ada', password: 's3cr:t!' },
    request: (base) => ({
      method: 'GET',
      url: '{{baseUrl}}/basic',
      query: [],
      headers: [],
      body: null,
      basic: { username: '{{user}}', password: '{{password}}' },
      baseUrl: base,
    }),
    expect: {
      method: 'GET',
      path: '/basic',
      headers: { authorization: `Basic ${Buffer.from('ada:s3cr:t!').toString('base64')}` },
    },
  },
  {
    name: 'Multipart with a file',
    source: 'environment',
    env: {},
    skip: { powershell: '-Form needs PowerShell 7; this machine has Windows PowerShell 5.1' },
    request: (base, file) => ({
      method: 'POST',
      url: '{{baseUrl}}/upload',
      query: [],
      headers: [],
      body: {
        mode: 'formData',
        parts: [
          { name: 'title', value: 'Quarterly report', file: false },
          { name: 'doc', value: file, file: true },
        ],
      },
      baseUrl: base,
    }),
    expect: {
      method: 'POST',
      path: '/upload',
      headers: { 'content-type': /^multipart\/form-data; boundary=/ },
      // .NET leaves simple names unquoted (`name=title`), which RFC 7578 servers accept.
      contains: [
        /name="?title"?/,
        'Quarterly report',
        /name="?doc"?/,
        /filename="?doc\.txt"?/,
        fileText,
      ],
    },
  },
  {
    name: 'Binary file body',
    source: 'environment',
    env: {},
    request: (base, file) => ({
      method: 'PUT',
      url: '{{baseUrl}}/blob',
      query: [],
      headers: [{ name: 'Content-Type', value: 'application/octet-stream' }],
      body: { mode: 'binary', path: file },
      baseUrl: base,
    }),
    expect: { method: 'PUT', path: '/blob', body: fileText },
  },
  {
    name: 'Values as written, with characters each language has to escape',
    source: 'template',
    env: {},
    request: (base) => ({
      method: 'PATCH',
      url: `${base}/plain`,
      query: [{ name: 'tag', value: 'it\'s 100% "ok"' }],
      headers: [
        { name: 'X-Note', value: 'it\'s "quoted" $x `y` %d' },
        { name: 'Content-Type', value: 'text/plain' },
      ],
      body: {
        mode: 'raw',
        text: 'line one \'\'\' """ \\n \\\\\n\'@ and "@ and {{literal}}\n  ${x} %s {0} ``\nlast "',
      },
    }),
    expect: {
      method: 'PATCH',
      path: '/plain',
      query: [['tag', 'it\'s 100% "ok"']],
      headers: { 'x-note': 'it\'s "quoted" $x `y` %d', 'content-type': /^text\/plain/ },
      body: 'line one \'\'\' """ \\n \\\\\n\'@ and "@ and {{literal}}\n  ${x} %s {0} ``\nlast "',
    },
  },
];

const phpDeps = path.join(here, '.work', 'php-deps');
const venvScripts = path.join(
  here,
  '.work',
  'venv',
  process.platform === 'win32' ? 'Scripts' : 'bin',
);
const withPath = (folder) => ({ PATH: `${folder}${path.delimiter}${process.env.PATH ?? ''}` });
const installed = (command) =>
  spawnSync(command, ['--version'], { stdio: 'ignore', shell: process.platform === 'win32' })
    .status === 0;

/**
 * How each language is saved and run: `file` is where the code goes, `files` writes the project
 * files around it, `missing` names what this machine lacks, and `docker` runs it in a container.
 */
const runners = [
  {
    language: 'curl',
    file: 'request.sh',
    // Git for Windows' curl sends non-ASCII arguments in the ANSI code page; Windows' own does not.
    run: (file) =>
      process.platform === 'win32'
        ? ['bash', ['-c', `PATH="/c/Windows/System32:$PATH"; . "$0"`, file]]
        : ['bash', [file]],
  },
  {
    language: 'httpie',
    file: 'request.sh',
    missing: () =>
      fs.existsSync(path.join(venvScripts, process.platform === 'win32' ? 'http.exe' : 'http'))
        ? null
        : 'install HTTPie into e2e/.work/venv (python -m venv, then pip install httpie)',
    run: (file) => ['bash', [file], withPath(venvScripts)],
  },
  {
    language: 'powershell',
    file: 'request.ps1',
    run: (file) => ['powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', file]],
  },
  { language: 'fetch', file: 'request.mjs', run: (file) => [process.execPath, [file]] },
  {
    language: 'axios',
    file: 'request.mjs',
    missing: () =>
      fs.existsSync(path.join(here, 'node_modules', 'axios'))
        ? null
        : 'run npm install --prefix e2e',
    run: (file) => [process.execPath, [file]],
  },
  { language: 'python', file: 'request.py', run: (file) => ['python', [file]] },
  {
    language: 'php',
    file: 'request.php',
    // Guzzle is installed once and shared, since Packagist sometimes times out from a container.
    prepare: async () => {
      if (fs.existsSync(path.join(phpDeps, 'vendor', 'autoload.php'))) {
        return null;
      }
      fs.mkdirSync(phpDeps, { recursive: true });
      fs.writeFileSync(
        path.join(phpDeps, 'composer.json'),
        '{ "require": { "guzzlehttp/guzzle": "^7.9" } }\n',
      );
      for (let attempt = 1; attempt <= 3; attempt += 1) {
        const install = await runProgram(
          'docker',
          [
            ...['run', '--rm', '--pull', 'never', '-v', `${phpDeps}:/deps`, '-w', '/deps'],
            ...['composer:2', 'composer', 'install', '--quiet', '--no-interaction'],
          ],
          {},
          300000,
        );
        if (install.status === 0) {
          return null;
        }
      }
      return 'Composer could not install Guzzle from Packagist';
    },
    docker: {
      image: 'composer:2',
      mounts: [[path.join(phpDeps, 'vendor'), '/work/vendor']],
      command: 'php request.php',
    },
  },
  {
    language: 'go',
    file: 'main.go',
    docker: { image: 'golang:1.24-alpine', command: 'go run main.go' },
  },
  { language: 'java', file: 'Main.java', run: (file) => ['java', [file]] },
  {
    language: 'kotlin',
    file: 'src/main/kotlin/Main.kt',
    files: {
      'settings.gradle.kts': 'rootProject.name = "request"\n',
      'build.gradle.kts': [
        'plugins {',
        '    kotlin("jvm") version "2.2.0"',
        '    application',
        '}',
        'repositories { mavenCentral() }',
        'dependencies { implementation("com.squareup.okhttp3:okhttp:5.1.0") }',
        'application { mainClass.set("MainKt") }',
        '',
      ].join('\n'),
    },
    docker: { image: 'gradle:jdk21', command: 'gradle run --quiet --console=plain' },
  },
  { language: 'csharp', file: 'request.cs', run: (file) => ['dotnet', ['run', file]] },
  {
    language: 'dart',
    file: 'request.dart',
    files: {
      'pubspec.yaml': 'name: request\nenvironment:\n  sdk: ^3.0.0\ndependencies:\n  http: ^1.2.0\n',
    },
    docker: { image: 'dart:stable', command: 'dart pub get >/dev/null && dart run request.dart' },
  },
  {
    language: 'swift',
    file: 'main.swift',
    docker: { image: 'swift:6.1', command: 'swiftc main.swift -o /tmp/request && /tmp/request' },
  },
  {
    language: 'ruby',
    file: 'request.rb',
    docker: { image: 'ruby:3-alpine', command: 'ruby request.rb' },
  },
  {
    language: 'rust',
    file: 'src/main.rs',
    // The dependency line is the one the generated code names in its first comment.
    files: (code) => ({
      'Cargo.toml': [
        '[package]',
        'name = "request"',
        'version = "0.1.0"',
        'edition = "2021"',
        '',
        '[dependencies]',
        code.split('\n')[0].replace('// Cargo.toml: ', ''),
        '',
      ].join('\n'),
    }),
    missing: () => (installed('cargo') ? null : 'Rust (cargo) is not installed'),
    // Offline: reqwest comes from the local Cargo cache; the build folder is shared by the cases.
    run: () => [
      'cargo',
      ['run', '--offline', '--quiet'],
      { CARGO_TARGET_DIR: path.join(here, '.work', 'rust-target') },
    ],
  },
];

const dockerReady = spawnSync('docker', ['info'], { stdio: 'ignore' }).status === 0;
const results = [];
// Printed as they come, since the compiled languages take a while.
const report = (line) => {
  results.push(line);
  console.log(line);
};
const check = (name, ok, detail = '') =>
  report(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' · ' + detail : ''}`);

/** Runs a program without blocking, so the server in this process can answer it meanwhile. */
function runProgram(command, args, options, timeout) {
  return new Promise((resolve) => {
    const child = spawn(command, args, options);
    let output = '';
    child.stdout.on('data', (chunk) => (output += chunk));
    child.stderr.on('data', (chunk) => (output += chunk));
    const timer = setTimeout(() => child.kill(), timeout);
    child.on('error', (error) => {
      clearTimeout(timer);
      resolve({ status: -1, output: error.message });
    });
    child.on('close', (status) => {
      clearTimeout(timer);
      resolve({ status, output });
    });
  });
}

function compare(expect, got) {
  if (!got) {
    return 'nothing arrived';
  }
  const url = new URL(got.url, 'http://local');
  if (got.method !== expect.method) {
    return `method ${got.method}`;
  }
  if (url.pathname !== expect.path) {
    return `path ${url.pathname}`;
  }
  if (expect.query && JSON.stringify([...url.searchParams]) !== JSON.stringify(expect.query)) {
    return `query ${JSON.stringify([...url.searchParams])}`;
  }
  for (const [name, value] of Object.entries(expect.headers ?? {})) {
    const actual = got.headers[name] ?? '';
    if (value instanceof RegExp ? !value.test(actual) : actual !== value) {
      return `${name}: ${actual}`;
    }
  }
  const body = got.body.toString('utf8');
  if (expect.body !== undefined && body !== expect.body) {
    return `body ${JSON.stringify(body).slice(0, 160)}`;
  }
  if (
    expect.form &&
    JSON.stringify([...new URLSearchParams(body)]) !== JSON.stringify(expect.form)
  ) {
    return `form ${body}`;
  }
  for (const part of expect.contains ?? []) {
    if (part instanceof RegExp ? !part.test(body) : !body.includes(part)) {
      return `body lacks ${part}`;
    }
  }
  return null;
}

for (const runner of runners) {
  // Images are never pulled here; one that is not on this machine is skipped.
  const absent = runner.docker
    ? !dockerReady
      ? 'Docker is not running'
      : spawnSync('docker', ['image', 'inspect', runner.docker.image], { stdio: 'ignore' })
            .status === 0
        ? null
        : `the ${runner.docker.image} image is not on this machine`
    : (runner.missing?.() ?? null);
  // A runner that is there may still need its libraries set up once.
  const reason = absent ?? (await runner.prepare?.()) ?? null;
  if (reason) {
    report(`SKIP  ${runner.language}: ${reason}`);
    continue;
  }
  for (const [index, item] of cases.entries()) {
    const label = `${runner.language} · ${item.name}`;
    if (item.skip?.[runner.language]) {
      report(`SKIP  ${label}: ${item.skip[runner.language]}`);
      continue;
    }
    const folder = path.join(work, runner.language, String(index + 1));
    fs.mkdirSync(folder, { recursive: true });
    const inDocker = Boolean(runner.docker);
    const base = inDocker ? `http://host.docker.internal:${port}` : `http://127.0.0.1:${port}`;
    const file = inDocker ? '/work/doc.txt' : path.join(work, 'doc.txt');
    if (inDocker) {
      fs.copyFileSync(path.join(work, 'doc.txt'), path.join(folder, 'doc.txt'));
    }
    const { baseUrl, ...request } = item.request(base, file);
    const code = generateCode(runner.language, request, item.source);
    const files = typeof runner.files === 'function' ? runner.files(code) : (runner.files ?? {});
    for (const [name, content] of Object.entries(files)) {
      fs.writeFileSync(path.join(folder, name), content);
    }
    const target = path.join(folder, runner.file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    // Saved the way the PowerShell code itself asks, with a byte order mark.
    fs.writeFileSync(target, runner.language === 'powershell' ? `${BOM}${code}` : code);
    const env = { ...item.env, ...(baseUrl ? { baseUrl } : {}) };
    // The generator names the environment variables; the case gives a value per `{{name}}`.
    const variables = Object.fromEntries(
      environmentVariables(request, item.source).map(({ name, env: variable }) => [
        variable,
        env[name],
      ]),
    );
    const before = received.length;
    let outcome;
    if (inDocker) {
      const flags = Object.entries(variables).flatMap(([name, value]) => [
        '-e',
        `${name}=${value}`,
      ]);
      outcome = await runProgram(
        'docker',
        [
          'run',
          '--rm',
          '--pull',
          'never',
          ...flags,
          '-v',
          `${folder}:/work`,
          ...(runner.docker.mounts ?? []).flatMap(([from, to]) => ['-v', `${from}:${to}`]),
          '-w',
          '/work',
          runner.docker.image,
          'sh',
          '-c',
          runner.docker.command,
        ],
        {},
        300000,
      );
    } else {
      const [command, args, extra = {}] = runner.run(target);
      outcome = await runProgram(
        command,
        args,
        { cwd: folder, env: { ...process.env, ...extra, ...variables } },
        300000,
      );
    }
    if (outcome.status !== 0) {
      check(label, false, `exit ${outcome.status}: ${outcome.output.trim().slice(0, 300)}`);
      continue;
    }
    const problem = compare(item.expect, received.length > before ? received.at(-1) : null);
    check(label, problem === null, problem ?? '');
  }
}

const count = (kind) => results.filter((line) => line.startsWith(kind)).length;
console.log(`\n${String(count('PASS'))} pass, ${String(count('FAIL'))} fail`);
server.close();
process.exit(0);
