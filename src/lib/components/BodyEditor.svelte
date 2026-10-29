<script lang="ts">
  import { pickUploadFile } from '../api/http';
  import { prettyJson } from '../format/json';
  import { formatMarkup, xmlProblem } from '../format/markup';
  import { formatBytes } from '../format/units';
  import { maskAny } from '../privacy/secrets';
  import { BODY_MODES, bodyLanguage, fileName } from '../request/body';
  import { isTextRow, type KeyValueRow, type RequestDraft } from '../request/draft';
  import { notices } from '../state/notices.svelte';
  import { preferenceSettings } from '../state/preferences.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import KeyValueTable from './KeyValueTable.svelte';

  interface Props {
    draft: RequestDraft;
    collectionId: string;
    /** Hides tokens on screen while presenting; editing raw bodies pauses meanwhile. */
    maskSecrets: boolean;
    /** SOAP envelopes are always XML, so the body type cannot be switched. */
    fixedMode?: boolean;
  }

  let { draft = $bindable(), collectionId, maskSecrets, fixedMode = false }: Props = $props();

  // Only known right after picking; a reopened request shows just the file name.
  let pickedSize: number | null = $state(null);

  const skippedFileRows = $derived(
    draft.bodyMode === 'urlencoded' &&
      draft.form.some((row) => !isTextRow(row) && row.key.trim() !== ''),
  );

  async function pickFormFile(row: KeyValueRow): Promise<void> {
    await notices.guard(async () => {
      const picked = await pickUploadFile();
      if (picked) {
        row.value = picked.path;
      }
    });
  }

  async function pickBodyFile(): Promise<void> {
    await notices.guard(async () => {
      const picked = await pickUploadFile();
      if (picked) {
        draft.bodyFile = picked.path;
        pickedSize = picked.size;
      }
    });
  }

  function clearBodyFile(): void {
    draft.bodyFile = '';
    pickedSize = null;
  }

  // Set when Beautify is pressed on text that is not complete JSON, XML or HTML.
  let beautifyProblem = $state<string | null>(null);
  const formattable = $derived(
    draft.bodyMode === 'json' || draft.bodyMode === 'xml' || draft.bodyMode === 'html',
  );
  const problem = $derived(draft.bodyMode === 'xml' ? xmlProblem(draft.body) : null);

  function formatted(): string | null {
    const indent = preferenceSettings.jsonIndent;
    switch (draft.bodyMode) {
      case 'json':
        return prettyJson(draft.body, indent);
      case 'xml':
      case 'html':
        return formatMarkup(draft.body, draft.bodyMode, indent);
      default:
        return null;
    }
  }

  function beautify(): void {
    const next = formatted();
    beautifyProblem =
      next === null ? `Finish the ${bodyLanguage(draft.bodyMode).toUpperCase()} first.` : null;
    draft.body = next ?? draft.body;
  }
</script>

<div class="body">
  <div class="tools">
    {#if fixedMode}
      <span class="hint">Envelope · XML</span>
    {:else}
      <div class="segmented" role="group" aria-label="Body type">
        {#each BODY_MODES as option (option.mode)}
          <button
            type="button"
            class:on={draft.bodyMode === option.mode}
            aria-pressed={draft.bodyMode === option.mode}
            onclick={() => (draft.bodyMode = option.mode)}>{option.label}</button
          >
        {/each}
      </div>
    {/if}
    {#if formattable && !maskSecrets}
      <button type="button" class="ghost" onclick={beautify}>Beautify</button>
    {/if}
  </div>

  {#if draft.bodyMode === 'none'}
    <p class="hint">This request has no body.</p>
  {:else if draft.bodyMode === 'urlencoded' || draft.bodyMode === 'formdata'}
    <KeyValueTable
      title="Form fields"
      bind:rows={draft.form}
      {collectionId}
      maskSensitive={maskSecrets}
      files={draft.bodyMode === 'formdata'}
      onPickFile={(row: KeyValueRow) => void pickFormFile(row)}
      note={skippedFileRows
        ? 'File fields are only sent as form data; they are left out here.'
        : draft.bodyMode === 'formdata'
          ? 'Sent as multipart/form-data. Files are streamed from disk, up to 100 MB in total.'
          : 'Sent as application/x-www-form-urlencoded.'}
    />
  {:else if draft.bodyMode === 'binary'}
    <div class="file-pick">
      <button type="button" class="ghost" onclick={() => void pickBodyFile()}>
        {draft.bodyFile === '' ? 'Choose file…' : 'Change file…'}
      </button>
      {#if draft.bodyFile !== ''}
        <span class="file-name mono" title={fileName(draft.bodyFile)}>
          {fileName(draft.bodyFile)}{pickedSize === null ? '' : ` · ${formatBytes(pickedSize)}`}
        </span>
        <button type="button" class="link" onclick={clearBodyFile}>Remove</button>
      {/if}
    </div>
    <p class="hint">
      The file's bytes are the body, streamed from disk (up to 100 MB), as application/octet-stream
      unless you set a Content-Type header. Only files chosen here can be sent.
    </p>
  {:else if maskSecrets}
    <pre class="masked mono">{maskAny(draft.body, 2).text}</pre>
    <p class="hint">Editing is paused while “Hide secrets on screen” is on.</p>
  {:else}
    <div class="body-editor">
      <CodeEditor
        value={draft.body}
        label={fixedMode ? 'SOAP envelope' : 'Request body'}
        language={bodyLanguage(draft.bodyMode)}
        variablesOf={collectionId}
        onChange={(next: string) => {
          draft.body = next;
          beautifyProblem = null;
        }}
      />
    </div>
    {#if problem ?? beautifyProblem}
      <p class="hint problem" role="status">{problem ?? beautifyProblem}</p>
    {/if}
  {/if}
</div>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .segmented {
    display: inline-flex;
    flex-wrap: wrap;
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

  .link {
    border: 0;
    background: transparent;
    color: var(--err);
    font-size: 13px;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .problem {
    color: var(--warn);
  }

  .file-pick {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
  }

  .file-name {
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  .masked {
    margin: 0;
    max-height: 240px;
    overflow: auto;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
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
