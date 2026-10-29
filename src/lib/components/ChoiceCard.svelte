<script lang="ts" generics="T extends string">
  import type { Snippet } from 'svelte';

  interface Props {
    /** The radio group this card belongs to. */
    name: string;
    value: T;
    checked: boolean;
    title: string;
    description: string;
    onSelect: (value: T) => void;
    /** The icon, drawn in the 32px box. */
    children: Snippet;
  }

  let { name, value, checked, title, description, onSelect, children }: Props = $props();
</script>

<label class="option" class:on={checked}>
  <input
    type="radio"
    {name}
    {value}
    {checked}
    onchange={() => {
      onSelect(value);
    }}
  />
  <span class="option-icon">{@render children()}</span>
  <span class="option-text">
    <span class="option-title">{title}</span>
    <span class="option-desc">{description}</span>
  </span>
</label>

<style>
  .option {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .option.on {
    border-color: var(--text);
    background: var(--surface-muted);
  }

  .option input {
    margin: 10px 0 0;
    accent-color: var(--accent);
  }

  .option-icon {
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .option-icon :global(svg) {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .option-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .option-title {
    font-weight: 500;
  }

  .option-desc {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-muted);
  }
</style>
