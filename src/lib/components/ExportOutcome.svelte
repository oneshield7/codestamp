<script lang="ts">
  import type { ExportOutcome } from '../api/export';

  let { outcome }: { outcome: ExportOutcome | null } = $props();
</script>

{#if outcome && 'error' in outcome}
  <p class="error" role="alert">{outcome.error}</p>
{:else if outcome}
  <div class="exported" role="status">
    <p>Exported to <span class="mono">{outcome.path}</span></p>
    {#each outcome.notes as note, index (index)}
      <p class="note">{note}</p>
    {/each}
  </div>
{/if}

<style>
  .exported {
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 13px;
  }

  .exported p {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .exported .note {
    margin-top: 4px;
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
</style>
