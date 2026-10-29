<script lang="ts">
  import NavIcon from './NavIcon.svelte';
  import { onMount, untrack } from 'svelte';
  import { createRequest } from '../api/collections';
  import { toAppError } from '../api/http';
  import {
    MAX_IMPORT_BYTES,
    parseCurl,
    fetchImport,
    previewImport,
    replaceWithImport,
    runImport,
    type CurlImport,
    type ImportPreview,
    type ImportResult,
  } from '../api/import';
  import { MAX_NAME_LENGTH } from '../format/names';
  import { plural } from '../format/plural';
  import { formatBytes } from '../format/units';
  import { methodColor } from '../request/methods';
  import { isWsdlUrl, looksLikeWsdl } from '../request/soap';
  import { previewWsdlImport, runWsdlImport, type WsdlSource } from '../api/soap';
  import { navLabel } from '../state/navigation';
  import { notices } from '../state/notices.svelte';
  import { variableRevision } from '../state/variable-revision.svelte';
  import type { TeamSync } from '../state/team.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import { accountFor } from '../team/account';
  import AccountFields from './AccountFields.svelte';
  import CollectionTarget from './CollectionTarget.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import Dropdown from './Dropdown.svelte';

  interface Props {
    workspace: Workspace;
    team: TeamSync;
    /** A Postman export chosen in the New collection dialog, previewed on arrival. */
    pendingFile: File | null;
    /** The collection a pasted cURL command goes to, when one was chosen before arriving. */
    preferredCollectionId: string | null;
    onPendingTaken: () => void;
    onImported: (result: ImportResult) => void;
    onOpenRequest: (collectionId: string, requestId: string) => void;
  }

  let {
    workspace,
    team,
    pendingFile,
    preferredCollectionId,
    onPendingTaken,
    onImported,
    onOpenRequest,
  }: Props = $props();

  const CURL_PARSE_DELAY_MS = 300;

  // Postman file
  let fileInput: HTMLInputElement | undefined = $state();
  let fileName = $state('');
  let fileSize = $state(0);
  let fileText = $state('');
  // Set for a WSDL, which is read with its imports in the Rust core (by URL when it has one).
  let wsdl = $state<WsdlSource | null>(null);
  let fileError: string | null = $state(null);
  let preview = $state<ImportPreview | null>(null);
  let importName = $state('');
  let importing = $state(false);
  // A collection file can also take the place of what an existing collection holds.
  let replacing = $state(false);
  let replaceId = $state('');
  let confirmReplace = $state(false);
  let dragging = $state(false);
  let fileUrl = $state('');
  let fetching = $state(false);

  // cURL
  let command = $state('');
  let commandInput: HTMLTextAreaElement | undefined = $state();
  let parsed: CurlImport | null = $state(null);
  let curlError: string | null = $state(null);
  let targetCollectionId = $state(initialTarget());
  let targetSectionId: string | null = $state(null);
  let added: { collectionId: string; requestId: string; name: string } | null = $state(null);
  let parseTimer: ReturnType<typeof setTimeout> | undefined;
  // Team collection
  let joinRemote = $state('');
  let joinUsername = $state('');
  let joinName = $state('');
  let joinEmail = $state('');
  let joining = $state(false);
  let joinError: string | null = $state(null);

  function initialTarget(): string {
    return preferredCollectionId ?? '';
  }

  onMount(() => {
    if (pendingFile) {
      void readFile(pendingFile);
      onPendingTaken();
    }
    if (preferredCollectionId) {
      commandInput?.scrollIntoView({ block: 'center' });
      commandInput?.focus();
    }
  });

  $effect(() => {
    const author = team.setup?.author;
    if (author && untrack(() => joinName === '' && joinEmail === '')) {
      joinName = author.name;
      joinEmail = author.email;
    }
  });

  async function joinTeam(): Promise<void> {
    joining = true;
    joinError = null;
    try {
      const joined = await team.join(
        joinRemote,
        accountFor(joinRemote, joinName, joinEmail, joinUsername),
      );
      joinRemote = '';
      joinUsername = '';
      onImported({ kind: 'collection', id: joined.id });
    } catch (error) {
      joinError = toAppError(error).message;
    } finally {
      joining = false;
    }
  }

  // Each new file or edit makes earlier, slower answers stale; those are ignored.
  let fileGeneration = 0;
  let parseGeneration = 0;

  async function readFile(file: File | undefined): Promise<void> {
    const generation = ++fileGeneration;
    resetFile();
    if (!file) {
      return;
    }
    if (file.size > MAX_IMPORT_BYTES) {
      fileError = `The file is ${formatBytes(file.size)}; files up to ${formatBytes(MAX_IMPORT_BYTES)} can be imported.`;
      return;
    }
    await show(generation, file.name, async () => fromText(await file.text(), null));
  }

  async function readUrl(): Promise<void> {
    const url = fileUrl.trim();
    const generation = ++fileGeneration;
    resetFile();
    fetching = true;
    const name = url.split(/[?#]/, 1)[0]?.split('/').filter(Boolean).at(-1) ?? url;
    await show(generation, name, async () => {
      // A WSDL address goes straight to the WSDL reader, which also allows plain http.
      if (isWsdlUrl(url)) {
        const source: WsdlSource = { kind: 'url', url };
        return { text: null, source, planned: await previewWsdlImport(source) };
      }
      return fromText(await fetchImport(url), url);
    });
    fetching = false;
    if (preview) {
      fileUrl = '';
    }
  }

  interface ReadImport {
    /** The file's text; null for a WSDL the Rust core read by its address. */
    text: string | null;
    source: WsdlSource | null;
    planned: ImportPreview;
  }

  async function fromText(text: string, url: string | null): Promise<ReadImport> {
    const source: WsdlSource | null = looksLikeWsdl(text)
      ? url === null
        ? { kind: 'content', content: text }
        : { kind: 'url', url }
      : null;
    const planned = source ? await previewWsdlImport(source) : await previewImport(text);
    return { text, source, planned };
  }

  async function show(
    generation: number,
    name: string,
    read: () => Promise<ReadImport>,
  ): Promise<void> {
    try {
      const { text, source, planned } = await read();
      if (generation !== fileGeneration) {
        return;
      }
      fileName = name;
      fileSize = text === null ? 0 : new TextEncoder().encode(text).length;
      fileText = text ?? '';
      wsdl = source;
      preview = planned;
      importName = planned.name;
    } catch (error) {
      if (generation !== fileGeneration) {
        return;
      }
      fileError = toAppError(error).message;
      fileText = '';
    }
  }

  function resetFile(): void {
    fileName = '';
    fileSize = 0;
    fileText = '';
    wsdl = null;
    fileError = null;
    preview = null;
    importName = '';
    replacing = false;
    confirmReplace = false;
    if (fileInput) {
      fileInput.value = '';
    }
  }

  async function importFile(): Promise<void> {
    if (!preview || importing) {
      return;
    }
    importing = true;
    await notices.guard(async () => {
      const result = wsdl
        ? await runWsdlImport(wsdl, importName)
        : await runImport(fileText, importName);
      resetFile();
      onImported(result);
    });
    importing = false;
  }

  const canReplace = $derived(
    preview?.kind === 'collection' && wsdl === null && workspace.collections.length > 0,
  );
  const replaceTarget = $derived(
    workspace.collections.find((collection) => collection.id === replaceId) ??
      workspace.collections.at(0) ??
      null,
  );

  const replaceMessage = $derived(
    `Its ${plural(replaceTarget?.requestCount ?? 0, 'request')}, sections, auth, docs and variables are removed, and the ${plural(preview?.requestCount ?? 0, 'request')} from ${fileName} take their place. A shared collection keeps the old version in its Git history.`,
  );

  async function replaceCollection(): Promise<void> {
    confirmReplace = false;
    const target = replaceTarget;
    if (!preview || !target || importing) {
      return;
    }
    importing = true;
    await notices.guard(async () => {
      const result = await replaceWithImport(fileText, target.id);
      resetFile();
      variableRevision.bump();
      // A shared collection sends the replacement on its next sync.
      workspace.notifyChanged(target.id);
      onImported(result);
    });
    importing = false;
  }

  function handleDrop(event: DragEvent): void {
    event.preventDefault();
    dragging = false;
    void readFile(event.dataTransfer?.files[0]);
  }

  function scheduleParse(): void {
    clearTimeout(parseTimer);
    const generation = ++parseGeneration;
    added = null;
    const text = command.trim();
    if (text === '') {
      parsed = null;
      curlError = null;
      return;
    }
    parseTimer = setTimeout(() => {
      parseCurl(text).then(
        (result) => {
          if (generation === parseGeneration) {
            parsed = result;
            curlError = null;
          }
        },
        (error: unknown) => {
          if (generation === parseGeneration) {
            parsed = null;
            curlError = toAppError(error).message;
          }
        },
      );
    }, CURL_PARSE_DELAY_MS);
  }

  async function addCurlRequest(): Promise<void> {
    const request = parsed?.request;
    const collectionId = targetCollectionId;
    if (!request || collectionId === '') {
      return;
    }
    await notices.guard(async () => {
      const saved = await createRequest(collectionId, targetSectionId, request);
      await workspace.refresh(collectionId);
      added = { collectionId, requestId: saved.id, name: saved.name };
      command = '';
      parsed = null;
    });
  }
</script>

<div class="view">
  <header>
    <h1 class="page-title"><NavIcon name="import" size="lg" />{navLabel('import')}</h1>
    <p>
      Bring in a collection, an environment or a single cURL command. Files are read on this device
      and nothing is uploaded. To save a collection as a file, use Export in that collection.
    </p>
  </header>

  <div class="panels">
    <section class="panel" aria-labelledby="file-title">
      <h2 id="file-title">Collection, environment or API spec</h2>
      <p class="hint">
        A collection exported from Codestamp, a Collection v2.0 or v2.1 or environment file from
        your API tool, an OpenAPI 3 or Swagger 2.0 spec in JSON or YAML, or a SOAP WSDL, which
        becomes a section per binding with a request per operation.
      </p>

      <input
        bind:this={fileInput}
        class="visually-hidden"
        type="file"
        accept=".json,.yaml,.yml,.wsdl,.xml,application/json,application/yaml,application/xml"
        aria-label="Collection or environment file"
        onchange={(event) => void readFile(event.currentTarget.files?.[0])}
      />

      {#if preview}
        <div class="file">
          <div class="file-text">
            <span class="file-name">{fileName}</span>
            <span class="hint"
              >{fileSize > 0 ? `${formatBytes(fileSize)} · ` : ''}{preview.source}</span
            >
          </div>
          <button type="button" class="secondary" onclick={resetFile}>Choose another file</button>
        </div>

        <div class="stats">
          {#if preview.kind === 'collection'}
            <div class="stat"><b>{preview.requestCount}</b><span>requests</span></div>
            <div class="stat"><b>{preview.sectionCount}</b><span>sections</span></div>
            <div class="stat"><b>{preview.scriptCount}</b><span>scripts</span></div>
          {/if}
          <div class="stat"><b>{preview.variableCount}</b><span>variables</span></div>
          {#if preview.environmentCount > 0}
            <div class="stat"><b>{preview.environmentCount}</b><span>environments</span></div>
          {/if}
        </div>

        {#if preview.warnings.length > 0}
          <details class="warnings" open={preview.warnings.length <= 5}>
            <summary>{preview.warnings.length} things to review</summary>
            <ul>
              {#each preview.warnings as warning, index (index)}
                <li>{warning}</li>
              {/each}
            </ul>
          </details>
        {:else}
          <p class="ok">✓ Everything in this file can be imported.</p>
        {/if}

        {#if canReplace}
          <fieldset class="mode">
            <legend>Import as</legend>
            <label>
              <input type="radio" name="import-mode" value={false} bind:group={replacing} />
              A new collection
            </label>
            <label>
              <input type="radio" name="import-mode" value={true} bind:group={replacing} />
              The content of an existing collection
            </label>
          </fieldset>
        {/if}

        {#if replacing && replaceTarget}
          <div class="field">
            <span>Collection to replace</span>
            <Dropdown
              variant="field"
              label="Collection to replace"
              value={replaceTarget.id}
              options={workspace.collections.map((collection) => ({
                value: collection.id,
                label: `${collection.icon} ${collection.name}`,
              }))}
              onChange={(id: string) => (replaceId = id)}
            />
          </div>
          <p class="hint">
            “{replaceTarget.name}” keeps its name, icon and Collaboration link. Its requests,
            sections, auth, docs and variables{preview.environmentCount > 0
              ? ' and its own environments'
              : ''} are replaced by the file's. A shared collection sends this as an ordinary change on
            its next sync, so the old version stays in its Git history.
          </p>
        {:else}
          <label class="field">
            {preview.kind === 'collection' ? 'Collection name' : 'Environment name'}
            <input bind:value={importName} maxlength={MAX_NAME_LENGTH} />
          </label>
        {/if}

        <div class="actions">
          <button type="button" class="secondary" onclick={resetFile}>Cancel</button>
          {#if replacing && replaceTarget}
            <button
              type="button"
              class="primary"
              disabled={importing}
              onclick={() => (confirmReplace = true)}
            >
              {importing ? 'Replacing…' : `Replace with ${plural(preview.requestCount, 'request')}`}
            </button>
          {:else}
            <button
              type="button"
              class="primary"
              disabled={importing || importName.trim() === ''}
              onclick={() => void importFile()}
            >
              {importing
                ? 'Importing…'
                : preview.kind === 'collection'
                  ? `Import ${plural(preview.requestCount, 'request')}`
                  : `Import ${plural(preview.variableCount, 'variable')}`}
            </button>
          {/if}
        </div>
      {:else}
        <button
          type="button"
          class="dropzone"
          class:dragging
          onclick={() => {
            fileInput?.click();
          }}
          ondragover={(event) => {
            event.preventDefault();
            dragging = true;
          }}
          ondragleave={() => (dragging = false)}
          ondrop={handleDrop}
        >
          <svg width="28" height="28" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 16V4M7 9l5-5 5 5M4 20h16" />
          </svg>
          <span class="drop-title">Drop a .json, .yaml or .wsdl file here or click to choose</span>
          <span class="hint">Up to {formatBytes(MAX_IMPORT_BYTES)}</span>
        </button>
        <form
          class="from-url"
          onsubmit={(event) => {
            event.preventDefault();
            void readUrl();
          }}
        >
          <label class="field url-field">
            Or from a URL
            <input
              bind:value={fileUrl}
              class="mono"
              type="url"
              placeholder="https://api.example.com/openapi.json or …/Service.asmx?wsdl"
              spellcheck="false"
              autocomplete="off"
            />
          </label>
          <button type="submit" class="secondary" disabled={fetching || fileUrl.trim() === ''}>
            {fetching ? 'Fetching…' : 'Fetch'}
          </button>
        </form>
        {#if fileError}
          <p class="error" role="alert">{fileError}</p>
        {/if}
      {/if}
    </section>

    <section class="panel" aria-labelledby="curl-title">
      <h2 id="curl-title">Paste cURL</h2>
      <label class="field">
        Command
        <textarea
          bind:this={commandInput}
          bind:value={command}
          oninput={scheduleParse}
          rows="6"
          spellcheck="false"
          placeholder="curl 'https://api.example.com/v1/users' -H 'Accept: application/json'"
        ></textarea>
      </label>

      {#if curlError}
        <p class="error" role="alert">{curlError}</p>
      {:else if parsed}
        <div class="parsed">
          <div class="parsed-line">
            <span class="method mono" style:color={methodColor(parsed.request.method)}
              >{parsed.request.method}</span
            >
            <span class="mono url">{parsed.request.url}</span>
          </div>
          <span class="hint">
            {parsed.request.params.length} params · {parsed.request.headers.length} headers ·
            {parsed.request.bodyMode === 'none' ? 'no body' : `${parsed.request.bodyMode} body`}
          </span>
          {#each parsed.warnings as warning, index (index)}
            <span class="warn">! {warning}</span>
          {/each}
        </div>
        <CollectionTarget
          collections={workspace.collections}
          bind:collectionId={targetCollectionId}
          bind:sectionId={targetSectionId}
        />
        <div class="actions">
          <button
            type="button"
            class="primary"
            disabled={targetCollectionId === ''}
            onclick={() => void addCurlRequest()}>Add request</button
          >
        </div>
      {/if}

      {#if added}
        {@const created = added}
        <p class="ok">
          ✓ Added “{created.name}”.
          <button
            type="button"
            class="link"
            onclick={() => {
              onOpenRequest(created.collectionId, created.requestId);
            }}>Open it</button
          >
        </p>
      {/if}
    </section>

    <section class="panel" aria-labelledby="join-title">
      <h2 id="join-title">Join a shared collection</h2>
      <p class="hint">
        Paste the address of a repository a collaborator shared from Codestamp. Your Git account
        needs access to it; Git asks you to sign in the first time.
      </p>
      {#if team.setup && team.setup.gitVersion === null}
        <p class="error">
          Git is not installed on this device. Install Git for Windows from git-scm.com, then reopen
          Codestamp.
        </p>
      {:else}
        <label class="field">
          Repository address
          <input
            bind:value={joinRemote}
            class="mono"
            placeholder="https://github.com/team/shop-api.git"
            spellcheck="false"
            autocomplete="off"
          />
        </label>
        <AccountFields
          remote={joinRemote}
          bind:username={joinUsername}
          bind:name={joinName}
          bind:email={joinEmail}
        />
        {#if joinError}
          <p class="error" role="alert">{joinError}</p>
        {/if}
        <div class="actions">
          <button
            type="button"
            class="primary"
            disabled={joining ||
              joinRemote.trim() === '' ||
              joinName.trim() === '' ||
              joinEmail.trim() === ''}
            onclick={() => void joinTeam()}
          >
            {joining ? 'Joining…' : 'Join'}
          </button>
        </div>
      {/if}
    </section>
  </div>
</div>

<ConfirmDialog
  open={confirmReplace}
  title="Replace “{replaceTarget?.name ?? ''}”?"
  message={replaceMessage}
  confirmLabel="Replace"
  onConfirm={() => void replaceCollection()}
  onCancel={() => (confirmReplace = false)}
/>

<style>
  .mode {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin: 0;
    padding: 0;
    border: 0;
    font-size: 13px;
    color: var(--text-2);
  }

  .mode legend {
    width: 100%;
    margin-bottom: 6px;
    padding: 0;
  }

  .mode label {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .mode input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }

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
    line-height: 1.5;
  }

  .panels {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    align-items: flex-start;
  }

  .panel {
    flex: 1 1 400px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .hint {
    margin: -8px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .from-url {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px;
    margin-top: 12px;
  }

  .url-field {
    flex: 1 1 260px;
  }

  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 32px 16px;
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface-muted);
    color: var(--text-2);
  }

  .dropzone.dragging,
  .dropzone:hover {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .dropzone svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .drop-title {
    font-weight: 500;
  }

  .dropzone .hint {
    margin: 0;
  }

  .file {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
  }

  .file-text {
    flex: 1 1 200px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .file-name {
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 90px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--bg);
  }

  .stat b {
    font-size: 18px;
    font-weight: 600;
  }

  .stat span {
    font-size: 12px;
    color: var(--text-muted);
  }

  .warnings {
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--warn-bg);
    color: var(--warn);
    font-size: 13px;
  }

  .warnings summary {
    cursor: pointer;
    font-weight: 600;
  }

  .warnings ul {
    margin: 8px 0 0;
    padding-left: 18px;
    color: var(--text-2);
  }

  .warnings li {
    margin: 4px 0;
  }

  .ok {
    margin: 0;
    color: var(--ok);
    font-size: 13px;
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .field input {
    height: 38px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    font-size: 14px;
  }

  textarea {
    padding: 10px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    font-family: var(--font-code);
    font-size: var(--font-size-code);
    line-height: 1.6;
    resize: vertical;
  }

  .parsed {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius-md);
    background: var(--bg);
  }

  .parsed .hint {
    margin: 0;
  }

  .parsed-line {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }

  .method {
    flex: none;
    font-weight: 600;
  }

  .url {
    overflow-wrap: anywhere;
  }

  .warn {
    font-size: 12px;
    color: var(--warn);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .primary,
  .secondary {
    height: 36px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 500;
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .primary:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-weight: 500;
  }
</style>
