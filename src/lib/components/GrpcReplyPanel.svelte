<script lang="ts">
  import type { NameValue } from '../api/http';
  import { prettyJson } from '../format/json';
  import { plural } from '../format/plural';
  import { formatDuration } from '../format/units';
  import { isSensitiveName, maskAny, maskSecret } from '../privacy/secrets';
  import type { GrpcCall } from '../state/grpc.svelte';
  import CodeEditor from './CodeEditor.svelte';

  interface Props {
    call: GrpcCall;
    /** Hides tokens on screen while presenting; the reply itself is unchanged. */
    maskSecrets: boolean;
  }

  let { call, maskSecrets }: Props = $props();

  type ReplyTab = 'messages' | 'headers' | 'trailers';
  let activeTab = $state<ReplyTab>('messages');

  const reply = $derived(call.reply);
  const ok = $derived(reply?.code === 0);

  function shown(text: string): string {
    const pretty = prettyJson(text) ?? text;
    return maskSecrets ? maskAny(pretty, 2).text : pretty;
  }

  function value(row: NameValue): string {
    return maskSecrets && isSensitiveName(row.name) ? maskSecret(row.value) : row.value;
  }
</script>

<section class="response" aria-label="Response" aria-busy={call.invoking}>
  <div class="summary">
    <span class="label">Response</span>
    {#if call.invoking}
      <span class="muted">Calling…</span>
    {:else if call.error}
      <span class="chip err">Failed</span>
    {:else if reply}
      <span class="chip" class:ok class:err={!ok}>{reply.code} {reply.status}</span>
      <span class="muted"
        >{formatDuration(reply.elapsedMs)} · {plural(reply.messages.length, 'message')}</span
      >
    {:else}
      <span class="muted">Choose a method, then press Invoke.</span>
    {/if}
  </div>

  {#if call.error && !call.invoking}
    <p class="error" role="alert">{maskSecrets ? maskAny(call.error, 0).text : call.error}</p>
  {:else if reply && !call.invoking}
    {#if !ok && reply.message}
      <p class="error" role="alert">
        {maskSecrets ? maskAny(reply.message, 0).text : reply.message}
      </p>
    {/if}
    <div class="tabs" role="tablist" aria-label="Response sections">
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === 'messages'}
        class:on={activeTab === 'messages'}
        onclick={() => (activeTab = 'messages')}>Messages · {reply.messages.length}</button
      >
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === 'headers'}
        class:on={activeTab === 'headers'}
        onclick={() => (activeTab = 'headers')}>Headers · {reply.headers.length}</button
      >
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === 'trailers'}
        class:on={activeTab === 'trailers'}
        onclick={() => (activeTab = 'trailers')}>Trailers · {reply.trailers.length}</button
      >
    </div>

    {#if reply.warning}
      <p class="notice">{reply.warning}</p>
    {/if}

    {#if activeTab === 'messages'}
      {#if reply.messages.length === 1}
        <div class="viewer">
          <CodeEditor
            value={shown(reply.messages[0] ?? '')}
            label="Response message"
            language="json"
            readOnly
          />
        </div>
      {:else}
        <ol class="messages mono" aria-label="Messages">
          {#each reply.messages as message, index (index)}
            <li>
              <span class="index">#{index + 1}</span>
              <pre>{shown(message)}</pre>
            </li>
          {:else}
            <li class="empty">No messages</li>
          {/each}
        </ol>
      {/if}
    {:else}
      {@const rows = activeTab === 'headers' ? reply.headers : reply.trailers}
      <dl class="rows mono">
        {#each rows as row, index (index)}
          <dt>{row.name}</dt>
          <dd>{value(row)}</dd>
        {:else}
          <dt class="empty">None</dt>
        {/each}
      </dl>
    {/if}
  {/if}
</section>

<style>
  .response {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface-muted);
  }

  .summary {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
  }

  .label {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .muted {
    font-size: 13px;
    color: var(--text-muted);
  }

  .chip {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
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

  .error {
    margin: 0 16px 12px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .notice {
    margin: 10px 16px 0;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: var(--warn-bg);
    color: var(--warn);
    font-size: 13px;
  }

  .tabs {
    display: flex;
    gap: 16px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .tabs button {
    padding: 8px 2px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
  }

  .tabs button.on {
    color: var(--text);
    border-bottom-color: var(--text);
    font-weight: 500;
  }

  .viewer {
    height: 360px;
    min-height: 140px;
    max-height: 80vh;
    resize: vertical;
    overflow: hidden;
    padding: 6px 4px;
  }

  .messages {
    margin: 0;
    padding: 10px 16px 16px;
    list-style: none;
    max-height: 420px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .messages li {
    display: flex;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .messages .index {
    flex: none;
    color: var(--text-muted);
    font-size: 12px;
  }

  .messages pre {
    margin: 0;
    min-width: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 12px;
  }

  .empty {
    color: var(--text-muted);
    font-size: 13px;
  }

  .rows {
    display: grid;
    grid-template-columns: minmax(140px, max-content) 1fr;
    gap: 6px 16px;
    margin: 0;
    padding: 12px 16px 16px;
    max-height: 360px;
    overflow: auto;
  }

  .rows dt {
    color: var(--text-muted);
  }

  .rows dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
