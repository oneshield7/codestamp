import { invoke } from '@tauri-apps/api/core';
import type { Auth } from './collections';

export const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const;
export type HttpMethod = (typeof HTTP_METHODS)[number];

export interface NameValue {
  name: string;
  value: string;
}

/** For a file part, `value` is the path of a file picked with `pickUploadFile`. */
export interface FormPart {
  name: string;
  value: string;
  file: boolean;
}

/** Only files picked in the system dialog can be sent; the Rust core checks every path. */
export type RequestBody =
  | { mode: 'raw'; text: string }
  | { mode: 'urlencoded'; fields: NameValue[] }
  | { mode: 'formData'; parts: FormPart[] }
  | { mode: 'binary'; path: string }
  | { mode: 'graphql'; query: string; variables: string };

/** `{{variables}}` are left in place; the Rust core fills them in when sending. */
export interface HttpRequestInput {
  method: HttpMethod;
  url: string;
  params: NameValue[];
  headers: NameValue[];
  body: RequestBody | null;
  timeoutMs: number | null;
  collectionId: string | null;
  requestId: string | null;
  name: string | null;
  preRequestScript: string | null;
  testScript: string | null;
  /** `inherit` uses the collection's auth; null sends none. */
  auth: Auth | null;
  /** False for collection runs, which do not add to history. */
  recordHistory: boolean;
  /** One value per `:name` in the URL path. */
  pathVariables: NameValue[];
  /** The current row of a run's data file; it wins over every saved scope. */
  iterationData?: NameValue[];
  /** Which pass of a run this is, for `pm.info.iteration`. */
  iteration?: RunIteration;
}

export interface RunIteration {
  /** Counted from 0, as in `pm.info.iteration`. */
  index: number;
  count: number;
}

export interface TestResult {
  name: string;
  passed: boolean;
  error: string | null;
}

export interface LogLine {
  level: 'log' | 'info' | 'warn' | 'error';
  message: string;
}

export interface ScriptReport {
  tests: TestResult[];
  logs: LogLine[];
  errors: string[];
}

/** A cookie set by a response. `expires` is Unix time in seconds; null for a session cookie. */
export interface ResponseCookie {
  name: string;
  value: string;
  domain: string | null;
  path: string | null;
  expires: number | null;
  maxAge: number | null;
  httpOnly: boolean;
  secure: boolean;
  sameSite: string | null;
}

export interface HttpResponseOutput {
  status: number;
  statusText: string;
  httpVersion: string;
  headers: NameValue[];
  contentType: string | null;
  bodyPreview: string;
  bodySize: number;
  bodyTruncated: boolean;
  /** Set while the whole body is kept in the Rust core and can be saved to a file. */
  bodyId: string | null;
  /** Set for an event stream: its events arrive afterwards (see api/streams). */
  streamId: string | null;
  elapsedMs: number;
  cookies: ResponseCookie[];
  warning: string | null;
  script: ScriptReport | null;
}

/** The request as it would be sent: variables filled in (secrets included) and auth added. */
export interface ResolvedRequest {
  method: HttpMethod;
  url: string;
  headers: NameValue[];
  body: RequestBody | null;
}

export interface PickedFile {
  path: string;
  name: string;
  size: number;
}

export interface AppError {
  code: string;
  message: string;
}

export function sendHttpRequest(request: HttpRequestInput): Promise<HttpResponseOutput> {
  return invoke<HttpResponseOutput>('send_http_request', { request });
}

/** Sends a GraphQL introspection query and returns the whole answer (up to 32 MB). */
export function graphqlSchema(request: HttpRequestInput): Promise<string> {
  return invoke<string>('graphql_schema', { request });
}

/** Scripts are not run; this only fills in variables and adds auth. */
export function resolveRequestForCode(request: HttpRequestInput): Promise<ResolvedRequest> {
  return invoke<ResolvedRequest>('resolve_request_for_code', { request });
}

/** Opens the system file dialog. Only files picked here can be uploaded. Null if cancelled. */
export function pickUploadFile(): Promise<PickedFile | null> {
  return invoke<PickedFile | null>('pick_upload_file');
}

export function toAppError(error: unknown): AppError {
  if (isAppError(error)) {
    return error;
  }
  return { code: 'unknown', message: 'Something went wrong. Please try again.' };
}

function isAppError(value: unknown): value is AppError {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const candidate = value as Partial<Record<keyof AppError, unknown>>;
  return typeof candidate.code === 'string' && typeof candidate.message === 'string';
}
