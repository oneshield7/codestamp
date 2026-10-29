<script lang="ts">
  import type { Health } from '../../api/catalog';
  import { HEALTH_LABELS, healthTone } from '../../catalog/present';

  interface Props {
    health: Health;
  }

  let { health }: Props = $props();
</script>

<span class="chip {healthTone(health.status)}">
  {#if health.score !== null}<span class="score mono">{health.score}</span>{/if}
  {HEALTH_LABELS[health.status]}
</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 9px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--surface-sunken);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .score {
    font-size: 12px;
  }

  .ok {
    border-color: var(--ok-line);
    background: var(--ok-bg);
    color: var(--ok);
  }

  .warn {
    border-color: var(--warn-line);
    background: var(--warn-bg);
    color: var(--warn);
  }

  .err {
    border-color: var(--err-line);
    background: var(--err-bg);
    color: var(--err);
  }
</style>
