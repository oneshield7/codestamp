import type { AppError, HttpResponseOutput, NameValue } from '../api/http';
import { prettyJson } from '../format/json';
import { statusTone } from '../format/status';
import { formatClock, formatDateTime } from '../format/time';
import { formatBytes, formatDuration } from '../format/units';
import {
  isSensitiveName,
  maskJson,
  maskQuery,
  maskSecret,
  maskText,
  SECRET_MASK,
} from '../privacy/secrets';
import type { Auth } from '../api/collections';
import { methodTone, type MethodTone } from '../request/methods';
import {
  addAuthHeader,
  addAuthQuery,
  authPair,
  effectiveAuth,
  type AuthPair,
} from '../request/auth';
import { fileName } from '../request/body';
import { sendsPlainHttp } from '../request/kinds';
import {
  activePairs,
  buildRequestInput,
  isTextRow,
  withQuery,
  type RequestDraft,
} from '../request/draft';
import { LOG_MARKS, STATUS_LABELS } from '../request/websocket';
import type { ConnectionStatus, LogEntry } from '../state/websocket.svelte';

export type Tone =
  | 'text'
  | 'muted'
  | 'key'
  | 'string'
  | 'number'
  | 'literal'
  | 'punct'
  | 'ok'
  | 'err'
  | 'accent'
  | 'mask';

export interface Span {
  text: string;
  tone: Tone;
}

export type CaptureBlock =
  | { kind: 'label'; spans: Span[] }
  | { kind: 'lines'; lines: Span[][]; more: string | null }
  | { kind: 'note'; text: string; tone: Tone };

/** What the image shows, already masked; the renderer only lays it out. */
export interface CaptureDocument {
  method: { text: string; tone: MethodTone } | null;
  url: string | null;
  request: CaptureBlock[];
  response: CaptureBlock[];
  footer: string | null;
  /** Names whose values were hidden, for the Privacy hint. */
  hidden: string[];
  /** Some shown text was not JSON, so it was only checked for common patterns. */
  heuristic: boolean;
}

export type LineLimit = 8 | 20 | 50 | 'all';

export interface CaptureOptions {
  methodAndUrl: boolean;
  requestHeaders: boolean;
  requestBody: boolean;
  responseSummary: boolean;
  responseHeaders: boolean;
  responseBody: boolean;
  lineLimit: LineLimit;
  hideSecrets: boolean;
  resolvedUrl: boolean;
  theme: 'light' | 'dark';
  layout: 'side' | 'stacked';
  footer: boolean;
}

export interface CaptureSource {
  draft: RequestDraft;
  /** For requests whose auth is "Inherit from collection". */
  collectionAuth: Auth | null;
  /** The URL as sent, with secret variables masked by the Rust core. */
  resolvedUrl: string | null;
  response: HttpResponseOutput | null;
  error: AppError | null;
  /** WebSocket cards capture their message log instead of a response. */
  log: LogEntry[] | null;
  connection: ConnectionStatus | null;
  environmentName: string | null;
  capturedAt: number;
}

// "All" still has a ceiling so a huge body cannot produce an image too big to hold in memory.
const ALL_LINES_LIMIT = 300;
const JSON_LITERALS = ['true', 'false', 'null'];
const NUMBER_CHARS = '0123456789.eE+-';
const MASKED_STRING = `"${SECRET_MASK}"`;
const MARK_TONES = new Map<LogEntry['kind'], Tone>([
  ['sent', 'accent'],
  ['received', 'ok'],
  ['info', 'muted'],
  ['error', 'err'],
]);

class Collector {
  readonly names = new Set<string>();
  heuristic = false;

  /** Masks the value when the name looks sensitive and hiding is on; an auth scheme stays readable. */
  value(name: string, value: string, hide: boolean): Span[] {
    if (!hide || !isSensitiveName(name)) {
      return [{ text: value, tone: 'text' }];
    }
    this.names.add(name);
    const scheme = maskSecret(value).slice(0, -SECRET_MASK.length);
    const mask: Span = { text: SECRET_MASK, tone: 'mask' };
    return scheme ? [{ text: scheme, tone: 'text' }, mask] : [mask];
  }

  /** Indented (JSON) text with sensitive keys hidden when hiding is on. */
  text(text: string, hide: boolean, indent: number): { text: string; json: boolean } {
    if (hide) {
      const masked = maskJson(text, indent);
      if (masked) {
        masked.hidden.forEach((name) => this.names.add(name));
        return { text: masked.text, json: true };
      }
      const plain = maskText(text);
      plain.hidden.forEach((name) => this.names.add(name));
      this.heuristic = true;
      return { text: plain.text, json: false };
    }
    const pretty = indent > 0 ? prettyJson(text) : null;
    return pretty === null ? { text, json: false } : { text: pretty, json: true };
  }
}

export function buildDocument(source: CaptureSource, options: CaptureOptions): CaptureDocument {
  const collector = new Collector();
  const { draft } = source;
  const isWebSocket = draft.kind === 'websocket';
  return {
    method: options.methodAndUrl ? methodLabel(draft) : null,
    url: options.methodAndUrl ? displayUrl(source, options, collector) : null,
    request: requestBlocks(source, options, collector),
    response: isWebSocket
      ? messageBlocks(source, options, collector)
      : responseBlocks(source, options, collector),
    footer: options.footer ? footerText(source) : null,
    hidden: [...collector.names],
    heuristic: collector.heuristic,
  };
}

export function hasRequestBody(draft: RequestDraft): boolean {
  if (!sendsPlainHttp(draft.kind)) {
    return false;
  }
  switch (draft.bodyMode) {
    case 'none':
      return false;
    case 'json':
    case 'xml':
    case 'html':
    case 'text':
      return draft.body.trim() !== '';
    case 'urlencoded':
    case 'formdata':
      return formPairs(draft).length > 0;
    case 'binary':
      return draft.bodyFile !== '';
  }
}

/** The form rows that are sent, as `key: value`; file rows show the file's name. */
function formPairs(draft: RequestDraft): NameValue[] {
  return draft.form
    .filter(
      (row) =>
        row.enabled && row.key.trim() !== '' && (draft.bodyMode === 'formdata' || isTextRow(row)),
    )
    .map((row) => ({
      name: row.key.trim(),
      value: isTextRow(row)
        ? row.value
        : `${row.value === '' ? 'no file chosen' : fileName(row.value)} (file)`,
    }));
}

function methodLabel(draft: RequestDraft): { text: string; tone: MethodTone } {
  if (draft.kind === 'websocket') {
    return { text: 'WS', tone: 'put' };
  }
  return { text: draft.method, tone: methodTone(draft.method) };
}

function displayUrl(source: CaptureSource, options: CaptureOptions, collector: Collector): string {
  const params = activePairs(source.draft.params);
  addAuthQuery(generatedAuth(source), params);
  const typed = withQuery(source.draft.url.trim(), params);
  const url = options.resolvedUrl && source.resolvedUrl !== null ? source.resolvedUrl : typed;
  if (!options.hideSecrets) {
    return url;
  }
  const masked = maskQuery(url);
  masked.hidden.forEach((name) => collector.names.add(name));
  return masked.text;
}

/** The header or query parameter that the request's auth adds (HTTP and SOAP requests only). */
function generatedAuth(source: CaptureSource): AuthPair | null {
  if (!sendsPlainHttp(source.draft.kind)) {
    return null;
  }
  return authPair(effectiveAuth(source.draft.auth, source.collectionAuth));
}

function requestBlocks(
  source: CaptureSource,
  options: CaptureOptions,
  collector: Collector,
): CaptureBlock[] {
  const { draft } = source;
  const blocks: CaptureBlock[] = [];
  if (options.requestHeaders) {
    // The headers as sent, including the Content-Type added for a body and the auth header.
    const headers =
      draft.kind === 'websocket'
        ? activePairs(draft.headers)
        : buildRequestInput(draft, { collectionId: null, requestId: null }).headers;
    addAuthHeader(generatedAuth(source), headers);
    if (headers.length > 0) {
      blocks.push(label(draft.kind === 'websocket' ? 'Handshake headers' : 'Request headers'));
      blocks.push(pairLines(headers, options.hideSecrets, collector));
    }
  }
  if (options.requestBody && hasRequestBody(draft)) {
    blocks.push(label('Request body'));
    if (draft.bodyMode === 'urlencoded' || draft.bodyMode === 'formdata') {
      blocks.push(pairLines(formPairs(draft), options.hideSecrets, collector));
    } else if (draft.bodyMode === 'binary') {
      blocks.push({ kind: 'note', text: `File: ${fileName(draft.bodyFile)}`, tone: 'muted' });
    } else {
      const body = collector.text(draft.body, options.hideSecrets, 2);
      blocks.push(codeLines(body.text, body.json, options.lineLimit));
    }
  }
  return blocks;
}

function responseBlocks(
  source: CaptureSource,
  options: CaptureOptions,
  collector: Collector,
): CaptureBlock[] {
  const wanted = options.responseSummary || options.responseHeaders || options.responseBody;
  if (!wanted) {
    return [];
  }
  const { response, error } = source;
  if (error) {
    return [label('Response'), { kind: 'note', text: error.message, tone: 'err' }];
  }
  if (!response) {
    return [
      label('Response'),
      { kind: 'note', text: 'No response yet. Send the request first.', tone: 'muted' },
    ];
  }
  const blocks: CaptureBlock[] = [summaryLabel(response, options.responseSummary)];
  if (options.responseHeaders && response.headers.length > 0) {
    blocks.push(label('Response headers'));
    blocks.push(pairLines(response.headers, options.hideSecrets, collector));
  }
  if (options.responseBody) {
    if (response.bodyPreview === '') {
      blocks.push({ kind: 'note', text: 'Empty body', tone: 'muted' });
    } else {
      const body = collector.text(response.bodyPreview, options.hideSecrets, 2);
      blocks.push(codeLines(body.text, body.json, options.lineLimit));
    }
  }
  return blocks;
}

function summaryLabel(response: HttpResponseOutput, withSummary: boolean): CaptureBlock {
  if (!withSummary) {
    return label('Response');
  }
  const tone = statusTone(response.status);
  return {
    kind: 'label',
    spans: [
      { text: 'RESPONSE · ', tone: 'muted' },
      {
        text: `${String(response.status)} ${response.statusText}`.toUpperCase(),
        tone: tone === 'neutral' ? 'text' : tone,
      },
      {
        text: ` · ${formatDuration(response.elapsedMs)} · ${formatBytes(response.bodySize)}`,
        tone: 'muted',
      },
    ],
  };
}

function messageBlocks(
  source: CaptureSource,
  options: CaptureOptions,
  collector: Collector,
): CaptureBlock[] {
  if (!options.responseBody && !options.responseSummary) {
    return [];
  }
  const status = STATUS_LABELS.get(source.connection ?? 'idle') ?? '';
  const blocks: CaptureBlock[] = [
    options.responseSummary
      ? {
          kind: 'label',
          spans: [
            { text: 'MESSAGES · ', tone: 'muted' },
            {
              text: status.toUpperCase(),
              tone: source.connection === 'open' ? 'ok' : 'muted',
            },
          ],
        }
      : label('Messages'),
  ];
  if (!options.responseBody) {
    return blocks;
  }
  const log = source.log ?? [];
  if (log.length === 0) {
    blocks.push({ kind: 'note', text: 'No messages yet.', tone: 'muted' });
    return blocks;
  }
  const limit = options.lineLimit === 'all' ? ALL_LINES_LIMIT : options.lineLimit;
  const shown = log.slice(-limit);
  const earlier = log.length - shown.length;
  blocks.push({
    kind: 'lines',
    lines: shown.map((entry) => messageLine(entry, options.hideSecrets, collector)),
    more: earlier > 0 ? `… ${String(earlier)} earlier messages` : null,
  });
  return blocks;
}

function messageLine(entry: LogEntry, hide: boolean, collector: Collector): Span[] {
  const text = entry.binary
    ? `Binary message · ${formatBytes(entry.size ?? 0)}`
    : collector.text(entry.text, hide, 0).text;
  return [
    { text: `${LOG_MARKS.get(entry.kind) ?? ''} `, tone: MARK_TONES.get(entry.kind) ?? 'muted' },
    { text: `${formatClock(entry.at, true)}  `, tone: 'muted' },
    { text, tone: entry.kind === 'error' ? 'err' : entry.kind === 'info' ? 'muted' : 'text' },
  ];
}

function label(text: string): CaptureBlock {
  return { kind: 'label', spans: [{ text: text.toUpperCase(), tone: 'muted' }] };
}

function pairLines(pairs: NameValue[], hide: boolean, collector: Collector): CaptureBlock {
  return {
    kind: 'lines',
    lines: pairs.map((pair) => [
      { text: `${pair.name}: `, tone: 'muted' },
      ...collector.value(pair.name, pair.value, hide),
    ]),
    more: null,
  };
}

function codeLines(text: string, json: boolean, limit: LineLimit): CaptureBlock {
  const all = text.split(/\r?\n/);
  const max = limit === 'all' ? ALL_LINES_LIMIT : limit;
  const shown = all.slice(0, max);
  const hiddenCount = all.length - shown.length;
  return {
    kind: 'lines',
    lines: shown.map((line) => (json ? jsonSpans(line) : [{ text: line, tone: 'text' }])),
    more: hiddenCount > 0 ? `… ${String(hiddenCount)} more lines` : null,
  };
}

/**
 * Colours one line of indented JSON: keys, strings, numbers, literals and punctuation.
 * Outside strings, valid JSON only has those tokens, so a simple scan is enough.
 */
export function jsonSpans(line: string): Span[] {
  const spans: Span[] = [];
  let plainStart = 0;
  let index = 0;
  const push = (end: number, tone: Tone): void => {
    if (index > plainStart) {
      spans.push({ text: line.slice(plainStart, index), tone: 'punct' });
    }
    spans.push({ text: line.slice(index, end), tone });
    index = end;
    plainStart = end;
  };
  while (index < line.length) {
    const char = line.charAt(index);
    if (char === '"') {
      const end = stringEnd(line, index);
      const text = line.slice(index, end);
      const isKey = line.slice(end).trimStart().startsWith(':');
      push(end, isKey ? 'key' : text === MASKED_STRING ? 'mask' : 'string');
    } else if (char === '-' || isDigit(char)) {
      push(numberEnd(line, index), 'number');
    } else {
      const literal = JSON_LITERALS.find((word) => line.startsWith(word, index));
      if (literal) {
        push(index + literal.length, 'literal');
      } else {
        index += 1;
      }
    }
  }
  if (plainStart < line.length) {
    spans.push({ text: line.slice(plainStart), tone: 'punct' });
  }
  return spans;
}

function stringEnd(line: string, start: number): number {
  let index = start + 1;
  while (index < line.length) {
    const char = line.charAt(index);
    if (char === '\\') {
      index += 2;
    } else if (char === '"') {
      return index + 1;
    } else {
      index += 1;
    }
  }
  return line.length;
}

function numberEnd(line: string, start: number): number {
  let index = start + 1;
  while (index < line.length && NUMBER_CHARS.includes(line.charAt(index))) {
    index += 1;
  }
  return index;
}

function isDigit(char: string): boolean {
  return char >= '0' && char <= '9';
}

function footerText(source: CaptureSource): string {
  const environment = source.environmentName
    ? `Environment: ${source.environmentName}`
    : 'No environment';
  return `${formatDateTime(source.capturedAt)} · ${environment}`;
}
