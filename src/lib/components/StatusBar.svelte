<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';
  import { NAV_ICON_PATHS, navLabel, type NavPage, type View } from '../state/navigation';
  import { notices } from '../state/notices.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    view: View;
    onNavigate: (view: View) => void;
  }

  let { view, onNavigate }: Props = $props();

  const PLACES: readonly NavPage[] = ['settings', 'trash', 'cookies', 'environments'];

  let version = $state<string | null>(null);

  onMount(() => {
    void notices.guard(async () => {
      version = await getVersion();
    });
  });
</script>

<footer class="statusbar" aria-label="Status bar">
  <div class="places" role="group" aria-label="Settings and data">
    {#each PLACES as place, index (place)}
      <IconButton
        label={navLabel(place)}
        quiet
        placement="above"
        align={index === 0 ? 'start' : 'center'}
        current={view === place}
        onclick={() => {
          onNavigate(place);
        }}
      >
        <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
          <path d={NAV_ICON_PATHS.get(place)} />
        </svg>
      </IconButton>
    {/each}
  </div>
  {#if version}
    <span class="version">v{version}</span>
  {/if}
</footer>

<style>
  /* A thin line across the whole window, below the sidebar and the content. */
  .statusbar {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    height: var(--statusbar-height);
    padding: 0 12px 0 6px;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }

  .places {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .version {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
