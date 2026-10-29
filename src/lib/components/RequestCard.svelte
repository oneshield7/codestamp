<script lang="ts">
  import { isSaveShortcut } from '../dom/keys';
  import {
    sendHttpRequest,
    toAppError,
    type AppError,
    type HttpMethod,
    type HttpResponseOutput,
  } from '../api/http';
  import type { Auth, McpTransportName, SectionInfo } from '../api/collections';
  import { previewUrl, type UrlPreview } from '../api/environments';
  import { onDestroy } from 'svelte';
  import {
    buildMcpConnectInput,
    activePairs,
    buildGrpcTarget,
    buildRequestInput,
    buildWsConnectInput,
    countActiveRows,
    type RequestDraft,
  } from '../request/draft';
  import { maskQuery } from '../privacy/secrets';
  import { authLabel } from '../request/auth';
  import { bodyModeLabel } from '../request/body';
  import { MAX_NAME_LENGTH } from '../format/names';
  import { MCP_TRANSPORT_OPTIONS, METHOD_OPTIONS, methodTone, stampTone } from '../request/methods';
  import { pathVariableNames, pathVariableValues } from '../request/path-variables';
  import { appendSnippet, snippetsFor, type ScriptStage, type Snippet } from '../request/snippets';
  import { WebSocketSession, type ConnectionStatus } from '../state/websocket.svelte';
  import { McpConnection, type McpStatus } from '../state/mcp.svelte';
  import { accessGroups } from '../state/access.svelte';
  import { preferenceSettings } from '../state/preferences.svelte';
  import { variableRevision } from '../state/variable-revision.svelte';
  import type { SaveState } from '../state/workspace.svelte';
  import AuthEditor from './AuthEditor.svelte';
  import BodyEditor from './BodyEditor.svelte';
  import ExamplesPanel from './ExamplesPanel.svelte';
  import { saveExample } from '../api/examples';
  import { ExampleList } from '../state/examples.svelte';
  import { EventStreamView } from '../state/streams.svelte';
  import { GrpcCall } from '../state/grpc.svelte';
  import { WsdlLoader } from '../state/soap.svelte';
  import SoapPanel from './SoapPanel.svelte';
  import { notices } from '../state/notices.svelte';
  import GraphqlEditor from './GraphqlEditor.svelte';
  import GrpcPanel from './GrpcPanel.svelte';
  import GrpcReplyPanel from './GrpcReplyPanel.svelte';
  import CaptureDialog from './CaptureDialog.svelte';
  import CodeDialog from './CodeDialog.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import KeyValueTable from './KeyValueTable.svelte';
  import PathVariablesTable from './PathVariablesTable.svelte';
  import Dropdown from './Dropdown.svelte';
  import McpPanel from './McpPanel.svelte';
  import MessageLog from './MessageLog.svelte';
  import ResponsePanel from './ResponsePanel.svelte';
  import VariableInput from './VariableInput.svelte';
  import WebSocketComposer from './WebSocketComposer.svelte';

  interface Props {
    index: number;
    draft: RequestDraft;
    collectionId: string;
    requestId: string;
    sections: SectionInfo[];
    sectionId: string | null;
    saveState: SaveState;
    saveError: string | null;
    onEdit: () => void;
    onSave: () => void;
    onClose: () => void;
    onDelete: () => void;
    onDuplicate: () => void;
    /** The Duplicate dialog for this request is open. */
    duplicateOpen: boolean;
    /** Opens the request's history; null when the collection is not shared. */
    onHistory: (() => void) | null;
    /** The request's history dialog is open. */
    historyOpen: boolean;
    /** The request's delete confirmation is open. */
    deleteOpen: boolean;
    onMove: (sectionId: string | null) => void;
    onResult: (response: HttpResponseOutput) => void;
    /** Used when the request's auth is "Inherit from collection". */
    collectionAuth: Auth;
    /** Shown alone in focus mode, larger and with fewer controls. */
    focused: boolean;
    /** Hides tokens and cookies on screen while presenting. */
    maskSecrets: boolean;
    environmentName: string | null;
    onFocus: () => void;
    /** Saved examples changed, so collaboration has something to share. */
    onExamplesChanged: () => void;
  }

  let {
    index,
    draft = $bindable(),
    collectionId,
    requestId,
    sections,
    sectionId,
    saveState,
    saveError,
    onEdit,
    onSave,
    onClose,
    onDelete,
    onDuplicate,
    duplicateOpen,
    onHistory,
    historyOpen,
    deleteOpen,
    onMove,
    onResult,
    collectionAuth,
    focused,
    maskSecrets,
    environmentName,
    onFocus,
    onExamplesChanged,
  }: Props = $props();

  const examples = new ExampleList();
  $effect(() => {
    const id = requestId;
    void notices.guard(() => examples.load(id));
  });

  async function saveAsExample(): Promise<void> {
    const shown = response;
    if (!shown) {
      return;
    }
    await notices.guard(async () => {
      const name = `${String(shown.status)} ${shown.statusText}`.trim();
      await saveExample(requestId, {
        name,
        status: shown.status,
        headers: shown.headers,
        body: shown.bodyPreview,
      });
      await examples.refresh();
      onExamplesChanged();
      notices.inform(`Saved the response as the example “${name}”.`);
    });
  }

  let capturing = $state(false);
  let showingCode = $state(false);

  // Focus mode is mostly used for screenshots, where request and response read best side by side.
  function enterFocus(): void {
    layout = 'side';
    onFocus();
  }

  const NO_SECTION = '';
  const PREVIEW_DELAY_MS = 250;

  // Shows where `{{variables}}` and params lead, with secrets masked by the Rust core.
  // It is refreshed when saved variables or the active environment change, and an answer that
  // arrives after a newer edit is ignored.
  let preview: UrlPreview | null = $state(null);

  // A name whose value is restricted to a group this device is not in says so, so nobody hunts
  // for a typo.
  function unresolved(names: readonly string[]): string {
    return names
      .map((name) => {
        const group = accessGroups.lockedGroup(collectionId, name);
        return group === null ? `{{${name}}}` : `{{${name}}} (restricted to ${group})`;
      })
      .join(', ');
  }
  const previewInput = $derived({
    url: draft.url.trim(),
    params: activePairs(draft.params),
    pathVariables: pathVariableValues(draft.url, draft.pathVariables),
    revision: variableRevision.value,
  });
  $effect(() => {
    const { url, params, pathVariables } = previewInput;
    const plain = !url.includes('{{') && params.length === 0 && pathVariables.length === 0;
    // A command line is not a URL; its variables are filled in when the program starts.
    if (plain || (isMcp && draft.transport === 'STDIO')) {
      preview = null;
      return;
    }
    const run = { cancelled: false };
    const timer = setTimeout(() => {
      previewUrl(url, params, pathVariables, collectionId).then(
        (result) => {
          if (!run.cancelled) {
            preview = result;
          }
        },
        () => {
          if (!run.cancelled) {
            preview = null;
          }
        },
      );
    }, PREVIEW_DELAY_MS);
    return () => {
      run.cancelled = true;
      clearTimeout(timer);
    };
  });

  // While presenting, values that look secret are shown masked and read-only.
  const maskedUrl = $derived(maskSecrets ? maskQuery(draft.url) : null);
  const urlHidden = $derived(maskedUrl !== null && maskedUrl.hidden.length > 0);

  const sectionOptions = $derived([
    { value: NO_SECTION, label: 'No section' },
    ...sections.map((section) => ({ value: section.id, label: section.name })),
  ]);

  const SAVE_LABELS = new Map<SaveState, string>([
    ['saved', 'Saved'],
    ['unsaved', 'Unsaved changes'],
    ['saving', 'Saving…'],
    ['failed', 'Not saved'],
  ]);

  // Any change to the draft (after it first loads) schedules an autosave.
  let isFirstRun = true;
  $effect(() => {
    JSON.stringify(draft);
    if (isFirstRun) {
      isFirstRun = false;
      return;
    }
    onEdit();
  });

  type RequestTab = 'params' | 'body' | 'auth' | 'scripts' | 'examples';
  type Layout = 'stacked' | 'side';

  let activeTab: RequestTab = $state('params');
  let layout: Layout = $state(preferenceSettings.editor.layout);
  let sending = $state(false);
  let response: HttpResponseOutput | null = $state(null);
  let error: AppError | null = $state(null);

  const paramsAndHeadersCount = $derived(
    countActiveRows(draft.params) +
      countActiveRows(draft.headers) +
      pathVariableNames(draft.url).length,
  );

  // An open event stream of the last response; a new send or closing the card ends it.
  let stream = $state<EventStreamView | null>(null);
  onDestroy(() => {
    void stream?.stop();
  });

  async function send(): Promise<void> {
    if (sending) {
      return;
    }
    sending = true;
    error = null;
    await stream?.stop();
    stream = null;
    try {
      response = await sendHttpRequest(buildRequestInput(draft, { collectionId, requestId }));
      if (response.streamId !== null) {
        const opened = new EventStreamView(response.streamId);
        stream = opened;
        await opened.listen();
      }
      onResult(response);
    } catch (caught) {
      response = null;
      error = toAppError(caught);
    } finally {
      sending = false;
    }
  }

  // Only WebSocket cards use it; the connection closes together with the card.
  const session = new WebSocketSession();
  onDestroy(() => {
    session.dispose();
  });
  const isWebSocket = $derived(draft.kind === 'websocket');
  // Only MCP cards use it; the session (and a started program) ends together with the card.
  const mcp = new McpConnection();
  onDestroy(() => {
    mcp.dispose();
  });
  const isMcp = $derived(draft.kind === 'mcp');
  const isGraphql = $derived(draft.kind === 'graphql');
  // Only gRPC cards use it.
  const grpc = new GrpcCall();
  const isGrpc = $derived(draft.kind === 'grpc');
  // Only SOAP cards use it.
  const wsdl = new WsdlLoader();
  const isSoap = $derived(draft.kind === 'soap');
  const grpcTarget = $derived(buildGrpcTarget(draft, { collectionId, requestId }));

  function invokeGrpc(): void {
    if (draft.grpc.method !== '') {
      void grpc.invoke(grpcTarget, draft.grpc.method, draft.body);
    }
  }
  // Send carries the colour of the method it sends.
  const sendTone = $derived(stampTone(draft.kind, draft.method));
  const MCP_LABELS = new Map<McpStatus, string>([
    ['idle', 'Connect'],
    ['connecting', 'Connecting…'],
    ['connected', 'Disconnect'],
    ['closing', 'Disconnecting…'],
  ]);

  function toggleMcp(): void {
    if (mcp.status === 'idle') {
      void mcp.connect(buildMcpConnectInput(draft, collectionId));
    } else {
      void mcp.disconnect();
    }
  }

  function runMcp(): void {
    if (draft.mcpOperation) {
      void mcp.call(draft.mcpOperation, draft.body, collectionId);
    }
  }
  const CONNECT_LABELS = new Map<ConnectionStatus, string>([
    ['idle', 'Connect'],
    ['connecting', 'Connecting…'],
    ['open', 'Disconnect'],
    ['closing', 'Disconnecting…'],
  ]);

  function toggleConnection(): void {
    if (session.status === 'idle') {
      void session.connect(buildWsConnectInput(draft, collectionId));
    } else {
      void session.disconnect();
    }
  }

  function sendMessage(): void {
    void session.send(draft.body, collectionId);
  }

  // Ctrl+Enter sends the request, or for WebSocket connects first and then sends messages.
  function primaryAction(): void {
    if (isGrpc) {
      invokeGrpc();
    } else if (isMcp) {
      if (mcp.status === 'connected' && draft.mcpOperation) {
        runMcp();
      } else if (mcp.status === 'idle') {
        toggleMcp();
      }
    } else if (!isWebSocket) {
      void send();
    } else if (session.status === 'open') {
      // Same rule as the Send message button, which is disabled for an empty message.
      if (draft.body !== '') {
        sendMessage();
      }
    } else if (session.status === 'idle') {
      toggleConnection();
    }
  }

  function handleShortcut(event: KeyboardEvent): void {
    const withModifier = event.ctrlKey || event.metaKey;
    if (withModifier && event.key === 'Enter') {
      event.preventDefault();
      primaryAction();
    } else if (isSaveShortcut(event)) {
      event.preventDefault();
      onSave();
    }
  }

  let scriptStage: ScriptStage = $state('preRequestScript');
  const scriptCount = $derived(
    [draft.preRequestScript, draft.testScript].filter((script) => script.trim() !== '').length,
  );

  const currentScript = $derived(
    scriptStage === 'preRequestScript' ? draft.preRequestScript : draft.testScript,
  );

  function updateScript(next: string): void {
    if (scriptStage === 'preRequestScript') {
      draft.preRequestScript = next;
    } else {
      draft.testScript = next;
    }
  }

  function insertSnippet(snippet: Snippet): void {
    updateScript(appendSnippet(currentScript, snippet));
  }
</script>

<article
  class="card"
  class:focused
  data-card-id={requestId}
  tabindex="-1"
  aria-label={focused ? `${draft.name}, focus mode` : draft.name}
  onkeydowncapture={handleShortcut}
>
  <header class="head">
    {#if !focused}
      <span class="step">{index}</span>
    {/if}
    <label class="name">
      <span class="visually-hidden">Request name</span>
      <input bind:value={draft.name} maxlength={MAX_NAME_LENGTH} spellcheck="false" />
    </label>
    {#if !focused}
      <span class="save {saveState}" role="status" title={saveError ?? undefined}>
        {SAVE_LABELS.get(saveState)}{saveState === 'failed' && saveError ? `: ${saveError}` : ''}
      </span>
      {#if sections.length > 0}
        <Dropdown
          variant="chip"
          label="Section"
          value={sectionId ?? NO_SECTION}
          options={sectionOptions}
          onChange={(value: string) => {
            onMove(value === NO_SECTION ? null : value);
          }}
        />
      {/if}
    {/if}
    <div class="layout" role="group" aria-label="Card layout">
      <button
        type="button"
        class:on={layout === 'stacked'}
        aria-pressed={layout === 'stacked'}
        aria-label="Response below the request"
        onclick={() => (layout = 'stacked')}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="4" y="4" width="16" height="16" rx="2" /><path d="M4 12h16" />
        </svg>
      </button>
      <button
        type="button"
        class:on={layout === 'side'}
        aria-pressed={layout === 'side'}
        aria-label="Response beside the request"
        onclick={() => (layout = 'side')}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="4" y="4" width="16" height="16" rx="2" /><path d="M12 4v16" />
        </svg>
      </button>
    </div>
    {#if focused && !isMcp && !isGrpc}
      <button type="button" class="tool" onclick={() => (capturing = true)}>
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 8h3l2-3h6l2 3h3v11H4z" /><circle cx="12" cy="13" r="3" />
        </svg>
        Capture
      </button>
    {:else if !focused}
      {#if !isWebSocket && !isMcp && !isGrpc}
        <button
          type="button"
          class="icon"
          aria-label="Copy as code"
          title="Copy as code (cURL, fetch, Python)"
          aria-haspopup="dialog"
          aria-expanded={showingCode}
          onclick={() => (showingCode = true)}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M9 7l-5 5 5 5M15 7l5 5-5 5" />
          </svg>
        </button>
      {/if}
      {#if onHistory}
        <button
          type="button"
          class="icon"
          aria-label="Request history"
          title="History: earlier shared versions"
          aria-haspopup="dialog"
          aria-expanded={historyOpen}
          onclick={onHistory}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M3 12a9 9 0 1 0 3-6.7M3 4v4h4M12 7v5l3 2" />
          </svg>
        </button>
      {/if}
      <button
        type="button"
        class="icon"
        aria-label="Duplicate request"
        title="Duplicate request"
        aria-haspopup="dialog"
        aria-expanded={duplicateOpen}
        onclick={onDuplicate}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="8" y="8" width="12" height="12" rx="2" /><path
            d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"
          />
        </svg>
      </button>
      {#if !isMcp && !isGrpc}
        <button
          type="button"
          class="icon"
          aria-label="Capture an image"
          title="Capture an image"
          aria-haspopup="dialog"
          aria-expanded={capturing}
          onclick={() => (capturing = true)}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 8h3l2-3h6l2 3h3v11H4z" /><circle cx="12" cy="13" r="3" />
          </svg>
        </button>
      {/if}
      <button
        type="button"
        class="icon"
        aria-label="Focus mode"
        title="Focus mode"
        onclick={enterFocus}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5" />
        </svg>
      </button>
      <button
        type="button"
        class="icon"
        aria-label="Delete request"
        title="Delete request"
        aria-haspopup="dialog"
        aria-expanded={deleteOpen}
        onclick={onDelete}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
        </svg>
      </button>
      <button type="button" class="icon" aria-label="Collapse request" onclick={onClose}>
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 15l6-6 6 6" />
        </svg>
      </button>
    {/if}
  </header>

  <div class="target">
    <div class="url-field">
      {#if isWebSocket}
        <span class="method ws mono">WS</span>
      {:else if isGraphql}
        <span class="method gql mono" title="GraphQL is sent with POST">GQL</span>
      {:else if isGrpc}
        <span class="method grpc mono">gRPC</span>
      {:else if isSoap}
        <span class="method soap mono" title="SOAP {draft.soap.version} is sent with POST"
          >SOAP</span
        >
      {:else if isMcp}
        <Dropdown
          id="method-{index}"
          variant="method"
          label="MCP transport"
          value={draft.transport}
          options={MCP_TRANSPORT_OPTIONS}
          tint="var(--method-mcp-bg)"
          mono
          disabled={mcp.status !== 'idle'}
          onChange={(transport: McpTransportName) => (draft.transport = transport)}
        />
      {:else}
        <Dropdown
          id="method-{index}"
          variant="method"
          label="Method"
          value={draft.method}
          options={METHOD_OPTIONS}
          tint="var(--method-{methodTone(draft.method)}-bg)"
          mono
          onChange={(method: HttpMethod) => (draft.method = method)}
        />
      {/if}
      {#if urlHidden && maskedUrl}
        <span
          class="url-masked mono"
          title="Shown masked while “Hide secrets on screen” is on"
          aria-label={isWebSocket ? 'WebSocket URL, masked' : 'Request URL, masked'}
          >{maskedUrl.text}</span
        >
      {:else}
        <VariableInput
          id="url-{index}"
          bind:value={draft.url}
          label={isMcp
            ? draft.transport === 'HTTP'
              ? 'MCP server URL'
              : 'Command that starts the MCP server'
            : isWebSocket
              ? 'WebSocket URL'
              : isGraphql
                ? 'GraphQL endpoint'
                : isGrpc
                  ? 'gRPC server address'
                  : isSoap
                    ? 'SOAP endpoint'
                    : 'Request URL'}
          variant="url"
          placeholder={isMcp
            ? draft.transport === 'HTTP'
              ? 'https://…/mcp'
              : 'npx -y @modelcontextprotocol/server-everything'
            : isWebSocket
              ? '{{wsUrl}}/chat or wss://…'
              : isGraphql
                ? '{{baseUrl}}/graphql or https://…/graphql'
                : isGrpc
                  ? 'grpc://localhost:50051 or grpcs://api.example.com'
                  : isSoap
                    ? '{{baseUrl}}/Service.asmx or https://…'
                    : '{{baseUrl}}/v1/users or https://…'}
          {collectionId}
        />
      {/if}
    </div>
    {#if isMcp}
      <button
        type="button"
        class="send"
        style:--send-bg="var(--method-{sendTone})"
        class:connected={mcp.status === 'connected'}
        disabled={mcp.status === 'connecting' ||
          mcp.status === 'closing' ||
          draft.url.trim() === ''}
        onclick={toggleMcp}
      >
        {MCP_LABELS.get(mcp.status)}
      </button>
    {:else if isGrpc}
      <button
        type="button"
        class="send"
        style:--send-bg="var(--method-{sendTone})"
        disabled={grpc.invoking || draft.grpc.method === ''}
        title={draft.grpc.method === '' ? 'Load the services and choose a method first' : undefined}
        onclick={invokeGrpc}
      >
        {grpc.invoking ? 'Calling…' : 'Invoke'}
      </button>
    {:else if isWebSocket}
      <button
        type="button"
        class="send"
        style:--send-bg="var(--method-{sendTone})"
        class:connected={session.status === 'open'}
        disabled={session.status === 'connecting' || session.status === 'closing'}
        onclick={toggleConnection}
      >
        {CONNECT_LABELS.get(session.status)}
      </button>
    {:else}
      <button
        type="button"
        class="send"
        style:--send-bg="var(--method-{sendTone})"
        disabled={sending}
        onclick={() => void send()}
      >
        {sending ? 'Sending…' : 'Send'}
      </button>
    {/if}
  </div>
  {#if preview}
    <p class="preview mono" class:missing={preview.missing.length > 0}>
      {#if preview.missing.length > 0}
        Not defined: {unresolved(preview.missing)}
      {:else}
        Resolves to {maskSecrets ? maskQuery(preview.url).text : preview.url}
      {/if}
    </p>
  {/if}

  <div class="panes" class:side={layout === 'side'}>
    {#if isMcp}
      <McpPanel bind:draft connection={mcp} {collectionId} {maskSecrets} onRun={runMcp} />
    {:else if isGrpc}
      <GrpcPanel
        bind:draft
        call={grpc}
        target={grpcTarget}
        {collectionAuth}
        {collectionId}
        {requestId}
        {maskSecrets}
      />
      <GrpcReplyPanel call={grpc} {maskSecrets} />
    {:else if isWebSocket}
      <WebSocketComposer bind:draft {session} {collectionId} {maskSecrets} onSend={sendMessage} />
      <MessageLog {session} {maskSecrets} />
    {:else}
      <section class="request" aria-label="Request">
        <div class="tabs" role="tablist" aria-label="Request sections">
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'params'}
            class:on={activeTab === 'params'}
            onclick={() => (activeTab = 'params')}
          >
            Params &amp; Headers{paramsAndHeadersCount > 0
              ? ` · ${String(paramsAndHeadersCount)}`
              : ''}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'body'}
            class:on={activeTab === 'body'}
            onclick={() => (activeTab = 'body')}
          >
            {#if isGraphql}
              Query
            {:else if isSoap}
              Envelope
            {:else}
              Body{draft.bodyMode === 'none' ? '' : ` · ${bodyModeLabel(draft.bodyMode)}`}
            {/if}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'auth'}
            class:on={activeTab === 'auth'}
            onclick={() => (activeTab = 'auth')}
          >
            Auth · {authLabel(draft.auth)}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'scripts'}
            class:on={activeTab === 'scripts'}
            onclick={() => (activeTab = 'scripts')}
          >
            Scripts{scriptCount > 0 ? ` · ${String(scriptCount)}` : ''}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'examples'}
            class:on={activeTab === 'examples'}
            onclick={() => (activeTab = 'examples')}
          >
            Examples{examples.items.length > 0 ? ` · ${String(examples.items.length)}` : ''}
          </button>
        </div>

        {#if activeTab === 'params'}
          <div class="tab-body stack">
            <PathVariablesTable
              url={draft.url}
              bind:variables={draft.pathVariables}
              {collectionId}
              maskSensitive={maskSecrets}
            />
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
          </div>
        {:else if activeTab === 'auth'}
          <div class="tab-body">
            <AuthEditor
              bind:auth={draft.auth}
              allowInherit
              {collectionAuth}
              {collectionId}
              ownerId={requestId}
            />
          </div>
        {:else if activeTab === 'examples'}
          <div class="tab-body">
            <ExamplesPanel {examples} onChanged={onExamplesChanged} />
          </div>
        {:else if activeTab === 'scripts' && maskSecrets}
          <div class="tab-body">
            <p class="hint">Scripts are hidden while “Hide secrets on screen” is on.</p>
          </div>
        {:else if activeTab === 'scripts'}
          <div class="tab-body scripts">
            <div class="script-main">
              <div class="body-tools">
                <div class="segmented" role="group" aria-label="Script stage">
                  <button
                    type="button"
                    class:on={scriptStage === 'preRequestScript'}
                    aria-pressed={scriptStage === 'preRequestScript'}
                    onclick={() => (scriptStage = 'preRequestScript')}>Pre-request</button
                  >
                  <button
                    type="button"
                    class:on={scriptStage === 'testScript'}
                    aria-pressed={scriptStage === 'testScript'}
                    onclick={() => (scriptStage = 'testScript')}>Tests · after response</button
                  >
                </div>
                <span class="hint">JavaScript · pm.* API</span>
              </div>
              {#key scriptStage}
                <div class="body-editor">
                  <CodeEditor
                    value={currentScript}
                    label={scriptStage === 'preRequestScript'
                      ? 'Pre-request script'
                      : 'Test script'}
                    language="javascript"
                    onChange={updateScript}
                  />
                </div>
              {/key}
              <p class="hint">
                Runs in an isolated sandbox · 2 s time limit · network only through pm.sendRequest
              </p>
            </div>
            <div class="snippets" aria-label="Snippets">
              <span class="snippets-title">Snippets</span>
              {#each snippetsFor(scriptStage) as snippet (snippet.label)}
                <button
                  type="button"
                  onclick={() => {
                    insertSnippet(snippet);
                  }}>{snippet.label}</button
                >
              {/each}
            </div>
          </div>
        {:else}
          <div class="tab-body">
            {#if isGraphql}
              <GraphqlEditor bind:draft {collectionId} {requestId} />
            {:else if isSoap}
              <div class="stack">
                <SoapPanel bind:draft loader={wsdl} />
                <BodyEditor bind:draft {collectionId} {maskSecrets} fixedMode />
              </div>
            {:else}
              <BodyEditor bind:draft {collectionId} {maskSecrets} />
            {/if}
          </div>
        {/if}
      </section>

      <ResponsePanel
        {response}
        {error}
        {sending}
        {maskSecrets}
        name={draft.name}
        {stream}
        onSaveExample={() => void saveAsExample()}
      />
    {/if}
  </div>
</article>

<!-- Outside the card so the card's Ctrl+Enter / Ctrl+S shortcuts do not fire inside the dialog. -->
{#if capturing}
  <CaptureDialog
    {draft}
    {collectionId}
    {collectionAuth}
    {environmentName}
    {response}
    {error}
    log={isWebSocket ? session.log : null}
    connection={isWebSocket ? session.status : null}
    onClose={() => (capturing = false)}
  />
{/if}

{#if showingCode}
  <CodeDialog {draft} {collectionId} {collectionAuth} onClose={() => (showingCode = false)} />
{/if}

<style>
  .card {
    container-type: inline-size;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-card);
    overflow: hidden;
    scroll-margin-top: 16px;
  }

  .card.focused {
    border-color: var(--border);
    border-radius: 18px;
    box-shadow: var(--shadow-card);
  }

  .card.focused .head {
    padding: 18px 22px 10px;
  }

  .card.focused .name input {
    font-size: 20px;
  }

  .card.focused .target {
    padding: 4px 22px 16px;
  }

  .card.focused .url-field,
  .card.focused .send {
    height: 46px;
  }

  .card.focused .preview {
    margin: -8px 22px 12px;
  }

  .tool {
    flex: none;
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .tool:hover {
    border-color: var(--text-muted);
  }

  .tool svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px 6px;
  }

  .step {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .name {
    flex: 1;
    min-width: 0;
  }

  .name input {
    width: 100%;
    height: 34px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: 17px;
    font-weight: 600;
  }

  .name input:hover,
  .name input:focus {
    border-color: var(--border);
  }

  /* On a narrow card the name gets its own row and the tools move below it. */
  @container (max-width: 640px) {
    .head {
      flex-wrap: wrap;
      row-gap: 6px;
    }

    .name {
      flex: 1 1 calc(100% - 38px);
    }
  }

  .layout {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .layout button {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .layout button.on {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .layout svg,
  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .icon {
    flex: none;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .icon:active,
  .icon[aria-expanded='true'] {
    background: var(--accent-soft);
    color: var(--text);
  }

  .icon[aria-expanded='true'] {
    box-shadow: inset 0 0 0 1px var(--text);
  }

  .save {
    flex: none;
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }

  .save.failed {
    color: var(--err);
  }

  .target {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 6px 16px 12px;
  }

  .url-field {
    flex: 1 1 420px;
    min-width: 0;
    display: flex;
    height: 42px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--surface);
  }

  .url-field:focus-within {
    border-color: var(--focus);
  }

  .method {
    padding: 0 10px;
    border: 0;
    border-right: 1px solid var(--border);
    background: var(--surface-muted);
    font-weight: 600;
  }

  .url-masked {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    padding: 0 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-2);
  }

  .preview {
    margin: -4px 16px 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }

  .preview.missing {
    color: var(--warn);
  }

  .send {
    height: 42px;
    padding: 0 20px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--send-bg);
    color: var(--on-accent);
    font-size: 14px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }

  .send:disabled {
    opacity: 0.7;
  }

  .send:active:not(:disabled) {
    filter: brightness(0.9);
  }

  .send.connected {
    background: var(--surface);
    color: var(--err);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .method.ws,
  .method.gql,
  .method.grpc,
  .method.soap {
    display: inline-flex;
    align-items: center;
  }

  .method.ws {
    color: var(--method-ws);
    background: var(--method-ws-bg);
  }

  .method.gql {
    color: var(--method-gql);
    background: var(--method-gql-bg);
  }

  .method.soap {
    color: var(--method-soap);
    background: var(--method-soap-bg);
  }

  .method.grpc {
    color: var(--method-grpc);
    background: var(--method-grpc-bg);
  }

  .panes {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border-soft);
  }

  .panes > :global(*) {
    min-width: 0;
  }

  .panes.side {
    flex-direction: row;
    flex-wrap: wrap;
  }

  .panes.side > :global(*) {
    flex: 1 1 420px;
  }

  .panes:not(.side) > :global(.response) {
    border-top: 1px dashed var(--border);
  }

  .panes.side > :global(.response) {
    border-left: 1px solid var(--border-soft);
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
    gap: 16px;
  }

  .body-tools {
    display: flex;
    align-items: center;
    gap: 10px;
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

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .scripts {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
  }

  .script-main {
    flex: 3 1 380px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .snippets {
    flex: 1 1 200px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .snippets-title {
    padding: 0 10px 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }

  .snippets button {
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }

  .snippets button:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .body-editor {
    height: 240px;
    min-height: 120px;
    resize: vertical;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    padding: 4px;
  }
</style>
