<script lang="ts">
  interface Props {
    open: boolean;
    title: string;
    message: string;
    confirmLabel: string;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { open, title, message, confirmLabel, onConfirm, onCancel }: Props = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });
</script>

<dialog bind:this={dialog} aria-labelledby="confirm-title" oncancel={onCancel}>
  <h2 id="confirm-title">{title}</h2>
  <p>{message}</p>
  <div class="actions">
    <button type="button" class="secondary" onclick={onCancel}>Cancel</button>
    <button type="button" class="danger" onclick={onConfirm}>{confirmLabel}</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(420px, calc(100vw - 32px));
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
    margin: 0 0 8px;
    font-size: 17px;
    font-weight: 600;
  }

  p {
    margin: 0 0 20px;
    color: var(--text-2);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
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

  .danger {
    border: 0;
    background: var(--err);
    color: var(--on-accent);
  }
</style>
