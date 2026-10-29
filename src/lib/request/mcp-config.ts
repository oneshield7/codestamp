import type { McpTransportName } from '../api/collections';
import { parseJson } from '../format/json';

export interface ParsedServer {
  transport: McpTransportName;
  /** The URL, or the command line with its arguments. */
  target: string;
  /** HTTP headers, or the program's environment. */
  pairs: { key: string; value: string }[];
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function stringPairs(value: unknown): { key: string; value: string }[] {
  const record = asRecord(value);
  if (!record) {
    return [];
  }
  return Object.entries(record)
    .filter((entry): entry is [string, string] => typeof entry[1] === 'string')
    .map(([key, text]) => ({ key, value: text }));
}

/** Quotes an argument the way Codestamp splits commands: only quotes are special. */
function quote(arg: string): string {
  if (arg !== '' && !/[\s"']/.test(arg)) {
    return arg;
  }
  return arg.includes('"') ? `'${arg}'` : `"${arg}"`;
}

/**
 * Reads one server from the JSON other MCP clients use (`{"mcpServers": {"name": {...}}}`, or a
 * single server): `command`, `args` and `env` for a program, `url` and `headers` over HTTP.
 */
export function parseServerConfig(text: string): ParsedServer | null {
  const root = asRecord(parseJson(text)?.value);
  if (!root) {
    return null;
  }
  const servers = asRecord(root.mcpServers);
  const server = servers ? asRecord(Object.values(servers)[0]) : root;
  if (!server) {
    return null;
  }
  if (typeof server.url === 'string') {
    return { transport: 'HTTP', target: server.url, pairs: stringPairs(server.headers) };
  }
  if (typeof server.command === 'string') {
    const args = Array.isArray(server.args)
      ? server.args.filter((arg): arg is string => typeof arg === 'string')
      : [];
    return {
      transport: 'STDIO',
      target: [server.command, ...args].map(quote).join(' '),
      pairs: stringPairs(server.env),
    };
  }
  return null;
}
