import { Channel, invoke } from '@tauri-apps/api/core';

export type UpdateCheck =
  | { kind: 'notConfigured'; current: string }
  | { kind: 'upToDate'; current: string }
  | {
      kind: 'available';
      current: string;
      version: string;
      /** Plain text from the release file. */
      notes: string | null;
      /** As written in the release file (RFC 3339). */
      date: string | null;
      /** `apt`: a .deb install, which apt updates; the app never installs it. */
      via: 'app' | 'apt';
    };

export const APT_UPGRADE_COMMAND = 'sudo apt update && sudo apt upgrade';
export const RELEASES_URL = 'https://github.com/oneshield7/codestamp/releases/latest';

export interface DownloadProgress {
  received: number;
  total: number | null;
}

export function checkForUpdate(): Promise<UpdateCheck> {
  return invoke('check_for_update');
}

/** On success Codestamp closes while the installer runs, so this resolves only on failure. */
export function installUpdate(onProgress: (progress: DownloadProgress) => void): Promise<void> {
  const channel = new Channel<DownloadProgress>();
  channel.onmessage = onProgress;
  return invoke('install_update', { onProgress: channel });
}
