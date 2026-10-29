<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { exportDocs, type RequestDocs } from '../api/docs';
  import { toAppError } from '../api/http';
  import { filterDocs } from '../docs/filter';
  import { revealElement } from '../dom/reveal';
  import { plural } from '../format/plural';
  import { documentedValue } from '../privacy/secrets';
  import { authSummary } from '../request/auth';
  import { stampLabel } from '../request/kinds';
  import { stampTone } from '../request/methods';
  import type { Docs } from '../state/docs.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import DescriptionBlock from './docs/DescriptionBlock.svelte';
  import IndexEntry from './docs/IndexEntry.svelte';
  import RequestDoc from './docs/RequestDoc.svelte';
  import Dropdown, { type DropdownOption } from './Dropdown.svelte';
  import FoldButton from './FoldButton.svelte';
  import MethodStamp from './MethodStamp.svelte';
  import NavIcon from './NavIcon.svelte';

  interface Props {
    docs: Docs;
    workspace: Workspace;
    onOpenRequest: (collectionId: string, requestId: string) => void;
    onNewCollection: () => void;
  }

  let { docs, workspace, onOpenRequest, onNewCollection }: Props = $props();

  let exportNote = $state<{ text: string; failed: boolean } | null>(null);
  let exporting = $state(false);
  let docQuery = $state('');
  let findField: HTMLInputElement | undefined = $state();
  let docColumn: HTMLElement | undefined = $state();
  let indexNav: HTMLElement | undefined = $state();

  const current = $derived(docs.current);
  const filter = $derived(current ? filterDocs(current, docQuery) : null);
  const collectionOptions = $derived<DropdownOption<string>[]>(
    workspace.collections.map((collection) => ({
      value: collection.id,
      label: `${collection.icon} ${collection.name}`,
    })),
  );
  const loose = $derived(
    current?.requests.filter((request) => request.sectionId === null && shows(request.id)) ?? [],
  );
  const sections = $derived(
    current?.sections.filter((section) => filter === null || filter.sectionIds.has(section.id)) ??
      [],
  );
  /** The requests the index lists, in its order; Previous and Next walk through them. */
  const listed = $derived([
    ...loose,
    ...sections.flatMap((section) => sectionRequests(section.id)),
  ]);
  const nothingFound = $derived(
    filter !== null && filter.requestIds.size === 0 && filter.sectionIds.size === 0,
  );
  const selectedRequest = $derived(
    current?.requests.find((request) => request.id === docs.selected),
  );
  const selectedSection = $derived(
    current?.sections.find((section) => section.id === docs.selected),
  );
  const position = $derived(
    selectedRequest ? listed.findIndex((request) => request.id === selectedRequest.id) : -1,
  );
  const previous = $derived(position > 0 ? listed.at(position - 1) : undefined);
  const next = $derived(position >= 0 ? listed.at(position + 1) : undefined);

  function shows(requestId: string): boolean {
    return filter === null || filter.requestIds.has(requestId);
  }

  function sectionRequests(sectionId: string): RequestDocs[] {
    return (
      current?.requests.filter((request) => request.sectionId === sectionId && shows(request.id)) ??
      []
    );
  }

  function allIn(sectionId: string): number {
    return current?.requests.filter((request) => request.sectionId === sectionId).length ?? 0;
  }

  /** Unfolds the section a request sits in, so the request shows in the index. */
  function openSectionOf(id: string | null): void {
    const sectionId = current?.requests.find((request) => request.id === id)?.sectionId;
    if (sectionId) {
      docs.openSections.add(sectionId);
    }
  }

  /** Shows one doc; a long one may have been scrolled, so the next starts at its top. */
  async function show(id: string | null): Promise<void> {
    docs.selected = id;
    openSectionOf(id);
    await tick();
    // Beside the sticky index, a doc whose top went above the index's top was scrolled past.
    if (docColumn && indexNav) {
      const scrolledPast =
        docColumn.getBoundingClientRect().top < indexNav.getBoundingClientRect().top - 1;
      if (scrolledPast) {
        docColumn.scrollIntoView({ block: 'start' });
      }
    }
  }

  // An item picked in quick search: its doc is scrolled to and focused once it is showing.
  $effect(() => {
    const docsOf = current;
    if (!docs.revealPending || docs.loading || !docsOf || docsOf.id !== docs.collectionId) {
      return;
    }
    docs.revealPending = false;
    docQuery = '';
    openSectionOf(docs.selected);
    const id = docs.selected ?? docsOf.id;
    void tick().then(() => {
      const target = document.getElementById(`doc-${id}`);
      if (target) {
        revealElement(target);
      }
    });
  });

  /** Up and Down move through the index and show each doc on the way. */
  function moveInIndex(event: KeyboardEvent): void {
    if ((event.key !== 'ArrowDown' && event.key !== 'ArrowUp') || !docColumn) {
      return;
    }
    const index = event.currentTarget;
    if (!(index instanceof HTMLElement)) {
      return;
    }
    const entries = [...index.querySelectorAll<HTMLButtonElement>('button.entry')];
    const at = entries.findIndex((entry) => entry === document.activeElement);
    if (at === -1) {
      return;
    }
    event.preventDefault();
    const step = event.key === 'ArrowDown' ? 1 : -1;
    const target = entries.at(Math.min(entries.length - 1, Math.max(0, at + step)));
    target?.focus();
    target?.click();
  }

  function focusFind(event: KeyboardEvent): void {
    const find = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f';
    if (!find || !findField || document.querySelector('dialog[open]') !== null) {
      return;
    }
    event.preventDefault();
    findField.focus();
    findField.select();
  }

  function clearFind(): void {
    docQuery = '';
    findField?.focus();
  }

  /** The open collection, else the one picked before, else the first. */
  function startingId(): string | null {
    const ids = workspace.collections.map((collection) => collection.id);
    const wanted = [docs.collectionId, workspace.collection?.id ?? null, ids.at(0) ?? null];
    return wanted.find((id) => id !== null && ids.includes(id)) ?? null;
  }

  // Requests may have changed in their cards since Docs was last open.
  onMount(() => {
    const id = startingId();
    if (id !== null) {
      void docs.open(id);
    }
  });

  function choose(id: string): void {
    exportNote = null;
    docQuery = '';
    void docs.open(id);
  }

  async function save(): Promise<void> {
    const id = current?.id;
    if (!id) {
      return;
    }
    exporting = true;
    exportNote = null;
    try {
      const path = await exportDocs(id);
      if (path !== null) {
        exportNote = { text: `Saved to ${path}`, failed: false };
      }
    } catch (caught) {
      exportNote = { text: toAppError(caught).message, failed: true };
    } finally {
      exporting = false;
    }
  }
</script>

<svelte:window onkeydown={focusFind} />

<div class="view">
  <header class="top">
    <div class="title">
      <h1 class="page-title"><NavIcon name="docs" size="lg" />Docs</h1>
      <p>
        Write what each collection, section and request is for, next to what the request sends.
        Descriptions use Markdown, save on this device and travel with the collection when you share
        it.
      </p>
    </div>
    {#if current}
      <button
        type="button"
        class="quiet"
        disabled={exporting}
        aria-busy={exporting}
        onclick={() => void save()}>{exporting ? 'Saving…' : 'Save as Markdown'}</button
      >
    {/if}
  </header>

  {#if exportNote}
    <p class="note" class:failed={exportNote.failed} role={exportNote.failed ? 'alert' : 'status'}>
      {exportNote.text}
    </p>
  {/if}

  {#if workspace.loaded && workspace.collections.length === 0}
    <section class="empty" aria-label="No collections yet">
      <h2>No collections yet</h2>
      <p>Docs describe the requests of a collection. Create or import one to start writing.</p>
      <button type="button" class="quiet" onclick={onNewCollection}>Create a collection</button>
    </section>
  {:else}
    <div class="layout">
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <nav class="index" aria-label="Docs index" bind:this={indexNav} onkeydown={moveInIndex}>
        <Dropdown
          label="Collection"
          value={docs.collectionId ?? ''}
          options={collectionOptions}
          onChange={choose}
          variant="field"
        />
        {#if current}
          <label class="search">
            <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" />
            </svg>
            <input
              bind:this={findField}
              type="search"
              bind:value={docQuery}
              placeholder="Search docs"
              aria-label="Search the docs of {current.name}"
              aria-describedby="docs-find-result"
              aria-keyshortcuts="Control+F"
              maxlength="200"
              spellcheck="false"
              autocomplete="off"
            />
          </label>
          <p id="docs-find-result" class="find-result" aria-live="polite">
            {#if filter}
              {filter.requestIds.size} of {plural(current.requests.length, 'request')}
            {:else}
              {plural(current.requests.length, 'request')}
            {/if}
          </p>
          <div class="entries">
            <button
              type="button"
              class="entry overview"
              class:on={docs.selected === null}
              aria-current={docs.selected === null ? 'page' : undefined}
              onclick={() => void show(null)}
            >
              <span class="name">Overview</span>
            </button>
            {#each loose as request (request.id)}
              <IndexEntry
                {request}
                nested={false}
                selected={docs.selected === request.id}
                onSelect={() => void show(request.id)}
              />
            {/each}
            {#each sections as section (section.id)}
              <!-- While searching, every section shows what matched. -->
              {@const open = filter !== null || docs.openSections.has(section.id)}
              <div class="group-row">
                <FoldButton
                  expanded={open}
                  label="section {section.name}"
                  title={open ? 'Hide requests' : 'Show requests'}
                  disabled={filter !== null}
                  onToggle={() => {
                    docs.openSections.toggle(section.id);
                  }}
                />
                <button
                  type="button"
                  class="entry group"
                  class:on={docs.selected === section.id}
                  aria-current={docs.selected === section.id ? 'page' : undefined}
                  title={section.name}
                  onclick={() => {
                    docs.openSections.add(section.id);
                    void show(section.id);
                  }}
                >
                  <span class="name">{section.name}</span>
                  <span class="count">{allIn(section.id)}</span>
                </button>
              </div>
              {#if open}
                {#each sectionRequests(section.id) as request (request.id)}
                  <IndexEntry
                    {request}
                    nested={true}
                    selected={docs.selected === request.id}
                    onSelect={() => void show(request.id)}
                  />
                {/each}
              {/if}
            {/each}
            {#if nothingFound}
              <div class="no-match" role="status">
                <p>Nothing in these docs matches “{docQuery.trim()}”.</p>
                <button type="button" class="quiet" onclick={clearFind}>Clear search</button>
              </div>
            {/if}
          </div>
        {/if}
      </nav>

      <div class="document" bind:this={docColumn}>
        {#if docs.error}
          <div class="problem" role="alert">
            <span>These docs could not load: {docs.error}</span>
            <button type="button" class="quiet" onclick={() => void docs.reload()}>Try again</button
            >
          </div>
        {:else if !current}
          <p class="muted" role="status">
            {docs.loading ? 'Loading the docs…' : 'Pick a collection.'}
          </p>
        {:else if selectedRequest}
          {#key selectedRequest.id}
            <RequestDoc
              {docs}
              request={selectedRequest}
              collectionAuth={current.auth}
              onOpenRequest={(requestId: string) => {
                onOpenRequest(current.id, requestId);
              }}
            />
          {/key}
          {#if previous || next}
            <nav class="pager" aria-label="Previous and next request">
              {#if previous}
                {@const target = previous}
                <button type="button" class="pager-link" onclick={() => void show(target.id)}>
                  <span class="hint">← Previous</span>
                  <span class="name">{target.name}</span>
                </button>
              {/if}
              {#if next}
                {@const target = next}
                <button type="button" class="pager-link next" onclick={() => void show(target.id)}>
                  <span class="hint">Next →</span>
                  <span class="name">{target.name}</span>
                </button>
              {/if}
            </nav>
          {/if}
        {:else if selectedSection}
          <section
            class="card section-doc"
            id={`doc-${selectedSection.id}`}
            tabindex="-1"
            aria-labelledby="doc-section-title"
          >
            <header>
              <h2 id="doc-section-title">{selectedSection.name}</h2>
              <p class="meta">{plural(allIn(selectedSection.id), 'request')}</p>
            </header>
            {#key selectedSection.id}
              <DescriptionBlock
                {docs}
                item="section"
                id={selectedSection.id}
                text={selectedSection.description}
                subject="this section"
                headingOffset={2}
              />
            {/key}
            <ul class="links">
              {#each current.requests.filter((request) => request.sectionId === selectedSection.id) as request (request.id)}
                <li>
                  <button type="button" class="link-row" onclick={() => void show(request.id)}>
                    <MethodStamp
                      label={stampLabel(request.kind, request.method)}
                      tone={stampTone(request.kind, request.method)}
                      size="sm"
                    />
                    <span class="name">{request.name}</span>
                    <span class="url mono">{request.url}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {:else}
          <article
            class="card collection"
            id={`doc-${current.id}`}
            tabindex="-1"
            aria-labelledby="doc-collection-title"
          >
            <header class="collection-head">
              <span class="icon emoji" aria-hidden="true">{current.icon}</span>
              <div>
                <h2 id="doc-collection-title">{current.name}</h2>
                <p class="meta">
                  {plural(current.requests.length, 'request')}{current.sections.length > 0
                    ? ` in ${plural(current.sections.length, 'section')}`
                    : ''}
                </p>
              </div>
            </header>
            <DescriptionBlock
              {docs}
              item="collection"
              id={current.id}
              text={current.description}
              subject="this collection"
              headingOffset={2}
            />
            {#if current.auth.type !== 'none'}
              <p class="fact"><span>Auth</span>{authSummary(current.auth)}</p>
            {/if}
            {#if current.variables.length > 0}
              <details class="variables">
                <summary>Variables <span class="count">{current.variables.length}</span></summary>
                <table>
                  <thead>
                    <tr><th scope="col">Name</th><th scope="col">Value</th></tr>
                  </thead>
                  <tbody>
                    {#each current.variables as variable (variable.key)}
                      <tr>
                        <td class="mono">{`{{${variable.key}}}`}</td>
                        {#if variable.secret}
                          <td class="muted-cell">Secret, set on each device</td>
                        {:else}
                          <td class="mono">{documentedValue(variable.key, variable.value)}</td>
                        {/if}
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </details>
            {/if}
            {#if current.sections.length > 0}
              <div class="sections">
                <h3>Sections</h3>
                <ul class="links">
                  {#each current.sections as section (section.id)}
                    <li>
                      <button type="button" class="link-row" onclick={() => void show(section.id)}>
                        <span class="name">{section.name}</span>
                        <span class="count">{plural(allIn(section.id), 'request')}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}
          </article>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
  }

  .title {
    flex: 1 1 360px;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .title p {
    margin: 4px 0 0;
    max-width: 680px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .quiet {
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
  }

  .quiet:hover:not(:disabled) {
    border-color: var(--border-strong);
  }

  .quiet:active:not(:disabled) {
    background: var(--surface-sunken);
  }

  .quiet:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .note {
    margin: 0;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .note.failed {
    background: var(--err-bg);
    color: var(--err);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 28px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
  }

  .empty h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .empty p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.5;
  }

  /* Index and doc side by side from 700px (280px index, 20px gap, 400px doc); stacked below. */
  .layout {
    container-type: inline-size;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 20px;
  }

  .index {
    flex: 1 1 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* A long index scrolls on its own; padding keeps focus rings clear of the scroll edge. */
  .entries {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-height: 0;
    max-height: 260px;
    padding: 3px;
    margin: -3px;
    overflow-y: auto;
  }

  @container (min-width: 700px) {
    .index {
      flex: 0 1 280px;
      position: sticky;
      top: 16px;
      max-height: calc(100vh - var(--titlebar-height) - var(--statusbar-height) - 32px);
    }

    .entries {
      flex: 1 1 auto;
      max-height: none;
    }
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
  }

  .search:focus-within {
    border-color: var(--focus);
  }

  .search svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  /* The label's border turns blue on focus instead. */
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    outline: none;
  }

  .search input::placeholder {
    color: var(--text-muted);
  }

  .find-result {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 2px;
    color: var(--text-muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 4px 8px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }

  .entry:hover {
    background: var(--surface-sunken);
  }

  .entry.on {
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text);
    font-weight: 500;
  }

  .group-row {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-top: 10px;
  }

  .entry.group {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-weight: 600;
  }

  .entry .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    flex: none;
    font-family: var(--font-code);
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .no-match {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    margin-top: 8px;
    padding: 12px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
  }

  .no-match p {
    margin: 0;
    color: var(--text-2);
    font-size: 13px;
  }

  .document {
    flex: 1 1 400px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-margin-top: 16px;
  }

  .problem {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid var(--err-line);
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .problem span {
    flex: 1 1 240px;
  }

  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    scroll-margin-top: 16px;
  }

  .collection-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon {
    flex: none;
    width: 44px;
    height: 44px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
  }

  .card h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .meta {
    margin: 2px 0 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .fact {
    display: flex;
    gap: 12px;
    margin: 0;
    padding-top: 12px;
    border-top: 1px solid var(--border-soft);
    color: var(--text-2);
    font-size: 13px;
  }

  .fact span {
    width: 80px;
    color: var(--text-muted);
  }

  .variables {
    padding-top: 12px;
    border-top: 1px solid var(--border-soft);
    font-size: 13px;
  }

  .variables summary {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    cursor: pointer;
  }

  .variables table {
    width: 100%;
    margin-top: 10px;
    border-collapse: collapse;
    font-size: 12px;
  }

  .variables th {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    font-weight: 500;
    text-align: left;
  }

  .variables td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-soft);
    color: var(--text-2);
    overflow-wrap: anywhere;
    vertical-align: top;
  }

  .muted-cell {
    color: var(--text-muted);
  }

  .sections h3 {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .sections {
    padding-top: 12px;
    border-top: 1px solid var(--border-soft);
  }

  .links {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .link-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 34px;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text);
    font-size: 13px;
    text-align: left;
  }

  .link-row:hover {
    background: var(--surface-sunken);
  }

  .link-row .name {
    flex: none;
    max-width: 50%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .link-row .url {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: 12px;
  }

  .link-row .count {
    margin-left: auto;
  }

  .section-doc header p {
    margin-top: 2px;
  }

  .pager {
    display: flex;
    gap: 12px;
  }

  .pager-link {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    max-width: 48%;
    min-width: 0;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    text-align: left;
  }

  .pager-link:hover {
    border-color: var(--border-strong);
  }

  .pager-link.next {
    margin-left: auto;
    align-items: flex-end;
    text-align: right;
  }

  .pager-link .hint {
    color: var(--text-muted);
    font-size: 12px;
  }

  .pager-link .name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }
</style>
