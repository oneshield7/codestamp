<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { CollectionSummary } from '../api/collections';
  import { searchDocs, type DocHit } from '../api/docs';
  import type { EnvironmentSummary } from '../api/environments';
  import { toAppError } from '../api/http';
  import { searchIndex, type SearchEntry } from '../api/search';
  import {
    pathCompletion,
    searchResults,
    type SearchResult,
    type SearchTarget,
  } from '../search/results';
  import MethodStamp from './MethodStamp.svelte';
  import NavIcon from './NavIcon.svelte';

  interface Props {
    collections: readonly CollectionSummary[];
    environments: readonly EnvironmentSummary[];
    /** Docs is open, so its results lead the list. */
    docsFirst: boolean;
    onPick: (target: SearchTarget) => void;
    onClose: () => void;
  }

  let { collections, environments, docsFirst, onPick, onClose }: Props = $props();

  const LIST_ID = 'quick-search-results';
  /** Docs are searched in the core, so typing settles before each search. */
  const DOCS_DELAY_MS = 120;

  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let list: HTMLDivElement | undefined = $state();
  let query = $state('');
  let requests = $state.raw<SearchEntry[]>([]);
  let loadError = $state<string | null>(null);
  let activeIndex = $state(0);
  let docs = $state.raw<DocHit[]>([]);

  const sources = $derived({ requests, collections, environments, docs });
  const groups = $derived(searchResults(sources, query, docsFirst));
  const results = $derived(groups.flatMap((group) => group.results));
  const active = $derived(results.at(Math.min(activeIndex, results.length - 1)));
  const completion = $derived(pathCompletion(query, groups, sources));

  onMount(() => {
    dialog.showModal();
    input.focus();
    void loadRequests();
  });

  async function loadRequests(): Promise<void> {
    try {
      requests = await searchIndex();
    } catch (caught) {
      loadError = toAppError(caught).message;
    }
  }

  $effect(() => {
    const text = query.trim();
    if (text === '') {
      docs = [];
      return;
    }
    const timer = setTimeout(() => {
      void findDocs(text);
    }, DOCS_DELAY_MS);
    return () => {
      clearTimeout(timer);
    };
  });

  async function findDocs(text: string): Promise<void> {
    try {
      const hits = await searchDocs(text);
      // A slower answer for older text must not replace a newer one.
      if (query.trim() === text) {
        docs = hits;
      }
    } catch (caught) {
      docs = [];
      loadError = toAppError(caught).message;
    }
  }

  function indexOf(result: SearchResult): number {
    return results.indexOf(result);
  }

  async function moveTo(index: number): Promise<void> {
    if (results.length === 0) {
      return;
    }
    activeIndex = (index + results.length) % results.length;
    await tick();
    list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  }

  function acceptCompletion(): boolean {
    if (completion === null) {
      return false;
    }
    query += completion;
    activeIndex = 0;
    return true;
  }

  function onKey(event: KeyboardEvent): void {
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        void moveTo(activeIndex + 1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        void moveTo(activeIndex - 1);
        break;
      case 'Enter':
        event.preventDefault();
        if (active) {
          onPick(active.target);
        }
        break;
      case 'Tab':
        // Focus stays in the field; Tab takes the suggested path instead.
        event.preventDefault();
        acceptCompletion();
        break;
      case 'ArrowRight':
        if (input.selectionStart === query.length && acceptCompletion()) {
          event.preventDefault();
        }
        break;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="quick-search"
  aria-label="Quick search"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
  onclick={(event) => {
    // A click on the dimmed backdrop lands on the dialog itself.
    if (event.target === dialog) {
      onClose();
    }
  }}
>
  <div class="panel">
    <div class="field">
      <svg class="glass" width="20" height="20" viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" />
      </svg>
      <div class="input">
        <span class="ghost" aria-hidden="true"
          ><span class="typed">{query}</span>{completion ?? ''}</span
        >
        <input
          bind:this={input}
          bind:value={query}
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls={LIST_ID}
          aria-activedescendant={active ? `quick-search-${String(indexOf(active))}` : undefined}
          aria-autocomplete="list"
          placeholder="Search requests, paths, collections and settings"
          spellcheck="false"
          autocomplete="off"
          oninput={() => (activeIndex = 0)}
          onkeydown={onKey}
        />
      </div>
    </div>

    <div bind:this={list} id={LIST_ID} class="results" role="listbox" aria-label="Results">
      {#each groups as group (group.label)}
        <div class="group" role="group" aria-label={group.label}>
          <div class="group-label" aria-hidden="true">{group.label}</div>
          {#each group.results as result (result.key)}
            {@const index = indexOf(result)}
            <div
              id="quick-search-{index}"
              class="result"
              class:active={result === active}
              role="option"
              aria-selected={result === active}
              tabindex="-1"
              onmousemove={() => (activeIndex = index)}
              onmousedown={(event) => {
                event.preventDefault();
                onPick(result.target);
              }}
            >
              <span class="icon">
                {#if result.icon.kind === 'stamp'}
                  <MethodStamp label={result.icon.label} tone={result.icon.tone} size="sm" />
                {:else if result.icon.kind === 'emoji'}
                  <span class="emoji">{result.icon.emoji}</span>
                {:else if result.icon.kind === 'nav'}
                  <NavIcon name={result.icon.name} />
                {:else}
                  <span class="tile" data-hue={result.icon.hue}>
                    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                      <path d={result.icon.path} />
                    </svg>
                  </span>
                {/if}
              </span>
              <span class="text">
                <span class="title"
                  >{#each result.title as part, partIndex (partIndex)}{#if part.hit}<mark
                        >{part.text}</mark
                      >{:else}{part.text}{/if}{/each}</span
                >
                <span class="detail" class:mono={result.mono}
                  >{#each result.detail as part, partIndex (partIndex)}{#if part.hit}<mark
                        >{part.text}</mark
                      >{:else}{part.text}{/if}{/each}</span
                >
              </span>
              {#if result.place}
                <span class="place">{result.place}</span>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <p class="empty">
          {query.trim() === '' ? 'Type to search.' : `Nothing matches “${query.trim()}”.`}
        </p>
      {/each}
    </div>

    <div class="footer">
      {#if loadError}
        <span class="error">{loadError}</span>
      {:else}
        <span>Type a path such as <span class="mono">/users</span> for suggestions</span>
      {/if}
    </div>
  </div>
</dialog>

<style>
  .quick-search {
    width: min(680px, calc(100vw - 32px));
    /* Fits below its top margin on short windows too. */
    max-height: min(560px, calc(86vh - 16px));
    margin: 14vh auto auto;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
  }

  /* Only opacity and transform move, so the opening runs on the compositor like Home's hub
     pill. A blurred backdrop would repaint the whole window on every frame. */
  .quick-search[open] {
    display: flex;
    animation: rise-in 240ms cubic-bezier(0.2, 0.7, 0.2, 1);
  }

  .quick-search::backdrop {
    background: var(--backdrop);
    animation: fade-in 200ms ease-out;
  }

  @keyframes rise-in {
    from {
      opacity: 0;
      transform: translateY(-10px) scale(0.98);
    }
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .quick-search[open],
    .quick-search::backdrop {
      animation: none;
    }
  }

  /* min-width: 0 keeps long names and paths from widening the dialog; they end in an ellipsis. */
  .panel {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .glass {
    flex: none;
    color: var(--text-muted);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .input {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .input input,
  .ghost {
    width: 100%;
    font: inherit;
    font-size: 20px;
    letter-spacing: -0.01em;
    line-height: 30px;
  }

  .input input {
    position: relative;
    padding: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
  }

  .input input::placeholder {
    color: var(--text-muted);
  }

  .ghost {
    position: absolute;
    inset: 0;
    overflow: hidden;
    white-space: pre;
    color: var(--text-muted);
    pointer-events: none;
  }

  .typed {
    color: transparent;
  }

  .results {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px;
  }

  .group + .group {
    margin-top: 6px;
  }

  .group-label {
    padding: 8px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .result {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 10px;
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .result.active {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--focus);
  }

  .icon {
    flex: none;
    display: inline-flex;
    justify-content: center;
    width: 58px;
  }

  .emoji {
    font-family: var(--font-emoji);
    font-size: 18px;
    line-height: 1;
  }

  .tile {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: var(--hue-bg);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .title,
  .detail,
  .place {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: 14px;
    font-weight: 500;
  }

  .detail {
    font-size: 12px;
    color: var(--text-muted);
  }

  .place {
    flex: none;
    max-width: 34%;
    font-size: 12px;
    color: var(--text-muted);
  }

  mark {
    border-radius: 2px;
    background: transparent;
    color: var(--accent-text);
    font-weight: 600;
    text-decoration: underline;
    text-decoration-color: var(--focus);
    text-underline-offset: 2px;
  }

  .empty {
    margin: 0;
    padding: 28px 12px;
    text-align: center;
    font-size: 13px;
    color: var(--text-muted);
  }

  .footer {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    padding: 9px 16px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-muted);
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    color: var(--err);
  }
</style>
