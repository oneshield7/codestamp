import * as api from '../api/settings';
import {
  CODE_FONT_PRESETS,
  DEFAULT_APPEARANCE,
  fontAlias,
  fontStack,
  type ResolvedTheme,
  UI_FONT_PRESETS,
} from '../appearance/presets';
import { notices } from './notices.svelte';

const SAVE_DELAY_MS = 300;
/** Read by the launch screen on a reload; the name matches `THEME_KEY` in Rust. */
const THEME_SESSION_KEY = 'codestamp.theme';

/** Theme, fonts and sizes of the whole app. Changes apply at once and are saved shortly after. */
class AppearanceSettings {
  appearance = $state<api.Appearance>({ ...DEFAULT_APPEARANCE });
  fonts = $state<api.ImportedFont[]>([]);
  #systemDark = $state(false);

  readonly resolvedTheme: ResolvedTheme = $derived.by(() => {
    if (this.appearance.theme !== 'system') {
      return this.appearance.theme;
    }
    return this.#systemDark ? 'dark' : 'light';
  });

  /** Imported fonts currently registered with the page; `null` when the file failed to load. */
  readonly #faces = new Map<string, FontFace | null>();
  #applying: Promise<void> = Promise.resolve();
  #saveTimer: ReturnType<typeof setTimeout> | undefined;

  async load(): Promise<void> {
    const scheme = window.matchMedia('(prefers-color-scheme: dark)');
    this.#systemDark = scheme.matches;
    scheme.addEventListener('change', (event) => {
      this.#systemDark = event.matches;
      this.#applyTheme();
    });
    await notices.guard(async () => {
      const state = await api.getAppearance();
      this.fonts = state.fonts;
      this.appearance = state.appearance;
    });
    await this.#apply();
  }

  update(patch: Partial<api.Appearance>): void {
    this.appearance = { ...this.appearance, ...patch };
    void this.#apply();
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => void this.#save(), SAVE_DELAY_MS);
  }

  reset(): void {
    this.update({ ...DEFAULT_APPEARANCE });
  }

  async importFont(): Promise<void> {
    await notices.guard(async () => {
      const imported = await api.importFont();
      if (imported) {
        this.fonts.push(imported);
      }
    });
  }

  async removeFont(id: string): Promise<void> {
    await notices.guard(async () => {
      const state = await api.deleteFont(id);
      this.fonts = state.fonts;
      this.appearance = state.appearance;
    });
    await this.#apply();
  }

  async #save(): Promise<void> {
    await notices.guard(async () => {
      await api.saveAppearance($state.snapshot(this.appearance));
    });
  }

  // Runs one change at a time so overlapping changes never load the same font twice.
  #apply(): Promise<void> {
    this.#applying = this.#applying.then(() => this.#applyNow());
    return this.#applying;
  }

  #applyTheme(): void {
    document.documentElement.dataset.theme = this.resolvedTheme;
    // A reload draws its launch screen in this theme (`theme_script` in Rust reads it).
    sessionStorage.setItem(THEME_SESSION_KEY, this.appearance.theme);
  }

  async #applyNow(): Promise<void> {
    this.#applyTheme();
    const { uiFont, codeFont, codeFontSize, codeLineHeight, ligatures } = this.appearance;
    const root = document.documentElement.style;
    root.setProperty('--font-ui', fontStack(uiFont, UI_FONT_PRESETS));
    root.setProperty('--font-code', fontStack(codeFont, CODE_FONT_PRESETS));
    root.setProperty('--font-size-code', `${String(codeFontSize)}px`);
    root.setProperty('--code-line-height', String(codeLineHeight));
    root.setProperty('--code-ligatures', ligatures ? 'normal' : 'none');
    await this.#syncFaces([uiFont, codeFont]);
  }

  /** Only imported fonts that are in use stay loaded, so the others cost no memory. */
  async #syncFaces(choices: string[]): Promise<void> {
    const wanted = new Set(choices.filter((id) => this.fonts.some((font) => font.id === id)));
    for (const [id, face] of this.#faces) {
      if (!wanted.has(id)) {
        if (face) {
          document.fonts.delete(face);
        }
        this.#faces.delete(id);
      }
    }
    for (const id of wanted) {
      if (!this.#faces.has(id)) {
        this.#faces.set(id, await this.#loadFace(id));
      }
    }
  }

  async #loadFace(id: string): Promise<FontFace | null> {
    try {
      const face = await new FontFace(fontAlias(id), await api.readFont(id)).load();
      document.fonts.add(face);
      return face;
    } catch (error) {
      // The browser rejects font files it cannot decode; the error says nothing useful then.
      const name = this.fonts.find((font) => font.id === id)?.name ?? 'the font';
      notices.report(
        error instanceof DOMException
          ? { code: 'font', message: `Could not load “${name}”. The file may be damaged.` }
          : error,
      );
      return null;
    }
  }
}

export const appearanceSettings = new AppearanceSettings();
