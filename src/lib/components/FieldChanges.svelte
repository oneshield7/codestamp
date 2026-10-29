<script lang="ts">
  import type { FieldChange } from '../api/team';

  interface Props {
    changes: FieldChange[];
    /** Column titles for `then` and `now`. */
    thenLabel: string;
    nowLabel: string;
  }

  let { changes, thenLabel, nowLabel }: Props = $props();
</script>

<div class="grid">
  <span class="col">Field</span>
  <span class="col">{thenLabel}</span>
  <span class="col">{nowLabel}</span>
  {#each changes as change (change.label)}
    <span class="label">{change.label}</span>
    <pre class="value mono">{change.then}</pre>
    <pre class="value mono">{change.now}</pre>
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: minmax(90px, max-content) minmax(0, 1fr) minmax(0, 1fr);
    gap: 6px 12px;
    align-items: start;
  }

  .col {
    font-size: 12px;
    color: var(--text-muted);
  }

  .label {
    font-size: 13px;
    color: var(--text-2);
  }

  .value {
    margin: 0;
    max-height: 200px;
    overflow: auto;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-muted);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
