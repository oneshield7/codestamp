<script lang="ts">
  import { formatBytes } from '../format/units';
  import type { Updates } from '../state/updates.svelte';

  interface Props {
    updates: Updates;
    onInstall: () => void;
    onShowNotes: () => void;
  }

  let { updates, onInstall, onShowNotes }: Props = $props();

  const update = $derived(updates.available);
  const progress = $derived(updates.progress);
  const percent = $derived(
    progress?.total ? Math.min(100, Math.floor((progress.received / progress.total) * 100)) : null,
  );
  const downloaded = $derived(
    typeof progress?.total === 'number' && progress.received >= progress.total,
  );
</script>

{#if update && (updates.installing || updates.installError || !updates.dismissed)}
  <div
    class="update"
    class:error={!!updates.installError && !updates.installing}
    role={updates.installError && !updates.installing ? 'alert' : 'status'}
  >
    {#if updates.installing}
      <span>
        {#if downloaded}
          Starting the installer. Codestamp will close and open again in a moment.
        {:else if percent !== null}
          Downloading Codestamp {update.version}: {percent}%
        {:else if progress?.received}
          Downloading Codestamp {update.version}: {formatBytes(progress.received)}
        {:else}
          Downloading Codestamp {update.version}…
        {/if}
      </span>
      <progress
        max={progress?.total ?? undefined}
        value={progress?.total ? progress.received : undefined}
        aria-label="Download progress"
      ></progress>
    {:else if updates.installError}
      <span>{updates.installError}</span>
      <button type="button" onclick={onInstall}>Try again</button>
      <button
        type="button"
        onclick={() => {
          updates.installError = null;
          updates.dismissed = true;
        }}>Later</button
      >
    {:else}
      <span>Codestamp {update.version} is available. You have {update.current}.</span>
      {#if update.notes}
        <button type="button" onclick={onShowNotes}>What's new</button>
      {/if}
      <button type="button" onclick={onInstall}>Update and restart</button>
      <button
        type="button"
        onclick={() => {
          updates.dismissed = true;
        }}>Later</button
      >
    {/if}
  </div>
{/if}

<style>
  .update {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 10px 14px;
    border: 1px solid var(--accent-soft);
    border-radius: var(--radius-md);
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 13px;
  }

  .update.error {
    border-color: var(--err-line);
    background: var(--err-bg);
    color: var(--err);
  }

  span {
    flex: 1 1 260px;
  }

  button {
    border: 0;
    background: transparent;
    color: inherit;
    font-weight: 600;
  }

  progress {
    flex: 1 1 160px;
    height: 6px;
    border: 0;
    border-radius: 3px;
    overflow: hidden;
    background: var(--surface);
    accent-color: var(--accent);
  }

  progress::-webkit-progress-bar {
    background: var(--surface);
  }

  progress::-webkit-progress-value {
    background: var(--accent);
  }
</style>
