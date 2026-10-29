// Makes .work/updater for stage34: a throwaway signing key, the config that points the test build
// at a local update server, a fake installer signed for the next version, and release.json with
// the versions the suite announces. Run it before building the stage34 test copy.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, '..', '..');
const out = path.join(here, '..', '.work', 'updater');
const tauri = path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
const key = path.join(out, 'test.key');
const installer = path.join(out, 'fake-installer.exe');

const { version: current } = JSON.parse(
  fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'),
);
const [major, minor] = current.split('.').map(Number);
const versions = { current, next: `${major}.${minor + 1}.0`, other: `${major}.${minor + 2}.0` };

// The key has no password and is used by this test only, never for a release.
const env = { ...process.env, TAURI_SIGNING_PRIVATE_KEY_PASSWORD: '' };
const run = (command, args) => execFileSync(command, args, { cwd: root, env, stdio: 'inherit' });

fs.mkdirSync(out, { recursive: true });
run(process.execPath, [tauri, 'signer', 'generate', '--ci', '--force', '--write-keys', key]);
run('rustc', ['-O', '--edition', '2021', path.join(here, 'fake-installer.rs'), '-o', installer]);
run(process.execPath, [
  tauri,
  'signer',
  'sign',
  '--private-key-path',
  key,
  '--app-version',
  versions.next,
  installer,
]);

const pubkey = fs.readFileSync(`${key}.pub`, 'utf8').trim();
const config = {
  plugins: {
    updater: {
      pubkey,
      endpoints: ['http://127.0.0.1:9455/latest.json'],
      dangerousInsecureTransportProtocol: true,
    },
  },
};
fs.writeFileSync(path.join(out, 'test.conf.json'), `${JSON.stringify(config, null, 2)}\n`);
fs.writeFileSync(path.join(out, 'release.json'), `${JSON.stringify(versions, null, 2)}\n`);
console.log(`stage34 is ready: the app is ${versions.current}, the update is ${versions.next}.`);
