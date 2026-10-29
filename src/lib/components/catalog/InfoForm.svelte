<script lang="ts">
  import { untrack } from 'svelte';
  import { parseTags } from '../../catalog/present';

  interface Props {
    owner: string;
    tags: string[];
    /** Resolves to why saving failed, or null once it is saved. */
    onSave: (owner: string, tags: string[]) => Promise<string | null>;
    onClose: () => void;
  }

  let { owner, tags, onSave, onClose }: Props = $props();

  // The form edits copies taken once; the saved values come back through the props.
  let ownerText = $state(untrack(() => owner));
  let tagsText = $state(untrack(() => tags.join(', ')));
  let saving = $state(false);
  let error = $state<string | null>(null);

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    saving = true;
    error = await onSave(ownerText, parseTags(tagsText));
    saving = false;
    if (error === null) {
      onClose();
    }
  }
</script>

<form class="panel" aria-label="Owner and tags" onsubmit={(event) => void save(event)}>
  <div class="fields">
    <label class="field">
      <span>Owner</span>
      <input
        type="text"
        bind:value={ownerText}
        maxlength="80"
        placeholder="A person or a team"
        autocomplete="off"
      />
    </label>
    <label class="field">
      <span>Tags</span>
      <input
        type="text"
        bind:value={tagsText}
        placeholder="public, payments, v2"
        autocomplete="off"
        aria-describedby="tags-hint"
      />
      <span class="hint" id="tags-hint"
        >Separate tags with commas: up to 12, each up to 30 characters.</span
      >
    </label>
  </div>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  <p class="note">Kept on this device only: owner and tags are not shared through Collaboration.</p>
  <div class="actions">
    <button type="button" class="secondary" disabled={saving} onclick={onClose}>Cancel</button>
    <button type="submit" class="primary" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
  </div>
</form>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .field {
    flex: 1 1 260px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  input {
    height: 38px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
  }

  /* The border turns blue instead of a second ring. */
  input:focus {
    outline: none;
    border-color: var(--focus);
  }

  .hint,
  .note {
    font-size: 12px;
    color: var(--text-muted);
  }

  .note {
    margin: 0;
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions button {
    height: 36px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 600;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--text-2);
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
  }

  .actions button:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }
</style>
