<script lang="ts">
  import { MARK_LABELS, type StepMark } from '../flow/outcome';

  interface Props {
    mark: StepMark;
    size?: number;
  }

  let { mark, size = 18 }: Props = $props();
</script>

<span
  class="mark {mark}"
  role="img"
  title={MARK_LABELS.get(mark)}
  aria-label={MARK_LABELS.get(mark)}
>
  <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
    {#if mark === 'passed'}
      <circle cx="12" cy="12" r="9" /><path d="M8 12.5l3 3 5-6" />
    {:else if mark === 'failed'}
      <circle cx="12" cy="12" r="9" /><path d="M9 9l6 6M15 9l-6 6" />
    {:else if mark === 'running'}
      <circle class="spin" cx="12" cy="12" r="9" />
    {:else}
      <circle cx="12" cy="12" r="9" stroke-dasharray={mark === 'skipped' ? '3 3' : undefined} />
    {/if}
  </svg>
</span>

<style>
  .mark {
    display: inline-flex;
    flex: none;
  }

  svg {
    display: block;
    fill: none;
    stroke: var(--border-strong);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .passed svg {
    stroke: var(--ok);
  }

  .failed svg {
    stroke: var(--err);
  }

  .running svg {
    stroke: var(--focus);
  }

  .spin {
    stroke-dasharray: 40 20;
    transform-origin: center;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
