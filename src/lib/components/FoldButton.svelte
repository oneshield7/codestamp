<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    expanded: boolean;
    /** What it folds, read after "Expand" or "Collapse", e.g. "section Users". */
    label: string;
    title?: string;
    disabled?: boolean;
    onToggle: () => void;
    /** Visible text beside the chevron, for a heading that folds when clicked. */
    children?: Snippet;
  }

  let { expanded, label, title, disabled = false, onToggle, children }: Props = $props();
</script>

<!-- The chevron points down when open and right when folded. -->
<button
  type="button"
  class="fold"
  class:labelled={children !== undefined}
  aria-expanded={expanded}
  aria-label="{expanded ? 'Collapse' : 'Expand'} {label}"
  {title}
  {disabled}
  onclick={onToggle}
>
  <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
    <path d="M6 9l6 6 6-6" />
  </svg>
  {@render children?.()}
</button>

<style>
  .fold {
    flex: none;
    width: 24px;
    height: 28px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .fold.labelled {
    width: auto;
    gap: 4px;
    padding: 0 6px 0 2px;
    font: inherit;
  }

  .fold:hover:not(:disabled) {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .fold:disabled {
    opacity: 0.45;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .fold[aria-expanded='false'] svg {
    transform: rotate(-90deg);
  }
</style>
