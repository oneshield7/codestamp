<script lang="ts">
  import { prettyJson } from '../format/json';
  import { maskAny } from '../privacy/secrets';
  import { countActiveRows, type BodyMode, type RequestDraft } from '../request/draft';
  import { preferenceSettings } from '../state/preferences.svelte';
  import type { WebSocketSession } from '../state/websocket.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import KeyValueTable from './KeyValueTable.svelte';

  interface Props {
    draft: RequestDraft;
    session: WebSocketSession;
    collectionId: string;
    maskSecrets: boolean;
    onSend: () => void;
  }

  let { draft = $bindable(), session, collectionId, maskSecrets, onSend }: Props = $props();

  type ComposerTab = 'message' | 'params';
  const MESSAGE_MODES: { mode: BodyMode; label: string }[] = [
    { mode: 'text', label: 'Text' },
    { mode: 'json', label: 'JSON' },
  ];

  let activeTab: ComposerTab = $state('message');
  const paramsAndHeadersCount = $derived(
    countActiveRows(draft.params) + countActiveRows(draft.headers),
  );
  const isOpen = $derived(session.status === 'open');

  function updateMessage(next: string): void {
    draft.body = next;
  }

  function beautifyMessage(): void {
    draft.body = prettyJson(draft.body, preferenceSettings.jsonIndent) ?? draft.body;
  }
</script>

<section class="request" aria-label="Message">
  <div class="tabs" role="tablist" aria-label="WebSocket sections">
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'message'}
      class:on={activeTab === 'message'}
      onclick={() => (activeTab = 'message')}
    >
      Message
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'params'}
      class:on={activeTab === 'params'}
      onclick={() => (activeTab = 'params')}
    >
      Params &amp; Headers{paramsAndHeadersCount > 0 ? ` · ${String(paramsAndHeadersCount)}` : ''}
    </button>
  </div>

  {#if activeTab === 'params'}
    <div class="tab-body stack">
      <KeyValueTable
        title="Query parameters"
        bind:rows={draft.params}
        {collectionId}
        maskSensitive={maskSecrets}
      />
      <KeyValueTable
        title="Headers"
        bind:rows={draft.headers}
        {collectionId}
        maskSensitive={maskSecrets}
      />
      <p class="hint">Sent with the handshake. Changes apply on the next connect.</p>
    </div>
  {:else}
    <div class="tab-body stack">
      <div class="tools">
        <div class="segmented" role="group" aria-label="Message type">
          {#each MESSAGE_MODES as option (option.mode)}
            <button
              type="button"
              class:on={draft.bodyMode === option.mode}
              aria-pressed={draft.bodyMode === option.mode}
              onclick={() => (draft.bodyMode = option.mode)}>{option.label}</button
            >
          {/each}
        </div>
        {#if draft.bodyMode === 'json'}
          <button type="button" class="ghost" onclick={beautifyMessage}>Beautify</button>
        {/if}
        <span class="spacer"></span>
        <button type="button" class="send" disabled={!isOpen || draft.body === ''} onclick={onSend}>
          Send message
        </button>
      </div>
      {#if maskSecrets}
        <pre class="masked mono">{maskAny(draft.body, 2).text}</pre>
      {:else}
        <div class="editor">
          <CodeEditor
            value={draft.body}
            label="Message"
            language={draft.bodyMode === 'json' ? 'json' : 'text'}
            variablesOf={collectionId}
            onChange={updateMessage}
          />
        </div>
      {/if}
      <p class="hint">
        {isOpen
          ? '{{variables}} are filled in when the message is sent.'
          : 'Connect first, then send messages.'}
      </p>
    </div>
  {/if}
</section>

<style>
  .tabs {
    display: flex;
    gap: 16px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border-soft);
    overflow-x: auto;
  }

  .tabs button {
    padding: 10px 2px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    white-space: nowrap;
  }

  .tabs button.on {
    color: var(--text);
    border-bottom-color: var(--text);
    font-weight: 500;
  }

  .tab-body {
    padding: 14px 16px 16px;
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .spacer {
    flex: 1;
  }

  .segmented {
    display: inline-flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .segmented button {
    padding: 5px 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
  }

  .segmented button.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 500;
    box-shadow: var(--shadow-segment);
  }

  .ghost {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .send {
    height: 34px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .send:disabled {
    opacity: 0.5;
  }

  .editor {
    height: 180px;
    min-height: 100px;
    resize: vertical;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    padding: 4px;
  }

  .masked {
    margin: 0;
    max-height: 180px;
    overflow: auto;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
