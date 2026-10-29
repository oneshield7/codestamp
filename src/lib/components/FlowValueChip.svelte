<script lang="ts">
  import type { ValueState } from '../flow/lineage';

  interface Props {
    name: string;
    state: ValueState;
  }

  let { name, state }: Props = $props();
</script>

<span
  class="chip mono {state}"
  title={state === 'missing' ? 'Not found in the last run' : undefined}>{`{{${name}}}`}</span
>

<style>
  .chip {
    display: inline-block;
    padding: 1px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    white-space: nowrap;
  }

  .passed {
    border-color: var(--ok-line);
    background: var(--ok-bg);
    color: var(--ok);
  }

  .missing {
    border-color: var(--err-line);
    background: var(--err-bg);
    color: var(--err);
    text-decoration: line-through;
  }
</style>
