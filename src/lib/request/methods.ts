import type { McpTransportName, RequestKind, RequestMethod } from '../api/collections';
import { HTTP_METHODS, type HttpMethod } from '../api/http';
import type { DropdownOption } from '../components/Dropdown.svelte';

/** Each tone has `--method-<tone>` (text) plus `-bg` and `-line` tokens for its stamp. */
export type MethodTone = 'get' | 'post' | 'put' | 'patch' | 'delete' | 'other';

const METHOD_TONES = new Map<HttpMethod, MethodTone>([
  ['GET', 'get'],
  ['POST', 'post'],
  ['PUT', 'put'],
  ['PATCH', 'patch'],
  ['DELETE', 'delete'],
]);

export function methodTone(method: RequestMethod): MethodTone {
  return isHttpMethod(method) ? (METHOD_TONES.get(method) ?? 'other') : 'other';
}

/** Request rows stamp WebSocket, MCP, GraphQL, gRPC and SOAP requests in their own colours. */
export type StampTone = MethodTone | 'ws' | 'mcp' | 'gql' | 'grpc' | 'soap';

const KIND_TONES = new Map<RequestKind, StampTone>([
  ['websocket', 'ws'],
  ['mcp', 'mcp'],
  ['graphql', 'gql'],
  ['grpc', 'grpc'],
  ['soap', 'soap'],
]);

export function stampTone(kind: RequestKind, method: RequestMethod): StampTone {
  return KIND_TONES.get(kind) ?? methodTone(method);
}

export function isHttpMethod(method: RequestMethod): method is HttpMethod {
  return (HTTP_METHODS as readonly string[]).includes(method);
}

export function methodColor(method: RequestMethod): string {
  return `var(--method-${methodTone(method)})`;
}

/** The method picker: each method in its own colour, as on the request rows. */
export const METHOD_OPTIONS: DropdownOption<HttpMethod>[] = HTTP_METHODS.map((method) => ({
  value: method,
  label: method,
  color: methodColor(method),
}));

export const MCP_TRANSPORT_OPTIONS: DropdownOption<McpTransportName>[] = [
  { value: 'STDIO', label: 'STDIO', color: 'var(--method-mcp)' },
  { value: 'HTTP', label: 'HTTP', color: 'var(--method-mcp)' },
];
