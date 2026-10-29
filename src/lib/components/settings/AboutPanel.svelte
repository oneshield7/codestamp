<script lang="ts">
  import { onMount } from 'svelte';
  import { aboutApp, type AboutInfo } from '../../api/preferences';
  import { formatDateTime } from '../../format/time';
  import { formatBytes } from '../../format/units';
  import { notices } from '../../state/notices.svelte';
  import type { Updates } from '../../state/updates.svelte';
  import ShortcutList from '../ShortcutList.svelte';

  interface Props {
    updates: Updates;
    onInstallUpdate: () => void;
  }

  let { updates, onInstallUpdate }: Props = $props();

  let about: AboutInfo | null = $state(null);
  const update = $derived(updates.available);
  const published = $derived.by(() => {
    const time = update?.date ? Date.parse(update.date) : Number.NaN;
    return Number.isNaN(time) ? null : formatDateTime(time);
  });
  let copied = $state(false);

  onMount(() => {
    void notices.guard(async () => {
      about = await aboutApp();
    });
  });

  async function copyFolder(): Promise<void> {
    if (!about) {
      return;
    }
    await notices.guard(async () => {
      await navigator.clipboard.writeText(about?.dataDir ?? '');
      copied = true;
    });
  }
</script>

<section class="panel" aria-labelledby="shortcuts-title">
  <h2 id="shortcuts-title">Keyboard shortcuts</h2>
  <ShortcutList />
</section>

<section class="panel" aria-labelledby="about-title">
  <h2 id="about-title">About Codestamp</h2>
  {#if about}
    <dl class="facts">
      <dt>Version</dt>
      <dd class="mono">{about.version}</dd>
      <dt>Data folder</dt>
      <dd>
        <span class="mono path">{about.dataDir}</span>
        <button type="button" class="button small" onclick={() => void copyFolder()}>
          {copied ? 'Copied' : 'Copy'}
        </button>
      </dd>
      <dt>Database</dt>
      <dd>{formatBytes(about.databaseBytes)}</dd>
    </dl>
  {/if}
  <p class="hint">
    Everything stays on this device; Codestamp sends no usage data{updates.result?.kind ===
    'notConfigured'
      ? ''
      : ' and contacts the update server only to look for a newer version'}. Fonts: Geist and Geist
    Mono (SIL Open Font License).
  </p>
</section>

<section class="panel" aria-labelledby="updates-title">
  <h2 id="updates-title">Updates</h2>
  {#if updates.result?.kind === 'notConfigured'}
    <p class="hint">This build of Codestamp is not set up to receive updates.</p>
  {:else}
    {#if update}
      <p class="news">
        Codestamp {update.version} is available. You have {update.current}.{published
          ? ` Published ${published}.`
          : ''}
      </p>
      {#if update.notes}
        <div class="notes" aria-label="What's new in {update.version}">{update.notes}</div>
      {/if}
    {:else if updates.result?.kind === 'upToDate'}
      <p class="status">You have the latest version, {updates.result.current}.</p>
    {:else}
      <p class="hint">Codestamp looks for a newer version each time it starts.</p>
    {/if}
    {#if updates.checkError}
      <p class="error" role="alert">Could not check for updates. {updates.checkError}</p>
    {/if}
    {#if updates.installError && !updates.installing}
      <p class="error" role="alert">{updates.installError}</p>
    {/if}
    <div class="actions">
      {#if update}
        <button
          type="button"
          class="primary"
          disabled={updates.installing}
          onclick={onInstallUpdate}
        >
          {updates.installing ? 'Updating…' : 'Update and restart'}
        </button>
      {/if}
      <button
        type="button"
        class="button"
        disabled={updates.checking || updates.installing}
        onclick={() => void updates.check()}
      >
        {updates.checking ? 'Checking…' : 'Check for updates'}
      </button>
    </div>
  {/if}
</section>

<style>
  .facts {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 10px 18px;
    font-size: 13px;
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .path {
    overflow-wrap: anywhere;
  }

  .small {
    height: 28px;
    padding: 0 10px;
  }

  .news {
    margin: 0;
    font-size: 13px;
  }

  .notes {
    max-height: 240px;
    overflow-y: auto;
    padding: 10px 12px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    font-size: 13px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
