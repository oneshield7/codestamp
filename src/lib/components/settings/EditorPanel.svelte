<script lang="ts">
  import type { CardLayout, Indent } from '../../api/preferences';
  import { preferenceSettings } from '../../state/preferences.svelte';

  const INDENTS: { key: Indent; label: string }[] = [
    { key: 'none', label: 'None' },
    { key: 'two', label: '2 spaces' },
    { key: 'four', label: '4 spaces' },
    { key: 'tab', label: 'Tab' },
  ];
  const LAYOUTS: { key: CardLayout; label: string }[] = [
    { key: 'stacked', label: 'Response below the request' },
    { key: 'side', label: 'Response next to the request' },
  ];

  const editor = $derived(preferenceSettings.editor);
</script>

<section class="panel" aria-labelledby="editor-title">
  <h2 id="editor-title">Code editor</h2>
  <p class="hint">
    Bodies, scripts and messages. Beautify uses the same indentation. With None, new lines and
    Beautify start at the left edge, and Tab moves on to the next field.
  </p>
  <div class="field">
    Indentation
    <div class="segmented" role="radiogroup" aria-label="Indentation">
      {#each INDENTS as option (option.key)}
        <label class:on={editor.indent === option.key}>
          <input
            type="radio"
            name="indent"
            class="visually-hidden"
            checked={editor.indent === option.key}
            onchange={() => {
              preferenceSettings.updateEditor({ indent: option.key });
            }}
          />
          {option.label}
        </label>
      {/each}
    </div>
  </div>
  <label class="check">
    <input
      type="checkbox"
      checked={editor.closeBrackets}
      onchange={(event) => {
        preferenceSettings.updateEditor({ closeBrackets: event.currentTarget.checked });
      }}
    />
    Close brackets and quotes automatically
  </label>
  <label class="check">
    <input
      type="checkbox"
      checked={editor.wrapLines}
      onchange={(event) => {
        preferenceSettings.updateEditor({ wrapLines: event.currentTarget.checked });
      }}
    />
    Wrap long lines
  </label>
</section>

<section class="panel" aria-labelledby="cards-title">
  <h2 id="cards-title">Request cards</h2>
  <div class="field">
    New cards open with
    <div class="options" role="radiogroup" aria-label="New cards open with">
      {#each LAYOUTS as option (option.key)}
        <label class="check">
          <input
            type="radio"
            name="layout"
            checked={editor.layout === option.key}
            onchange={() => {
              preferenceSettings.updateEditor({ layout: option.key });
            }}
          />
          {option.label}
        </label>
      {/each}
    </div>
  </div>
  <label class="check">
    <input
      type="checkbox"
      checked={editor.variableHints}
      onchange={(event) => {
        preferenceSettings.updateEditor({ variableHints: event.currentTarget.checked });
      }}
    />
    Suggest variables after typing <span class="mono">{'{{'}</span>
  </label>
</section>

{#if preferenceSettings.saveError}
  <p class="error" role="alert">{preferenceSettings.saveError}</p>
{/if}
<div>
  <button
    type="button"
    class="button"
    onclick={() => {
      preferenceSettings.resetEditor();
    }}
  >
    Reset editor settings
  </button>
</div>

<style>
  .options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
