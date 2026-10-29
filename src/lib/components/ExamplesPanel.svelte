<script lang="ts">
  import { deleteExample, getExample, renameExample, type Example } from '../api/examples';
  import { prettyJson } from '../format/json';
  import { statusTone } from '../format/status';
  import { formatBytes } from '../format/units';
  import type { ExampleList } from '../state/examples.svelte';
  import { notices } from '../state/notices.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import EditableText from './EditableText.svelte';

  interface Props {
    examples: ExampleList;
    /** Tells collaboration that the collection changed. */
    onChanged: () => void;
  }

  let { examples, onChanged }: Props = $props();

  let opened = $state<Example | null>(null);

  async function toggle(id: string): Promise<void> {
    if (opened?.id === id) {
      opened = null;
      return;
    }
    await notices.guard(async () => {
      opened = await getExample(id);
    });
  }

  async function rename(id: string, name: string): Promise<void> {
    await notices.guard(async () => {
      await renameExample(id, name);
      await examples.refresh();
      onChanged();
    });
  }

  async function remove(id: string): Promise<void> {
    await notices.guard(async () => {
      await deleteExample(id);
      if (opened?.id === id) {
        opened = null;
      }
      await examples.refresh();
      onChanged();
    });
  }

  const openedBody = $derived(opened ? (prettyJson(opened.body) ?? opened.body) : '');
  const openedIsJson = $derived(opened !== null && prettyJson(opened.body) !== null);
</script>

<div class="examples">
  {#if examples.items.length === 0}
    <p class="hint">
      No examples yet. Send the request, then choose <strong>Save as example</strong> in the response.
      Examples answer for this request when the collection’s mock server runs.
    </p>
  {:else}
    <ul class="list">
      {#each examples.items as example (example.id)}
        <li class:open={opened?.id === example.id}>
          <div class="row">
            <button
              type="button"
              class="toggle"
              aria-expanded={opened?.id === example.id}
              aria-label="Show example {example.name}"
              onclick={() => void toggle(example.id)}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M9 6l6 6-6 6" />
              </svg>
            </button>
            <span class="chip mono {statusTone(example.status)}">{example.status}</span>
            <span class="name">
              <EditableText
                value={example.name}
                label="Example name"
                onCommit={(name: string) => void rename(example.id, name)}
              />
            </span>
            <span class="size mono">{formatBytes(example.size)}</span>
            <button
              type="button"
              class="remove"
              aria-label="Delete example {example.name}"
              onclick={() => void remove(example.id)}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M6 6l12 12M18 6L6 18" />
              </svg>
            </button>
          </div>
          {#if opened?.id === example.id}
            {#if opened.headers.length > 0}
              <dl class="headers mono">
                {#each opened.headers as header, index (index)}
                  <dt>{header.name}</dt>
                  <dd>{header.value}</dd>
                {/each}
              </dl>
            {/if}
            <div class="body">
              <CodeEditor
                value={openedBody}
                label="Example body"
                language={openedIsJson ? 'json' : 'text'}
                readOnly
              />
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .examples {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  li + li {
    border-top: 1px solid var(--border-soft);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
  }

  .toggle,
  .remove {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
  }

  .toggle:hover,
  .remove:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .remove:hover {
    color: var(--err);
  }

  li.open .toggle svg {
    transform: rotate(90deg);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
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

  .name {
    flex: 1;
    min-width: 0;
  }

  .size {
    font-size: 12px;
    color: var(--text-muted);
  }

  .headers {
    display: grid;
    grid-template-columns: minmax(120px, max-content) 1fr;
    gap: 4px 14px;
    margin: 0;
    padding: 4px 16px 8px 46px;
    font-size: 12px;
  }

  .headers dt {
    color: var(--text-muted);
  }

  .headers dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .body {
    height: 200px;
    margin: 0 10px 10px 46px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    resize: vertical;
    overflow: hidden;
    padding: 4px;
  }
</style>
