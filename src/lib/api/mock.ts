import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface MockStatus {
  port: number;
  url: string;
}

/** One request the mock server answered. */
export interface MockHit {
  collectionId: string;
  method: string;
  path: string;
  status: number;
  /** The example that answered; null when none matched. */
  example: string | null;
  at: number;
}

/** Port 0 picks any free port. */
export function mockStart(collectionId: string, port: number): Promise<MockStatus> {
  return invoke('mock_start', { collectionId, port });
}

export function mockStop(collectionId: string): Promise<void> {
  return invoke('mock_stop', { collectionId });
}

export function mockStatus(collectionId: string): Promise<MockStatus | null> {
  return invoke('mock_status', { collectionId });
}

export function onMockHit(handler: (hit: MockHit) => void): Promise<UnlistenFn> {
  return listen<MockHit>('mock://hit', (event) => {
    handler(event.payload);
  });
}
