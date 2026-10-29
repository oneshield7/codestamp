<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../api/http';
  import { copyName, isNameTaken, MAX_NAME_LENGTH } from '../format/names';

  interface Props {
    kind: 'collection' | 'section' | 'request';
    sourceName: string;
    /** What the copy brings along, in a sentence or two. */
    details: string;
    /** Names the copy may not repeat: the original's neighbours. */
    takenNames: string[];
    onConfirm: (name: string) => Promise<void>;
    onClose: () => void;
  }

  let { kind, sourceName, details, takenNames, onConfirm, onClose }: Props = $props();

  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let name = $state(initialName());
  let busy = $state(false);
  let error = $state<string | null>(null);

  const problem = $derived.by(() => {
    const trimmed = name.trim();
    if (trimmed === '') {
      return 'Enter a name for the copy.';
    }
    if (Array.from(trimmed).length > MAX_NAME_LENGTH) {
      return `Use up to ${String(MAX_NAME_LENGTH)} characters.`;
    }
    if (isNameTaken(trimmed, takenNames)) {
      return `A ${kind} named “${trimmed}” already exists here. Choose another name.`;
    }
    return null;
  });

  function initialName(): string {
    return copyName(sourceName);
  }

  onMount(() => {
    dialog.showModal();
    input.select();
  });

  async function confirm(): Promise<void> {
    if (problem !== null || busy) {
      return;
    }
    busy = true;
    error = null;
    try {
      await onConfirm(name.trim());
      onClose();
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="duplicate-title"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void confirm();
    }}
  >
    <header class="head">
      <h2 id="duplicate-title">Duplicate {kind}</h2>
      <p>Make a copy of “{sourceName}”.</p>
    </header>

    <div class="body">
      <div class="field">
        <label for="duplicate-name">Name of the copy</label>
        <input
          bind:this={input}
          id="duplicate-name"
          bind:value={name}
          maxlength={MAX_NAME_LENGTH}
          autocomplete="off"
          aria-invalid={problem !== null}
          aria-describedby="duplicate-name-hint"
        />
        <span id="duplicate-name-hint" class="hint" class:bad={problem !== null}>
          {problem ?? 'You can rename it later.'}
        </span>
      </div>
      <p class="info">{details}</p>
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>

    <footer class="foot">
      <button type="button" class="secondary" onclick={onClose}>Cancel</button>
      <button type="submit" class="primary" disabled={problem !== null || busy}>
        {busy ? 'Duplicating…' : 'Duplicate'}
      </button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(460px, calc(100vw - 32px));
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

  .head {
    padding: 18px 20px 14px;
    border-bottom: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .head p {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 20px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  label {
    font-weight: 500;
  }

  input {
    height: 40px;
    box-sizing: border-box;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 14px;
  }

  input:focus {
    border-color: var(--focus);
    outline: none;
  }

  input[aria-invalid='true'] {
    border-color: var(--err);
  }

  .hint {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .hint.bad {
    color: var(--err);
  }

  .info {
    margin: 0;
    padding: 10px 12px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-2);
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid var(--border-soft);
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
