<script lang="ts">
  import { tick } from 'svelte';
  import { MAX_DESCRIPTION_CHARS, type Described } from '../../api/docs';
  import type { Docs } from '../../state/docs.svelte';
  import Markdown from './Markdown.svelte';

  interface Props {
    docs: Docs;
    item: Described;
    id: string;
    text: string;
    /** What the description is about, for the empty state and labels: "this request". */
    subject: string;
    headingOffset: number;
  }

  let { docs, item, id, text, subject, headingOffset }: Props = $props();

  const key = $derived(`${item}:${id}`);
  const editing = $derived(docs.editing === key);

  let draft = $state('');
  let preview = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let field: HTMLTextAreaElement | undefined = $state();

  const length = $derived(Array.from(draft).length);
  const tooLong = $derived(length > MAX_DESCRIPTION_CHARS);

  async function edit(): Promise<void> {
    draft = text;
    preview = false;
    error = null;
    docs.editing = key;
    await tick();
    field?.focus();
  }

  function cancel(): void {
    docs.editing = null;
    error = null;
  }

  async function save(): Promise<void> {
    if (tooLong || saving) {
      return;
    }
    saving = true;
    error = await docs.save(item, id, draft);
    saving = false;
  }

  function keys(event: KeyboardEvent): void {
    if (event.key === 'Enter' && event.ctrlKey) {
      event.preventDefault();
      void save();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      cancel();
    }
  }
</script>

{#if editing}
  <div class="editor">
    <div class="bar">
      <div class="segmented" role="group" aria-label="Description view">
        <button
          type="button"
          class:on={!preview}
          aria-pressed={!preview}
          onclick={() => (preview = false)}>Write</button
        >
        <button
          type="button"
          class:on={preview}
          aria-pressed={preview}
          onclick={() => (preview = true)}>Preview</button
        >
      </div>
      <span class="count" class:over={tooLong}
        >{length.toLocaleString('en')} / {MAX_DESCRIPTION_CHARS.toLocaleString('en')}</span
      >
    </div>
    {#if preview}
      <div class="preview">
        {#if draft.trim() === ''}
          <p class="empty">Nothing to preview yet.</p>
        {:else}
          <Markdown text={draft} {headingOffset} />
        {/if}
      </div>
    {:else}
      <textarea
        bind:this={field}
        bind:value={draft}
        class="mono"
        rows="8"
        spellcheck="true"
        aria-label={`Description of ${subject}`}
        aria-invalid={tooLong}
        placeholder="Markdown: # headings, **bold**, `code`, lists, tables and links"
        onkeydown={keys}></textarea>
    {/if}
    {#if tooLong}
      <p class="error" role="alert">A description can be up to 20,000 characters.</p>
    {:else if error}
      <p class="error" role="alert">{error}</p>
    {/if}
    <div class="actions">
      <button type="button" class="secondary" disabled={saving} onclick={cancel}>Cancel</button>
      <button type="button" class="primary" disabled={saving || tooLong} onclick={() => void save()}
        >{saving ? 'Saving…' : 'Save'}</button
      >
    </div>
  </div>
{:else if text.trim() === ''}
  <div class="none">
    <p>No description for {subject} yet.</p>
    <button type="button" class="link" onclick={() => void edit()}>Add a description</button>
  </div>
{:else}
  <div class="shown">
    <Markdown {text} {headingOffset} />
    <button type="button" class="link" onclick={() => void edit()}
      >Edit description <span class="visually-hidden">of {subject}</span></button
    >
  </div>
{/if}

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .segmented {
    display: inline-flex;
    padding: 3px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .segmented button {
    padding: 4px 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
  }

  .segmented button.on {
    background: var(--surface);
    box-shadow: var(--shadow-segment);
    color: var(--text);
    font-weight: 500;
  }

  .count {
    font-family: var(--font-code);
    font-size: 12px;
    color: var(--text-muted);
  }

  .count.over {
    color: var(--err);
  }

  textarea {
    width: 100%;
    min-height: 160px;
    padding: 10px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    color: var(--text);
    line-height: var(--code-line-height);
    resize: vertical;
  }

  /* The border turns blue instead of a second ring. */
  textarea:focus {
    outline: none;
    border-color: var(--focus);
  }

  textarea[aria-invalid='true'] {
    border-color: var(--err);
  }

  .preview {
    min-height: 160px;
    padding: 10px 12px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-md);
  }

  .error {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions button {
    height: 32px;
    padding: 0 14px;
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

  .none {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .none p,
  .empty {
    margin: 0;
  }

  .empty {
    color: var(--text-muted);
    font-size: 13px;
  }

  .shown {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }

  .link {
    padding: 2px 4px;
    margin-left: -4px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
  }

  .link:hover {
    background: var(--surface-sunken);
  }
</style>
