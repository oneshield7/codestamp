import type { Appearance, Theme } from '../api/settings';

export interface FontPreset {
  key: string;
  label: string;
  stack: string;
}

export interface NumberRange {
  min: number;
  max: number;
}

export const DEFAULT_FONT = 'default';

/** A theme as painted: `system` resolves to one of these. */
export type ResolvedTheme = Exclude<Theme, 'system'>;

export interface ThemeOption {
  key: Theme;
  label: string;
  hint: string;
}

export const THEMES: ThemeOption[] = [
  { key: 'light', label: 'Light', hint: 'Ink on paper' },
  { key: 'dark', label: 'Dark', hint: 'Soft graphite' },
  { key: 'night', label: 'Dark Night', hint: 'Deep navy for late hours' },
  { key: 'system', label: 'System', hint: 'Follows Windows' },
];

// The `-default` stacks are defined once in app.css; every choice falls back to them.
export const UI_FONT_PRESETS: FontPreset[] = [
  { key: DEFAULT_FONT, label: 'Geist (default)', stack: 'var(--font-ui-default)' },
  { key: 'segoe-ui', label: 'System · Segoe UI', stack: "'Segoe UI', var(--font-ui-default)" },
];

export const CODE_FONT_PRESETS: FontPreset[] = [
  { key: DEFAULT_FONT, label: 'Geist Mono (default)', stack: 'var(--font-code-default)' },
  {
    key: 'cascadia-code',
    label: 'System · Cascadia Code',
    stack: "'Cascadia Code', 'Cascadia Mono', var(--font-code-default)",
  },
  { key: 'consolas', label: 'System · Consolas', stack: 'Consolas, var(--font-code-default)' },
];

export const DEFAULT_APPEARANCE: Appearance = {
  theme: 'light',
  uiFont: DEFAULT_FONT,
  uiFontSize: 14,
  codeFont: DEFAULT_FONT,
  codeFontSize: 13,
  codeLineHeight: 1.6,
  ligatures: true,
};

export const UI_FONT_SIZES: NumberRange = { min: 11, max: 20 };
export const CODE_FONT_SIZES: NumberRange = { min: 10, max: 24 };
export const LINE_HEIGHTS: NumberRange = { min: 1.2, max: 2.2 };

/** The CSS family name an imported font is registered under. */
export function fontAlias(id: string): string {
  return `codestamp-font-${id}`;
}

/** The CSS font stack for a choice; imported fonts fall back to the bundled font. */
export function fontStack(choice: string, presets: FontPreset[]): string {
  const preset = presets.find((item) => item.key === choice);
  if (preset) {
    return preset.stack;
  }
  const fallback = presets.at(0)?.stack ?? 'sans-serif';
  return `'${fontAlias(choice)}', ${fallback}`;
}

export function isInRange(value: number, range: NumberRange): boolean {
  return Number.isFinite(value) && value >= range.min && value <= range.max;
}
