import { Channel, invoke } from '@tauri-apps/api/core';
import type { NameValue } from './http';

/** `{{variables}}` are left in place; the Rust core fills them in when connecting. */
export interface WsConnectInput {
  url: string;
  params: NameValue[];
  headers: NameValue[];
  collectionId: string | null;
}

export interface WsSession {
  connectionId: string;
  protocol: string | null;
}

/** `closed` is always the last event of a connection. */
export type WsEvent =
  | { type: 'message'; text: string; binary: boolean; size: number; truncated: boolean }
  | { type: 'error'; message: string }
  | { type: 'closed'; code: number | null; reason: string };

export function wsConnect(
  input: WsConnectInput,
  onEvent: (event: WsEvent) => void,
): Promise<WsSession> {
  const channel = new Channel<WsEvent>();
  channel.onmessage = onEvent;
  return invoke('ws_connect', { input, onEvent: channel });
}

export function wsSend(
  connectionId: string,
  text: string,
  collectionId: string | null,
): Promise<void> {
  return invoke('ws_send', { connectionId, text, collectionId });
}

export function wsClose(connectionId: string): Promise<void> {
  return invoke('ws_close', { connectionId });
}

/** Connections opened before a page reload cannot be reached anymore, so they are closed. */
export function wsCloseAll(): Promise<void> {
  return invoke('ws_close_all');
}
