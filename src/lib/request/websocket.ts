import type { ConnectionStatus, LogKind } from '../state/websocket.svelte';

export const STATUS_LABELS = new Map<ConnectionStatus, string>([
  ['idle', 'Disconnected'],
  ['connecting', 'Connecting…'],
  ['open', 'Connected'],
  ['closing', 'Disconnecting…'],
]);

export const LOG_MARKS = new Map<LogKind, string>([
  ['sent', '↑'],
  ['received', '↓'],
  ['info', '•'],
  ['error', '!'],
]);
