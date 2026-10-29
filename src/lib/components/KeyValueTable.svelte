<script lang="ts">
  import Dropdown from './Dropdown.svelte';
  import type { FormFieldKind } from '../api/collections';
  import { isSensitiveName } from '../privacy/secrets';
  import { fileName } from '../request/body';
  import { createRow, type KeyValueRow } from '../request/draft';
  import VariableInput from './VariableInput.svelte';

  interface Props {
    title: string;
    rows: KeyValueRow[];
    collectionId: string | null;
    /** Shows dots for values of rows such as Authorization or api_key. */
    maskSensitive?: boolean;
    /** Form data: each row is text or a file picked in the system dialog. */
    files?: boolean;
    onPickFile?: (row: KeyValueRow) => void;
    /** Shown under the table, e.g. why some rows are not sent. */
    note?: string;
  }

  let {
    title,
    rows = $bindable(),
    collectionId,
    maskSensitive = false,
    files = false,
    onPickFile,
    note,
  }: Props = $props();

  // Typing into the last row adds a fresh empty one, so there is always a place to add more.
  function ensureTrailingRow(index: number): void {
    if (index === rows.length - 1) {
      rows.push(createRow());
    }
  }

  function removeRow(id: string): void {
    rows = rows.filter((row) => row.id !== id);
    if (rows.length === 0) {
      rows.push(createRow());
    }
  }

  // A text value must never be taken for a file path, so switching the type starts empty.
  const FIELD_KINDS: { value: FormFieldKind; label: string }[] = [
    { value: 'text', label: 'Text' },
    { value: 'file', label: 'File' },
  ];

  function setKind(row: KeyValueRow, index: number, kind: FormFieldKind): void {
    row.kind = kind;
    row.value = '';
    ensureTrailingRow(index);
  }
</script>

<section class="kv" aria-label={title}>
  <h3>{title}</h3>
  <div class="scroll">
    <table>
      <thead>
        <tr>
          <th scope="col" class="toggle"><span class="visually-hidden">Enabled</span></th>
          <th scope="col">Key</th>
          {#if files}
            <th scope="col" class="type">Type</th>
          {/if}
          <th scope="col">Value</th>
          <th scope="col" class="actions"><span class="visually-hidden">Actions</span></th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row, index (row.id)}
          <tr class:disabled={!row.enabled}>
            <td class="toggle">
              <input type="checkbox" bind:checked={row.enabled} aria-label="Enable row" />
            </td>
            <td>
              <input
                class="cell mono"
                bind:value={row.key}
                oninput={() => {
                  ensureTrailingRow(index);
                }}
                placeholder="Key"
                aria-label="{title} key"
                spellcheck="false"
              />
            </td>
            {#if files}
              <td class="type">
                <Dropdown
                  variant="cell"
                  label="{title} type"
                  value={row.kind ?? 'text'}
                  options={FIELD_KINDS}
                  onChange={(kind: FormFieldKind) => {
                    setKind(row, index, kind);
                  }}
                />
              </td>
            {/if}
            <td>
              {#if files && row.kind === 'file'}
                <button
                  type="button"
                  class="file"
                  aria-label="{title} file"
                  title={row.value === '' ? 'Choose a file to upload' : fileName(row.value)}
                  onclick={() => onPickFile?.(row)}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M14 3H6v18h12V7zM14 3v4h4" />
                  </svg>
                  <span class="file-name"
                    >{row.value === '' ? 'Choose file…' : fileName(row.value)}</span
                  >
                </button>
              {:else}
                <VariableInput
                  bind:value={row.value}
                  label="{title} value"
                  variant="cell"
                  placeholder="Value"
                  masked={maskSensitive && isSensitiveName(row.key)}
                  {collectionId}
                  onInput={() => {
                    ensureTrailingRow(index);
                  }}
                />
              {/if}
            </td>
            <td class="actions">
              {#if index < rows.length - 1}
                <button
                  type="button"
                  class="remove"
                  aria-label="Remove row"
                  onclick={() => {
                    removeRow(row.id);
                  }}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M6 6l12 12M18 6L6 18" />
                  </svg>
                </button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if note}
    <p class="note">{note}</p>
  {/if}
</section>

<style>
  .kv {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }

  .scroll {
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 420px;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    padding: 6px 8px;
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-soft);
  }

  td {
    padding: 3px 8px;
    border-bottom: 1px solid var(--border-soft);
  }

  .toggle,
  .actions {
    width: 36px;
  }

  .type {
    width: 84px;
  }

  tr.disabled .cell {
    color: var(--text-muted);
    text-decoration: line-through;
  }

  .cell {
    width: 100%;
    height: 30px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
  }

  .cell:hover {
    border-color: var(--border);
  }

  .cell:focus {
    border-color: var(--focus);
    background: var(--surface);
    outline: none;
  }

  .file {
    width: 100%;
    max-width: 100%;
    height: 30px;
    padding: 0 8px;
    border: 1px dashed var(--border-strong);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    text-align: left;
  }

  .file:hover {
    border-color: var(--text-muted);
    color: var(--text);
  }

  .file-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .remove {
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

  .remove:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .remove path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
</style>
