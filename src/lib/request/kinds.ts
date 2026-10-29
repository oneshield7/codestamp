import type { RequestKind, RequestMethod } from '../api/collections';

/** What a row or report shows instead of the method. */
export const KIND_LABELS = new Map<RequestKind, string>([
  ['websocket', 'WS'],
  ['mcp', 'MCP'],
  ['graphql', 'GQL'],
  ['grpc', 'gRPC'],
  ['soap', 'SOAP'],
]);

/** Requests sent as one plain HTTP request whose body and auth are shown as written. */
export function sendsPlainHttp(kind: RequestKind): boolean {
  return kind === 'http' || kind === 'soap';
}

export function stampLabel(kind: RequestKind, method: RequestMethod): string {
  return KIND_LABELS.get(kind) ?? method;
}

/** Requests that are not sent one after another in a run, with why. */
export const SKIPPED_IN_RUNS = new Map<RequestKind, string>([
  ['websocket', 'WebSocket requests are not part of a run'],
  ['mcp', 'MCP requests are not part of a run'],
  ['grpc', 'gRPC requests are not part of a run'],
]);
