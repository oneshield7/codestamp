<script lang="ts">
  import {
    CODE_FONT_PRESETS,
    CODE_FONT_SIZES,
    fontStack,
    LINE_HEIGHTS,
    UI_FONT_PRESETS,
    UI_FONT_SIZES,
    type FontPreset,
  } from '../../appearance/presets';
  import { formatBytes } from '../../format/units';
  import type { Theme } from '../../api/settings';
  import { appearanceSettings } from '../../state/appearance.svelte';
  import Dropdown from '../Dropdown.svelte';
  import NumberField from '../NumberField.svelte';
  import ThemePicker from '../ThemePicker.svelte';

  const appearance = $derived(appearanceSettings.appearance);
  const fonts = $derived(appearanceSettings.fonts);
  const imported = $derived(
    fonts.map((font) => ({ value: font.id, label: `Imported · ${font.name}` })),
  );
  let importing = $state(false);

  type Tone = 'k' | 's' | 'n' | 'p';
  const SAMPLE_CODE: { tone: Tone; text: string }[][] = [
    [{ tone: 'p', text: '{' }],
    [
      { tone: 'k', text: '  "total"' },
      { tone: 'p', text: ': ' },
      { tone: 'n', text: '248' },
      { tone: 'p', text: ',' },
    ],
    [
      { tone: 'k', text: '  "name"' },
      { tone: 'p', text: ': ' },
      { tone: 's', text: '"Rina Saputra"' },
      { tone: 'p', text: ',' },
    ],
    [
      { tone: 'k', text: '  "check"' },
      { tone: 'p', text: ': a !== b => ' },
      { tone: 's', text: '"ok"' },
    ],
    [{ tone: 'p', text: '}' }],
  ];

  function options(presets: FontPreset[]): { value: string; label: string }[] {
    return [...presets.map(({ key, label }) => ({ value: key, label })), ...imported];
  }

  async function importFont(): Promise<void> {
    importing = true;
    await appearanceSettings.importFont();
    importing = false;
  }
</script>

<div class="layout">
  <div class="panels">
    <section class="panel" aria-labelledby="theme-title">
      <h2 id="theme-title">Theme</h2>
      <p class="hint">Dark Night is the darkest, for working late. The title bar follows too.</p>
      <ThemePicker
        value={appearance.theme}
        systemTheme={appearanceSettings.resolvedTheme}
        onChange={(theme: Theme) => {
          appearanceSettings.update({ theme });
        }}
      />
    </section>

    <section class="panel" aria-labelledby="ui-font-title">
      <h2 id="ui-font-title">Interface font</h2>
      <p class="hint">Menus, buttons, card titles. The size scales the whole window.</p>
      <div class="fields">
        <div class="field wide">
          Font family
          <Dropdown
            variant="field"
            label="Interface font family"
            value={appearance.uiFont}
            options={options(UI_FONT_PRESETS)}
            onChange={(uiFont: string) => {
              appearanceSettings.update({ uiFont });
            }}
          />
        </div>
        <NumberField
          label="Size (px)"
          value={appearance.uiFontSize}
          range={UI_FONT_SIZES}
          onChange={(uiFontSize: number) => {
            appearanceSettings.update({ uiFontSize });
          }}
        />
      </div>
    </section>

    <section class="panel" aria-labelledby="code-font-title">
      <h2 id="code-font-title">Editor font</h2>
      <p class="hint">Bodies, scripts and responses. A monospace font keeps JSON aligned.</p>
      <div class="fields">
        <div class="field wide">
          Font family
          <Dropdown
            variant="field"
            label="Editor font family"
            value={appearance.codeFont}
            options={options(CODE_FONT_PRESETS)}
            onChange={(codeFont: string) => {
              appearanceSettings.update({ codeFont });
            }}
          />
        </div>
        <NumberField
          label="Size (px)"
          value={appearance.codeFontSize}
          range={CODE_FONT_SIZES}
          onChange={(codeFontSize: number) => {
            appearanceSettings.update({ codeFontSize });
          }}
        />
        <NumberField
          label="Line height"
          value={appearance.codeLineHeight}
          range={LINE_HEIGHTS}
          decimals={1}
          onChange={(codeLineHeight: number) => {
            appearanceSettings.update({ codeLineHeight });
          }}
        />
      </div>
      <label class="check">
        <input
          type="checkbox"
          checked={appearance.ligatures}
          onchange={(event) => {
            appearanceSettings.update({ ligatures: event.currentTarget.checked });
          }}
        />
        Font ligatures (for example <span class="mono">=&gt;</span> and
        <span class="mono">!==</span>)
      </label>
    </section>

    <section class="panel" aria-labelledby="fonts-title">
      <h2 id="fonts-title">Your fonts</h2>
      {#each fonts as font (font.id)}
        <div class="font-row">
          <span class="sample" style:font-family={fontStack(font.id, UI_FONT_PRESETS)}>Aa</span>
          <div class="font-text">
            <span class="font-name">{font.name}</span>
            <span class="font-meta">
              Imported · {formatBytes(font.size)} · {font.format.toUpperCase()}
            </span>
          </div>
          <button
            type="button"
            class="button"
            onclick={() => void appearanceSettings.removeFont(font.id)}
          >
            Remove
          </button>
        </div>
      {/each}
      <div class="import">
        <div class="font-text">
          <span class="font-name">Import a font file</span>
          <span class="font-meta">.ttf, .otf or .woff2 · up to 10 MB · stays on this device</span>
        </div>
        <button type="button" class="button" disabled={importing} onclick={() => void importFont()}>
          {importing ? 'Choosing…' : 'Choose file'}
        </button>
      </div>
    </section>

    <div>
      <button
        type="button"
        class="button"
        onclick={() => {
          appearanceSettings.reset();
        }}
      >
        Reset to defaults
      </button>
    </div>
  </div>

  <aside class="preview" aria-label="Live preview">
    <span class="preview-title">Live preview</span>
    <article class="mini-card">
      <div class="mini-head">
        <span class="step">3</span>
        <span class="method mono">GET</span>
        <span class="mini-name">List users</span>
        <span class="chip">200 · 184 ms</span>
      </div>
      <p class="sentence">The quick brown fox jumps over the lazy dog. 0123456789</p>
      <div class="code mono" aria-label="Code sample">
        {#each SAMPLE_CODE as line, index (index)}
          <div class="line">
            <span class="ln">{index + 1}</span>{#each line as token, position (position)}<span
                class={token.tone}>{token.text}</span
              >{/each}
          </div>
        {/each}
      </div>
    </article>
    <p class="note">Only the fonts in use are loaded, so switching fonts doesn’t add memory.</p>
  </aside>
</div>

<style>
  .layout {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    align-items: flex-start;
  }

  .panels {
    flex: 3 1 460px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .font-row,
  .import {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--radius-md);
  }

  .font-row {
    background: var(--bg);
  }

  .import {
    padding: 16px;
    border: 1.5px dashed var(--border-strong);
  }

  .sample {
    width: 34px;
    font-size: 18px;
    font-weight: 600;
  }

  .font-text {
    flex: 1 1 200px;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .font-name {
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .font-meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  .preview {
    flex: 2 1 320px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .preview-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .mini-card {
    border: 1px solid var(--accent);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .mini-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-soft);
  }

  .step {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .method {
    font-size: 11px;
    font-weight: 600;
    color: var(--method-get);
  }

  .mini-name {
    font-weight: 600;
  }

  .chip {
    margin-left: auto;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 12px;
    font-weight: 600;
  }

  .sentence {
    margin: 0;
    padding: 12px 14px 0;
    color: var(--text-2);
  }

  .code {
    padding: 12px 6px;
    background: var(--surface-muted);
    line-height: var(--code-line-height);
    overflow-x: auto;
  }

  .line {
    white-space: pre;
  }

  .ln {
    display: inline-block;
    width: 24px;
    padding-right: 12px;
    text-align: right;
    color: var(--text-muted);
    user-select: none;
  }

  .k {
    color: var(--code-property);
  }

  .s {
    color: var(--code-string);
  }

  .n {
    color: var(--code-number);
  }

  .p {
    color: var(--code-punctuation);
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
