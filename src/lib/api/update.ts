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
    };

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
