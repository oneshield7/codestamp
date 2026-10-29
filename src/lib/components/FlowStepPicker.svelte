<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../api/http';
  import { searchIndex, type SearchEntry } from '../api/search';
  import { matchFields, queryWords } from '../search/match';
  import { stampLabel } from '../request/kinds';
  import { stampTone } from '../request/methods';
  import MethodStamp from './MethodStamp.svelte';

  interface Props {
    title: string;
    onPick: (entry: SearchEntry) => void;
    onClose: () => void;
  }

  let { title, onPick, onClose }: Props = $props();

  /** Only these can be sent one after another; the rest keep a connection open. */
  const RUNNABLE = new Set(['http', 'graphql', 'soap']);
  const MAX_SHOWN = 60;

  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let query = $state('');
  let entries = $state.raw<SearchEntry[]>([]);
  let loaded = $state(false);
  let loadError = $state<string | null>(null);

  const shown = $derived.by(() => {
    const runnable = entries.filter((entry) => RUNNABLE.has(entry.kind));
    const words = queryWords(query);
    if (words.length === 0) {
      return runnable.slice(0, MAX_SHOWN);
    }
    return runnable
      .map((entry) => ({
        entry,
        match: matchFields(
          [
            { text: entry.name, weight: 1 },
            { text: entry.url, weight: 1 },
            { text: entry.collectionName, weight: 0.5 },
          ],
          words,
        ),
      }))
      .filter((item) => item.match !== null)
      .sort((a, b) => (b.match?.score ?? 0) - (a.match?.score ?? 0))
      .slice(0, MAX_SHOWN)
      .map((item) => item.entry);
  });

  onMount(() => {
    dialog.showModal();
    input.focus();
    void load();
  });

  async function load(): Promise<void> {
    try {
      entries = await searchIndex();
    } catch (caught) {
      loadError = toAppError(caught).message;
    } finally {
      loaded = true;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="picker"
  aria-labelledby="flow-picker-title"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <header>
    <h2 id="flow-picker-title">{title}</h2>
    <button type="button" class="close" aria-label="Close" onclick={onClose}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>
  </header>
  <label class="search">
    <span class="visually-hidden">Find a request</span>
    <input
      bind:this={input}
      bind:value={query}
      placeholder="Login, /checkout or a collection name"
      spellcheck="false"
    />
  </label>
  {#if loadError}
    <p class="error" role="alert">{loadError}</p>
  {:else if loaded && shown.length === 0}
    <p class="empty">
      {query.trim() === ''
        ? 'Save HTTP or GraphQL requests in a collection first; flows call them.'
        : 'No HTTP or GraphQL request matches.'}
    </p>
  {/if}
  <ul class="results" aria-label="Requests">
    {#each shown as entry (entry.requestId)}
      <li>
        <button
          type="button"
          onclick={() => {
            onPick(entry);
          }}
        >
          <MethodStamp
            label={stampLabel(entry.kind, entry.method)}
            tone={stampTone(entry.kind, entry.method)}
          />
          <span class="text">
            <span class="name">{entry.name}</span>
            <span class="url mono">{entry.url || 'No URL yet'}</span>
          </span>
          <span class="place">{entry.collectionIcon} {entry.collectionName}</span>
        </button>
      </li>
    {/each}
  </ul>
</dialog>

<style>
  .picker {
    width: min(620px, calc(100vw - 32px));
    max-height: min(560px, calc(100vh - 64px));
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
  }

  .picker[open] {
    display: flex;
    flex-direction: column;
  }

  .picker::backdrop {
    background: var(--backdrop);
  }

  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px 10px;
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .close {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .close:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .close svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .search {
    padding: 0 16px 10px;
    border-bottom: 1px solid var(--border-soft);
  }

  .search input {
    width: 100%;
    height: 38px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 14px;
  }

  .search input:focus {
    border-color: var(--focus);
    outline: none;
  }

  .results {
    margin: 0;
    padding: 6px;
    list-style: none;
    overflow: auto;
  }

  .results button {
    width: 100%;
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 9px 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    text-align: left;
  }

  .results button:hover,
  .results button:focus-visible {
    background: var(--surface-sunken);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 500;
  }

  .url,
  .place {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }

  .place {
    max-width: 180px;
  }

  .empty,
  .error {
    margin: 16px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .error {
    color: var(--err);
  }
</style>
