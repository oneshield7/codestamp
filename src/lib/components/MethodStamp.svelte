<script lang="ts">
  import type { StampTone } from '../request/methods';

  interface Props {
    /** The method, or the kind's label (WS, MCP, GQL, gRPC); see `stampLabel`. */
    label: string;
    tone: StampTone;
    size?: 'sm' | 'md';
  }

  let { label, tone, size = 'md' }: Props = $props();
</script>

<span
  class="stamp mono {size}"
  style:color="var(--method-{tone})"
  style:background="var(--method-{tone}-bg)"
  style:border-color="var(--method-{tone}-line)">{label}</span
>

<style>
  .stamp {
    justify-self: start;
    padding: 2px 7px;
    border: 1px solid;
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .sm {
    padding: 2px 6px;
    font-size: 10px;
  }
</style>
