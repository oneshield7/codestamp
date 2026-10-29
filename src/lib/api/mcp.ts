import { Channel, invoke } from '@tauri-apps/api/core';
import type { StoredKeyValue } from './collections';

export type McpTransport = 'http' | 'stdio';

/** `{{variables}}` are left in place; the Rust core fills them in when connecting. */
export interface McpConnectInput {
  transport: McpTransport;
  /** The server URL (HTTP) or the command that starts it (STDIO). */
  target: string;
  /** HTTP headers, or environment variables for a STDIO server. */
  pairs: StoredKeyValue[];
  collectionId: string | null;
}

export interface McpServerInfo {
  name: string;
  version: string;
  protocolVersion: string;
  instructions: string | null;
  tools: boolean;
  prompts: boolean;
  resources: boolean;
}

export interface McpSessionInfo {
  sessionId: string;
  server: McpServerInfo;
}

/** A program that has not been approved on this device with exactly these settings. */
export interface McpLaunch {
  program: string;
  args: string[];
  /** Names only; values may be secrets. */
  env: string[];
  fingerprint: string;
}

export type McpConnectOutcome =
  { status: 'connected'; session: McpSessionInfo } | { status: 'needsApproval'; launch: McpLaunch };

export interface McpRpcError {
  code: number;
  message: string;
  data?: unknown;
}

export interface McpReply {
  result: unknown;
  error: McpRpcError | null;
  elapsedMs: number;
}

/** `closed` is always the last event of a session. */
export type McpEvent =
  | {
      type: 'traffic';
      direction: 'sent' | 'received';
      text: string;
      size: number;
      truncated: boolean;
    }
  | { type: 'log'; text: string }
  | { type: 'closed'; reason: string };

export type McpMethod =
  | 'tools/list'
  | 'tools/call'
  | 'prompts/list'
  | 'prompts/get'
  | 'resources/list'
  | 'resources/read';

export function mcpConnect(
  input: McpConnectInput,
  approved: string | null,
  onEvent: (event: McpEvent) => void,
): Promise<McpConnectOutcome> {
  const channel = new Channel<McpEvent>();
  channel.onmessage = onEvent;
  return invoke('mcp_connect', { input, approved, onEvent: channel });
}

/** `params` is JSON text; `{{variables}}` in it are filled in first. */
export function mcpRequest(
  sessionId: string,
  method: McpMethod,
  params: string | null,
  collectionId: string | null,
): Promise<McpReply> {
  return invoke('mcp_request', { input: { sessionId, method, params, collectionId } });
}

export function mcpClose(sessionId: string): Promise<void> {
  return invoke('mcp_close', { sessionId });
}

/** Sessions opened before a page reload cannot be reached anymore, so they are closed. */
export function mcpCloseAll(): Promise<void> {
  return invoke('mcp_close_all');
}
