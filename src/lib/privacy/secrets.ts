import { parseJson, prettyJson } from '../format/json';

/** Hides values on screen and in captures. Saved data and what is sent are never changed. */
export const SECRET_MASK = '••••••••';

const SENSITIVE_NAMES = new Set([
  'authorization',
  'proxyauthorization',
  'cookie',
  'setcookie',
  'auth',
  'key',
  'pwd',
  'sig',
]);
const SENSITIVE_PARTS = [
  'token',
  'secret',
  'password',
  'passwd',
  'apikey',
  'session',
  'signature',
  'credential',
  'privatekey',
  'accesskey',
];
const AUTH_SCHEME = /^(Bearer|Basic|Digest|Token|Bot)\s+/i;

/** Header, parameter or JSON key names that usually carry credentials. */
export function isSensitiveName(name: string): boolean {
  const normalized = name.toLowerCase().replace(/[^a-z0-9]/g, '');
  return (
    SENSITIVE_NAMES.has(normalized) || SENSITIVE_PARTS.some((part) => normalized.includes(part))
  );
}

/**
 * A row's value for documentation: a credential typed in as it is gets masked, while one taken
 * from a `{{variable}}` is shown, since the reference says where it comes from.
 */
export function documentedValue(name: string, value: string): string {
  return isSensitiveName(name) && value.trim() !== '' && !value.includes('{{')
    ? maskSecret(value)
    : value;
}

/** Keeps a leading auth scheme (`Bearer ••••••••`) so the screenshot still explains itself. */
export function maskSecret(value: string): string {
  const scheme = AUTH_SCHEME.exec(value)?.[1];
  return scheme ? `${scheme} ${SECRET_MASK}` : SECRET_MASK;
}

export interface Masked {
  text: string;
  /** Names whose values were hidden. */
  hidden: string[];
}

/** JSON with sensitive keys hidden (indented by `indent` spaces), or null when it is not JSON. */
export function maskJson(text: string, indent: number): Masked | null {
  const parsed = parseJson(text);
  if (!parsed) {
    return null;
  }
  const hidden = new Set<string>();
  const masked = maskValue(parsed.value, hidden);
  // Re-serialising loses big-integer precision, so it is only done when something was hidden.
  if (hidden.size === 0) {
    return { text: indent > 0 ? (prettyJson(text, indent) ?? text) : text, hidden: [] };
  }
  return { text: JSON.stringify(masked, null, indent), hidden: [...hidden] };
}

/** Hides sensitive query values in a URL as typed, which may still contain `{{variables}}`. */
export function maskQuery(url: string): Masked {
  const queryStart = url.indexOf('?');
  if (queryStart < 0) {
    return { text: url, hidden: [] };
  }
  const hidden: string[] = [];
  const pairs = url
    .slice(queryStart + 1)
    .split('&')
    .map((pair) => {
      const separator = pair.indexOf('=');
      const name = separator < 0 ? pair : pair.slice(0, separator);
      if (separator < 0 || !isSensitiveName(decodeName(name))) {
        return pair;
      }
      hidden.push(decodeName(name));
      return `${name}=${SECRET_MASK}`;
    });
  return { text: `${url.slice(0, queryStart)}?${pairs.join('&')}`, hidden };
}

// Patterns for text that is not JSON (truncated bodies, forms, logs). Each is simple and
// bounded, so long texts are scanned in linear time.
const JSON_PAIR = /"([\w.-]{1,100})"(\s*:\s*)"[^"\n]{0,4096}"/g;
const FORM_PAIR = /([\w.-]{1,100})=([^&\s"']{1,4096})/g;
const AUTH_TOKEN = /\b(Bearer|Basic|Token)\s+[\w.~+/=-]{8,4096}/gi;

/**
 * Best-effort masking for text that is not valid JSON: `"key": "value"` and `key=value` pairs
 * with sensitive names, and `Bearer …` tokens. Unusual formats may still slip through.
 */
export function maskText(text: string): Masked {
  const hidden = new Set<string>();
  const masked = text
    .replace(JSON_PAIR, (match, name: string, separator: string) => {
      if (!isSensitiveName(name)) {
        return match;
      }
      hidden.add(name);
      return `"${name}"${separator}"${SECRET_MASK}"`;
    })
    .replace(FORM_PAIR, (match, name: string) => {
      if (!isSensitiveName(name)) {
        return match;
      }
      hidden.add(name);
      return `${name}=${SECRET_MASK}`;
    })
    .replace(AUTH_TOKEN, (_match, scheme: string) => {
      hidden.add(scheme);
      return `${scheme} ${SECRET_MASK}`;
    });
  return { text: masked, hidden: [...hidden] };
}

/** JSON masked by keys when it parses, otherwise the best-effort text masking. */
export function maskAny(text: string, indent: number): Masked {
  return maskJson(text, indent) ?? maskText(text);
}

function maskValue(value: unknown, hidden: Set<string>): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => maskValue(item, hidden));
  }
  if (value === null || typeof value !== 'object') {
    return value;
  }
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => {
      if (item !== null && isSensitiveName(key)) {
        hidden.add(key);
        return [key, SECRET_MASK];
      }
      return [key, maskValue(item, hidden)];
    }),
  );
}

function decodeName(name: string): string {
  try {
    return decodeURIComponent(name.replace(/\+/g, ' '));
  } catch {
    // A malformed escape is shown as typed; it is still checked by its raw name.
    return name;
  }
}
