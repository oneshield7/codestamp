<script lang="ts">
  import { preferenceSettings } from '../state/preferences.svelte';

  // One setting for every response viewer, so turning it off in one turns it off in all.
  const on = $derived(preferenceSettings.editor.wrapResponses);
</script>

<button
  type="button"
  class="wrap"
  aria-pressed={on}
  onclick={() => {
    preferenceSettings.updateEditor({ wrapResponses: !on });
  }}
>
  <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
    <path d="M4 6h16M4 12h13a3 3 0 0 1 0 6h-4M15 16l-2 2 2 2M4 18h5" />
  </svg>
  Wrap lines
</button>

<style>
  .wrap {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
  }

  .wrap:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  /* Lit like the Aa / .* toggles in Find: border and fill change too, not only the colour. */
  .wrap[aria-pressed='true'] {
    border-color: var(--border-strong);
    background: var(--accent-soft);
    color: var(--text);
  }

  .wrap:active {
    background: var(--surface-sunken);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
