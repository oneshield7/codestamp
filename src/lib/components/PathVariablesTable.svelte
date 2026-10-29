<script lang="ts">
  import type { PathVariable } from '../api/collections';
  import { isSensitiveName } from '../privacy/secrets';
  import { pathVariableValues, setPathVariable } from '../request/path-variables';
  import VariableInput from './VariableInput.svelte';

  interface Props {
    url: string;
    variables: PathVariable[];
    collectionId: string | null;
    maskSensitive?: boolean;
  }

  let { url, variables = $bindable(), collectionId, maskSensitive = false }: Props = $props();

  const rows = $derived(pathVariableValues(url, variables));
</script>

{#if rows.length > 0}
  <section class="path" aria-label="Path variables">
    <h3>Path variables</h3>
    <table>
      <thead>
        <tr>
          <th scope="col">Key</th>
          <th scope="col">Value</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.name)}
          <tr>
            <td class="key mono">:{row.name}</td>
            <td>
              <VariableInput
                bind:value={
                  () => row.value,
                  (value: string) => {
                    setPathVariable(variables, row.name, value);
                  }
                }
                label="Path variable {row.name}"
                variant="cell"
                placeholder="Value for :{row.name}"
                masked={maskSensitive && isSensitiveName(row.name)}
                {collectionId}
              />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="note">
      Each <span class="mono">:name</span> in the path is replaced by its value when sending.
    </p>
  </section>
{/if}

<style>
  .path {
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

  table {
    width: 100%;
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

  .key {
    width: 32%;
    color: var(--var);
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
