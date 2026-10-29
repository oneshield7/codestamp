import { toAppError } from '../api/http';
import { wsClose, wsConnect, wsSend, type WsConnectInput, type WsEvent } from '../api/websocket';
import { clipUtf8 } from '../format/clip';
import { notices } from './notices.svelte';

export type ConnectionStatus = 'idle' | 'connecting' | 'open' | 'closing';
export type LogKind = 'sent' | 'received' | 'info' | 'error';

export interface LogEntry {
  id: number;
  kind: LogKind;
  at: number;
  text: string;
  size: number | null;
  binary: boolean;
  truncated: boolean;
}

const MAX_LOG_ENTRIES = 500;
// Same limit the Rust core applies to received messages, so sent ones cannot grow the log either.
const MAX_LOGGED_BYTES = 64 * 1024;

/** One attempt to connect; `id` is set once the Rust core accepts the connection. */
interface Attempt {
  id: string | null;
}

const encoder = new TextEncoder();

/** The live connection of the open WebSocket card and its message log (newest last). */
export class WebSocketSession {
  status = $state<ConnectionStatus>('idle');
  protocol = $state<string | null>(null);
  log = $state<LogEntry[]>([]);

  #attempt: Attempt | null = null;
  #nextEntryId = 0;

  async connect(input: WsConnectInput): Promise<void> {
    if (this.status !== 'idle') {
      return;
    }
    const attempt: Attempt = { id: null };
    this.#attempt = attempt;
    this.status = 'connecting';
    this.#add('info', `Connecting to ${input.url}`);
    try {
      const session = await wsConnect(input, (event) => {
        this.#handle(attempt, event);
      });
      if (this.#attempt !== attempt) {
        // The card was closed (or the server hung up) while the handshake was finishing.
        await wsClose(session.connectionId);
        return;
      }
      attempt.id = session.connectionId;
      this.protocol = session.protocol;
      this.status = 'open';
      this.#add(
        'info',
        session.protocol ? `Connected · protocol ${session.protocol}` : 'Connected',
      );
    } catch (caught) {
      if (this.#attempt === attempt) {
        this.#reset();
      }
      this.#add('error', toAppError(caught).message);
    }
  }

  async send(text: string, collectionId: string | null): Promise<void> {
    const id = this.#attempt?.id;
    if (this.status !== 'open' || !id) {
      return;
    }
    // Logged before sending so a reply (or close) that arrives quickly still shows up after it.
    const shown = clipUtf8(text, MAX_LOGGED_BYTES);
    const entryId = this.#add(
      'sent',
      shown.text,
      encoder.encode(text).length,
      false,
      shown.truncated,
    );
    try {
      await wsSend(id, text, collectionId);
    } catch (caught) {
      this.log = this.log.filter((entry) => entry.id !== entryId);
      this.#add('error', toAppError(caught).message);
    }
  }

  async disconnect(): Promise<void> {
    const id = this.#attempt?.id;
    if (this.status !== 'open' || !id) {
      return;
    }
    this.status = 'closing';
    try {
      await wsClose(id);
    } catch (caught) {
      this.status = 'open';
      this.#add('error', toAppError(caught).message);
    }
  }

  clear(): void {
    this.log = [];
  }

  /** Called when the card goes away; the connection must not outlive it. */
  dispose(): void {
    const id = this.#attempt?.id;
    this.#reset();
    if (id) {
      void notices.guard(() => wsClose(id));
    }
  }

  #handle(attempt: Attempt, event: WsEvent): void {
    if (event.type === 'message') {
      this.#add('received', event.text, event.size, event.binary, event.truncated);
    } else if (event.type === 'error') {
      this.#add('error', event.message);
    } else {
      this.#add('info', closedText(event.code, event.reason));
      if (this.#attempt === attempt) {
        this.#reset();
      }
    }
  }

  #reset(): void {
    this.#attempt = null;
    this.status = 'idle';
    this.protocol = null;
  }

  #add(
    kind: LogKind,
    text: string,
    size: number | null = null,
    binary = false,
    truncated = false,
  ): number {
    this.#nextEntryId += 1;
    this.log.push({ id: this.#nextEntryId, kind, at: Date.now(), text, size, binary, truncated });
    if (this.log.length > MAX_LOG_ENTRIES) {
      this.log.splice(0, this.log.length - MAX_LOG_ENTRIES);
    }
    return this.#nextEntryId;
  }
}

function closedText(code: number | null, reason: string): string {
  const detail = [code === null ? '' : `code ${String(code)}`, reason].filter(Boolean).join(' · ');
  return detail ? `Disconnected · ${detail}` : 'Disconnected';
}
