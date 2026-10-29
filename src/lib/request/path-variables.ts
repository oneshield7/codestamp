import type { PathVariable } from '../api/collections';
import type { NameValue } from '../api/http';
import { splitUrl } from '../format/url';

/** A whole path segment `:name`, read the same way as in the Rust core. */
const VARIABLE_SEGMENT = /^:([A-Za-z_][A-Za-z0-9_-]*)$/;

/** The `:name` segments of a URL path, in order and without repeats. */
export function pathVariableNames(url: string): string[] {
  const path = splitUrl(url).path.split(/[?#]/, 1)[0] ?? '';
  const names: string[] = [];
  for (const segment of path.split('/')) {
    const name = VARIABLE_SEGMENT.exec(segment)?.[1];
    if (name !== undefined && !names.includes(name)) {
      names.push(name);
    }
  }
  return names;
}

/**
 * One value per `:name` in the URL. Values typed for names no longer in the URL are kept in
 * the draft while editing, so renaming a segment back brings its value back.
 */
export function pathVariableValues(url: string, stored: readonly PathVariable[]): NameValue[] {
  return pathVariableNames(url).map((name) => ({
    name,
    value: stored.find((variable) => variable.key === name)?.value ?? '',
  }));
}

export function setPathVariable(stored: PathVariable[], key: string, value: string): void {
  const existing = stored.find((variable) => variable.key === key);
  if (existing) {
    existing.value = value;
  } else {
    stored.push({ key, value });
  }
}
