<script lang="ts">
  import type { Auth, GrpcSource } from '../api/collections';
  import { pickProtoFile, type GrpcMethodInfo, type GrpcTarget } from '../api/grpc';
  import { parseJson, stringifyJson } from '../format/json';
  import { authLabel } from '../request/auth';
  import { fileName } from '../request/body';
  import { countActiveRows, type RequestDraft } from '../request/draft';
  import type { GrpcCall } from '../state/grpc.svelte';
  import { notices } from '../state/notices.svelte';
  import { preferenceSettings } from '../state/preferences.svelte';
  import AuthEditor from './AuthEditor.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import Dropdown from './Dropdown.svelte';
  import KeyValueTable from './KeyValueTable.svelte';

  interface Props {
    draft: RequestDraft;
    call: GrpcCall;
    target: GrpcTarget;
    collectionAuth: Auth;
    collectionId: string;
    requestId: string;
    maskSecrets: boolean;
  }

  let {
    draft = $bindable(),
    call,
    target,
    collectionAuth,
    collectionId,
    requestId,
    maskSecrets,
  }: Props = $props();

  type GrpcTab = 'message' | 'metadata' | 'auth';
  let activeTab = $state<GrpcTab>('message');

  const SOURCES: { value: GrpcSource; label: string }[] = [
    { value: 'reflection', label: 'Server reflection' },
    { value: 'proto', label: '.proto file' },
  ];
  const NO_METHOD = '';

  // Services loaded earlier for this server or file show up again without asking.
  $effect(() => {
    call.recall(target);
  });

  const ready = $derived(
    draft.grpc.source === 'proto' ? draft.grpc.protoFile !== '' : draft.url.trim() !== '',
  );
  const chosen = $derived(call.find(draft.grpc.method));
  const methodOptions = $derived.by(() => {
    const options = call.services.flatMap((service) =>
      service.methods.map((method) => ({
        value: method.fullName,
        label: `${method.fullName} · ${streamLabel(method)}`,
      })),
    );
    if (draft.grpc.method !== NO_METHOD && !options.some((o) => o.value === draft.grpc.method)) {
      options.unshift({ value: draft.grpc.method, label: draft.grpc.method });
    }
    if (options.length === 0) {
      options.push({ value: NO_METHOD, label: 'Load the services to choose a method' });
    }
    return options;
  });

  function streamLabel(method: GrpcMethodInfo): string {
    if (method.clientStreaming && method.serverStreaming) {
      return 'bidirectional stream';
    }
    if (method.clientStreaming) {
      return 'client stream';
    }
    return method.serverStreaming ? 'server stream' : 'unary';
  }

  /** The starting message: every field, in an array for methods that stream from the client. */
  function templateFor(method: GrpcMethodInfo): string {
    const parsed = parseJson(method.template);
    if (!parsed) {
      return method.template;
    }
    return stringifyJson(
      method.clientStreaming ? [parsed.value] : parsed.value,
      preferenceSettings.jsonIndent,
    );
  }

  function choose(fullName: string): void {
    const next = call.find(fullName);
    if (!next || fullName === draft.grpc.method) {
      return;
    }
    // A message the user wrote is kept; an empty one or the last template is replaced.
    const previous = chosen;
    const untouched =
      draft.body.trim() === '' || (previous !== null && draft.body === templateFor(previous));
    draft.grpc.method = fullName;
    if (untouched) {
      draft.body = templateFor(next);
    }
  }

  function chooseSource(source: GrpcSource): void {
    draft.grpc.source = source;
  }

  async function chooseProto(): Promise<void> {
    await notices.guard(async () => {
      const picked = await pickProtoFile();
      if (picked) {
        draft.grpc.protoFile = picked.path;
      }
    });
  }

  const messageHint = $derived.by(() => {
    if (chosen?.clientStreaming) {
      return 'A JSON array: each element is sent as one message';
    }
    if (chosen?.serverStreaming) {
      return 'Proto3 JSON · the answers are read until the stream ends (up to 200)';
    }
    return 'Proto3 JSON · field names in lowerCamelCase, 64-bit numbers as strings';
  });
</script>

<section class="request" aria-label="gRPC request">
  <div class="source">
    <div class="segmented" role="group" aria-label="Service definition">
      {#each SOURCES as source (source.value)}
        <button
          type="button"
          class:on={draft.grpc.source === source.value}
          aria-pressed={draft.grpc.source === source.value}
          onclick={() => {
            chooseSource(source.value);
          }}>{source.label}</button
        >
      {/each}
    </div>
    {#if draft.grpc.source === 'proto'}
      <button
        type="button"
        class="ghost file"
        title={draft.grpc.protoFile === ''
          ? 'Imports are read from the same folder'
          : 'Choose another .proto file'}
        onclick={() => void chooseProto()}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 3h8l4 4v14H6zM14 3v4h4" />
        </svg>
        {draft.grpc.protoFile === '' ? 'Choose .proto file…' : fileName(draft.grpc.protoFile)}
      </button>
    {/if}
    <span class="spacer"></span>
    <button
      type="button"
      class="ghost"
      disabled={call.loading || !ready}
      title={ready
        ? 'Ask for the services and methods again'
        : draft.grpc.source === 'proto'
          ? 'Choose the .proto file first'
          : 'Enter the server address first'}
      onclick={() => void call.load(target)}
    >
      {call.loading ? 'Loading…' : 'Load services'}
    </button>
  </div>
  {#if call.loadError}
    <p class="error" role="alert">{call.loadError}</p>
  {/if}
  <div class="method">
    <Dropdown
      variant="field"
      label="gRPC method"
      value={draft.grpc.method}
      options={methodOptions}
      mono
      disabled={call.services.length === 0}
      onChange={choose}
    />
  </div>

  <div class="tabs" role="tablist" aria-label="gRPC request sections">
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'message'}
      class:on={activeTab === 'message'}
      onclick={() => (activeTab = 'message')}>Message</button
    >
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'metadata'}
      class:on={activeTab === 'metadata'}
      onclick={() => (activeTab = 'metadata')}
    >
      Metadata{countActiveRows(draft.headers) > 0
        ? ` · ${String(countActiveRows(draft.headers))}`
        : ''}
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'auth'}
      class:on={activeTab === 'auth'}
      onclick={() => (activeTab = 'auth')}>Auth · {authLabel(draft.auth)}</button
    >
  </div>

  {#if activeTab === 'message'}
    <div class="tab-body">
      <div class="editor">
        <CodeEditor
          value={draft.body}
          label="gRPC message"
          language="json"
          onChange={(next: string) => (draft.body = next)}
        />
      </div>
      <p class="hint">{messageHint}</p>
    </div>
  {:else if activeTab === 'metadata'}
    <div class="tab-body">
      <KeyValueTable
        title="Metadata"
        bind:rows={draft.headers}
        {collectionId}
        maskSensitive={maskSecrets}
      />
    </div>
  {:else}
    <div class="tab-body">
      <AuthEditor
        bind:auth={draft.auth}
        allowInherit
        hidden={['digest', 'awsSigV4']}
        {collectionAuth}
        {collectionId}
        ownerId={requestId}
      />
    </div>
  {/if}
</section>

<style>
  .source {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 0;
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
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
  }

  .ghost:hover:not(:disabled) {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .ghost:disabled {
    color: var(--text-muted);
  }

  .ghost svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .file {
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .spacer {
    flex: 1;
  }

  .method {
    padding: 10px 16px 4px;
  }

  .error {
    margin: 10px 16px 0;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .tabs {
    display: flex;
    gap: 16px;
    margin-top: 6px;
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
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px 16px;
  }

  .editor {
    height: 240px;
    min-height: 120px;
    resize: vertical;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    padding: 4px;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
