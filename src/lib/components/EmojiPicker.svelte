<script lang="ts">
  import { onMount } from 'svelte';
  import { suggestCollectionIcon, type IconEntry } from '../api/collections';
  import { toAppError } from '../api/http';
  import { iconCatalogue } from '../state/icons';

  interface Props {
    /** The icon shown now. */
    current: string;
    /** The collection name the automatic icon and the suggestions come from. */
    name: string;
    /** False while the icon follows the name. */
    custom: boolean;
    /** `null` goes back to the automatic icon. */
    onPick: (icon: string | null) => void;
    onClose: () => void;
  }

  let { current, name, custom, onPick, onClose }: Props = $props();

  let panel: HTMLElement;
  let search: HTMLInputElement;
  let entries = $state<IconEntry[]>([]);
  let suggested = $state<string[]>([]);
  let query = $state('');
  let error = $state<string | null>(null);

  const labels = $derived(new Map(entries.map((entry) => [entry.emoji, entry.label])));
  const needle = $derived(query.trim().toLowerCase());
  const results = $derived(
    needle === '' ? entries : entries.filter((entry) => entry.keywords.includes(needle)),
  );

  onMount(() => {
    search.focus();
    iconCatalogue().then(
      (loaded) => (entries = loaded),
      (caught: unknown) => (error = toAppError(caught).message),
    );
    // Clicks outside the panel close it, like a menu.
    const closeOutside = (event: PointerEvent): void => {
      const target = event.target;
      if (
        target instanceof Node &&
        !panel.contains(target) &&
        !panel.parentElement?.contains(target)
      ) {
        onClose();
      }
    };
    document.addEventListener('pointerdown', closeOutside);
    // On the panel, so Escape does not also leave focus mode or close a surrounding dialog.
    panel.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOutside);
      panel.removeEventListener('keydown', closeOnEscape);
    };
  });

  $effect(() => {
    const forName = name;
    const run = { cancelled: false };
    suggestCollectionIcon(forName).then(
      (suggestion) => {
        if (!run.cancelled) {
          suggested = suggestion.related;
        }
      },
      () => {
        if (!run.cancelled) {
          suggested = [];
        }
      },
    );
    return () => {
      run.cancelled = true;
    };
  });

  function choose(icon: string): void {
    onPick(icon);
    onClose();
  }

  function shuffle(): void {
    const others = entries.filter((entry) => entry.emoji !== current);
    const next = others[Math.floor(Math.random() * others.length)];
    if (next) {
      onPick(next.emoji);
    }
  }

  function closeOnEscape(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    }
  }
</script>

<div bind:this={panel} class="picker" role="dialog" aria-label="Collection icon" tabindex="-1">
  <label class="search">
    <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="11" cy="11" r="6" /><path d="M20 20l-4.5-4.5" />
    </svg>
    <span class="visually-hidden">Search emoji</span>
    <input bind:this={search} bind:value={query} placeholder="Search emoji, e.g. card, rocket" />
  </label>

  {#if error}
    <p class="note error" role="alert">{error}</p>
  {:else}
    {#if needle === '' && suggested.length > 0}
      <p class="heading">Suggested for “{name.trim() || 'this name'}”</p>
      <div class="grid">
        {#each suggested as emoji (emoji)}
          <button
            type="button"
            class="emoji-button"
            class:on={emoji === current}
            aria-pressed={emoji === current}
            aria-label={labels.get(emoji) ?? 'emoji'}
            onclick={() => {
              choose(emoji);
            }}><span class="emoji">{emoji}</span></button
          >
        {/each}
      </div>
    {/if}
    <p class="heading">{needle === '' ? 'All' : 'Results'}</p>
    <div class="scroll">
      <div class="grid">
        {#each results as entry (entry.emoji)}
          <button
            type="button"
            class="emoji-button"
            class:on={entry.emoji === current}
            aria-pressed={entry.emoji === current}
            aria-label={entry.label}
            onclick={() => {
              choose(entry.emoji);
            }}><span class="emoji">{entry.emoji}</span></button
          >
        {/each}
      </div>
      {#if entries.length > 0 && results.length === 0}
        <p class="note">No emoji match “{query.trim()}”.</p>
      {/if}
    </div>
  {/if}

  <div class="foot">
    <span class="mode">{custom ? 'Chosen by you' : 'Automatic: picked from the name'}</span>
    <button type="button" class="small" disabled={entries.length === 0} onclick={shuffle}>
      <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M3 7h3c4 0 6 10 10 10h5M18 14l3 3-3 3M3 17h3c1.6 0 2.8-1.6 4-3.5M14 9.5C15.2 8 16.4 7 18 7h3M18 4l3 3-3 3"
        />
      </svg>
      Shuffle
    </button>
    <button
      type="button"
      class="small"
      disabled={!custom}
      onclick={() => {
        onPick(null);
      }}>Use automatic</button
    >
  </div>
</div>

<style>
  .picker {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    z-index: 20;
    width: min(372px, calc(100vw - 32px));
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    margin: 12px 12px 8px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text-muted);
  }

  .search:focus-within {
    border-color: var(--focus);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    outline: none;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .heading {
    margin: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    gap: 2px;
    padding: 0 10px 6px;
  }

  .scroll {
    max-height: 206px;
    overflow-y: auto;
  }

  .emoji-button {
    height: 38px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: transparent;
    font-size: 20px;
  }

  .emoji-button:hover {
    background: var(--surface-sunken);
  }

  .emoji-button.on {
    border-color: var(--text);
    background: var(--surface-sunken);
  }

  .note {
    margin: 8px 14px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .note.error {
    color: var(--err);
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--border-soft);
  }

  .mode {
    flex: 1 1 140px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .small {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 12px;
  }

  .small:disabled {
    opacity: 0.5;
  }
</style>
