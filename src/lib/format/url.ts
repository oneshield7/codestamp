export interface UrlParts {
  /** `{{baseUrl}}`, or the scheme and host; empty when the URL has neither. */
  origin: string;
  /** Everything after the origin, query string included. */
  path: string;
}

const LEADING_VARIABLE = /^\{\{[^{}]+\}\}/;
const SCHEME_AND_HOST = /^[a-z][a-z0-9+.-]*:\/\/[^/?#]*/i;

export function splitUrl(url: string): UrlParts {
  const trimmed = url.trim();
  const origin = LEADING_VARIABLE.exec(trimmed)?.[0] ?? SCHEME_AND_HOST.exec(trimmed)?.[0] ?? '';
  return { origin, path: trimmed.slice(origin.length) };
}

/** The path without query string or fragment, `/` when there is none. */
export function urlPath(url: string): string {
  const path = splitUrl(url).path.split(/[?#]/, 1)[0] ?? '';
  return path === '' ? '/' : path;
}
