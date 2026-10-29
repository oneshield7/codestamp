import { toAppError } from '../api/http';
import * as api from '../api/preferences';
import { notices } from './notices.svelte';

const SAVE_DELAY_MS = 400;

export const DEFAULT_PREFERENCES: api.Preferences = {
  editor: {
    indent: 'none',
    closeBrackets: true,
    wrapLines: false,
    layout: 'stacked',
    variableHints: true,
  },
  requests: {
    timeoutSeconds: 30,
    maxResponseMb: 0,
    followRedirects: true,
    cookies: true,
    noCache: false,
    userAgent: '',
    minTlsVersion: '1.2',
  },
  history: { maxEntries: 1000, keepDays: 30 },
};

/**
 * Settings › Editor, Requests and Data. Editor choices apply at once; everything is checked and
 * saved by the Rust core shortly after the last change, which refuses values it cannot use.
 */
class PreferenceSettings {
  preferences = $state<api.Preferences>(structuredClone(DEFAULT_PREFERENCES));
  /** The last save's error, shown next to the settings until the next change. */
  saveError = $state<string | null>(null);

  #saveTimer: ReturnType<typeof setTimeout> | undefined;

  get editor(): api.EditorPreferences {
    return this.preferences.editor;
  }

  /** The indent `prettyJson` uses: spaces per level (0 for none), or a tab. */
  get jsonIndent(): number | 'tab' {
    switch (this.preferences.editor.indent) {
      case 'none':
        return 0;
      case 'two':
        return 2;
      case 'four':
        return 4;
      case 'tab':
        return 'tab';
    }
  }

  async load(): Promise<void> {
    await notices.guard(async () => {
      this.preferences = await api.getPreferences();
    });
  }

  updateEditor(patch: Partial<api.EditorPreferences>): void {
    this.#change({ ...this.preferences, editor: { ...this.preferences.editor, ...patch } });
  }

  updateRequests(patch: Partial<api.RequestPreferences>): void {
    this.#change({ ...this.preferences, requests: { ...this.preferences.requests, ...patch } });
  }

  updateHistory(patch: Partial<api.HistoryPreferences>): void {
    this.#change({ ...this.preferences, history: { ...this.preferences.history, ...patch } });
  }

  resetEditor(): void {
    this.updateEditor(DEFAULT_PREFERENCES.editor);
  }

  /** The TLS version lives under SSL/TLS, so resetting Requests leaves it as it is. */
  resetRequests(): void {
    this.updateRequests({
      ...DEFAULT_PREFERENCES.requests,
      minTlsVersion: this.preferences.requests.minTlsVersion,
    });
  }

  #change(next: api.Preferences): void {
    this.preferences = next;
    this.saveError = null;
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => void this.#save(), SAVE_DELAY_MS);
  }

  async #save(): Promise<void> {
    try {
      this.preferences = await api.savePreferences($state.snapshot(this.preferences));
    } catch (error) {
      this.saveError = toAppError(error).message;
    }
  }
}

export const preferenceSettings = new PreferenceSettings();
