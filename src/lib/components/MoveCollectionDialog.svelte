<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../api/http';
  import type { Workspace } from '../api/workspaces';
  import { plural } from '../format/plural';
  import { workspaces } from '../state/workspaces.svelte';
  import WorkspaceTile from './WorkspaceTile.svelte';

  interface Props {
    collectionName: string;
    /** Resolves once moved; a failure is thrown so the dialog can show it. */
    onMove: (target: Workspace) => Promise<void>;
    onClose: () => void;
  }

  let { collectionName, onMove, onClose }: Props = $props();

  let dialog: HTMLDialogElement;
  let busy = $state(false);
  let error = $state<string | null>(null);

  const others = $derived(workspaces.list.filter((item) => item.id !== workspaces.currentId));
  const here = $derived(workspaces.current?.name ?? 'this workspace');
  let chosenId = $state<string | null>(null);
  const target = $derived(others.find((item) => item.id === chosenId) ?? others.at(0) ?? null);

  onMount(() => {
    dialog.showModal();
  });

  async function move(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!target) {
      return;
    }
    busy = true;
    error = null;
    try {
      await onMove(target);
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="move-collection-title"
  aria-describedby="move-collection-about"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <form onsubmit={(event) => void move(event)}>
    <header class="head">
      <div>
        <h2 id="move-collection-title">Move to workspace</h2>
        <p id="move-collection-about">
          “{collectionName}” moves with its requests, history and the environments it owns.
          Environments kept on this device stay in {here}.
        </p>
      </div>
      <button type="button" class="close" aria-label="Close" onclick={onClose}>
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </header>

    <div class="body">
      <fieldset>
        <legend>Workspace</legend>
        {#each others as workspace (workspace.id)}
          <label class="option">
            <input
              type="radio"
              name="move-target"
              value={workspace.id}
              checked={workspace.id === target?.id}
              onchange={() => (chosenId = workspace.id)}
            />
            <WorkspaceTile name={workspace.name} hue={workspace.hue} icon={workspace.icon} />
            <span class="name">{workspace.name}</span>
            <span class="count">{plural(workspace.collectionCount, 'collection')}</span>
          </label>
        {/each}
      </fieldset>

      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>

    <footer class="foot">
      <button type="button" class="secondary" onclick={onClose}>Cancel</button>
      <button type="submit" class="primary" disabled={busy || target === null}>
        {busy ? 'Moving…' : 'Move collection'}
      </button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(460px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  form {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 34px);
  }

  .head,
  .foot {
    flex: none;
    display: flex;
    gap: 10px;
    padding: 16px 22px;
  }

  .head {
    align-items: flex-start;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-soft);
  }

  .foot {
    align-items: center;
    justify-content: flex-end;
    border-top: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .head p {
    margin: 4px 0 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .close {
    flex: none;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .close:hover {
    background: var(--surface-sunken);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .body {
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px 22px 20px;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  legend {
    margin-bottom: 8px;
    padding: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 14px;
    cursor: pointer;
  }

  .option:hover {
    border-color: var(--border-strong);
  }

  .option:has(input:checked) {
    border-color: var(--text);
    background: var(--surface-muted);
  }

  .option:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  .option input {
    margin: 0;
    accent-color: var(--accent);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .count {
    color: var(--text-muted);
    font-size: 12px;
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
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
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }
</style>
