// Builds the latest.json that installed copies of Codestamp read from the GitHub release.
import { Buffer } from 'node:buffer';

/** The app cuts release notes at this size too (src-tauri/src/update/settings.rs). */
export const MAX_NOTES_BYTES = 4000;

const LATEST_ADDRESS =
  /^https:\/\/github\.com\/([^/]+)\/([^/]+)\/releases\/latest\/download\/latest\.json$/;

export interface ReleaseInput {
  version: string;
  tag: string;
  productName: string;
  endpoint: string;
  changelog: string;
  signatures: { installer: string; appImage: string };
  pubDate: string;
}

export interface Manifest {
  version: string;
  notes?: string;
  pub_date: string;
  platforms: Record<string, { signature: string; url: string }>;
}

/** Downloads point at the tagged release, so a newer release cannot swap the file under its signature. */
export function downloadBase(endpoint: string, tag: string): string {
  const match = LATEST_ADDRESS.exec(endpoint);
  if (!match) {
    throw new Error(
      `The update address must be https://github.com/<owner>/<repo>/releases/latest/download/latest.json, not ${endpoint}.`,
    );
  }
  const [, owner, repo] = match;
  return `https://github.com/${owner}/${repo}/releases/download/${encodeURIComponent(tag)}/`;
}

export function releaseNotes(changelog: string, version: string): string | undefined {
  const lines = changelog.split(/\r?\n/);
  const start = lines.findIndex((line) => line.startsWith(`## [${version}]`));
  if (start === -1) {
    return undefined;
  }
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((line) => line.startsWith('## ['));
  const notes = (end === -1 ? rest : rest.slice(0, end))
    .map((line) => line.replace(/^#{1,6}\s+/, ''))
    .join('\n')
    .trim();
  if (!notes) {
    return undefined;
  }
  if (Buffer.byteLength(notes) > MAX_NOTES_BYTES) {
    throw new Error(
      `The ${version} section of CHANGELOG.md is longer than ${String(MAX_NOTES_BYTES)} bytes, which Codestamp would cut off.`,
    );
  }
  return notes;
}

export function buildManifest(input: ReleaseInput): Manifest {
  if (input.tag !== `v${input.version}`) {
    throw new Error(
      `The tag ${input.tag} does not match version ${input.version} in tauri.conf.json. Push the tag v${input.version}.`,
    );
  }
  const base = downloadBase(input.endpoint, input.tag);
  const file = (name: string): string => new URL(encodeURIComponent(name), base).href;
  const installer = {
    signature: input.signatures.installer.trim(),
    url: file(`${input.productName}_${input.version}_x64-setup.exe`),
  };
  return {
    version: input.version,
    notes: releaseNotes(input.changelog, input.version),
    pub_date: input.pubDate,
    // No bare linux-x86_64: a .deb install would fall back to it and try to install the AppImage.
    platforms: {
      'windows-x86_64': installer,
      'windows-x86_64-nsis': installer,
      'linux-x86_64-appimage': {
        signature: input.signatures.appImage.trim(),
        url: file(`${input.productName}_${input.version}_amd64.AppImage`),
      },
    },
  };
}
