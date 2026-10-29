<script lang="ts">
  import type { CollectionSummary } from '../api/collections';
  import CollectionTarget from './CollectionTarget.svelte';

  interface Props {
    open: boolean;
    requestName: string;
    collections: CollectionSummary[];
    onSave: (collectionId: string, sectionId: string | null) => void;
    onCancel: () => void;
  }

  let { open, requestName, collections, onSave, onCancel }: Props = $props();

  let dialog: HTMLDialogElement;
  let collectionId = $state('');
  let sectionId: string | null = $state(null);

  $effect(() => {
    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function save(): void {
    if (collectionId !== '') {
      onSave(collectionId, sectionId);
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="save-title" oncancel={onCancel}>
  <h2 id="save-title">Save to a collection</h2>
  <p>“{requestName}” will be added as a new request.</p>
  {#if open}
    <CollectionTarget {collections} bind:collectionId bind:sectionId />
  {/if}
  <div class="actions">
    <button type="button" class="secondary" onclick={onCancel}>Cancel</button>
    <button type="button" class="primary" disabled={collectionId === ''} onclick={save}>Save</button
    >
  </div>
</dialog>

<style>
  dialog {
    width: min(440px, calc(100vw - 32px));
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  h2 {
    margin: 0 0 6px;
    font-size: 17px;
    font-weight: 600;
  }

  p {
    margin: 0 0 16px;
    color: var(--text-2);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }

  button {
    height: 36px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 500;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
  }

  .primary:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }
</style>
