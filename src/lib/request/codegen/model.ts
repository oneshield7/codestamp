import type { Auth } from '../../api/collections';
import type { HttpMethod, NameValue, RequestBody, ResolvedRequest } from '../../api/http';
import { prettyJson } from '../../format/json';
import { addAuthHeader, addAuthQuery, authPair, effectiveAuth } from '../auth';
import { activePairs, buildBody, buildRequestInput, type RequestDraft } from '../draft';
import { pathVariableValues } from '../path-variables';

/** Auth the tool itself has to add, because it depends on the server or the clock. */
export type CodeSigning =
  | { kind: 'digest'; username: string; password: string }
  | {
      kind: 'aws';
      accessKey: string;
      secretKey: string;
      sessionToken: string;
      region: string;
      service: string;
    };

export interface BasicCredentials {
  username: string;
  password: string;
}

/** What the generated code sends. Each language encodes the query and auth its own way. */
export interface CodeRequest {
  method: HttpMethod;
  /** Without the parameters in `query`; a query typed into the URL itself stays here. */
  url: string;
  /** Names and values as typed, not encoded yet. */
  query: NameValue[];
  headers: NameValue[];
  /** GraphQL arrives as the raw JSON it is sent as. */
  body: Exclude<RequestBody, { mode: 'graphql' }> | null;
  /** Basic auth from the Auth tab, so the code can encode the credentials when it runs. */
  basic?: BasicCredentials;
  signing?: CodeSigning;
}

/** The request as typed, `{{variables}}` kept, with the header or parameter its auth adds. */
export function templateRequest(draft: RequestDraft, collectionAuth: Auth): CodeRequest {
  const query = activePairs(draft.params);
  const { headers, method } = buildRequestInput(draft, { collectionId: null, requestId: null });
  const auth = effectiveAuth(draft.auth, collectionAuth);
  const basic =
    auth.type === 'basic' && auth.username !== ''
      ? { username: auth.username, password: auth.password }
      : undefined;
  if (!basic) {
    const pair = authPair(auth);
    addAuthHeader(pair, headers);
    addAuthQuery(pair, query);
  }
  const body =
    draft.kind === 'graphql'
      ? { mode: 'raw' as const, text: graphqlJson(draft) }
      : tidyBody(buildBody(draft));
  return {
    method,
    url: fillPathVariables(draft.url.trim(), pathVariableValues(draft.url, draft.pathVariables)),
    query,
    headers,
    body,
    basic,
    signing: signingFor(auth),
  };
}

/** A request the Rust core filled in; its query is split off again so it reads like the rest. */
export function filledRequest(resolved: ResolvedRequest): CodeRequest {
  const [address = '', fragment] = resolved.url.split('#', 2);
  const at = address.indexOf('?');
  const query =
    at < 0
      ? []
      : address
          .slice(at + 1)
          .split('&')
          .filter((pair) => pair !== '')
          .map((pair) => {
            const equals = pair.indexOf('=');
            return equals < 0
              ? { name: decoded(pair), value: '' }
              : { name: decoded(pair.slice(0, equals)), value: decoded(pair.slice(equals + 1)) };
          });
  const base = at < 0 ? address : address.slice(0, at);
  return {
    method: resolved.method,
    url: fragment === undefined ? base : `${base}#${fragment}`,
    query,
    headers: resolved.headers,
    body: tidyBody(resolved.body),
  };
}

/** Form encoding writes a space as `+`; a malformed escape is kept as it was. */
function decoded(text: string): string {
  const spaced = text.replaceAll('+', ' ');
  try {
    return decodeURIComponent(spaced);
  } catch {
    return spaced;
  }
}

/** A `:name` path segment with a value typed for it takes that value, as when sending. */
function fillPathVariables(url: string, values: NameValue[]): string {
  const end = url.search(/[?#]/);
  const path = end < 0 ? url : url.slice(0, end);
  const filled = path
    .split('/')
    .map((segment) => {
      const value = values.find((item) => `:${item.name}` === segment)?.value ?? '';
      return value === '' ? segment : value;
    })
    .join('/');
  return end < 0 ? filled : `${filled}${url.slice(end)}`;
}

/** JSON is laid out one field per line; any other body stays exactly as typed. */
function tidyBody(body: RequestBody | null): CodeRequest['body'] {
  if (!body || body.mode === 'graphql') {
    return null;
  }
  if (body.mode !== 'raw') {
    return body;
  }
  return { mode: 'raw', text: prettyJson(body.text) ?? body.text };
}

/** The JSON a GraphQL request is sent as; variables that are not JSON yet are left out. */
function graphqlJson(draft: RequestDraft): string {
  let variables: unknown;
  try {
    variables =
      draft.graphqlVariables.trim() === '' ? undefined : JSON.parse(draft.graphqlVariables);
  } catch {
    variables = undefined;
  }
  return JSON.stringify({ query: draft.body, variables }, null, 2);
}

function signingFor(auth: Auth): CodeSigning | undefined {
  if (auth.type === 'digest') {
    return { kind: 'digest', username: auth.username, password: auth.password };
  }
  if (auth.type === 'awsSigV4') {
    const { accessKey, secretKey, sessionToken, region, service } = auth;
    return { kind: 'aws', accessKey, secretKey, sessionToken, region, service };
  }
  return undefined;
}

/** Multipart bodies need the boundary the tool picks, so their typed Content-Type is dropped. */
export function sentHeaders(request: CodeRequest): NameValue[] {
  if (request.body?.mode !== 'formData') {
    return request.headers;
  }
  return request.headers.filter((header) => header.name.toLowerCase() !== 'content-type');
}

export function headerValue(headers: NameValue[], name: string): string | undefined {
  return headers.find((header) => header.name.toLowerCase() === name.toLowerCase())?.value;
}

/** Headers with the same name, merged into one comma-separated value for maps that keep one. */
export function mergedHeaders(headers: NameValue[]): NameValue[] {
  const merged: NameValue[] = [];
  for (const header of headers) {
    const existing = merged.find((item) => item.name.toLowerCase() === header.name.toLowerCase());
    if (existing) {
      existing.value = `${existing.value}, ${header.value}`;
    } else {
      merged.push({ ...header });
    }
  }
  return merged;
}

export function hasDuplicateNames(pairs: NameValue[]): boolean {
  const names = pairs.map((pair) => pair.name.toLowerCase());
  return new Set(names).size !== names.length;
}

/** The method as most HTTP libraries spell it in their helpers, such as `Post`. */
export function titleCase(method: string): string {
  return method.charAt(0) + method.slice(1).toLowerCase();
}
