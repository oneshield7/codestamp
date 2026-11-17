<script lang="ts">
  import { notices } from '../state/notices.svelte';

  interface Props {
    command: string;
  }

  let { command }: Props = $props();

  let copied = $state(false);

  async function copy(): Promise<void> {
    await notices.guard(async () => {
      await navigator.clipboard.writeText(command);
      copied = true;
    });
  }
</script>

<span class="command">
  <code class="mono">{command}</code>
  <button type="button" onclick={() => void copy()}>{copied ? 'Copied' : 'Copy command'}</button>
</span>

<style>
  .command {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  code {
    padding: 2px 6px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-sm);
    background: var(--surface-muted);
    color: var(--text);
    overflow-wrap: anywhere;
  }

  button {
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
  }

  button:hover {
    border-color: var(--text-2);
    color: var(--text);
  }

  button:active {
    background: var(--surface-sunken);
  }
</style>
