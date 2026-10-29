import type { BodyMode } from '../api/collections';
import type { EditorLanguage } from '../editor/setup';

export interface BodyModeOption {
  mode: BodyMode;
  label: string;
}

export const BODY_MODES: BodyModeOption[] = [
  { mode: 'none', label: 'None' },
  { mode: 'json', label: 'JSON' },
  { mode: 'xml', label: 'XML' },
  { mode: 'html', label: 'HTML' },
  { mode: 'text', label: 'Text' },
  { mode: 'urlencoded', label: 'URL-encoded' },
  { mode: 'formdata', label: 'Form data' },
  { mode: 'binary', label: 'Binary' },
];

/** The editor highlighting for a raw body. */
export function bodyLanguage(mode: BodyMode): EditorLanguage {
  return mode === 'json' || mode === 'xml' || mode === 'html' ? mode : 'text';
}

export function bodyModeLabel(mode: BodyMode): string {
  return BODY_MODES.find((option) => option.mode === mode)?.label ?? '';
}

/** The file name of a picked path, for labels; full paths stay out of the UI. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).at(-1) ?? path;
}
