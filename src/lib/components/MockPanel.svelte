<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { toAppError } from '../api/http';
  import {
    mockStart,
    mockStatus,
    mockStop,
    onMockHit,
    type MockHit,
    type MockStatus,
  } from '../api/mock';
  import { statusTone } from '../format/status';
  import { formatClock } from '../format/time';
  import PanelDialog from './PanelDialog.svelte';

  interface Props {
    collectionId: string;
    onStatus: (status: MockStatus | null) => void;
    onClose: () => void;
  }

  let { collectionId, onStatus, onClose }: Props = $props();

  const MAX_HITS = 30;
  const COPIED_MS = 1500;

  let status = $state<MockStatus | null>(null);
  let port = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let hits = $state<MockHit[]>([]);
  let copied = $state(false);
  let unlisten: UnlistenFn | undefined;

  onMount(() => {
    void refresh();
    void onMockHit((hit) => {
      if (hit.collectionId === collectionId) {
        hits = [hit, ...hits].slice(0, MAX_HITS);
      }
    }).then((stop) => {
      unlisten = stop;
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  async function refresh(): Promise<void> {
    try {
      status = await mockStatus(collectionId);
      onStatus(status);
    } catch (caught) {
      error = toAppError(caught).message;
    }
  }

  async function start(): Promise<void> {
    const chosen = port.trim() === '' ? 0 : Number(port);
    if (!Number.isInteger(chosen) || chosen < 0 || chosen > 65535) {
      error = 'Enter a port from 1024 to 65535, or leave it empty for any free one.';
      return;
    }
    busy = true;
    error = null;
    try {
      status = await mockStart(collectionId, chosen);
      onStatus(status);
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }

  async function stop(): Promise<void> {
    busy = true;
    error = null;
    try {
      await mockStop(collectionId);
      status = null;
      onStatus(null);
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }

  async function copy(url: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(url);
      copied = true;
      setTimeout(() => (copied = false), COPIED_MS);
    } catch {
      error = 'Could not copy here. Select the address and copy it instead.';
    }
  }
</script>

<PanelDialog title="Mock server" {onClose}>
  <section class="panel" aria-label="Mock server">
    <div class="head">
      <div class="title">
        <h2>Mock server</h2>
        <p>Answers with the saved examples of this collection, on this computer only.</p>
      </div>
      <button type="button" class="quiet" onclick={onClose}>Close</button>
    </div>
    <div class="body">
      {#if status}
        <div class="running" role="status">
          <span class="dot" aria-hidden="true"></span>
          <span>Running at <span class="mono url">{status.url}</span></span>
          <button type="button" class="link" onclick={() => void copy(status?.url ?? '')}>
            {copied ? 'Copied' : 'Copy'}
          </button>
          <span class="spacer"></span>
          <button type="button" class="secondary" disabled={busy} onclick={() => void stop()}>
            Stop
          </button>
        </div>
        <p class="hint">
          Point <span class="mono">{'{{baseUrl}}'}</span> at this address (in an environment, say)
          and send requests as usual. A request matches by method and path, where
          <span class="mono">:id</span> and <span class="mono">{'{{variables}}'}</span> match any
          segment. The first successful example answers unless the
          <span class="mono">x-mock-response-name</span> or
          <span class="mono">x-mock-response-code</span> header picks another.
        </p>
      {:else}
        <div class="start">
          <label class="field">
            Port
            <input
              class="mono"
              bind:value={port}
              inputmode="numeric"
              placeholder="Any free port"
              maxlength="5"
            />
          </label>
          <button type="button" class="primary" disabled={busy} onclick={() => void start()}>
            {busy ? 'Starting…' : 'Start mock server'}
          </button>
        </div>
        <p class="hint">
          Save responses as examples first (Save as example in a response). Importing an OpenAPI
          spec brings examples for every documented status.
        </p>
      {/if}
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
      {#if hits.length > 0}
        <ol class="hits mono" aria-label="Requests the mock answered">
          {#each hits as hit, index (index)}
            <li>
              <span class="time">{formatClock(hit.at, true)}</span>
              <span class="chip {statusTone(hit.status)}">{hit.status}</span>
              <span class="request">{hit.method} {hit.path}</span>
              <span class="example">{hit.example ?? 'no matching example'}</span>
            </li>
          {/each}
        </ol>
      {/if}
    </div>
  </section>
</PanelDialog>

<style>
  .panel {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .title {
    flex: 1 1 260px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .title p {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
  }

  .running {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 13px;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
  }

  .url {
    font-weight: 600;
  }

  .spacer {
    flex: 1;
  }

  .start {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .field input {
    width: 160px;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .hint {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .hits {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 220px;
    overflow: auto;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
    font-size: 12px;
  }

  .hits li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 5px 10px;
  }

  .hits li + li {
    border-top: 1px solid var(--border-soft);
  }

  .time,
  .example {
    color: var(--text-muted);
  }

  .request {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .chip {
    padding: 1px 7px;
    border-radius: 999px;
    font-weight: 600;
    background: var(--surface-sunken);
    color: var(--text-2);
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .primary,
  .secondary,
  .quiet {
    height: 32px;
    padding: 0 14px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }

  .primary {
    height: 36px;
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .secondary,
  .quiet {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-2);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font-size: 13px;
    font-weight: 600;
    text-decoration: underline;
  }
</style>
