import type {
  Auth,
  BodyMode,
  FormFieldKind,
  GrpcSettings,
  McpOperation,
  McpTransportName,
  PathVariable,
  RequestContent,
  RequestKind,
  SavedRequest,
  SoapSettings,
  StoredFormField,
  StoredKeyValue,
} from '../api/collections';
import type { GrpcTarget } from '../api/grpc';
import type { HttpMethod, HttpRequestInput, NameValue, RequestBody } from '../api/http';
import type { McpConnectInput } from '../api/mcp';
import type { WsConnectInput } from '../api/websocket';
import { isHttpMethod } from './methods';
import { pathVariableValues } from './path-variables';
import { soapContentType } from './soap';

export type { BodyMode };

export interface KeyValueRow {
  id: string;
  enabled: boolean;
  key: string;
  value: string;
  /** Only form rows use it; a file row's value is the picked file's path. */
  kind?: FormFieldKind;
}

export interface RequestDraft {
  kind: RequestKind;
  name: string;
  method: HttpMethod;
  url: string;
  params: KeyValueRow[];
  headers: KeyValueRow[];
  bodyMode: BodyMode;
  body: string;
  form: KeyValueRow[];
  bodyFile: string;
  preRequestScript: string;
  testScript: string;
  auth: Auth;
  /** MCP requests: how the server is reached; `url` is its URL or the command that starts it. */
  transport: McpTransportName;
  /** MCP requests: what is called; `body` holds its arguments (JSON). */
  mcpOperation: McpOperation | null;
  /** Values for `:name` path segments; ones whose name left the URL are kept until saving. */
  pathVariables: PathVariable[];
  /** GraphQL requests: the variables (JSON); `body` holds the query. */
  graphqlVariables: string;
  /** gRPC requests: the method called; `headers` are the metadata, `body` the message (JSON). */
  grpc: GrpcSettings;
  /** SOAP requests: the WSDL operation; `url` is the endpoint, `body` the envelope (XML). */
  soap: SoapSettings;
}

/** The Content-Type a body gets unless one is typed; form data's is set with its boundary. */
const CONTENT_TYPES = new Map<BodyMode, string>([
  ['json', 'application/json'],
  ['text', 'text/plain; charset=utf-8'],
  ['xml', 'application/xml'],
  ['html', 'text/html; charset=utf-8'],
  ['urlencoded', 'application/x-www-form-urlencoded'],
  ['binary', 'application/octet-stream'],
]);

export function createRow(): KeyValueRow {
  return { id: crypto.randomUUID(), enabled: true, key: '', value: '' };
}

const NEW_NAMES = new Map<RequestKind, string>([
  ['http', 'New request'],
  ['websocket', 'New WebSocket'],
  ['mcp', 'New MCP server'],
  ['graphql', 'New GraphQL request'],
  ['grpc', 'New gRPC request'],
  ['soap', 'New SOAP request'],
]);
const NEW_BODY_MODES = new Map<RequestKind, BodyMode>([
  ['http', 'none'],
  ['websocket', 'text'],
  ['mcp', 'json'],
  ['graphql', 'json'],
  ['grpc', 'json'],
  ['soap', 'xml'],
]);

export function newRequestName(kind: RequestKind): string {
  return NEW_NAMES.get(kind) ?? 'New request';
}

// A WebSocket request keeps the message being composed in `body`, an MCP request the arguments
// of the tool or prompt it calls; MCP `headers` are HTTP headers or the program's environment.
export function createDraft(kind: RequestKind): RequestDraft {
  return {
    kind,
    name: newRequestName(kind),
    // GraphQL and SOAP go out as POST; gRPC is stored with POST, as HTTP/2 sends it.
    method: kind === 'http' || kind === 'websocket' || kind === 'mcp' ? 'GET' : 'POST',
    url: '',
    params: [createRow()],
    headers: [createRow()],
    bodyMode: NEW_BODY_MODES.get(kind) ?? 'none',
    body: '',
    form: [createRow()],
    bodyFile: '',
    preRequestScript: '',
    testScript: '',
    auth: { type: 'inherit' },
    transport: 'STDIO',
    mcpOperation: null,
    pathVariables: [],
    graphqlVariables: '',
    grpc: blankGrpc(),
    soap: blankSoap(),
  };
}

function blankGrpc(): GrpcSettings {
  return { method: '', source: 'reflection', protoFile: '' };
}

function blankSoap(): SoapSettings {
  return { wsdl: '', version: '1.1', binding: '', operation: '', action: '' };
}

export function draftFromSaved(saved: SavedRequest): RequestDraft {
  return {
    kind: saved.kind,
    name: saved.name,
    method: isHttpMethod(saved.method) ? saved.method : 'GET',
    url: saved.url,
    params: rowsFromStored(saved.params),
    headers: rowsFromStored(saved.headers),
    bodyMode: saved.bodyMode,
    body: saved.body,
    form: rowsFromStored(saved.form),
    bodyFile: saved.bodyFile,
    preRequestScript: saved.preRequestScript,
    testScript: saved.testScript,
    auth: saved.auth,
    transport: saved.method === 'HTTP' ? 'HTTP' : 'STDIO',
    mcpOperation: saved.mcpOperation,
    pathVariables: saved.pathVariables.map((variable) => ({ ...variable })),
    graphqlVariables: saved.graphqlVariables,
    grpc: saved.grpc ? { ...saved.grpc } : blankGrpc(),
    soap: saved.soap ? { ...saved.soap } : blankSoap(),
  };
}

export function contentFromDraft(draft: RequestDraft): RequestContent {
  const isMcp = draft.kind === 'mcp';
  return {
    kind: draft.kind,
    name: draft.name,
    method: isMcp ? draft.transport : draft.method,
    url: draft.url,
    params: storedFromRows(draft.params),
    headers: storedFromRows(draft.headers),
    bodyMode: draft.bodyMode,
    body: draft.body,
    form: formFromRows(draft.form),
    bodyFile: draft.bodyFile,
    preRequestScript: draft.preRequestScript,
    testScript: draft.testScript,
    auth: draft.auth,
    mcpOperation: isMcp ? draft.mcpOperation : null,
    pathVariables: pathVariableValues(draft.url, draft.pathVariables).map(({ name, value }) => ({
      key: name,
      value,
    })),
    graphqlVariables: draft.kind === 'graphql' ? draft.graphqlVariables : '',
    grpc: draft.kind === 'grpc' ? { ...draft.grpc } : null,
    soap: draft.kind === 'soap' ? { ...draft.soap } : null,
  };
}

/** `{{variables}}` stay in place; the Rust core fills them in when connecting. */
export function buildMcpConnectInput(
  draft: RequestDraft,
  collectionId: string | null,
): McpConnectInput {
  return {
    transport: draft.transport === 'HTTP' ? 'http' : 'stdio',
    target: draft.url.trim(),
    pairs: storedFromRows(draft.headers),
    collectionId,
  };
}

/** `{{variables}}` stay in place; the Rust core fills them in when calling. */
export function buildGrpcTarget(draft: RequestDraft, location: RequestLocation): GrpcTarget {
  return {
    url: draft.url.trim(),
    source: draft.grpc.source,
    protoFile: draft.grpc.protoFile,
    metadata: activePairs(draft.headers),
    auth: draft.auth,
    ...location,
  };
}

export function countActiveRows(rows: KeyValueRow[]): number {
  return activeRows(rows).length;
}

export interface RequestLocation {
  collectionId: string | null;
  requestId: string | null;
}

export function buildRequestInput(
  draft: RequestDraft,
  location: RequestLocation,
): HttpRequestInput {
  const isSoap = draft.kind === 'soap';
  return {
    method: isSoap ? 'POST' : draft.method,
    url: draft.url.trim(),
    params: activePairs(draft.params),
    headers: isSoap ? soapHeaders(draft) : buildHeaders(draft.headers, draft.bodyMode),
    body:
      draft.kind === 'graphql'
        ? { mode: 'graphql', query: draft.body, variables: draft.graphqlVariables }
        : buildBody(draft),
    timeoutMs: null,
    name: draft.name,
    preRequestScript: blankToNull(draft.preRequestScript),
    testScript: blankToNull(draft.testScript),
    auth: draft.auth,
    recordHistory: true,
    pathVariables: pathVariableValues(draft.url, draft.pathVariables),
    ...location,
  };
}

/** URL-encoded bodies carry text only, so file rows are left out of them. */
export function buildBody(draft: RequestDraft): RequestBody | null {
  switch (draft.bodyMode) {
    case 'none':
      return null;
    case 'json':
    case 'xml':
    case 'html':
    case 'text':
      return { mode: 'raw', text: draft.body };
    case 'urlencoded':
      return { mode: 'urlencoded', fields: activePairs(draft.form.filter(isTextRow)) };
    case 'formdata':
      return {
        mode: 'formData',
        parts: activeRows(draft.form).map((row) => ({
          name: row.key.trim(),
          value: row.value,
          file: row.kind === 'file',
        })),
      };
    case 'binary':
      return { mode: 'binary', path: draft.bodyFile };
  }
}

export function isTextRow(row: KeyValueRow): boolean {
  return row.kind !== 'file';
}

export function buildWsConnectInput(
  draft: RequestDraft,
  collectionId: string | null,
): WsConnectInput {
  return {
    url: draft.url.trim(),
    params: activePairs(draft.params),
    headers: activePairs(draft.headers),
    collectionId,
  };
}

export function activePairs(rows: KeyValueRow[]): NameValue[] {
  return activeRows(rows).map((row) => ({ name: row.key.trim(), value: row.value }));
}

/** The URL with its parameters as typed (not encoded), for display and code templates. */
export function withQuery(url: string, params: NameValue[]): string {
  if (params.length === 0) {
    return url;
  }
  const query = params.map((param) => `${param.name}=${param.value}`).join('&');
  return `${url}${url.includes('?') ? '&' : '?'}${query}`;
}

function blankToNull(script: string): string | null {
  return script.trim() === '' ? null : script;
}

// The editor always keeps one empty row at the end for typing a new entry.
function rowsFromStored(stored: StoredKeyValue[] | StoredFormField[]): KeyValueRow[] {
  return [...stored.map((row) => ({ id: crypto.randomUUID(), ...row })), createRow()];
}

function filledRows(rows: KeyValueRow[]): KeyValueRow[] {
  return rows.filter((row) => row.key.trim() !== '' || row.value !== '');
}

function storedFromRows(rows: KeyValueRow[]): StoredKeyValue[] {
  return filledRows(rows).map(({ enabled, key, value }) => ({ enabled, key, value }));
}

function formFromRows(rows: KeyValueRow[]): StoredFormField[] {
  return filledRows(rows).map(({ enabled, key, value, kind }) => ({
    enabled,
    key,
    value,
    kind: kind ?? 'text',
  }));
}

function activeRows(rows: KeyValueRow[]): KeyValueRow[] {
  return rows.filter((row) => row.enabled && row.key.trim() !== '');
}

/** SOAP 1.1 names the action in a SOAPAction header, 1.2 in the Content-Type; typed headers win. */
function soapHeaders(draft: RequestDraft): NameValue[] {
  const headers = activePairs(draft.headers);
  const has = (name: string): boolean =>
    headers.some((header) => header.name.toLowerCase() === name);
  const { version, action } = draft.soap;
  if (!has('content-type')) {
    headers.push({ name: 'Content-Type', value: soapContentType(version, action) });
  }
  if (version === '1.1' && !has('soapaction')) {
    headers.push({ name: 'SOAPAction', value: `"${action}"` });
  }
  return headers;
}

function buildHeaders(rows: KeyValueRow[], bodyMode: BodyMode): NameValue[] {
  const headers = activePairs(rows);
  const hasContentType = headers.some((header) => header.name.toLowerCase() === 'content-type');
  const contentType = CONTENT_TYPES.get(bodyMode);
  if (contentType && !hasContentType) {
    headers.push({ name: 'Content-Type', value: contentType });
  }
  return headers;
}
