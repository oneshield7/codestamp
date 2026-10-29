<script lang="ts">
  import { saveResponseBody } from '../api/export';
  import type { AppError, HttpResponseOutput, ResponseCookie } from '../api/http';
  import { prettyJson } from '../format/json';
  import { formatMarkup, markupLanguage } from '../format/markup';
  import { soapFault } from '../request/soap';
  import { statusTone } from '../format/status';
  import { formatExpiry } from '../format/time';
  import { formatBytes, formatDuration } from '../format/units';
  import { isSensitiveName, maskJson, maskSecret, maskText } from '../privacy/secrets';
  import { notices } from '../state/notices.svelte';
  import type { EventStreamView } from '../state/streams.svelte';
  import { formatClock } from '../format/time';
  import CodeEditor from './CodeEditor.svelte';

  interface Props {
    response: HttpResponseOutput | null;
    error: AppError | null;
    sending: boolean;
    /** Hides tokens and cookies on screen (focus mode); the response itself is unchanged. */
    maskSecrets?: boolean;
    /** The request name, suggested as the file name when saving the body. */
    name?: string;
    /** Keeps the response with the request, for reference and the mock server. */
    onSaveExample?: () => void;
    /** The live events of an event-stream response. */
    stream?: EventStreamView | null;
  }

  let {
    response,
    error,
    sending,
    maskSecrets = false,
    name = 'response',
    onSaveExample,
    stream = null,
  }: Props = $props();

  /** Event data that is JSON is shown indented. */
  function eventText(data: string): string {
    const text = prettyJson(data) ?? data;
    return maskSecrets ? maskText(text).text : text;
  }

  async function saveBody(bodyId: string): Promise<void> {
    await notices.guard(async () => {
      const path = await saveResponseBody(bodyId, name);
      if (path !== null) {
        notices.inform(`Response body saved to ${path}`);
      }
    });
  }

  type ResponseTab = 'body' | 'headers' | 'cookies' | 'tests' | 'console';
  let activeTab = $state<ResponseTab>('body');

  const script = $derived(response?.script ?? null);
  const cookies = $derived(response?.cookies ?? []);
  // Tabs that only exist for some responses fall back to the body for the others.
  const tab = $derived.by((): ResponseTab => {
    if (script === null && (activeTab === 'tests' || activeTab === 'console')) {
      return 'body';
    }
    return activeTab === 'cookies' && cookies.length === 0 ? 'body' : activeTab;
  });
  const passedCount = $derived(script?.tests.filter((test) => test.passed).length ?? 0);
  const allPassed = $derived(script !== null && passedCount === script.tests.length);

  const formattedJson = $derived.by(() => {
    if (!response || response.bodyTruncated) {
      return null;
    }
    return maskSecrets
      ? (maskJson(response.bodyPreview, 2)?.text ?? null)
      : prettyJson(response.bodyPreview);
  });

  // XML (SOAP among it) and HTML responses are re-indented and highlighted like JSON.
  const markup = $derived.by(() => {
    if (!response || response.bodyTruncated || formattedJson !== null) {
      return null;
    }
    const contentType =
      response.headers.find((header) => header.name.toLowerCase() === 'content-type')?.value ?? '';
    const language = markupLanguage(contentType, response.bodyPreview);
    if (!language) {
      return null;
    }
    const text = formatMarkup(response.bodyPreview, language) ?? response.bodyPreview;
    return { language, text: shown(text) };
  });

  const fault = $derived(
    markup?.language === 'xml' && response ? soapFault(response.bodyPreview) : null,
  );

  function headerValue(name: string, value: string): string {
    return maskSecrets && isSensitiveName(name) ? maskSecret(value) : value;
  }
  const bodyText = $derived(
    formattedJson ?? markup?.text ?? (response ? shown(response.bodyPreview) : ''),
  );

  // Max-Age wins over Expires (RFC 6265) and counts from when the response arrived.
  function expirySeconds(cookie: ResponseCookie): number | null {
    return cookie.maxAge === null ? cookie.expires : Math.floor(Date.now() / 1000) + cookie.maxAge;
  }

  /** Text that is not JSON (or is cut off) gets the best-effort masking while presenting. */
  function shown(text: string): string {
    return maskSecrets ? maskText(text).text : text;
  }
  const tone = $derived(statusTone(response?.status ?? 0));
</script>

<section class="response" aria-label="Response" aria-busy={sending}>
  <div class="summary">
    <span class="label">Response</span>
    {#if sending}
      <span class="muted">Sending…</span>
    {:else if error}
      <span class="chip err">Failed</span>
    {:else if response}
      <span class="chip {tone}">{response.status} {response.statusText}</span>
      <span class="muted">
        {formatDuration(response.elapsedMs)} · {formatBytes(response.bodySize)} · {response.httpVersion}
      </span>
      {#if fault}
        <span class="chip err" title={fault.code}>SOAP Fault</span>
      {/if}
      {#if script && script.tests.length > 0}
        <span class="chip" class:ok={allPassed} class:err={!allPassed}>
          Tests {passedCount}/{script.tests.length}
        </span>
      {/if}
      {#if response.bodySize > 0}
        {@const bodyId = response.bodyId}
        <span class="spacer"></span>
        {#if onSaveExample}
          <button
            type="button"
            class="save"
            disabled={response.bodyTruncated}
            title={response.bodyTruncated
              ? 'Only responses up to 2 MB can be kept as examples'
              : 'Keep this response with the request; the mock server answers with it'}
            onclick={onSaveExample}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 4h12v16l-6-4-6 4z" />
            </svg>
            Save as example
          </button>
        {/if}
        <button
          type="button"
          class="save"
          disabled={bodyId === null}
          title={bodyId === null
            ? 'Only responses up to 32 MB can be saved'
            : 'Save the body to a file, exactly as it arrived'}
          onclick={() => {
            if (bodyId !== null) {
              void saveBody(bodyId);
            }
          }}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 4v11M7 10l5 5 5-5M4 20h16" />
          </svg>
          Save body
        </button>
      {/if}
    {:else}
      <span class="muted">Press Send to see the response here.</span>
    {/if}
  </div>

  {#if error && !sending}
    <p class="error" role="alert">{shown(error.message)}</p>
  {:else if response && !sending}
    <div class="tabs" role="tablist" aria-label="Response sections">
      <button
        type="button"
        role="tab"
        aria-selected={tab === 'body'}
        class:on={tab === 'body'}
        onclick={() => (activeTab = 'body')}
        >{stream ? `Events · ${String(stream.count)}` : 'Body'}</button
      >
      <button
        type="button"
        role="tab"
        aria-selected={tab === 'headers'}
        class:on={tab === 'headers'}
        onclick={() => (activeTab = 'headers')}>Headers · {response.headers.length}</button
      >
      {#if cookies.length > 0}
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'cookies'}
          class:on={tab === 'cookies'}
          onclick={() => (activeTab = 'cookies')}>Cookies · {cookies.length}</button
        >
      {/if}
      {#if script}
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'tests'}
          class:on={tab === 'tests'}
          onclick={() => (activeTab = 'tests')}>Tests · {passedCount}/{script.tests.length}</button
        >
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'console'}
          class:on={tab === 'console'}
          onclick={() => (activeTab = 'console')}>Console · {script.logs.length}</button
        >
      {/if}
    </div>

    {#if response.warning}
      <p class="notice">{response.warning}</p>
    {/if}

    {#if fault}
      <p class="error" role="alert">
        {fault.code === '' ? 'SOAP Fault' : fault.code}{fault.reason === ''
          ? ''
          : `: ${shown(fault.reason)}`}
      </p>
    {/if}

    {#each script?.errors ?? [] as message, index (index)}
      <p class="error" role="alert">{shown(message)}</p>
    {/each}

    {#if response.bodyTruncated}
      <p class="notice">
        Large response ({formatBytes(response.bodySize)}). Only the beginning is shown to keep
        memory low.
      </p>
    {/if}

    {#if tab === 'body' && stream}
      <div class="stream">
        <div class="stream-head" role="status">
          {#if !stream.ended}
            <span class="live" aria-hidden="true"></span>
            <span>Streaming · {stream.count} {stream.count === 1 ? 'event' : 'events'}</span>
            <span class="spacer"></span>
            <button type="button" class="save" onclick={() => void stream.stop()}>Stop</button>
          {:else if stream.error}
            <span class="stream-error">The stream broke off: {stream.error}</span>
          {:else}
            <span>Stream ended · {stream.count} {stream.count === 1 ? 'event' : 'events'}</span>
          {/if}
        </div>
        <ol class="events mono" aria-label="Events">
          {#each stream.events as event, index (index)}
            <li>
              <span class="event-meta">
                <span class="event-time">{formatClock(event.at, true)}</span>
                {#if event.event !== 'message'}<span class="event-name">{event.event}</span>{/if}
                {#if event.id}<span class="event-id">id {event.id}</span>{/if}
              </span>
              <pre class="event-data">{eventText(event.data)}</pre>
            </li>
          {:else}
            <li class="empty">Waiting for the first event…</li>
          {/each}
        </ol>
      </div>
    {:else if tab === 'body'}
      <div class="viewer" data-find>
        <CodeEditor
          value={bodyText}
          label="Response body"
          language={formattedJson === null ? (markup?.language ?? 'text') : 'json'}
          readOnly
        />
      </div>
    {:else if tab === 'headers'}
      <dl class="headers mono">
        {#each response.headers as header, index (index)}
          <dt>{header.name}</dt>
          <dd>{headerValue(header.name, header.value)}</dd>
        {/each}
      </dl>
    {:else if tab === 'cookies'}
      <div class="cookies">
        <table>
          <thead>
            <tr>
              <th scope="col">Name</th>
              <th scope="col">Value</th>
              <th scope="col">Domain</th>
              <th scope="col">Path</th>
              <th scope="col">Expires</th>
              <th scope="col">Flags</th>
            </tr>
          </thead>
          <tbody>
            {#each cookies as cookie, index (index)}
              <tr>
                <td class="mono">{cookie.name}</td>
                <td class="mono value">{maskSecrets ? maskSecret(cookie.value) : cookie.value}</td>
                <td class="mono">{cookie.domain ?? '(this host)'}</td>
                <td class="mono">{cookie.path ?? '/'}</td>
                <td>
                  {cookie.maxAge !== null && cookie.maxAge <= 0
                    ? 'Removed'
                    : formatExpiry(expirySeconds(cookie))}
                </td>
                <td class="flags">
                  {[
                    cookie.httpOnly ? 'HttpOnly' : '',
                    cookie.secure ? 'Secure' : '',
                    cookie.sameSite ? `SameSite=${cookie.sameSite}` : '',
                  ]
                    .filter(Boolean)
                    .join(' · ')}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="note">Saved in the cookie jar and sent back to matching hosts automatically.</p>
      </div>
    {:else if tab === 'tests'}
      <ul class="tests">
        {#each script?.tests ?? [] as test, index (index)}
          <li class:failed={!test.passed}>
            <span class="mark" aria-label={test.passed ? 'Passed' : 'Failed'}
              >{test.passed ? '✓' : '✗'}</span
            >
            <span class="test-text">
              <span>{shown(test.name)}</span>
              {#if test.error}<span class="test-error mono">{shown(test.error)}</span>{/if}
            </span>
          </li>
        {:else}
          <li class="empty">No tests ran. Add pm.test(...) in the Tests script.</li>
        {/each}
      </ul>
    {:else}
      <ol class="console mono">
        {#each script?.logs ?? [] as line, index (index)}
          <li class={line.level}>{shown(line.message)}</li>
        {:else}
          <li class="empty">Nothing logged. Use console.log(...) in a script.</li>
        {/each}
      </ol>
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

  .tests,
  .console {
    margin: 0;
    padding: 10px 16px 16px;
    list-style: none;
    max-height: 360px;
    overflow: auto;
    font-size: 13px;
  }

  .tests li {
    display: flex;
    gap: 10px;
    padding: 6px 0;
    color: var(--ok);
  }

  .tests li.failed {
    color: var(--err);
  }

  .mark {
    flex: none;
    width: 14px;
    font-weight: 700;
  }

  .test-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--text);
  }

  .test-error {
    font-size: 12px;
    color: var(--err);
    overflow-wrap: anywhere;
  }

  .console li {
    padding: 4px 0;
    border-bottom: 1px solid var(--border-soft);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .console li.warn {
    color: var(--warn);
  }

  .console li.error {
    color: var(--err);
  }

  .tests .empty,
  .console .empty {
    color: var(--text-muted);
    font-family: var(--font-ui);
  }

  .summary {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
  }

  .spacer {
    flex: 1;
  }

  .stream {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 16px 16px;
  }

  .stream-head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    font-size: 13px;
    color: var(--text-2);
  }

  .live {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    animation: pulse 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .live {
      animation: none;
    }
  }

  .stream-error {
    color: var(--err);
  }

  .events {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 360px;
    overflow: auto;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 12px;
  }

  .events li {
    padding: 6px 10px;
  }

  .events li + li {
    border-top: 1px solid var(--border-soft);
  }

  .event-meta {
    display: flex;
    gap: 10px;
    color: var(--text-muted);
  }

  .event-name {
    color: var(--accent-text);
    font-weight: 600;
  }

  .event-data {
    margin: 2px 0 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font: inherit;
    color: var(--text);
  }

  .events .empty {
    color: var(--text-muted);
  }

  .save {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
  }

  .save:hover:not(:disabled) {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .save:disabled {
    color: var(--text-muted);
  }

  .save svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
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
    margin: 0 16px 16px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
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

  /* Long responses scroll inside this box; drag the corner to resize it. */
  .viewer {
    height: 360px;
    min-height: 140px;
    max-height: 80vh;
    resize: vertical;
    overflow: hidden;
    padding: 6px 4px;
  }

  .headers {
    display: grid;
    grid-template-columns: minmax(140px, max-content) 1fr;
    gap: 6px 16px;
    margin: 0;
    padding: 12px 16px 16px;
    max-height: 360px;
    overflow: auto;
  }

  .headers dt {
    color: var(--text-muted);
  }

  .headers dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .cookies {
    max-height: 360px;
    overflow: auto;
    padding: 8px 16px 16px;
  }

  .cookies table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  .cookies th {
    padding: 6px 8px;
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-soft);
  }

  .cookies td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-soft);
    vertical-align: top;
  }

  .cookies .value {
    max-width: 260px;
    overflow-wrap: anywhere;
  }

  .cookies .flags {
    color: var(--text-muted);
    font-size: 12px;
  }

  .cookies .note {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
