import type { Auth, StoredKeyValue } from '../api/collections';
import type { RequestDocs } from '../api/docs';
import { documentedValue } from '../privacy/secrets';
import { authSummary } from '../request/auth';
import { fileName } from '../request/body';

export interface DocRow {
  name: string;
  value: string;
}

/** Enabled rows, with credentials typed into them masked. */
export function documentedRows(rows: StoredKeyValue[]): DocRow[] {
  return rows
    .filter((row) => row.enabled && row.key.trim() !== '')
    .map((row) => ({ name: row.key, value: documentedValue(row.key, row.value) }));
}

/** Short facts shown under the request: its auth and what it calls. */
export function requestFacts(request: RequestDocs, collectionAuth: Auth): DocRow[] {
  const facts: DocRow[] = [];
  if (request.kind !== 'mcp') {
    facts.push({
      name: 'Auth',
      value:
        request.auth.type === 'inherit'
          ? `From the collection: ${authSummary(collectionAuth)}`
          : authSummary(request.auth),
    });
  }
  if (request.kind === 'grpc' && request.grpc && request.grpc.method !== '') {
    facts.push({ name: 'Method', value: request.grpc.method });
  }
  if (request.kind === 'soap' && request.soap && request.soap.operation !== '') {
    facts.push({
      name: 'Operation',
      value: `${request.soap.operation} · SOAP ${request.soap.version}`,
    });
    if (request.soap.action !== '') {
      facts.push({ name: 'SOAPAction', value: request.soap.action });
    }
  }
  if (request.kind === 'mcp') {
    facts.push({ name: 'Transport', value: request.method });
    if (request.mcpOperation) {
      const { kind, name } = request.mcpOperation;
      facts.push({
        name: kind === 'tool' ? 'Tool' : kind === 'prompt' ? 'Prompt' : 'Resource',
        value: name,
      });
    }
  }
  return facts;
}

/** Form rows of a URL-encoded or form-data body; a file shows its name only. */
export function formRows(request: RequestDocs): DocRow[] {
  if (request.bodyMode !== 'urlencoded' && request.bodyMode !== 'formdata') {
    return [];
  }
  return request.form
    .filter((field) => field.enabled && field.key.trim() !== '')
    .map((field) => ({
      name: field.key,
      value:
        field.kind === 'file'
          ? `File: ${fileName(field.value)}`
          : documentedValue(field.key, field.value),
    }));
}

/** The text a request sends, under the name its card gives it; null when there is none. */
export function bodyOf(request: RequestDocs): { label: string; text: string } | null {
  if (request.bodyMode === 'binary') {
    return request.bodyFile === ''
      ? null
      : { label: 'Body', text: `File: ${fileName(request.bodyFile)}` };
  }
  if (
    request.bodyMode === 'none' ||
    request.bodyMode === 'urlencoded' ||
    request.bodyMode === 'formdata' ||
    request.body.trim() === ''
  ) {
    return null;
  }
  return { label: bodyLabel(request.kind), text: request.body };
}

function bodyLabel(kind: RequestDocs['kind']): string {
  switch (kind) {
    case 'http':
      return 'Body';
    case 'graphql':
      return 'Query';
    case 'grpc':
    case 'websocket':
      return 'Message';
    case 'soap':
      return 'Envelope';
    case 'mcp':
      return 'Arguments';
  }
}
