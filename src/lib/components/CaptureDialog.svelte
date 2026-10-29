<script lang="ts">
  import Dropdown from './Dropdown.svelte';
  import { onMount } from 'svelte';
  import { saveCapture } from '../api/export';
  import type { Auth } from '../api/collections';
  import { previewUrl } from '../api/environments';
  import { toAppError, type AppError, type HttpResponseOutput } from '../api/http';
  import {
    buildDocument,
    hasRequestBody,
    type CaptureOptions,
    type LineLimit,
  } from '../capture/model';
  import { renderCapture, toPngBlob } from '../capture/render';
  import { slugify } from '../format/slug';
  import { formatBytes } from '../format/units';
  import { activePairs, type RequestDraft } from '../request/draft';
  import { pathVariableValues } from '../request/path-variables';
  import type { ConnectionStatus, LogEntry } from '../state/websocket.svelte';

  interface Props {
    draft: RequestDraft;
    collectionId: string;
    collectionAuth: Auth;
    environmentName: string | null;
    response: HttpResponseOutput | null;
    error: AppError | null;
    /** The message log of a WebSocket card; null for HTTP requests. */
    log: LogEntry[] | null;
    connection: ConnectionStatus | null;
    onClose: () => void;
  }

  let {
    draft,
    collectionId,
    collectionAuth,
    environmentName,
    response,
    error,
    log,
    connection,
    onClose,
  }: Props = $props();

  interface Status {
    tone: 'ok' | 'err';
    text: string;
  }

  interface Image {
    blob: Blob;
    width: number;
    height: number;
  }

  const LINE_LIMITS: { value: LineLimit; label: string }[] = [
    { value: 8, label: '8 lines' },
    { value: 20, label: '20 lines' },
    { value: 50, label: '50 lines' },
    { value: 'all', label: 'All' },
  ];
  const LINE_OPTIONS = LINE_LIMITS.map((limit) => ({
    value: String(limit.value),
    label: limit.label,
  }));
  const capturedAt = Date.now();
  // A busy socket keeps adding messages; the image shows the log as it was when Capture opened.
  const frozen = freezeLive();

  let dialog: HTMLDialogElement;
  let canvas: HTMLCanvasElement;
  let options = $state<CaptureOptions>(initialOptions());
  let resolvedUrl = $state<string | null>(null);
  let image = $state<Image | null>(null);
  let status = $state<Status | null>(null);
  let saving = $state(false);

  const isWebSocket = $derived(draft.kind === 'websocket');
  const canIncludeBody = $derived(hasRequestBody(draft));
  const hasResponse = $derived(response !== null || error !== null);
  const doc = $derived(
    buildDocument(
      {
        draft,
        collectionAuth,
        resolvedUrl,
        response,
        error,
        log: frozen.log,
        connection: frozen.connection,
        environmentName,
        capturedAt,
      },
      options,
    ),
  );

  function freezeLive(): { log: LogEntry[] | null; connection: ConnectionStatus | null } {
    return { log: log ? $state.snapshot(log) : null, connection };
  }
  const isEmpty = $derived(
    doc.method === null &&
      doc.url === null &&
      doc.request.length === 0 &&
      doc.response.length === 0,
  );

  function initialOptions(): CaptureOptions {
    return {
      methodAndUrl: true,
      requestHeaders: true,
      requestBody: hasRequestBody(draft),
      responseSummary: true,
      responseHeaders: false,
      responseBody: true,
      lineLimit: 8,
      hideSecrets: true,
      resolvedUrl: false,
      theme: 'light',
      layout: 'side',
      footer: false,
    };
  }

  onMount(() => {
    dialog.showModal();
    return () => {
      // Releases the image memory right away instead of waiting for garbage collection.
      canvas.width = 0;
      canvas.height = 0;
    };
  });

  // The resolved URL comes from the Rust core, which masks secret variables.
  $effect(() => {
    if (!options.resolvedUrl) {
      return;
    }
    let cancelled = false;
    previewUrl(
      draft.url.trim(),
      activePairs(draft.params),
      pathVariableValues(draft.url, draft.pathVariables),
      collectionId,
    ).then(
      (result) => {
        if (!cancelled) {
          resolvedUrl = result.url;
        }
      },
      (caught: unknown) => {
        status = { tone: 'err', text: toAppError(caught).message };
      },
    );
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const current = doc;
    const look = { theme: options.theme, layout: options.layout };
    // A newer drawing replaces this one; its result must then be ignored.
    const run = { cancelled: false };
    void (async () => {
      try {
        await renderCapture(canvas, current, look);
        const blob = await toPngBlob(canvas);
        if (!run.cancelled) {
          image = { blob, width: canvas.width, height: canvas.height };
        }
      } catch (caught) {
        if (!run.cancelled) {
          image = null;
          status = {
            tone: 'err',
            text: caught instanceof Error ? caught.message : 'Could not create the image.',
          };
        }
      }
    })();
    return () => {
      run.cancelled = true;
    };
  });

  async function copyImage(): Promise<void> {
    if (!image || isEmpty) {
      return;
    }
    try {
      await navigator.clipboard.write([new ClipboardItem({ 'image/png': image.blob })]);
      status = { tone: 'ok', text: 'Copied. Paste it into a chat, document or issue.' };
    } catch {
      status = { tone: 'err', text: 'Could not copy the image here. Use Save PNG… instead.' };
    }
  }

  async function savePng(): Promise<void> {
    if (!image || isEmpty || saving) {
      return;
    }
    saving = true;
    try {
      const bytes = new Uint8Array(await image.blob.arrayBuffer());
      const path = await saveCapture(bytes, fileStem(draft.name));
      status = path === null ? null : { tone: 'ok', text: `Saved to ${path}` };
    } catch (caught) {
      status = { tone: 'err', text: toAppError(caught).message };
    } finally {
      saving = false;
    }
  }

  function fileStem(name: string): string {
    const slug = slugify(name);
    return slug ? `codestamp-${slug}` : 'codestamp-capture';
  }

  function handleKeys(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
    } else if (
      (event.ctrlKey || event.metaKey) &&
      event.shiftKey &&
      event.key.toLowerCase() === 'c'
    ) {
      event.preventDefault();
      void copyImage();
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="capture-title"
  onkeydown={handleKeys}
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <header class="head">
    <h2 id="capture-title">Capture “{draft.name}”</h2>
    <span class="muted">Creates a PNG on this device. Nothing is uploaded.</span>
    <span class="spacer"></span>
    <button type="button" class="close" aria-label="Close" onclick={onClose}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>
  </header>

  <div class="body">
    <div class="preview">
      <span class="preview-title" role="status">
        {#if isEmpty}
          Choose at least one thing to include.
        {:else if image}
          Preview · {image.width} × {image.height} px · about {formatBytes(image.blob.size)}
        {:else}
          Drawing the preview…
        {/if}
      </span>
      <figure class="frame">
        <canvas bind:this={canvas} aria-label="Image preview"></canvas>
      </figure>
    </div>

    <form class="options" aria-label="Capture options">
      <fieldset class="group">
        <legend>Include</legend>
        <label class="check">
          <input
            type="checkbox"
            checked={options.methodAndUrl}
            onchange={(event) => {
              options.methodAndUrl = event.currentTarget.checked;
            }}
          />
          {isWebSocket ? 'URL' : 'Method and URL'}
        </label>
        <label class="check">
          <input
            type="checkbox"
            checked={options.requestHeaders}
            onchange={(event) => {
              options.requestHeaders = event.currentTarget.checked;
            }}
          />
          {isWebSocket ? 'Handshake headers' : 'Request headers'}
        </label>
        {#if !isWebSocket}
          <label class="check" class:off={!canIncludeBody}>
            <input
              type="checkbox"
              checked={options.requestBody && canIncludeBody}
              disabled={!canIncludeBody}
              onchange={(event) => {
                options.requestBody = event.currentTarget.checked;
              }}
            />
            Request body
            {#if !canIncludeBody}<span class="muted">(this request has none)</span>{/if}
          </label>
        {/if}
        <label class="check" class:off={!isWebSocket && !hasResponse}>
          <input
            type="checkbox"
            checked={options.responseSummary}
            disabled={!isWebSocket && !hasResponse}
            onchange={(event) => {
              options.responseSummary = event.currentTarget.checked;
            }}
          />
          {isWebSocket ? 'Connection status' : 'Response status and time'}
        </label>
        {#if !isWebSocket}
          <label class="check" class:off={response === null}>
            <input
              type="checkbox"
              checked={options.responseHeaders}
              disabled={response === null}
              onchange={(event) => {
                options.responseHeaders = event.currentTarget.checked;
              }}
            />
            Response headers
          </label>
        {/if}
        <div class="check" class:off={!isWebSocket && !hasResponse}>
          <label class="check">
            <input
              type="checkbox"
              checked={options.responseBody}
              disabled={!isWebSocket && !hasResponse}
              onchange={(event) => {
                options.responseBody = event.currentTarget.checked;
              }}
            />
            {isWebSocket ? 'Messages, last' : 'Response body, first'}
          </label>
          <Dropdown
            label={isWebSocket ? 'Messages' : 'Response lines'}
            value={String(options.lineLimit)}
            options={LINE_OPTIONS}
            onChange={(value: string) => {
              options.lineLimit =
                LINE_LIMITS.find((limit) => String(limit.value) === value)?.value ?? 8;
            }}
          />
        </div>
        {#if !isWebSocket && !hasResponse}
          <p class="muted small">Send the request to include its response.</p>
        {/if}
      </fieldset>

      <fieldset class="group">
        <legend>Privacy</legend>
        <label class="check">
          <input
            type="checkbox"
            checked={options.hideSecrets}
            onchange={(event) => {
              options.hideSecrets = event.currentTarget.checked;
            }}
          />
          Hide secrets
        </label>
        {#if options.hideSecrets}
          <p class="hidden-list">
            {#if doc.hidden.length > 0}
              Hidden here: <span class="mono">{doc.hidden.join(', ')}</span>
            {:else}
              Nothing sensitive found in what is shown.
            {/if}
            {#if doc.heuristic}
              Some text is not JSON and was only checked for common patterns; look over the preview
              before sharing.
            {/if}
          </p>
        {/if}
        <label class="check">
          <input
            type="radio"
            name="capture-url"
            checked={!options.resolvedUrl}
            onchange={() => {
              options.resolvedUrl = false;
            }}
          />
          Show variables in URL <span class="mono muted">{'{{baseUrl}}'}</span>
        </label>
        <label class="check">
          <input
            type="radio"
            name="capture-url"
            checked={options.resolvedUrl}
            onchange={() => {
              options.resolvedUrl = true;
            }}
          />
          Show the resolved URL
        </label>
      </fieldset>

      <fieldset class="group last">
        <legend>Look</legend>
        <div class="row">
          <div class="segmented" role="group" aria-label="Theme">
            <button
              type="button"
              class:on={options.theme === 'light'}
              aria-pressed={options.theme === 'light'}
              onclick={() => (options.theme = 'light')}>Light</button
            >
            <button
              type="button"
              class:on={options.theme === 'dark'}
              aria-pressed={options.theme === 'dark'}
              onclick={() => (options.theme = 'dark')}>Dark</button
            >
          </div>
          <div class="segmented" role="group" aria-label="Layout">
            <button
              type="button"
              class:on={options.layout === 'side'}
              aria-pressed={options.layout === 'side'}
              onclick={() => (options.layout = 'side')}>Side by side</button
            >
            <button
              type="button"
              class:on={options.layout === 'stacked'}
              aria-pressed={options.layout === 'stacked'}
              onclick={() => (options.layout = 'stacked')}>Stacked</button
            >
          </div>
        </div>
        <label class="check">
          <input
            type="checkbox"
            checked={options.footer}
            onchange={(event) => {
              options.footer = event.currentTarget.checked;
            }}
          />
          Add date and environment name at the bottom
        </label>
      </fieldset>
    </form>
  </div>

  <footer class="foot">
    {#if status}
      <p class="status {status.tone}" role="status">{status.text}</p>
    {/if}

    <div class="actions">
      <button
        type="button"
        class="secondary"
        disabled={!image || isEmpty || saving}
        onclick={() => void savePng()}
      >
        {saving ? 'Saving…' : 'Save PNG…'}
      </button>
      <button
        type="button"
        class="primary"
        disabled={!image || isEmpty}
        aria-keyshortcuts="Control+Shift+C"
        onclick={() => void copyImage()}
      >
        Copy image
      </button>
    </div>
  </footer>
</dialog>

<style>
  dialog {
    width: min(1180px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 0;
    border: 0;
    border-radius: 18px;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
  }

  dialog[open] {
    display: flex;
    flex-direction: column;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .muted {
    color: var(--text-muted);
    font-size: 13px;
  }

  .small {
    margin: 0;
    font-size: 12px;
  }

  .spacer {
    flex: 1;
  }

  .close {
    width: 34px;
    height: 34px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .close:hover {
    background: var(--surface-sunken);
  }

  .close path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .body {
    display: flex;
    flex-wrap: wrap;
    min-height: 0;
    overflow: auto;
  }

  .preview {
    flex: 3 1 480px;
    min-width: 0;
    padding: 20px;
    background: var(--surface-sunken);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .preview-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .frame {
    margin: 0;
    max-height: calc(100vh - 220px);
    overflow: auto;
  }

  canvas {
    display: block;
    width: 100%;
    height: auto;
    border-radius: var(--radius-md);
  }

  .options {
    flex: 2 1 320px;
    min-width: 0;
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .group {
    margin: 0;
    padding: 0 0 14px;
    border: 0;
    border-bottom: 1px solid var(--border-soft);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group.last {
    border-bottom: 0;
    padding-bottom: 0;
  }

  legend {
    margin-bottom: 4px;
    padding: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }

  .check {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }

  .check.off {
    color: var(--text-muted);
  }

  .check input {
    accent-color: var(--accent);
  }

  .hidden-list {
    margin: 0 0 0 26px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--bg);
    font-size: 12px;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
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
    padding: 6px 12px;
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

  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-soft);
  }

  .status {
    flex: 1 1 240px;
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .status.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .status.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
    margin-left: auto;
  }

  .actions button {
    height: 38px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .actions button:disabled {
    opacity: 0.6;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
</style>
