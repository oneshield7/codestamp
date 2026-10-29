<script lang="ts">
  import type { Snippet } from 'svelte';
  import { isSaveShortcut } from '../dom/keys';
  import type { Workspace } from '../state/workspace.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import PanelDialog from './PanelDialog.svelte';

  interface Props {
    workspace: Workspace;
    title: string;
    description: string;
    dirty: boolean;
    saving: boolean;
    onSave: () => void;
    onClose: () => void;
    children: Snippet;
  }

  let { workspace, title, description, dirty, saving, onSave, onClose, children }: Props = $props();

  // Switching collection or view would drop these edits, so it is refused until they are saved.
  $effect(() => workspace.addLeaveGuard(() => (dirty ? title : null)));

  let confirmDiscard = $state(false);

  let panel: HTMLElement;

  // Only while the keyboard is in this panel's dialog, not in a dialog opened over it.
  function saveShortcut(event: KeyboardEvent): void {
    const inPanel = panel.closest('dialog')?.contains(document.activeElement) ?? false;
    if (!isSaveShortcut(event) || event.defaultPrevented || !inPanel) {
      return;
    }
    event.preventDefault();
    if (dirty && !saving) {
      onSave();
    }
  }

  function close(): void {
    if (dirty) {
      confirmDiscard = true;
      return;
    }
    onClose();
  }
</script>

<svelte:window onkeydown={saveShortcut} />

<PanelDialog {title} onClose={close}>
  <section class="panel" aria-label={title} bind:this={panel}>
    <div class="head">
      <div class="title">
        <h2>{title}</h2>
        <p>{description}</p>
      </div>
      <button type="button" class="primary" disabled={!dirty || saving} onclick={onSave}>
        {saving ? 'Saving…' : dirty ? 'Save' : 'Saved'}
      </button>
      <button type="button" class="quiet" onclick={close}>Close</button>
    </div>
    {@render children()}
  </section>
</PanelDialog>

<ConfirmDialog
  open={confirmDiscard}
  title="Discard changes?"
  message="You have unsaved changes in {title.toLowerCase()}. Closing will discard them."
  confirmLabel="Discard"
  onConfirm={() => {
    confirmDiscard = false;
    onClose();
  }}
  onCancel={() => (confirmDiscard = false)}
/>

<style>
  .panel {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .title {
    flex: 1 1 260px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  p {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .primary {
    height: 32px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
  }

  .primary:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .quiet {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }
</style>
