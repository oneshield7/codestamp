<script lang="ts">
  import type { Theme } from '../api/settings';
  import { THEMES, type ResolvedTheme } from '../appearance/presets';

  interface Props {
    value: Theme;
    /** What `system` currently paints, shown under its label. */
    systemTheme: ResolvedTheme;
    onChange: (theme: Theme) => void;
  }

  let { value, systemTheme, onChange }: Props = $props();

  const LABELS = new Map(THEMES.map((theme) => [theme.key, theme.label]));

  /** The themes a preview paints; System shows light and dark side by side. */
  function painted(theme: Theme): ResolvedTheme[] {
    return theme === 'system' ? ['light', 'dark'] : [theme];
  }
</script>

<div class="themes" role="radiogroup" aria-label="Theme">
  {#each THEMES as theme (theme.key)}
    <label class="option" class:on={value === theme.key}>
      <input
        type="radio"
        name="theme"
        class="visually-hidden"
        value={theme.key}
        checked={value === theme.key}
        onchange={() => {
          onChange(theme.key);
        }}
      />
      <span class="preview" class:split={theme.key === 'system'}>
        {#each painted(theme.key) as shade (shade)}
          <span class="mock" data-theme={shade}>
            <span class="side">
              <span class="dot"></span>
              <span class="bar"></span>
              <span class="bar short"></span>
            </span>
            <span class="main">
              <span class="row">
                <span class="stamp get mono">GET</span>
                <span class="bar"></span>
                <span class="send"></span>
              </span>
              <span class="row">
                <span class="stamp post mono">POST</span>
                <span class="var mono">{'{{baseUrl}}'}</span>
              </span>
            </span>
          </span>
        {/each}
      </span>
      <span class="label">{theme.label}</span>
      <span class="hint">
        {theme.key === 'system' ? `Now ${LABELS.get(systemTheme) ?? ''}` : theme.hint}
      </span>
    </label>
  {/each}
</div>

<style>
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
  }

  .option {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    cursor: pointer;
  }

  .option:hover {
    border-color: var(--border-strong);
  }

  .option.on {
    border-color: var(--text);
    box-shadow: 0 0 0 1px var(--text);
  }

  .option:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  .preview {
    display: flex;
    height: 86px;
    margin-bottom: 6px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .label {
    font-size: 13px;
    font-weight: 600;
  }

  .hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .mock {
    flex: 1;
    display: flex;
    min-width: 0;
    background: var(--bg);
    color: var(--text);
  }

  .side {
    width: 26%;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 8px 6px;
    border-right: 1px solid var(--border);
    background: var(--surface-muted);
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    background: var(--accent);
  }

  .bar {
    flex: 1;
    min-width: 0;
    height: 5px;
    border-radius: 3px;
    background: var(--border-strong);
  }

  .side .bar {
    flex: none;
  }

  .bar.short {
    width: 60%;
  }

  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    padding: 8px 6px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    padding: 5px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--surface);
  }

  .stamp {
    flex: none;
    padding: 0 3px;
    border-radius: 3px;
    font-size: 8px;
    font-weight: 600;
    line-height: 12px;
  }

  .stamp.get {
    color: var(--method-get);
    background: var(--method-get-bg);
  }

  .stamp.post {
    color: var(--method-post);
    background: var(--method-post-bg);
  }

  .send {
    flex: none;
    width: 18px;
    height: 10px;
    border-radius: 3px;
    background: var(--method-get);
  }

  .var {
    overflow: hidden;
    white-space: nowrap;
    font-size: 8px;
    color: var(--var);
  }

  .split .mock + .mock {
    border-left: 1px solid var(--border-strong);
  }

  .split .side {
    display: none;
  }
</style>
