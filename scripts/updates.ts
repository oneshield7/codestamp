// Sets up in-app updates and writes the release file the update server hands out.
//   node scripts/updates.ts setup https://host/codestamp/latest.json
//   node scripts/updates.ts manifest [--notes "text" | --notes-file notes.txt]
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

interface UpdaterConfig {
  pubkey: string;
  endpoints: string[];
  [key: string]: unknown;
}

interface TauriConfig {
  productName: string;
  version: string;
  bundle: Record<string, unknown>;
  plugins: { updater: UpdaterConfig };
  [key: string]: unknown;
}

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const configPath = path.join(root, 'src-tauri', 'tauri.conf.json');
const bundleDir = path.join(root, 'src-tauri', 'target', 'release', 'bundle', 'nsis');
const tauriCli = path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
const MAX_NOTES_BYTES = 4000;

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

function readConfig(): TauriConfig {
  return JSON.parse(fs.readFileSync(configPath, 'utf8')) as TauriConfig;
}

function defaultKeyPath(): string {
  return process.env.CODESTAMP_UPDATER_KEY ?? path.join(os.homedir(), '.codestamp', 'updater.key');
}

function releaseAddress(raw: string | undefined): URL {
  if (!raw) {
    fail('Usage: npm run updates:setup -- https://host/path/latest.json');
  }
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    fail(`Not a valid address: ${raw}`);
  }
  if (url.protocol !== 'https:') {
    fail('The update address must start with https://. Codestamp refuses plain http for updates.');
  }
  if (!url.pathname.endsWith('.json') || url.username || url.password || url.search) {
    fail(
      'Give the address of the release file itself, e.g. https://host/codestamp/latest.json, without a user name, password or query.',
    );
  }
  return url;
}

function setup(rawUrl: string | undefined): void {
  const url = releaseAddress(rawUrl);
  const keyPath = defaultKeyPath();
  if (!fs.existsSync(keyPath)) {
    fs.mkdirSync(path.dirname(keyPath), { recursive: true });
    console.log(
      `Creating the update signing key at ${keyPath}. Choose a password and keep it safe.`,
    );
    // The Tauri CLI asks for the password itself, so it never passes through this script.
    execFileSync(process.execPath, [tauriCli, 'signer', 'generate', '-w', keyPath], {
      stdio: 'inherit',
    });
  }
  const pubkey = fs.readFileSync(`${keyPath}.pub`, 'utf8').trim();
  const config = readConfig();
  config.plugins.updater.pubkey = pubkey;
  config.plugins.updater.endpoints = [url.href];
  config.bundle.createUpdaterArtifacts = true;
  fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
  console.log(`Updates are set up: builds now look for ${url.href}.`);
  console.log(
    `Back up ${keyPath} and its password. Without them, installed copies cannot be updated.`,
  );
}

function readNotes(args: string[]): string | undefined {
  const flag = args.at(0);
  const value = args.at(1);
  if (flag === undefined) {
    return undefined;
  }
  if (value === undefined || (flag !== '--notes' && flag !== '--notes-file')) {
    fail('Usage: npm run updates:manifest -- [--notes "text" | --notes-file notes.txt]');
  }
  const notes = (flag === '--notes' ? value : fs.readFileSync(value, 'utf8')).trim();
  if (Buffer.byteLength(notes) > MAX_NOTES_BYTES) {
    fail(
      `Release notes are limited to ${String(MAX_NOTES_BYTES)} bytes; Codestamp cuts longer ones.`,
    );
  }
  return notes || undefined;
}

function manifest(args: string[]): void {
  const config = readConfig();
  const endpoint = config.plugins.updater.endpoints.at(0);
  if (!endpoint) {
    fail(
      'Updates are not set up yet. Run npm run updates:setup -- https://host/path/latest.json first.',
    );
  }
  const installer = `${config.productName}_${config.version}_x64-setup.exe`;
  const installerPath = path.join(bundleDir, installer);
  const signaturePath = `${installerPath}.sig`;
  if (!fs.existsSync(installerPath) || !fs.existsSync(signaturePath)) {
    fail(
      `${installer} or its .sig file is missing. Run npm run installer with TAURI_SIGNING_PRIVATE_KEY set first.`,
    );
  }
  const release = {
    version: config.version,
    notes: readNotes(args),
    pub_date: new Date().toISOString(),
    platforms: {
      'windows-x86_64': {
        signature: fs.readFileSync(signaturePath, 'utf8').trim(),
        url: new URL(encodeURIComponent(installer), endpoint).href,
      },
    },
  };
  const releasePath = path.join(bundleDir, path.basename(new URL(endpoint).pathname));
  fs.writeFileSync(releasePath, `${JSON.stringify(release, null, 2)}\n`);
  console.log('Upload these two files next to each other on the update server:');
  console.log(`  ${installerPath}`);
  console.log(`  ${releasePath}`);
}

const [command, ...rest] = process.argv.slice(2);
if (command === 'setup') {
  setup(rest.at(0));
} else if (command === 'manifest') {
  manifest(rest);
} else {
  fail(
    'Usage: node scripts/updates.ts setup <https address of latest.json> | manifest [--notes text]',
  );
}
