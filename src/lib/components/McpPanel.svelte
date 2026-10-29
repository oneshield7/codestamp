<script lang="ts">
  import { tick } from 'svelte';
  import type { McpOperationKind } from '../api/collections';
  import { parseJson, prettyJson } from '../format/json';
  import { maskAny } from '../privacy/secrets';
  import { createRow, type RequestDraft } from '../request/draft';
  import { parseServerConfig } from '../request/mcp-config';
  import { argumentTemplate, offers, text as asText } from '../request/mcp-items';
  import type { McpConnection, McpItem } from '../state/mcp.svelte';
  import { preferenceSettings } from '../state/preferences.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import KeyValueTable from './KeyValueTable.svelte';
  import McpApprovalDialog from './McpApprovalDialog.svelte';

  interface Props {
    draft: RequestDraft;
    connection: McpConnection;
    collectionId: string;
    maskSecrets: boolean;
    onRun: () => void;
  }

  let { draft = $bindable(), connection, collectionId, maskSecrets, onRun }: Props = $props();

  type ServerTab = McpOperationKind | 'pairs' | 'config';
  type ResultTab = 'result' | 'messages';

  const KIND_LABELS = new Map<McpOperationKind, string>([
    ['tool', 'Tools'],
    ['prompt', 'Prompts'],
    ['resource', 'Resources'],
  ]);
  const SINGULAR = new Map<McpOperationKind, string>([
    ['tool', 'Tool'],
    ['prompt', 'Prompt'],
    ['resource', 'Resource'],
  ]);
  const RUN_LABELS = new Map<McpOperationKind, string>([
    ['tool', 'Run tool'],
    ['prompt', 'Get prompt'],
    ['resource', 'Read resource'],
  ]);
  const LOG_MARKS = new Map([
    ['sent', '↑'],
    ['received', '↓'],
    ['log', '·'],
    ['info', 'i'],
    ['error', '!'],
  ]);
  // Within this distance of the bottom, new messages keep the list scrolled down.
  const STICK_THRESHOLD_PX = 40;

  let serverTab = $state<ServerTab>('tool');
  let resultTab = $state<ResultTab>('result');
  let configText = $state('');
  let configError: string | null = $state(null);
  let showRaw = $state(false);
  let logList: HTMLOListElement | undefined = $state();

  const connected = $derived(connection.status === 'connected');
  const isHttp = $derived(draft.transport === 'HTTP');
  const kinds = $derived(
    (['tool', 'prompt', 'resource'] as const).filter((kind) =>
      connection.server ? offers(connection.server, kind) : kind === 'tool',
    ),
  );

  // Once connected, show what the server offers, starting with the saved operation's list.
  let wasConnected = false;
  $effect(() => {
    const now = connected;
    if (now && !wasConnected) {
      const saved = draft.mcpOperation?.kind;
      serverTab = saved !== undefined && kinds.includes(saved) ? saved : (kinds.at(0) ?? 'pairs');
    }
    wasConnected = now;
  });
  const listed = $derived(
    serverTab === 'pairs' || serverTab === 'config'
      ? []
      : connection.items.filter((item) => item.kind === serverTab),
  );
  const operation = $derived(draft.mcpOperation);
  const argumentsProblem = $derived.by(() => {
    if (!operation || operation.kind === 'resource' || draft.body.trim() === '') {
      return null;
    }
    // Variables are filled in by the Rust core, so they are checked as plain strings here.
    const withoutVariables = draft.body.replace(/\{\{[^}]*\}\}/g, '0');
    const parsed = parseJson(withoutVariables);
    return parsed !== null && typeof parsed.value === 'object' && !Array.isArray(parsed.value)
      ? null
      : 'The arguments must be a JSON object';
  });
  const reply = $derived(connection.last?.reply ?? null);
  const toolFailed = $derived(isRecord(reply?.result) && reply.result.isError === true);

  function count(kind: McpOperationKind): number {
    return connection.items.filter((item) => item.kind === kind).length;
  }

  function choose(item: McpItem): void {
    const changed = operation?.kind !== item.kind || operation.name !== item.name;
    draft.mcpOperation = { kind: item.kind, name: item.name };
    if (changed) {
      draft.body =
        item.kind === 'resource' ? '' : argumentTemplate(item, preferenceSettings.jsonIndent);
    }
  }

  function applyConfig(): void {
    const parsed = parseServerConfig(configText);
    if (!parsed) {
      configError = 'Paste a server with "command" (and "args", "env") or "url" (and "headers")';
      return;
    }
    draft.transport = parsed.transport;
    draft.url = parsed.target;
    draft.headers = [
      ...parsed.pairs.map((pair) => ({ ...createRow(), key: pair.key, value: pair.value })),
      createRow(),
    ];
    configText = '';
    configError = null;
    serverTab = 'pairs';
  }

  /** `undefined` (no result) stringifies to nothing, which JSON.stringify's type hides. */
  function stringify(value: unknown): string {
    return value === undefined ? '' : JSON.stringify(value, null, 2);
  }

  function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
  }

  function shown(text: string): string {
    return maskSecrets ? maskAny(text, 0).text : text;
  }

  /** Text blocks of a tool result, a prompt's messages or a resource's contents. */
  function blocks(result: unknown): { label: string; text: string }[] {
    if (!isRecord(result)) {
      return [];
    }
    const items: unknown[] = Array.isArray(result.content)
      ? result.content
      : Array.isArray(result.contents)
        ? result.contents
        : Array.isArray(result.messages)
          ? result.messages.map((message: unknown) =>
              isRecord(message) ? { ...asRecordOrEmpty(message.content), role: message.role } : {},
            )
          : [];
    const found = items.map((item) => describe(isRecord(item) ? item : {}));
    if (isRecord(result.structuredContent)) {
      found.push({
        label: 'Structured content',
        text: JSON.stringify(result.structuredContent, null, 2),
      });
    }
    return found;
  }

  function asRecordOrEmpty(value: unknown): Record<string, unknown> {
    return isRecord(value) ? value : {};
  }

  function describe(item: Record<string, unknown>): { label: string; text: string } {
    const role = typeof item.role === 'string' ? `${item.role} · ` : '';
    if (typeof item.text === 'string') {
      const label = typeof item.uri === 'string' ? item.uri : 'Text';
      return { label: role + label, text: prettyJson(item.text) ?? item.text };
    }
    const mime = typeof item.mimeType === 'string' ? item.mimeType : 'binary';
    if (typeof item.data === 'string' || typeof item.blob === 'string') {
      const encoded = typeof item.data === 'string' ? item.data : asText(item.blob);
      const bytes = Math.floor((encoded.length * 3) / 4);
      return {
        label: `${role}${asText(item.type) || 'Data'} · ${mime}`,
        text: `${String(bytes)} bytes`,
      };
    }
    return { label: `${role}${asText(item.type) || 'Item'}`, text: JSON.stringify(item, null, 2) };
  }

  $effect.pre(() => {
    const total = connection.log.length;
    const element = logList;
    if (!element || total === 0) {
      return;
    }
    const nearBottom =
      element.scrollHeight - element.scrollTop - element.clientHeight < STICK_THRESHOLD_PX;
    if (nearBottom) {
      void tick().then(() => {
        element.scrollTop = element.scrollHeight;
      });
    }
  });
</script>

<section class="request" aria-label="MCP server">
  {#if connection.server}
    <div class="server">
      <span class="server-name">{connection.server.name || 'MCP server'}</span>
      <span class="muted mono"
        >{connection.server.version} · MCP {connection.server.protocolVersion}</span
      >
      {#if connection.server.instructions}
        <p class="instructions">{connection.server.instructions}</p>
      {/if}
    </div>
  {/if}
  <div class="tabs" role="tablist" aria-label="MCP sections">
    {#each kinds as kind (kind)}
      <button
        type="button"
        role="tab"
        aria-selected={serverTab === kind}
        class:on={serverTab === kind}
        onclick={() => (serverTab = kind)}
      >
        {KIND_LABELS.get(kind)}{connected ? ` · ${String(count(kind))}` : ''}
      </button>
    {/each}
    <button
      type="button"
      role="tab"
      aria-selected={serverTab === 'pairs'}
      class:on={serverTab === 'pairs'}
      onclick={() => (serverTab = 'pairs')}
    >
      {isHttp ? 'Headers' : 'Environment'}
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={serverTab === 'config'}
      class:on={serverTab === 'config'}
      onclick={() => (serverTab = 'config')}
    >
      Paste config
    </button>
  </div>

  <div class="tab-body stack">
    {#if serverTab === 'pairs'}
      <KeyValueTable
        title={isHttp ? 'Headers' : 'Environment variables'}
        bind:rows={draft.headers}
        {collectionId}
        maskSensitive={maskSecrets}
      />
      <p class="hint">
        {isHttp
          ? 'Sent with every message, for example Authorization. Changes apply on the next connect.'
          : 'Given to the program when it starts, for example API keys. {{variables}} are filled in.'}
      </p>
    {:else if serverTab === 'config'}
      <p class="hint">
        Paste a server from another MCP client’s config, such as
        <span class="mono">{'{"mcpServers": {…}}'}</span>.
      </p>
      <textarea
        class="config mono"
        bind:value={configText}
        aria-label="Server config"
        spellcheck="false"
        placeholder={'{ "command": "npx", "args": ["-y", "@modelcontextprotocol/server-everything"] }'}
      ></textarea>
      {#if configError}
        <p class="problem" role="alert">{configError}</p>
      {/if}
      <div>
        <button
          type="button"
          class="ghost"
          disabled={configText.trim() === ''}
          onclick={applyConfig}
        >
          Use this server
        </button>
      </div>
    {:else}
      {#if !connected}
        <p class="hint">
          {operation
            ? `Saved to call the ${operation.kind} “${operation.name}”. Connect to run it.`
            : 'Connect to see the tools, prompts and resources this server offers.'}
        </p>
      {:else if listed.length === 0}
        <p class="hint">This server offers no {KIND_LABELS.get(serverTab)?.toLowerCase()}.</p>
      {:else}
        <ul class="items" aria-label={KIND_LABELS.get(serverTab)}>
          {#each listed as item (item.name)}
            <li>
              <button
                type="button"
                class="item"
                class:on={operation?.kind === item.kind && operation.name === item.name}
                aria-pressed={operation?.kind === item.kind && operation.name === item.name}
                onclick={() => {
                  choose(item);
                }}
              >
                <span class="item-name mono">{item.title}</span>
                {#if item.description}
                  <span class="item-text">{item.description}</span>
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if operation && operation.kind === serverTab}
        <div class="call">
          <div class="call-head">
            <span class="call-title">
              {SINGULAR.get(operation.kind)}
              <span class="mono">{operation.name}</span>
            </span>
            <span class="spacer"></span>
            <button
              type="button"
              class="run"
              disabled={!connected || connection.calling || argumentsProblem !== null}
              onclick={onRun}
            >
              {connection.calling ? 'Running…' : RUN_LABELS.get(operation.kind)}
            </button>
          </div>
          {#if operation.kind !== 'resource'}
            {#if maskSecrets}
              <pre class="masked mono">{maskAny(draft.body, 2).text}</pre>
            {:else}
              <div class="editor">
                <CodeEditor
                  value={draft.body}
                  label="Arguments"
                  language="json"
                  variablesOf={collectionId}
                  onChange={(next: string) => (draft.body = next)}
                />
              </div>
            {/if}
            {#if argumentsProblem}
              <p class="problem" role="alert">{argumentsProblem}</p>
            {:else}
              <p class="hint">Arguments (JSON). {'{{variables}}'} are filled in when it runs.</p>
            {/if}
          {/if}
        </div>
      {/if}
    {/if}
  </div>
</section>

<section class="response" aria-label="MCP result">
  <div class="tabs" role="tablist" aria-label="Result sections">
    <button
      type="button"
      role="tab"
      aria-selected={resultTab === 'result'}
      class:on={resultTab === 'result'}
      onclick={() => (resultTab = 'result')}
    >
      Result
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={resultTab === 'messages'}
      class:on={resultTab === 'messages'}
      onclick={() => (resultTab = 'messages')}
    >
      Messages · {connection.log.length}
    </button>
  </div>

  {#if resultTab === 'result'}
    <div class="tab-body stack">
      {#if connection.error}
        <p class="problem" role="alert">{connection.error}</p>
      {/if}
      {#if !reply}
        <p class="hint">Run a tool, get a prompt or read a resource to see the result here.</p>
      {:else}
        <div class="summary">
          {#if reply.error}
            <span class="chip err">Error {reply.error.code}</span>
            <span class="summary-text">{reply.error.message}</span>
          {:else if toolFailed}
            <span class="chip err">Tool error</span>
          {:else}
            <span class="chip ok">Result</span>
          {/if}
          <span class="spacer"></span>
          <span class="muted mono">{reply.elapsedMs} ms</span>
          <button type="button" class="ghost small" onclick={() => (showRaw = !showRaw)}>
            {showRaw ? 'Readable' : 'Raw JSON'}
          </button>
        </div>
        {#if showRaw || reply.error}
          <pre class="block mono">{shown(stringify(reply.error ?? reply.result))}</pre>
        {:else}
          {#each blocks(reply.result) as block, index (index)}
            <div class="part">
              <span class="part-label">{block.label}</span>
              <pre class="block mono">{shown(block.text)}</pre>
            </div>
          {:else}
            <pre class="block mono">{shown(stringify(reply.result))}</pre>
          {/each}
        {/if}
      {/if}
    </div>
  {:else}
    {#if connection.log.length === 0}
      <p class="hint pad">Messages to and from the server appear here.</p>
    {/if}
    <ol class="messages" bind:this={logList} aria-label="Messages">
      {#each connection.log as entry (entry.id)}
        <li class={entry.kind}>
          <span class="mark" aria-label={entry.kind}>{LOG_MARKS.get(entry.kind)}</span>
          <span class="text mono">{shown(entry.text)}{entry.truncated ? ' …' : ''}</span>
        </li>
      {/each}
    </ol>
  {/if}
</section>

{#if connection.launch}
  <McpApprovalDialog
    launch={connection.launch}
    onApprove={() => void connection.approve()}
    onCancel={() => {
      connection.cancelApproval();
    }}
  />
{/if}

<style>
  .server {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    padding: 12px 16px 0;
  }

  .server-name {
    font-weight: 600;
  }

  .instructions {
    flex-basis: 100%;
    margin: 2px 0 0;
    max-height: 4.5em;
    overflow: auto;
    font-size: 12px;
    color: var(--text-muted);
  }

  .muted {
    font-size: 12px;
    color: var(--text-muted);
  }

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

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .pad {
    padding: 14px 16px 0;
  }

  .problem {
    margin: 0;
    font-size: 12px;
    color: var(--err);
  }

  .config {
    min-height: 110px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    resize: vertical;
  }

  .items {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 260px;
    overflow: auto;
  }

  .item {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    text-align: left;
  }

  .item:hover {
    border-color: var(--border-strong);
  }

  .item.on {
    border-color: var(--method-mcp-line);
    background: var(--method-mcp-bg);
  }

  .item-name {
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .item.on .item-name {
    color: var(--method-mcp);
  }

  .item-text {
    font-size: 12px;
    color: var(--text-muted);
  }

  .call {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 4px;
  }

  .call-head,
  .summary {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .call-title {
    font-size: 13px;
    color: var(--text-2);
    text-transform: capitalize;
  }

  .call-title .mono {
    text-transform: none;
    color: var(--text);
  }

  .spacer {
    flex: 1;
  }

  .run {
    height: 34px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--method-mcp);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .run:disabled {
    opacity: 0.5;
  }

  .run:active:not(:disabled) {
    filter: brightness(0.9);
  }

  .ghost {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .ghost.small {
    height: 28px;
    font-size: 12px;
  }

  .ghost:active {
    background: var(--accent-soft);
  }

  .editor {
    height: 180px;
    min-height: 100px;
    resize: vertical;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .masked,
  .block {
    margin: 0;
    padding: 10px 12px;
    max-height: 360px;
    overflow: auto;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .chip {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .summary-text {
    font-size: 13px;
    color: var(--err);
  }

  .part {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .part-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  /* The log scrolls inside this box; drag the corner to resize it. */
  .messages {
    height: 320px;
    min-height: 120px;
    max-height: 80vh;
    resize: vertical;
    overflow: auto;
    margin: 0;
    padding: 6px 16px 12px;
    list-style: none;
  }

  .messages li {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border-soft);
  }

  .mark {
    flex: none;
    width: 14px;
    font-weight: 700;
    text-align: center;
  }

  .text {
    min-width: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .sent .mark {
    color: var(--method-mcp);
  }

  .received .mark {
    color: var(--ok);
  }

  .info .mark,
  .info .text,
  .log .mark,
  .log .text {
    color: var(--text-muted);
  }

  .error .mark,
  .error .text {
    color: var(--err);
  }
</style>
