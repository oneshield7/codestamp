import type { McpOperation } from '../api/collections';
import { toAppError } from '../api/http';
import {
  mcpClose,
  mcpConnect,
  mcpRequest,
  type McpConnectInput,
  type McpEvent,
  type McpLaunch,
  type McpReply,
  type McpServerInfo,
} from '../api/mcp';
import {
  callMethod,
  callParams,
  MCP_LISTS,
  nextCursor,
  offers,
  readItems,
  type McpItem,
} from '../request/mcp-items';

export type { McpItem };

export type McpStatus = 'idle' | 'connecting' | 'connected' | 'closing';
export type McpLogKind = 'sent' | 'received' | 'log' | 'info' | 'error';

export interface McpLogEntry {
  id: number;
  kind: McpLogKind;
  at: number;
  text: string;
  size: number | null;
  truncated: boolean;
}

export interface McpCallResult {
  operation: McpOperation;
  reply: McpReply;
}

const MAX_LOG_ENTRIES = 500;
const MAX_PAGES = 10;

/** One attempt to connect; `id` is set once the Rust core started the session. */
interface Attempt {
  id: string | null;
}

/** The live session of the open MCP card: what the server offers, calls and the message log. */
export class McpConnection {
  status = $state<McpStatus>('idle');
  server = $state<McpServerInfo | null>(null);
  items = $state<McpItem[]>([]);
  log = $state<McpLogEntry[]>([]);
  /** A program waiting for the user's approval before it is started. */
  launch = $state<McpLaunch | null>(null);
  error = $state<string | null>(null);
  calling = $state(false);
  last = $state<McpCallResult | null>(null);

  #attempt: Attempt | null = null;
  #input: McpConnectInput | null = null;
  #nextEntryId = 0;

  get sessionId(): string | null {
    return this.#attempt?.id ?? null;
  }

  async connect(input: McpConnectInput, approved: string | null = null): Promise<void> {
    if (this.status !== 'idle') {
      return;
    }
    const attempt: Attempt = { id: null };
    this.#attempt = attempt;
    this.#input = input;
    this.status = 'connecting';
    this.error = null;
    this.launch = null;
    this.#add('info', `Connecting to ${input.target}`);
    try {
      const outcome = await mcpConnect(input, approved, (event) => {
        this.#handle(attempt, event);
      });
      if (outcome.status === 'needsApproval') {
        this.#reset();
        this.launch = outcome.launch;
        this.#add('info', 'Waiting for approval to start the program');
        return;
      }
      if (this.#attempt !== attempt) {
        // The card was closed while the session was starting.
        await mcpClose(outcome.session.sessionId);
        return;
      }
      attempt.id = outcome.session.sessionId;
      this.server = outcome.session.server;
      this.status = 'connected';
      const { name, version, protocolVersion } = outcome.session.server;
      this.#add('info', `Connected to ${name || 'the server'} ${version} · MCP ${protocolVersion}`);
      await this.refresh();
    } catch (caught) {
      const message = toAppError(caught).message;
      this.#add('error', message);
      this.error = message;
      this.#reset();
    }
  }

  /** Starts the program the user just approved, exactly as it was shown. */
  async approve(): Promise<void> {
    const launch = this.launch;
    const input = this.#input;
    if (!launch || !input) {
      return;
    }
    await this.connect(input, launch.fingerprint);
  }

  cancelApproval(): void {
    this.launch = null;
    this.#add('info', 'The program was not started');
  }

  /** Lists everything the server offers, following pages up to a limit. */
  async refresh(): Promise<void> {
    const sessionId = this.sessionId;
    const server = this.server;
    if (!sessionId || !server) {
      return;
    }
    const items: McpItem[] = [];
    try {
      for (const list of MCP_LISTS) {
        if (!offers(server, list.kind)) {
          continue;
        }
        let cursor: string | null = null;
        for (let page = 0; page < MAX_PAGES; page += 1) {
          const params: string | null = cursor === null ? null : JSON.stringify({ cursor });
          const reply = await mcpRequest(sessionId, list.method, params, null);
          if (reply.error) {
            this.#add('error', `${list.method}: ${reply.error.message}`);
            break;
          }
          items.push(...readItems(list.kind, reply.result));
          cursor = nextCursor(reply.result);
          if (cursor === null) {
            break;
          }
        }
      }
      this.items = items;
    } catch (caught) {
      this.#add('error', toAppError(caught).message);
    }
  }

  /** Calls a tool or prompt with `args` (JSON text), or reads a resource. */
  async call(operation: McpOperation, args: string, collectionId: string | null): Promise<void> {
    const sessionId = this.sessionId;
    if (!sessionId || this.calling) {
      return;
    }
    this.calling = true;
    this.error = null;
    try {
      const reply = await mcpRequest(
        sessionId,
        callMethod(operation.kind),
        callParams(operation, args),
        collectionId,
      );
      this.last = { operation, reply };
    } catch (caught) {
      const message = toAppError(caught).message;
      this.error = message;
      this.#add('error', message);
    } finally {
      this.calling = false;
    }
  }

  async disconnect(): Promise<void> {
    const attempt = this.#attempt;
    if (!attempt) {
      return;
    }
    this.#attempt = null;
    if (attempt.id === null) {
      this.#reset();
      return;
    }
    this.status = 'closing';
    try {
      await mcpClose(attempt.id);
    } catch (caught) {
      this.#add('error', toAppError(caught).message);
    }
    this.#reset();
  }

  /** Called when the card closes; the session ends with it. */
  dispose(): void {
    void this.disconnect();
  }

  #handle(attempt: Attempt, event: McpEvent): void {
    if (this.#attempt !== attempt && event.type !== 'closed') {
      return;
    }
    switch (event.type) {
      case 'traffic':
        this.#add(event.direction, event.text, event.size, event.truncated);
        break;
      case 'log':
        this.#add('log', event.text);
        break;
      case 'closed':
        this.#add('info', event.reason);
        if (this.#attempt === attempt) {
          const id = attempt.id;
          this.#reset();
          if (id !== null) {
            // The server ended on its own; the Rust core still holds the finished session.
            void mcpClose(id).catch((caught: unknown) => {
              this.#add('error', toAppError(caught).message);
            });
          }
        }
        break;
    }
  }

  #reset(): void {
    this.#attempt = null;
    this.status = 'idle';
    this.server = null;
    this.items = [];
  }

  #add(kind: McpLogKind, text: string, size: number | null = null, truncated = false): void {
    this.#nextEntryId += 1;
    this.log.push({ id: this.#nextEntryId, kind, at: Date.now(), text, size, truncated });
    if (this.log.length > MAX_LOG_ENTRIES) {
      this.log.splice(0, this.log.length - MAX_LOG_ENTRIES);
    }
  }
}
