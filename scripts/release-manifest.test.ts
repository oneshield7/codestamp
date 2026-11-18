import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildManifest,
  downloadBase,
  releaseNotes,
  type ReleaseInput,
} from './release-manifest.ts';

const ENDPOINT = 'https://github.com/oneshield7/codestamp/releases/latest/download/latest.json';

const CHANGELOG = `# Changelog

## [Unreleased]

### Added

- Something not released yet.

## [0.5.0] - 2026-11-03

Codestamp now runs on Linux.

### Added

- Linux packages.

## [0.4.0] - 2026-11-02

- Workspaces.
`;

function input(overrides: Partial<ReleaseInput> = {}): ReleaseInput {
  return {
    version: '0.6.0',
    tag: 'v0.6.0',
    productName: 'Codestamp',
    endpoint: ENDPOINT,
    changelog: CHANGELOG,
    signatures: { installer: ' sigA\n', appImage: 'sigB' },
    pubDate: '2026-11-17T09:00:00.000Z',
    ...overrides,
  };
}

void test('downloads come from the tagged release, not from latest', () => {
  assert.equal(
    downloadBase(ENDPOINT, 'v0.6.0'),
    'https://github.com/oneshield7/codestamp/releases/download/v0.6.0/',
  );
});

void test('an update address outside GitHub releases is refused', () => {
  assert.throws(
    () => downloadBase('https://updates.example.com/codestamp/latest.json', 'v0.6.0'),
    /releases\/latest\/download\/latest\.json/,
  );
});

void test('notes are the version section of the changelog', () => {
  const notes = releaseNotes(CHANGELOG, '0.5.0') ?? '';
  assert.ok(notes.startsWith('Codestamp now runs on Linux.'), notes);
  assert.ok(notes.includes('Added\n\n- Linux packages.'), notes);
  assert.ok(!notes.includes('## [0.5.0]') && !notes.includes('2026-11-03'), notes);
  assert.ok(!notes.includes('0.4.0') && !notes.includes('Workspaces'), notes);
});

void test('a version missing from the changelog has no notes', () => {
  assert.equal(releaseNotes(CHANGELOG, '9.9.9'), undefined);
  assert.equal(releaseNotes('## [1.0.0]\n\n## [0.9.0]\n- x\n', '1.0.0'), undefined);
});

void test('notes longer than the app shows are refused', () => {
  assert.throws(() => releaseNotes(`## [1.0.0]\n\n${'x'.repeat(4001)}\n`, '1.0.0'), /4000/);
});

void test('the manifest names the installer and the AppImage for each target', () => {
  const manifest = buildManifest(input());
  assert.deepEqual(Object.keys(manifest.platforms).sort(), [
    'linux-x86_64-appimage',
    'windows-x86_64',
    'windows-x86_64-nsis',
  ]);
  const base = 'https://github.com/oneshield7/codestamp/releases/download/v0.6.0/';
  assert.deepEqual(manifest.platforms['windows-x86_64'], {
    signature: 'sigA',
    url: `${base}Codestamp_0.6.0_x64-setup.exe`,
  });
  assert.deepEqual(manifest.platforms['windows-x86_64-nsis'], manifest.platforms['windows-x86_64']);
  assert.deepEqual(manifest.platforms['linux-x86_64-appimage'], {
    signature: 'sigB',
    url: `${base}Codestamp_0.6.0_amd64.AppImage`,
  });
  assert.equal(manifest.version, '0.6.0');
  assert.equal(manifest.pub_date, '2026-11-17T09:00:00.000Z');
  assert.equal(manifest.notes, undefined);
});

void test('a tag that does not match the app version is refused', () => {
  assert.throws(() => buildManifest(input({ tag: 'v0.5.9' })), /0\.6\.0/);
});
