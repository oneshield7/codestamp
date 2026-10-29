<script lang="ts">
  import { SETTINGS_SECTIONS, type SettingsSection } from '../state/navigation';
  import type { Updates } from '../state/updates.svelte';
  import NavIcon from './NavIcon.svelte';
  import AboutPanel from './settings/AboutPanel.svelte';
  import AppearancePanel from './settings/AppearancePanel.svelte';
  import DataPanel from './settings/DataPanel.svelte';
  import EditorPanel from './settings/EditorPanel.svelte';
  import GitAccountsPanel from './settings/GitAccountsPanel.svelte';
  import ProxyPanel from './settings/ProxyPanel.svelte';
  import RequestsPanel from './settings/RequestsPanel.svelte';
  import TlsPanel from './settings/TlsPanel.svelte';

  interface Props {
    section?: SettingsSection;
    updates: Updates;
    onInstallUpdate: () => void;
  }

  let { section = $bindable('appearance'), updates, onInstallUpdate }: Props = $props();
</script>

<div class="view">
  <header>
    <h1 class="page-title"><NavIcon name="settings" size="lg" />Settings</h1>
    <p>Saved on this device and applied right away.</p>
  </header>

  <div class="layout">
    <nav class="sections" aria-label="Settings sections">
      {#each SETTINGS_SECTIONS as item (item.key)}
        <button
          type="button"
          class="section"
          class:on={section === item.key}
          aria-current={section === item.key ? 'page' : undefined}
          onclick={() => (section = item.key)}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true" data-hue={item.hue}>
            <path d={item.icon} />
          </svg>
          {item.label}
        </button>
      {/each}
    </nav>

    <div class="content">
      {#if section === 'appearance'}
        <AppearancePanel />
      {:else if section === 'editor'}
        <EditorPanel />
      {:else if section === 'requests'}
        <RequestsPanel onOpenTls={() => (section = 'tls')} />
      {:else if section === 'proxy'}
        <ProxyPanel />
      {:else if section === 'tls'}
        <TlsPanel />
      {:else if section === 'git'}
        <GitAccountsPanel />
      {:else if section === 'data'}
        <DataPanel />
      {:else}
        <AboutPanel {updates} {onInstallUpdate} />
      {/if}
    </div>
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  header p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .layout {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 18px;
  }

  .sections {
    flex: 0 0 200px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    position: sticky;
    top: 0;
  }

  .section {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 12px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }

  .section:hover {
    background: var(--surface);
    color: var(--text);
  }

  .section.on {
    border-color: var(--border);
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    box-shadow: var(--shadow-raise);
  }

  .section:active {
    background: var(--accent-soft);
  }

  .section svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .content {
    flex: 1 1 520px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* Shared by every settings panel. */
  .content :global(.panel) {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .content :global(h2) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .content :global(.hint) {
    margin: -8px 0 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .content :global(.fields) {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 14px;
  }

  .content :global(.field) {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .content :global(.field.wide) {
    flex: 3 1 240px;
  }

  .content :global(.input) {
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 14px;
  }

  .content :global(.input:focus) {
    border-color: var(--focus);
    outline: none;
  }

  .content :global(.check) {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-2);
  }

  .content :global(.check input) {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }

  .content :global(.button),
  .content :global(.primary) {
    height: 36px;
    padding: 0 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .content :global(.button:hover) {
    border-color: var(--text-muted);
  }

  .content :global(.button:active),
  .content :global(.button[aria-expanded='true']) {
    border-color: var(--text);
    background: var(--accent-soft);
  }

  .content :global(.primary) {
    border-color: transparent;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .content :global(.primary:disabled),
  .content :global(.button:disabled) {
    opacity: 0.55;
  }

  .content :global(.actions) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .content :global(.status) {
    margin: 0;
    font-size: 13px;
    color: var(--ok);
  }

  .content :global(.error) {
    margin: 0;
    padding: 10px 12px;
    border: 1px solid var(--err-line);
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .content :global(.segmented) {
    display: inline-flex;
    align-self: flex-start;
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
  }

  .content :global(.segmented label) {
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    cursor: pointer;
  }

  .content :global(.segmented label.on) {
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    box-shadow: var(--shadow-segment);
  }

  .content :global(.segmented label:has(input:focus-visible)) {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
</style>
