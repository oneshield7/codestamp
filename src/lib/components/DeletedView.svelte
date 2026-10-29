<script lang="ts">
  import { onMount } from 'svelte';
  import { KEEP_DAYS, type DeletedItem } from '../api/trash';
  import { plural } from '../format/plural';
  import { formatSince } from '../format/time';
  import { recentlyDeleted } from '../state/deleted.svelte';
  import { navLabel } from '../state/navigation';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import NavIcon from './NavIcon.svelte';

  const DAY_MS = 24 * 60 * 60 * 1000;

  /** A copy to delete for good, or every copy. */
  let confirming = $state<DeletedItem | 'all' | null>(null);
  let restoring = $state<string | null>(null);

  onMount(() => {
    void recentlyDeleted.load();
  });

  function whereText(item: DeletedItem): string {
    if (item.kind === 'collection') {
      return plural(item.requestCount, 'request');
    }
    if (item.collectionExists) {
      return `In “${item.collectionName}”`;
    }
    return `“${item.collectionName}” was deleted too. Restore puts it in a new collection with that name.`;
  }

  function daysLeft(item: DeletedItem): string {
    const left = Math.max(
      1,
      Math.ceil((item.deletedAt + KEEP_DAYS * DAY_MS - Date.now()) / DAY_MS),
    );
    return `${plural(left, 'day')} left`;
  }

  async function restore(item: DeletedItem): Promise<void> {
    restoring = item.id;
    await recentlyDeleted.restore(item, false);
    restoring = null;
  }

  function forget(): void {
    const target = confirming;
    confirming = null;
    if (target === 'all') {
      void recentlyDeleted.forgetAll();
    } else if (target) {
      void recentlyDeleted.forget(target.id);
    }
  }
</script>

<div class="view">
  <header class="top">
    <div class="title">
      <h1 class="page-title"><NavIcon name="trash" size="lg" />{navLabel('trash')}</h1>
      <p>
        Collections and requests you delete stay here for {KEEP_DAYS} days. Restore puts them back as
        they were, with their examples, Docs, variables and environments.
      </p>
    </div>
    <button
      type="button"
      class="quiet"
      disabled={recentlyDeleted.items.length === 0}
      onclick={() => (confirming = 'all')}
    >
      Delete all
    </button>
  </header>

  {#if !recentlyDeleted.loaded}
    <p class="empty" role="status">Loading…</p>
  {:else if recentlyDeleted.items.length === 0}
    <p class="empty">Nothing deleted in the last {KEEP_DAYS} days.</p>
  {:else}
    <ul class="list">
      {#each recentlyDeleted.items as item (item.id)}
        <li class="item">
          <span class="chip">{item.kind === 'collection' ? 'Collection' : 'Request'}</span>
          <span class="text">
            <span class="name">{item.name}</span>
            <span class="where">{whereText(item)}</span>
          </span>
          <span class="meta">
            Deleted {formatSince(item.deletedAt)} · {daysLeft(item)}
          </span>
          <button
            type="button"
            class="restore"
            disabled={restoring !== null}
            onclick={() => void restore(item)}
          >
            {restoring === item.id ? 'Restoring…' : 'Restore'}
          </button>
          <button
            type="button"
            class="icon"
            aria-label="Delete “{item.name}” for good"
            onclick={() => (confirming = item)}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<ConfirmDialog
  open={confirming !== null}
  title="Delete for good?"
  message={confirming === 'all'
    ? `${plural(recentlyDeleted.items.length, 'item')} will be deleted for good. This cannot be undone.`
    : `“${confirming?.name ?? ''}” will be deleted for good. This cannot be undone.`}
  confirmLabel={confirming === 'all' ? 'Delete all' : 'Delete'}
  onConfirm={forget}
  onCancel={() => (confirming = null)}
/>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
  }

  .title {
    flex: 1 1 320px;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .title p {
    margin: 4px 0 0;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .quiet {
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .quiet:hover:not(:disabled) {
    border-color: var(--err);
    color: var(--err);
  }

  .empty {
    margin: 24px 0;
    color: var(--text-muted);
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .item {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-soft);
  }

  .item:last-child {
    border-bottom: 0;
  }

  .chip {
    width: 78px;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--surface-sunken);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    text-align: center;
  }

  .text {
    flex: 1 1 240px;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .where,
  .meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  .meta {
    white-space: nowrap;
  }

  .restore {
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
    font-weight: 500;
  }

  .restore:hover:not(:disabled) {
    border-color: var(--text);
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

  .icon:hover {
    background: var(--surface-sunken);
    color: var(--err);
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
</style>
