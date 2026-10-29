<script lang="ts">
  import VariableInput from './VariableInput.svelte';

  interface Props {
    value: string;
    label: string;
    collectionId: string;
    placeholder?: string;
  }

  let { value = $bindable(), label, collectionId, placeholder = '' }: Props = $props();

  let revealed = $state(false);
</script>

<div class="secret">
  <VariableInput
    bind:value
    {label}
    variant="field"
    {placeholder}
    masked={!revealed}
    {collectionId}
  />
  <button
    type="button"
    class="reveal"
    aria-pressed={revealed}
    aria-label="{revealed ? 'Hide' : 'Show'} {label.toLowerCase()}"
    onclick={() => (revealed = !revealed)}
  >
    <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z" /><circle cx="12" cy="12" r="3" />
    </svg>
  </button>
</div>

<style>
  .secret {
    display: flex;
    gap: 8px;
  }

  .reveal {
    flex: none;
    width: 40px;
    height: 40px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .reveal[aria-pressed='true'] {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .reveal svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
