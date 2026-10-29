<script lang="ts">
  import type { Tone } from '../../catalog/present';

  interface Props {
    label: string;
    value: string;
    note?: string;
    /** Colours the note when it says something good or bad, such as more errors. */
    noteTone?: Tone;
  }

  let { label, value, note, noteTone = 'neutral' }: Props = $props();
</script>

<div class="tile">
  <span class="label">{label}</span>
  <span class="value">{value}</span>
  {#if note}
    <span class="note {noteTone}">{note}</span>
  {/if}
</div>

<style>
  .tile {
    flex: 1 1 170px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .label {
    font-size: 13px;
    color: var(--text-muted);
  }

  .value {
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }

  .note {
    font-size: 12px;
    color: var(--text-muted);
  }

  .note.ok {
    color: var(--ok);
  }

  .note.warn {
    color: var(--warn);
  }

  .note.err {
    color: var(--err);
  }
</style>
