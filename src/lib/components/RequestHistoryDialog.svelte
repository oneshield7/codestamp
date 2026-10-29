<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../api/http';
  import {
    requestVersionChanges,
    requestVersions,
    restoreRequestVersion,
    type FieldChange,
    type Version,
  } from '../api/team';
  import { formatDateTime } from '../format/time';
  import type { Workspace } from '../state/workspace.svelte';
  import FieldChanges from './FieldChanges.svelte';

  interface Props {
    workspace: Workspace;
    collectionId: string;
    requestId: string;
    requestName: string;
    onClose: () => void;
  }

  let { workspace, collectionId, requestId, requestName, onClose }: Props = $props();

  const SECONDS_TO_MS = 1000;

  let dialog: HTMLDialogElement;
  let versions = $state<Version[] | null>(null);
  let selected = $state<string | null>(null);
  let changes = $state<FieldChange[] | null>(null);
  let error = $state<string | null>(null);
  let restoring = $state(false);
  // A slower answer for a version clicked earlier is ignored.
  let generation = 0;

  onMount(() => {
    dialog.showModal();
    void load();
  });

  // Compared with what is saved, so pending edits are saved first.
  async function load(): Promise<void> {
    try {
      await workspace.flushSave();
      const loaded = await requestVersions(collectionId, requestId);
      versions = loaded;
      // The newest version is usually the request as it is now; the one before is more useful.
      const first = loaded.at(1) ?? loaded.at(0);
      if (first) {
        await select(first.commit);
      }
    } catch (caught) {
      error = toAppError(caught).message;
    }
  }

  async function select(commit: string): Promise<void> {
    const run = ++generation;
    selected = commit;
    changes = null;
    error = null;
    try {
      const result = await requestVersionChanges(collectionId, requestId, commit);
      if (run === generation) {
        changes = result;
      }
    } catch (caught) {
      if (run === generation) {
        error = toAppError(caught).message;
      }
    }
  }

  async function restore(): Promise<void> {
    const commit = selected;
    if (!commit) {
      return;
    }
    restoring = true;
    error = null;
    try {
      if (!(await workspace.flushSave())) {
        error = 'The open edits could not be saved. Fix or undo them before restoring.';
        return;
      }
      const saved = await restoreRequestVersion(collectionId, requestId, commit);
      workspace.showSaved(saved);
      workspace.notifyChanged(collectionId);
      onClose();
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      restoring = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="history-title"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <header class="head">
    <div>
      <h2 id="history-title">History of “{requestName}”</h2>
      <p>
        Versions are recorded when changes sync. Restoring shares the old content as a new change.
      </p>
    </div>
    <span class="spacer"></span>
    <button type="button" class="close" aria-label="Close" onclick={onClose}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>
  </header>

  <div class="body">
    {#if versions === null && !error}
      <p class="muted pad">Loading versions…</p>
    {:else if versions && versions.length === 0}
      <p class="muted pad">No versions yet. They appear after the first sync.</p>
    {:else if versions}
      <ul class="versions" aria-label="Versions">
        {#each versions as version (version.commit)}
          <li>
            <button
              type="button"
              class="version"
              class:on={selected === version.commit}
              aria-pressed={selected === version.commit}
              onclick={() => void select(version.commit)}
            >
              <span class="message">{version.message}</span>
              <span class="meta">
                {version.author} · {formatDateTime(version.time * SECONDS_TO_MS)}
              </span>
            </button>
          </li>
        {/each}
      </ul>
      <div class="detail">
        {#if changes === null}
          <p class="muted">{selected ? 'Comparing…' : 'Pick a version.'}</p>
        {:else if changes.length === 0}
          <p class="muted">This version is the same as the request now.</p>
        {:else}
          <FieldChanges {changes} thenLabel="That version" nowLabel="Now" />
        {/if}
      </div>
    {/if}
  </div>

  <footer class="foot">
    {#if error}
      <span class="error" role="alert">{error}</span>
    {/if}
    <span class="spacer"></span>
    <button type="button" class="secondary" onclick={onClose}>Close</button>
    <button
      type="button"
      class="primary"
      disabled={restoring || !changes || changes.length === 0}
      onclick={() => void restore()}
    >
      {restoring ? 'Restoring…' : 'Restore this version'}
    </button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(920px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
  }

  dialog[open] {
    display: flex;
    flex-direction: column;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  .head,
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 18px;
  }

  .head {
    align-items: flex-start;
    border-bottom: 1px solid var(--border-soft);
  }

  .foot {
    border-top: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .head p {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .spacer {
    flex: 1;
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
  }

  .close path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .body {
    flex: 1;
    min-height: 240px;
    overflow: hidden;
    display: flex;
    flex-wrap: wrap;
  }

  .versions {
    flex: 1 1 240px;
    max-width: 320px;
    max-height: 60vh;
    overflow: auto;
    margin: 0;
    padding: 8px;
    list-style: none;
    border-right: 1px solid var(--border-soft);
  }

  .version {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
  }

  .version:hover {
    background: var(--surface-sunken);
  }

  .version.on {
    background: var(--accent-soft);
  }

  .message {
    font-size: 13px;
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  .detail {
    flex: 999 1 360px;
    min-width: 0;
    max-height: 60vh;
    overflow: auto;
    padding: 14px 18px;
  }

  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .pad {
    padding: 18px;
  }

  .error {
    font-size: 13px;
    color: var(--err);
  }

  .primary,
  .secondary {
    height: 36px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .primary:disabled {
    opacity: 0.6;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }
</style>
