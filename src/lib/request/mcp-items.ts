import type { McpOperation, McpOperationKind } from '../api/collections';
import type { McpMethod, McpServerInfo } from '../api/mcp';
import { stringifyJson } from '../format/json';

/** One tool, prompt or resource a server offers; `name` is the URI for a resource. */
export interface McpItem {
  kind: McpOperationKind;
  name: string;
  title: string;
  description: string;
  /** A tool's JSON Schema for its arguments, or a prompt's argument list. */
  schema: unknown;
}

/** Where each kind is listed, in the order the UI shows them. */
export const MCP_LISTS: { kind: McpOperationKind; method: McpMethod }[] = [
  { kind: 'tool', method: 'tools/list' },
  { kind: 'prompt', method: 'prompts/list' },
  { kind: 'resource', method: 'resources/list' },
];

export function offers(server: McpServerInfo, kind: McpOperationKind): boolean {
  switch (kind) {
    case 'tool':
      return server.tools;
    case 'prompt':
      return server.prompts;
    case 'resource':
      return server.resources;
  }
}

export function callMethod(kind: McpOperationKind): McpMethod {
  switch (kind) {
    case 'tool':
      return 'tools/call';
    case 'prompt':
      return 'prompts/get';
    case 'resource':
      return 'resources/read';
  }
}

/** The JSON-RPC params as text, with the arguments left as typed so variables can be filled. */
export function callParams(operation: McpOperation, args: string): string {
  const name = JSON.stringify(operation.name);
  if (operation.kind === 'resource') {
    return `{"uri":${name}}`;
  }
  const typed = args.trim() === '' ? '{}' : args;
  return `{"name":${name},"arguments":${typed}}`;
}

export function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

export function text(value: unknown): string {
  return typeof value === 'string' ? value : '';
}

export function nextCursor(result: unknown): string | null {
  const cursor = asRecord(result)?.nextCursor;
  return typeof cursor === 'string' && cursor !== '' ? cursor : null;
}

function listOf(kind: McpOperationKind, result: Record<string, unknown> | null): unknown {
  switch (kind) {
    case 'tool':
      return result?.tools;
    case 'prompt':
      return result?.prompts;
    case 'resource':
      return result?.resources;
  }
}

export function readItems(kind: McpOperationKind, result: unknown): McpItem[] {
  const list = listOf(kind, asRecord(result));
  if (!Array.isArray(list)) {
    return [];
  }
  return list.flatMap((entry: unknown) => {
    const item = asRecord(entry);
    const name = text(kind === 'resource' ? item?.uri : item?.name);
    if (!item || name === '') {
      return [];
    }
    return [
      {
        kind,
        name,
        title: text(item.title) || text(kind === 'resource' ? item.name : '') || name,
        description: text(item.description),
        schema: kind === 'tool' ? item.inputSchema : kind === 'prompt' ? item.arguments : null,
      },
    ];
  });
}

/** Arguments to start from: every property of the tool's schema, or every prompt argument. */
export function argumentTemplate(item: McpItem, indent: number | 'tab'): string {
  if (item.kind === 'prompt') {
    const list: unknown[] = Array.isArray(item.schema) ? (item.schema as unknown[]) : [];
    const names = list.map((argument) => text(asRecord(argument)?.name)).filter(Boolean);
    return stringifyJson(Object.fromEntries(names.map((name) => [name, ''])), indent);
  }
  const properties = asRecord(asRecord(item.schema)?.properties);
  if (!properties) {
    return '{}';
  }
  const entries = Object.entries(properties).map(([name, definition]) => [
    name,
    sampleValue(asRecord(definition)),
  ]);
  return stringifyJson(Object.fromEntries(entries), indent);
}

function sampleValue(definition: Record<string, unknown> | null): unknown {
  if (definition && 'default' in definition) {
    return definition.default;
  }
  const declared: unknown = definition?.type;
  const type: unknown = Array.isArray(declared) ? (declared as unknown[])[0] : declared;
  switch (type) {
    case 'number':
    case 'integer':
      return 0;
    case 'boolean':
      return false;
    case 'array':
      return [];
    case 'object':
      return {};
    default:
      return '';
  }
}
