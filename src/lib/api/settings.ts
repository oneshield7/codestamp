import { invoke } from '@tauri-apps/api/core';

/** `system` follows the light or dark setting of the operating system. */
export type Theme = 'system' | 'light' | 'dark' | 'night';

/** Font choices are `default`, a system preset key, or the id of an imported font. */
export interface Appearance {
  theme: Theme;
  uiFont: string;
  uiFontSize: number;
  codeFont: string;
  codeFontSize: number;
  codeLineHeight: number;
  ligatures: boolean;
}

export interface ImportedFont {
  id: string;
  name: string;
  format: string;
  size: number;
}

export interface AppearanceState {
  appearance: Appearance;
  fonts: ImportedFont[];
}

export function getAppearance(): Promise<AppearanceState> {
  return invoke('get_appearance');
}

export function saveAppearance(appearance: Appearance): Promise<Appearance> {
  return invoke('save_appearance', { appearance });
}

/** Opens the system file picker; `null` when the user cancels. */
export function importFont(): Promise<ImportedFont | null> {
  return invoke('import_font');
}

export function readFont(id: string): Promise<ArrayBuffer> {
  return invoke('read_font', { id });
}

export function deleteFont(id: string): Promise<AppearanceState> {
  return invoke('delete_font', { id });
}
