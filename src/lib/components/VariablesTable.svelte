<script lang="ts">
  import type { AccessGroup } from '../api/team';
  import Dropdown from './Dropdown.svelte';
  import {
    createVariableRow,
    RESTRICTED_PREFIX,
    typeOf,
    type VariableEditor,
    type VariableRow,
    type VariableType,
  } from '../state/variable-editor.svelte';

  interface Props {
    editor: VariableEditor;
    /** Names the active environment also defines; its values are the ones sent. */
    replacedBy?: { environment: string; keys: ReadonlySet<string> } | null;
    /** Access groups of the shared collection these variables travel with. */
    groups?: AccessGroup[];
  }

  let { editor, replacedBy = null, groups = [] }: Props = $props();

  // Only groups this device can use are offered; a value restricted to another one stays as it is.
  const types = $derived<{ value: VariableType; label: string }[]>([
    { value: 'text', label: 'Text' },
    { value: 'secret', label: 'Secret' },
    ...groups
      .filter((group) => group.access && group.problem === null)
      .map((group) => ({
        value: `${RESTRICTED_PREFIX}${group.id}`,
        label: `Restricted: ${group.name}`,
      })),
  ]);

  function groupName(id: string | null): string {
    return groups.find((group) => group.id === id)?.name ?? 'a group';
  }

  // Typing into the last row adds a fresh empty one, so there is always a place to add more.
  function edited(index: number): void {
    if (index === editor.rows.length - 1) {
      editor.rows.push(createVariableRow());
    }
    editor.markDirty();
  }

  function editValue(row: VariableRow, index: number): void {
    // Typing over a hidden stored secret replaces it with the new value.
    row.hiddenSecret = false;
    edited(index);
  }

  function removeRow(rowId: string): void {
    editor.rows = editor.rows.filter((row) => row.rowId !== rowId);
    if (editor.rows.length === 0) {
      editor.rows.push(createVariableRow());
    }
    editor.markDirty();
  }
</script>

<div class="scroll">
  <table>
    <thead>
      <tr>
        <th scope="col" class="narrow"><span class="visually-hidden">Enabled</span></th>
        <th scope="col">Variable</th>
        <th scope="col">Value</th>
        <th scope="col" class="type">Type</th>
        <th scope="col" class="narrow"><span class="visually-hidden">Actions</span></th>
      </tr>
    </thead>
    <tbody>
      {#each editor.rows as row, index (row.rowId)}
        <tr class:disabled={!row.enabled}>
          <td class="narrow">
            <input
              type="checkbox"
              bind:checked={row.enabled}
              onchange={() => {
                editor.markDirty();
              }}
              aria-label="Enable variable"
            />
          </td>
          <td>
            <input
              class="cell mono"
              bind:value={row.key}
              oninput={() => {
                edited(index);
              }}
              placeholder="Add variable"
              aria-label="Variable name"
              spellcheck="false"
              maxlength="100"
              readonly={row.locked}
            />
            {#if replacedBy && row.enabled && replacedBy.keys.has(row.key.trim())}
              <span class="replaced">Replaced by “{replacedBy.environment}”</span>
            {/if}
          </td>
          <td>
            {#if row.locked}
              <span class="locked">
                Restricted to {groupName(row.restricted)}. Ask a member for access in Collaboration.
              </span>
            {:else}
              <span class="value">
                <input
                  class="cell mono"
                  type={row.isSecret && !row.visible ? 'password' : 'text'}
                  bind:value={row.value}
                  oninput={() => {
                    editValue(row, index);
                  }}
                  placeholder={row.hiddenSecret ? 'Encrypted · type to replace' : 'Value'}
                  aria-label="Variable value"
                  spellcheck="false"
                  autocomplete="off"
                />
                {#if row.isSecret}
                  <button
                    type="button"
                    class="icon"
                    aria-label={row.visible ? 'Hide value' : 'Show value'}
                    onclick={() => void editor.toggleVisible(row)}
                  >
                    <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z" />
                      <circle cx="12" cy="12" r="3" />
                    </svg>
                  </button>
                {/if}
              </span>
            {/if}
          </td>
          <td class="type">
            {#if row.locked}
              <span class="locked">Restricted: {groupName(row.restricted)}</span>
            {:else}
              <Dropdown
                variant="cell"
                label="Variable type"
                value={typeOf(row)}
                options={types.some((type) => type.value === typeOf(row))
                  ? types
                  : [
                      ...types,
                      { value: typeOf(row), label: `Restricted: ${groupName(row.restricted)}` },
                    ]}
                onChange={(type: VariableType) => void editor.setType(row, type)}
              />
            {/if}
          </td>
          <td class="narrow">
            {#if index < editor.rows.length - 1 && !row.locked}
              <button
                type="button"
                class="icon"
                aria-label="Remove variable"
                onclick={() => {
                  removeRow(row.rowId);
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

<style>
  .scroll {
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 600px;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    padding: 10px;
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-soft);
  }

  td {
    padding: 4px 10px;
    border-bottom: 1px solid var(--border-soft);
  }

  .replaced {
    display: block;
    padding: 0 8px 4px;
    font-size: 12px;
    color: var(--warn);
  }

  .narrow {
    width: 40px;
  }

  .type {
    width: 150px;
  }

  .locked {
    display: block;
    padding: 0 8px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-muted);
  }

  tr.disabled .cell {
    color: var(--text-muted);
    text-decoration: line-through;
  }

  .value {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .cell {
    width: 100%;
    height: 32px;
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

  .icon {
    flex: none;
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

  .icon:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
