<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import type { HistoryEntry, StatusFilter } from '../api/history';
  import { statusTone } from '../format/status';
  import { dayLabel, formatClock } from '../format/time';
  import { formatBytes, formatDuration } from '../format/units';
  import { methodColor } from '../request/methods';
  import { History } from '../state/history.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import SaveToCollectionDialog from './SaveToCollectionDialog.svelte';
  import ViewPanel from './ViewPanel.svelte';

  interface Props {
    workspace: Workspace;
    collectionId: string;
    collectionName: string;
    onOpenRequest: (requestId: string) => void;
    onClose: () => void;
  }

  let { workspace, collectionId, collectionName, onOpenRequest, onClose }: Props = $props();

  const SEARCH_DELAY_MS = 250;
  const FILTERS: { value: StatusFilter; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'success', label: '2xx' },
    { value: 'redirect', label: '3xx' },
    { value: 'clientError', label: '4xx' },
    { value: 'serverError', label: '5xx' },
    { value: 'failed', label: 'Failed' },
  ];

  // The panel is keyed by collection, so this belongs to one collection for its whole life.
  const history = new History(untrack(() => collectionId));
  let confirmClear = $state(false);
  let saving: HistoryEntry | null = $state(null);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  const days = $derived.by(() => {
    const grouped: { label: string; entries: HistoryEntry[] }[] = [];
    for (const entry of history.entries) {
      const label = dayLabel(entry.createdAt);
      const last = grouped.at(-1);
      if (last?.label === label) {
        last.entries.push(entry);
      } else {
        grouped.push({ label, entries: [entry] });
      }
    }
    return grouped;
  });
  const filtered = $derived(history.query.trim() !== '' || history.status !== 'all');

  onMount(() => {
    void history.load();
    return () => {
      clearTimeout(searchTimer);
    };
  });

  function search(): void {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void history.load(), SEARCH_DELAY_MS);
  }

  function filterBy(status: StatusFilter): void {
    history.status = status;
    void history.load();
  }

  async function saveEntry(targetId: string, sectionId: string | null): Promise<void> {
    const entry = saving;
    saving = null;
    if (!entry) {
      return;
    }
    const saved = await history.saveTo(entry.id, targetId, sectionId);
    if (saved) {
      await workspace.refresh(targetId);
    }
  }
</script>

<ViewPanel
  title="History"
  description="Requests sent from “{collectionName}”, newest first. Variables stay as {'{{name}}'}, so secrets are never stored here."
  {onClose}
>
  {#snippet actions()}
    <button
      type="button"
      class="clear"
      disabled={history.entries.length === 0 || filtered}
      onclick={() => (confirmClear = true)}
    >
      Clear history
    </button>
  {/snippet}

  <div class="filters">
    <label class="search">
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" />
      </svg>
      <span class="visually-hidden">Search history</span>
      <input
        type="search"
        bind:value={history.query}
        oninput={search}
        placeholder="Search URL or name"
        maxlength="200"
      />
    </label>
    <div class="pills" role="group" aria-label="Status filter">
      {#each FILTERS as filter (filter.value)}
        <button
          type="button"
          class:on={history.status === filter.value}
          aria-pressed={history.status === filter.value}
          onclick={() => {
            filterBy(filter.value);
          }}>{filter.label}</button
        >
      {/each}
    </div>
  </div>

  {#if !history.loaded}
    <p class="empty" role="status">Loading…</p>
  {:else if history.entries.length === 0}
    <p class="empty">
      {filtered ? 'No requests match.' : 'Nothing sent from this collection yet.'}
    </p>
  {:else}
    <div class="days">
      {#each days as day (day.label)}
        <h3>{day.label}</h3>
        <ul class="list">
          {#each day.entries as entry (entry.id)}
            <li class="item">
              <span class="time">{formatClock(entry.createdAt)}</span>
              <span class="method mono" style:color={methodColor(entry.method)}>{entry.method}</span
              >
              <span class="text">
                <span class="name">{entry.name}</span>
                <span class="url mono">{entry.url}</span>
              </span>
              {#if entry.status !== null}
                <span class="chip {statusTone(entry.status)}">{entry.status}</span>
              {:else}
                <span class="chip err" title={entry.error ?? undefined}>Failed</span>
              {/if}
              <span class="meta">
                {entry.elapsedMs === null ? '' : formatDuration(entry.elapsedMs)}
                {entry.bodySize === null ? '' : `· ${formatBytes(entry.bodySize)}`}
              </span>
              {#if entry.requestId}
                {@const requestId = entry.requestId}
                <button
                  type="button"
                  class="link"
                  onclick={() => {
                    onOpenRequest(requestId);
                  }}>Open</button
                >
              {:else}
                <button type="button" class="link" onclick={() => (saving = entry)}
                  >Save to collection</button
                >
              {/if}
              <button
                type="button"
                class="icon"
                aria-label="Remove from history"
                onclick={() => void history.remove(entry.id)}
              >
                <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M6 6l12 12M18 6L6 18" />
                </svg>
              </button>
            </li>
          {/each}
        </ul>
      {/each}
    </div>
  {/if}
</ViewPanel>

<ConfirmDialog
  open={confirmClear}
  title="Clear history?"
  message="Every request sent from “{collectionName}” will be removed from its history. The collection and its requests are not affected."
  confirmLabel="Clear"
  onConfirm={() => {
    confirmClear = false;
    void history.clear();
  }}
  onCancel={() => (confirmClear = false)}
/>

<SaveToCollectionDialog
  open={saving !== null}
  requestName={saving?.name ?? ''}
  collections={workspace.collections}
  onSave={(targetId: string, sectionId: string | null) => void saveEntry(targetId, sectionId)}
  onCancel={() => (saving = null)}
/>

<style>
  .clear {
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
  }

  .clear:hover:not(:disabled) {
    border-color: var(--err);
    color: var(--err);
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .search {
    flex: 1 1 240px;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
  }

  .search svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    outline: none;
  }

  .search:focus-within {
    border-color: var(--focus);
  }

  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pills button {
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
  }

  .pills button.on {
    border-color: var(--text);
    background: var(--text);
    color: var(--surface);
  }

  .empty {
    margin: 8px 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  /* A long history scrolls inside the panel, so the requests below stay close. */
  .days {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 420px;
    overflow-y: auto;
    padding: 2px;
  }

  h3 {
    margin: 4px 0 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .item {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-soft);
  }

  .item:last-child {
    border-bottom: 0;
  }

  .time {
    width: 44px;
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .method {
    width: 52px;
    font-size: 11px;
    font-weight: 600;
  }

  .text {
    flex: 1 1 220px;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 500;
  }

  .url {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .chip {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: var(--surface-sunken);
    color: var(--text-2);
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .link {
    padding: 4px 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
  }

  .link:hover,
  .icon:hover {
    background: var(--surface-sunken);
  }

  .icon {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
</style>
