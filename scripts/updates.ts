// Sets up in-app updates and writes the latest.json attached to each GitHub release.
//   node scripts/updates.ts setup https://github.com/<owner>/<repo>/releases/latest/download/latest.json
//   node scripts/updates.ts manifest --tag vX.Y.Z --signatures <folder of .sig files> [--out latest.json]
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildManifest } from './release-manifest.ts';

interface UpdaterConfig {
  pubkey: string;
  endpoints: string[];
  [key: string]: unknown;
}

interface TauriConfig {
  productName: string;
  version: string;
  plugins: { updater: UpdaterConfig };
  [key: string]: unknown;
}

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const configPath = path.join(root, 'src-tauri', 'tauri.conf.json');
const changelogPath = path.join(root, 'CHANGELOG.md');
const tauriCli = path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
const MANIFEST_USAGE =
  'Usage: node scripts/updates.ts manifest --tag vX.Y.Z --signatures <folder> [--out latest.json]';

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
  // Signing stays in src-tauri/tauri.updater.conf.json, so builds without the key still work.
  fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
  console.log(`Updates are set up: builds now look for ${url.href}.`);
  console.log(
    `Back up ${keyPath} and its password. Without them, installed copies cannot be updated.`,
  );
}

function readFlags(args: string[]): Map<string, string> {
  const flags = new Map<string, string>();
  for (let index = 0; index < args.length; index += 2) {
    const flag = args.at(index);
    const value = args.at(index + 1);
    if (!flag || !['--tag', '--signatures', '--out'].includes(flag) || value === undefined) {
      fail(MANIFEST_USAGE);
    }
    flags.set(flag, value);
  }
  return flags;
}

function readSignature(folder: string, file: string): string {
  const signaturePath = path.join(folder, `${file}.sig`);
  if (!fs.existsSync(signaturePath)) {
    fail(`${file}.sig is missing from ${folder}. Build with TAURI_SIGNING_PRIVATE_KEY set.`);
  }
  return fs.readFileSync(signaturePath, 'utf8');
}

function manifest(args: string[]): void {
  const flags = readFlags(args);
  const tag = flags.get('--tag');
  const folder = flags.get('--signatures');
  if (!tag || !folder) {
    fail(MANIFEST_USAGE);
  }
  const config = readConfig();
  const endpoint = config.plugins.updater.endpoints.at(0);
  if (!endpoint) {
    fail('Updates are not set up yet. Run npm run updates:setup first.');
  }
  const { productName, version } = config;
  let release;
  try {
    release = buildManifest({
      version,
      tag,
      productName,
      endpoint,
      changelog: fs.readFileSync(changelogPath, 'utf8'),
      signatures: {
        installer: readSignature(folder, `${productName}_${version}_x64-setup.exe`),
        appImage: readSignature(folder, `${productName}_${version}_amd64.AppImage`),
      },
      pubDate: new Date().toISOString(),
    });
  } catch (error) {
    fail(error instanceof Error ? error.message : String(error));
  }
  const out = path.resolve(flags.get('--out') ?? 'latest.json');
  fs.writeFileSync(out, `${JSON.stringify(release, null, 2)}\n`);
  console.log(`Wrote ${out} for ${tag}.`);
}

const [command, ...rest] = process.argv.slice(2);
if (command === 'setup') {
  setup(rest.at(0));
} else if (command === 'manifest') {
  manifest(rest);
} else {
  fail(
    'Usage: node scripts/updates.ts setup <https address of latest.json> | manifest --tag vX.Y.Z --signatures <folder>',
  );
}
