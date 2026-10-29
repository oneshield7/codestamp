<script lang="ts">
  import { MAX_NAME_LENGTH } from '../format/names';

  interface Props {
    value: string;
    label: string;
    variant?: 'title' | 'section';
    onCommit: (next: string) => void;
  }

  let { value, label, variant = 'section', onCommit }: Props = $props();

  let text = $derived(value);

  function commit(): void {
    const next = text.trim();
    if (next === '' || next === value) {
      text = value;
      return;
    }
    onCommit(next);
  }

  function handleKey(event: KeyboardEvent & { currentTarget: HTMLInputElement }): void {
    if (event.key === 'Enter') {
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      text = value;
      event.currentTarget.blur();
    }
  }
</script>

<input
  class="editable {variant}"
  bind:value={text}
  aria-label={label}
  maxlength={MAX_NAME_LENGTH}
  spellcheck="false"
  onblur={commit}
  onkeydown={handleKey}
/>

<style>
  .editable {
    min-width: 0;
    padding: 2px 8px;
    margin-left: -8px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
  }

  .editable:hover {
    border-color: var(--border);
  }

  .editable:focus {
    border-color: var(--focus);
    background: var(--surface);
    outline: none;
  }

  .title {
    width: 100%;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .section {
    width: 100%;
    max-width: 360px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }
</style>
