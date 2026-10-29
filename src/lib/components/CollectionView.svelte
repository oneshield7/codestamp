<script lang="ts">
  import { tick } from 'svelte';
  import type { RequestKind, RequestSummary, SectionInfo } from '../api/collections';
  import { revealCard } from '../dom/reveal';
  import { isInEditor, openFindIn } from '../editor/search';
  import type { HttpResponseOutput } from '../api/http';
  import type { Environments } from '../state/environments.svelte';
  import { panelRequest } from '../state/panel-request.svelte';
  import type { TeamSync } from '../state/team.svelte';
  import { summarizeSync } from '../team/status';
  import { ADDABLE_KINDS } from '../request/kind-icons';
  import { SKIPPED_IN_RUNS } from '../request/kinds';
  import { collectionGroups } from '../request/order';
  import { focusMode } from '../state/focus.svelte';
  import { collapsedSections } from '../state/sections.svelte';
  import FoldButton from './FoldButton.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import CollectionAuth from './CollectionAuth.svelte';
  import CollectionExport from './CollectionExport.svelte';
  import CollectionHistory from './CollectionHistory.svelte';
  import CollectionStart from './CollectionStart.svelte';
  import CollectionVariables from './CollectionVariables.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import ConflictDialog from './ConflictDialog.svelte';
  import DuplicateDialog from './DuplicateDialog.svelte';
  import EditableText from './EditableText.svelte';
  import EmojiPicker from './EmojiPicker.svelte';
  import IconButton from './IconButton.svelte';
  import EnvironmentSelect from './EnvironmentSelect.svelte';
  import RequestCard from './RequestCard.svelte';
  import RequestHistoryDialog from './RequestHistoryDialog.svelte';
  import RequestRow from './RequestRow.svelte';
  import MockPanel from './MockPanel.svelte';
  import TeamPanel from './TeamPanel.svelte';
  import { mockStatus, type MockStatus } from '../api/mock';
  import { NAV_ICON_PATHS } from '../state/navigation';
  import { KEEP_DAYS } from '../api/trash';
  import { plural } from '../format/plural';

  interface Props {
    workspace: Workspace;
    environments: Environments;
    team: TeamSync;
    onRunAll: () => void;
    /** Opens Import with this collection chosen for the pasted command. */
    onPasteCurl: () => void;
  }

  let { workspace, environments, team, onRunAll, onPasteCurl }: Props = $props();

  type ListItem =
    | { type: 'section'; key: string; section: SectionInfo; count: number; collapsed: boolean }
    | { type: 'request'; key: string; step: number; request: RequestSummary }
    | { type: 'end'; key: string; section: SectionInfo | null; empty: boolean }
    | { type: 'sectionsEnd'; key: string };

  type PendingDelete =
    | { kind: 'collection'; name: string }
    | { kind: 'section'; id: string; name: string }
    | { kind: 'request'; id: string; name: string };

  let pendingDelete: PendingDelete | null = $state(null);
  /** What the open Duplicate dialog copies. */
  let pendingDuplicate: {
    kind: 'collection' | 'section' | 'request';
    id: string;
    name: string;
  } | null = $state(null);

  const collection = $derived(workspace.collection);
  const link = $derived(collection ? team.links.get(collection.id) : undefined);

  // Whether this collection's mock server runs, for the header button.
  let mockUrl = $state<string | null>(null);
  $effect(() => {
    const id = collection?.id;
    mockUrl = null;
    if (id === undefined) {
      return;
    }
    let cancelled = false;
    mockStatus(id).then(
      (status) => {
        if (!cancelled) {
          mockUrl = status?.url ?? null;
        }
      },
      () => {
        if (!cancelled) {
          mockUrl = null;
        }
      },
    );
    return () => {
      cancelled = true;
    };
  });
  const teamSummary = $derived(
    link && collection
      ? summarizeSync(
          link,
          team.outcomes.get(collection.id),
          team.syncing.has(collection.id),
          team.unsent.has(collection.id),
        )
      : null,
  );
  const canRun = $derived(
    collection?.requests.some((request) => !SKIPPED_IN_RUNS.has(request.kind)) ?? false,
  );
  let resolving = $state(false);
  let pickingIcon = $state(false);
  let historyOf: { id: string; name: string } | null = $state(null);

  // Steps run across all sections, in notebook order.
  const items = $derived.by((): ListItem[] => {
    if (!collection) {
      return [];
    }
    const list: ListItem[] = [];
    let step = 0;
    for (const group of collectionGroups(collection)) {
      const { section, requests } = group;
      const collapsed = section ? collapsedSections.has(section.id) : false;
      if (section) {
        list.push({
          type: 'section',
          key: `s:${section.id}`,
          section,
          count: requests.length,
          collapsed,
        });
      }
      if (collapsed) {
        // A folded section's requests keep their step numbers but are not listed.
        step += requests.length;
        continue;
      }
      for (const request of requests) {
        step += 1;
        list.push({ type: 'request', key: `r:${request.id}`, step, request });
      }
      const end = `e:${section?.id ?? 'loose'}`;
      list.push({ type: 'end', key: end, section, empty: requests.length === 0 });
    }
    list.push({ type: 'sectionsEnd', key: 'sections-end' });
    return list;
  });

  const duplicateInfo = $derived.by(() => {
    const target = pendingDuplicate;
    const current = collection;
    if (!target || !current) {
      return { details: '', taken: [] };
    }
    switch (target.kind) {
      case 'collection':
        return {
          details: `${plural(current.requests.length, 'request')}, ${plural(current.sections.length, 'section')}, the auth and the collection variables (secret values too) are copied.`,
          taken: workspace.collections.map((item) => item.name),
        };
      case 'section':
        return {
          details: `The copy holds copies of its ${plural(current.requests.filter((request) => request.sectionId === target.id).length, 'request')} and goes right after “${target.name}”.`,
          taken: current.sections.map((section) => section.name),
        };
      case 'request':
        return {
          details:
            'Params, headers, body, auth and scripts are copied. The copy goes right after the original and opens.',
          taken: current.requests.map((request) => request.name),
        };
    }
  });

  async function confirmDuplicate(name: string): Promise<void> {
    const target = pendingDuplicate;
    if (target?.kind === 'collection') {
      await workspace.duplicateCollection(name);
    } else if (target?.kind === 'section') {
      await workspace.duplicateSection(target.id, name);
    } else if (target) {
      focusMode.exit();
      await workspace.duplicateRequest(target.id, name);
      await revealCard(workspace.openRequestId);
    }
  }

  /** A request opened from History may sit far below the panel. */
  async function openFromHistory(requestId: string): Promise<void> {
    focusMode.exit();
    await workspace.openRequest(requestId);
    await revealCard(workspace.openRequestId);
  }

  /** The new card opens where the request lands, often far below the button that added it. */
  async function addRequest(sectionId: string | null, kind: RequestKind): Promise<void> {
    await workspace.addRequest(sectionId, kind);
    await revealCard(workspace.openRequestId);
  }

  const deleteMessage = $derived.by(() => {
    switch (pendingDelete?.kind) {
      case 'collection':
        return `“${pendingDelete.name}” and all of its requests will be deleted. You can restore it from Recently deleted for ${String(KEEP_DAYS)} days.`;
      case 'section':
        return `The section “${pendingDelete.name}” will be removed. Its requests stay in the collection.`;
      case 'request':
        return `“${pendingDelete.name}” will be deleted. You can restore it from Recently deleted for ${String(KEEP_DAYS)} days.`;
      default:
        return '';
    }
  });

  type DragKind = 'request' | 'section';
  let dragging: { kind: DragKind; id: string } | null = $state(null);
  let dropHint: string | null = $state(null);
  type Panel = 'variables' | 'auth' | 'team' | 'mock' | 'history' | 'export';
  let openPanel: Panel | null = $state(null);

  $effect(() => {
    const requested = collection ? panelRequest.take(collection.id) : null;
    if (requested !== null) {
      openPanel = requested;
    }
  });

  function togglePanel(panel: Panel): void {
    openPanel = openPanel === panel ? null : panel;
  }

  function startDrag(event: DragEvent, kind: DragKind, id: string): void {
    dragging = { kind, id };
    event.dataTransfer?.setData('text/plain', id);
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function endDrag(): void {
    dragging = null;
    dropHint = null;
  }

  function allowDrop(event: DragEvent, target: string, only?: DragKind): void {
    if (!dragging || (only && dragging.kind !== only)) {
      return;
    }
    event.preventDefault();
    dropHint = target;
  }

  function dropRequest(event: DragEvent, sectionId: string | null, beforeId: string | null): void {
    event.preventDefault();
    const source = dragging;
    endDrag();
    if (source?.kind === 'request') {
      void workspace.moveRequest(source.id, sectionId, beforeId);
    }
  }

  // A section header accepts sections (reorder) and requests (move into that section).
  function dropOnSection(event: DragEvent, sectionId: string): void {
    event.preventDefault();
    const source = dragging;
    endDrag();
    if (source?.kind === 'section') {
      void workspace.moveSection(source.id, sectionId);
    } else if (source) {
      void workspace.moveRequest(source.id, sectionId, null);
    }
  }

  // Moving re-renders the list, so focus is put back on the moved item for the next keypress.
  // A request opened inside a folded section (from History, a duplicate, a run) unfolds it.
  $effect(() => {
    const openId = workspace.openRequestId;
    const sectionId = collection?.requests.find((request) => request.id === openId)?.sectionId;
    if (sectionId) {
      collapsedSections.delete(sectionId);
    }
  });

  async function nudgeRequest(id: string, step: -1 | 1): Promise<void> {
    await workspace.nudgeRequest(id, step);
    // Moving into a folded section unfolds it, so the request stays in view and keeps focus.
    const sectionId = workspace.collection?.requests.find(
      (request) => request.id === id,
    )?.sectionId;
    if (sectionId) {
      collapsedSections.delete(sectionId);
    }
    await tick();
    document.querySelector<HTMLElement>(`[data-request-id="${id}"]`)?.focus();
  }

  async function nudgeSection(id: string, step: -1 | 1): Promise<void> {
    await workspace.nudgeSection(id, step);
    await tick();
    document.querySelector<HTMLElement>(`[data-section-id="${id}"]`)?.focus();
  }

  function dropSectionAtEnd(event: DragEvent): void {
    event.preventDefault();
    const source = dragging;
    endDrag();
    if (source?.kind === 'section') {
      void workspace.moveSection(source.id, null);
    }
  }

  function nudgeWithKeys(event: KeyboardEvent, nudge: (step: -1 | 1) => Promise<void>): void {
    if (!event.altKey || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) {
      return;
    }
    event.preventDefault();
    void nudge(event.key === 'ArrowUp' ? -1 : 1);
  }

  async function confirmDelete(): Promise<void> {
    const target = pendingDelete;
    pendingDelete = null;
    if (target?.kind === 'collection') {
      await workspace.deleteCollection();
    } else if (target?.kind === 'section') {
      await workspace.deleteSection(target.id);
    } else if (target) {
      focusMode.exit();
      await workspace.deleteRequest(target.id);
    }
  }

  function closeCard(): void {
    focusMode.exit();
    void workspace.closeRequest();
  }

  function onWindowKey(event: KeyboardEvent): void {
    if (event.defaultPrevented || isInEditor(event.target)) {
      return;
    }
    if (document.querySelector('dialog[open]') !== null) {
      return;
    }
    // Escape inside the code editor belongs to it, not to focus mode (CodeMirror uses Escape
    // then Tab to move focus out of the editor).
    if (focusMode.active && event.key === 'Escape') {
      focusMode.exit();
      return;
    }
    // Wherever the focus is, Ctrl+F searches the open card next to its body, instead of
    // the browser's own find bar at the top of the window.
    const card = document.querySelector('[data-card-id]');
    const find = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f';
    if (find && card !== null && openFindIn(card)) {
      event.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={onWindowKey} />

{#if collection}
  <div class="view">
    {#if focusMode.active}
      <div class="focus-bar">
        <span class="focus-title">
          Focus mode · {collection.name} › {workspace.draft?.name ?? ''}
        </span>
        <label class="focus-check">
          <input type="checkbox" bind:checked={focusMode.hideSecrets} />
          Hide secrets on screen
        </label>
        <button
          type="button"
          class="focus-exit"
          onclick={() => {
            focusMode.exit();
          }}
        >
          Exit focus mode
        </button>
      </div>
    {:else}
      <header class="top">
        <div class="title">
          <div class="title-row">
            <div class="icon-wrap">
              <button
                type="button"
                class="icon-tile emoji"
                aria-label="Change icon"
                title="Change icon"
                aria-haspopup="dialog"
                aria-expanded={pickingIcon}
                onclick={() => (pickingIcon = !pickingIcon)}>{collection.icon}</button
              >
              {#if pickingIcon}
                <EmojiPicker
                  current={collection.icon}
                  name={collection.name}
                  custom={collection.iconCustom}
                  onPick={(icon: string | null) => void workspace.setCollectionIcon(icon)}
                  onClose={() => (pickingIcon = false)}
                />
              {/if}
            </div>
            <EditableText
              value={collection.name}
              label="Collection name"
              variant="title"
              onCommit={(name: string) => void workspace.renameCollection(name)}
            />
          </div>
          <p class="meta">
            {#if collection.requests.length === 0}
              No requests yet · saved automatically
            {:else}
              {collection.requests.length}
              {collection.requests.length === 1 ? 'request' : 'requests'} · saved automatically
            {/if}
            {#if workspace.createdId === collection.id && collection.requests.length === 0}
              <span class="created" role="status">Created just now</span>
            {/if}
          </p>
        </div>
        <EnvironmentSelect {environments} collections={workspace.collections} />
        <div class="tools" role="group" aria-label="Collection actions">
          <IconButton
            label={canRun ? 'Run all' : 'Run all · add an HTTP or GraphQL request first'}
            disabled={!canRun}
            onclick={onRunAll}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8 5.5v13l10.5-6.5z" />
            </svg>
          </IconButton>
          <span class="divider" aria-hidden="true"></span>
          <IconButton
            label="Variables"
            expanded={openPanel === 'variables'}
            onclick={() => {
              togglePanel('variables');
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M8 4c-2 0-3 1-3 3v2c0 1.5-1 2.5-2 3 1 .5 2 1.5 2 3v2c0 2 1 3 3 3M16 4c2 0 3 1 3 3v2c0 1.5 1 2.5 2 3-1 .5-2 1.5-2 3v2c0 2-1 3-3 3"
              />
            </svg>
          </IconButton>
          <IconButton
            label={collection.auth.type === 'none' ? 'Auth' : 'Auth · on'}
            dot={collection.auth.type === 'none' ? null : 'on'}
            expanded={openPanel === 'auth'}
            onclick={() => {
              togglePanel('auth');
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" />
            </svg>
          </IconButton>
          <IconButton
            label={teamSummary ? `Collaboration · ${teamSummary.short}` : 'Collaboration'}
            dot={teamSummary?.tone ?? null}
            expanded={openPanel === 'team'}
            onclick={() => {
              togglePanel('team');
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="9" cy="8" r="3" /><path
                d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6M16 5a3 3 0 0 1 0 6M21 20c0-2.6-1.6-4.8-4-5.6"
              />
            </svg>
          </IconButton>
          <IconButton
            label={mockUrl ? `Mock server · running at ${mockUrl}` : 'Mock server'}
            dot={mockUrl ? 'ok' : null}
            expanded={openPanel === 'mock'}
            onclick={() => {
              togglePanel('mock');
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="4" y="4" width="16" height="7" rx="1.5" /><rect
                x="4"
                y="13"
                width="16"
                height="7"
                rx="1.5"
              /><path d="M8 7.5h.01M8 16.5h.01" />
            </svg>
          </IconButton>
          <IconButton
            label="History"
            expanded={openPanel === 'history'}
            onclick={() => {
              togglePanel('history');
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path d={NAV_ICON_PATHS.get('history')} />
            </svg>
          </IconButton>
          <span class="divider" aria-hidden="true"></span>
          <IconButton
            label="Duplicate collection"
            popup="dialog"
            expanded={pendingDuplicate?.kind === 'collection'}
            onclick={() =>
              (pendingDuplicate = { kind: 'collection', id: collection.id, name: collection.name })}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="8" y="8" width="12" height="12" rx="2" /><path
                d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"
              />
            </svg>
          </IconButton>
          <IconButton
            label="Export collection"
            expanded={openPanel === 'export'}
            onclick={() => {
              togglePanel('export');
            }}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 15V4M7 9l5-5 5 5M4 15v4a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-4" />
            </svg>
          </IconButton>
          <IconButton
            label="Delete collection"
            danger
            popup="dialog"
            align="end"
            expanded={pendingDelete?.kind === 'collection'}
            onclick={() => (pendingDelete = { kind: 'collection', name: collection.name })}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
            </svg>
          </IconButton>
        </div>
      </header>
      {#if link && link.conflicts.length > 0 && openPanel !== 'team'}
        <div class="overlap" role="status">
          <span>
            Your changes and a collaborator's overlap in {link.conflicts.length === 1
              ? 'one place'
              : `${String(link.conflicts.length)} places`}. Syncing waits until you choose.
          </span>
          <button type="button" class="link" onclick={() => (resolving = true)}>Resolve</button>
        </div>
      {/if}
    {/if}

    {#if openPanel === 'variables' && !focusMode.active}
      <CollectionVariables
        {workspace}
        collectionId={collection.id}
        environmentName={environments.active?.name ?? null}
        shared={link !== undefined}
        onClose={() => (openPanel = null)}
      />
    {:else if openPanel === 'auth' && !focusMode.active}
      {#key collection.id}
        <CollectionAuth
          {workspace}
          collectionId={collection.id}
          saved={collection.auth}
          onClose={() => (openPanel = null)}
        />
      {/key}
    {:else if openPanel === 'mock' && !focusMode.active}
      {#key collection.id}
        <MockPanel
          collectionId={collection.id}
          onStatus={(status: MockStatus | null) => (mockUrl = status?.url ?? null)}
          onClose={() => (openPanel = null)}
        />
      {/key}
    {:else if openPanel === 'team' && !focusMode.active}
      {#key collection.id}
        <TeamPanel
          {team}
          collectionId={collection.id}
          collectionName={collection.name}
          onResolve={() => (resolving = true)}
          onClose={() => (openPanel = null)}
        />
      {/key}
    {:else if openPanel === 'history' && !focusMode.active}
      {#key collection.id}
        <CollectionHistory
          {workspace}
          collectionId={collection.id}
          collectionName={collection.name}
          onOpenRequest={(requestId: string) => void openFromHistory(requestId)}
          onClose={() => (openPanel = null)}
        />
      {/key}
    {:else if openPanel === 'export' && !focusMode.active}
      {#key collection.id}
        <CollectionExport
          {workspace}
          collectionId={collection.id}
          onClose={() => (openPanel = null)}
        />
      {/key}
    {/if}

    <!-- One flat keyed list, so the open card keeps its state when it moves between sections. -->
    {#each items as item (item.key)}
      {#if item.type === 'section'}
        {#if !focusMode.active}
          {@const section = item.section}
          <div
            class="section-head"
            role="group"
            aria-label="Section {section.name}"
            class:drop={dropHint === section.id}
            ondragover={(event) => {
              allowDrop(event, section.id);
            }}
            ondrop={(event) => {
              dropOnSection(event, section.id);
            }}
          >
            <FoldButton
              expanded={!item.collapsed}
              label="section {section.name}"
              title={item.collapsed ? 'Show requests' : 'Hide requests'}
              onToggle={() => {
                collapsedSections.toggle(section.id);
              }}
            />
            <button
              type="button"
              class="handle"
              data-section-id={section.id}
              draggable="true"
              aria-label="Move section {section.name}"
              aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
              ondragstart={(event) => {
                startDrag(event, 'section', section.id);
              }}
              ondragend={endDrag}
              onkeydown={(event) => {
                nudgeWithKeys(event, (step) => nudgeSection(section.id, step));
              }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="9" cy="6" r="1" /><circle cx="15" cy="6" r="1" />
                <circle cx="9" cy="12" r="1" /><circle cx="15" cy="12" r="1" />
                <circle cx="9" cy="18" r="1" /><circle cx="15" cy="18" r="1" />
              </svg>
            </button>
            <EditableText
              value={section.name}
              label="Section name"
              onCommit={(name: string) => void workspace.renameSection(section.id, name)}
            />
            <span class="count">{item.count}</span>
            <span class="spacer"></span>
            <div
              class="section-tools"
              role="group"
              aria-label="Section {section.name}: add or change"
            >
              {#each ADDABLE_KINDS as option (option.kind)}
                <IconButton
                  label={option.label}
                  onclick={() => void addRequest(section.id, option.kind)}
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" data-hue={option.hue}>
                    <path d={option.icon} />
                  </svg>
                </IconButton>
              {/each}
              <span class="tools-divider" aria-hidden="true"></span>
              <IconButton
                label="Duplicate section"
                popup="dialog"
                expanded={pendingDuplicate?.kind === 'section' &&
                  pendingDuplicate.id === section.id}
                onclick={() =>
                  (pendingDuplicate = { kind: 'section', id: section.id, name: section.name })}
              >
                <svg width="16" height="16" viewBox="0 0 24 24">
                  <rect x="8" y="8" width="12" height="12" rx="2" /><path
                    d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"
                  />
                </svg>
              </IconButton>
              <IconButton
                label="Remove section"
                danger
                align="end"
                popup="dialog"
                onclick={() =>
                  (pendingDelete = { kind: 'section', id: section.id, name: section.name })}
              >
                <svg width="16" height="16" viewBox="0 0 24 24">
                  <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
                </svg>
              </IconButton>
            </div>
          </div>
        {/if}
      {:else if item.type === 'request'}
        {@const request = item.request}
        {#if request.id === workspace.openRequestId && workspace.draft}
          <RequestCard
            index={item.step}
            bind:draft={workspace.draft}
            collectionId={collection.id}
            requestId={request.id}
            sections={collection.sections}
            sectionId={request.sectionId}
            onMove={(sectionId: string | null) =>
              void workspace.moveRequest(request.id, sectionId, null)}
            saveState={workspace.saveState}
            saveError={workspace.saveError}
            onEdit={() => {
              workspace.markEdited();
            }}
            onSave={() => void workspace.flushSave()}
            onClose={closeCard}
            onDelete={() =>
              (pendingDelete = { kind: 'request', id: request.id, name: request.name })}
            onDuplicate={() =>
              (pendingDuplicate = { kind: 'request', id: request.id, name: request.name })}
            duplicateOpen={pendingDuplicate?.kind === 'request' &&
              pendingDuplicate.id === request.id}
            onHistory={link ? () => (historyOf = { id: request.id, name: request.name }) : null}
            historyOpen={historyOf?.id === request.id}
            deleteOpen={pendingDelete?.kind === 'request' && pendingDelete.id === request.id}
            onExamplesChanged={() => {
              workspace.notifyChanged(collection.id);
            }}
            onResult={(response: HttpResponseOutput) => {
              workspace.recordResult(request.id, response);
            }}
            collectionAuth={collection.auth}
            focused={focusMode.active}
            maskSecrets={focusMode.active && focusMode.hideSecrets}
            environmentName={environments.active?.name ?? null}
            onFocus={() => {
              focusMode.enter();
            }}
          />
        {:else if !focusMode.active}
          <RequestRow
            step={item.step}
            {request}
            collectionId={collection.id}
            lastResult={workspace.lastResults.get(request.id)}
            dropTarget={dropHint === request.id}
            onOpen={() => void workspace.openRequest(request.id)}
            onDragStart={(event: DragEvent) => {
              startDrag(event, 'request', request.id);
            }}
            onDragEnd={endDrag}
            onDragOver={(event: DragEvent) => {
              allowDrop(event, request.id, 'request');
            }}
            onDrop={(event: DragEvent) => {
              dropRequest(event, request.sectionId, request.id);
            }}
            onNudge={(step: -1 | 1) => void nudgeRequest(request.id, step)}
          />
        {/if}
      {:else if item.type === 'end'}
        {#if item.section && collapsedSections.has(item.section.id)}
          <!-- Folded: requests are dropped on the section's header instead. -->
        {:else if dragging?.kind === 'request'}
          {@const zone = item.key}
          <div
            class="dropzone"
            class:drop={dropHint === zone}
            role="presentation"
            ondragover={(event) => {
              allowDrop(event, zone, 'request');
            }}
            ondrop={(event) => {
              dropRequest(event, item.section?.id ?? null, null);
            }}
          >
            {item.section ? `Move to the end of ${item.section.name}` : 'Move out of sections'}
          </div>
        {:else if item.section && item.empty && !focusMode.active}
          <p class="empty-section">No requests in this section yet.</p>
        {/if}
      {:else if dragging?.kind === 'section'}
        <div
          class="dropzone"
          class:drop={dropHint === item.key}
          role="presentation"
          ondragover={(event) => {
            allowDrop(event, item.key, 'section');
          }}
          ondrop={(event) => {
            dropSectionAtEnd(event);
          }}
        >
          Move the section to the end
        </div>
      {/if}
    {/each}

    {#if focusMode.active}
      <p class="focus-hint">
        Sidebar and other cards are hidden, so a screenshot shows only this request. Capture saves
        one as an image.
      </p>
    {:else}
      {#if collection.requests.length === 0}
        <CollectionStart
          {workspace}
          collectionId={collection.id}
          onOpenPanel={(panel: 'variables' | 'team') => (openPanel = panel)}
          {onPasteCurl}
        />
      {:else}
        <div class="adders">
          {#each ADDABLE_KINDS as option (option.kind)}
            <button type="button" class="adder" onclick={() => void addRequest(null, option.kind)}>
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                aria-hidden="true"
                data-hue={option.hue}
              >
                <path d={option.icon} />
              </svg>
              {option.label}
            </button>
          {/each}
          <button type="button" class="adder" onclick={() => void workspace.addSection()}>
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true" data-hue="orange">
              <path d="M3 7h6l2 2h10v10H3z" />
            </svg>
            Add section
          </button>
        </div>
      {/if}
    {/if}
  </div>

  {#if resolving && link && link.conflicts.length > 0}
    <ConflictDialog
      {team}
      collectionId={collection.id}
      conflicts={link.conflicts}
      onClose={() => (resolving = false)}
    />
  {/if}

  {#if historyOf}
    <RequestHistoryDialog
      {workspace}
      collectionId={collection.id}
      requestId={historyOf.id}
      requestName={historyOf.name}
      onClose={() => (historyOf = null)}
    />
  {/if}

  {#if pendingDuplicate}
    <DuplicateDialog
      kind={pendingDuplicate.kind}
      sourceName={pendingDuplicate.name}
      details={duplicateInfo.details}
      takenNames={duplicateInfo.taken}
      onConfirm={confirmDuplicate}
      onClose={() => (pendingDuplicate = null)}
    />
  {/if}

  <ConfirmDialog
    open={pendingDelete !== null}
    title="Delete?"
    message={deleteMessage}
    confirmLabel="Delete"
    onConfirm={() => void confirmDelete()}
    onCancel={() => (pendingDelete = null)}
  />
{/if}

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .focus-bar {
    align-self: center;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
    padding: 6px 6px 6px 16px;
    border-radius: 999px;
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  .focus-title {
    font-size: 13px;
    color: var(--text-muted);
  }

  .focus-check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-2);
  }

  .focus-check input {
    accent-color: var(--accent);
  }

  .focus-exit {
    height: 32px;
    padding: 0 14px;
    border: 0;
    border-radius: 999px;
    background: var(--text);
    color: var(--surface);
    font-size: 13px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .focus-hint {
    margin: 10px 0 0;
    text-align: center;
    font-size: 13px;
    color: var(--text-muted);
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 8px;
  }

  .title {
    flex: 1 1 280px;
    min-width: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon-wrap {
    position: relative;
    flex: none;
  }

  .icon-tile {
    width: 44px;
    height: 44px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    font-size: 24px;
  }

  .icon-tile:hover,
  .icon-tile[aria-expanded='true'] {
    border-color: var(--text);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    margin: 8px 0 0;
    color: var(--text-muted);
  }

  .created {
    padding: 2px 10px;
    border: 1px solid var(--ok-line);
    border-radius: 999px;
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 12px;
    font-weight: 500;
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .divider {
    width: 1px;
    height: 20px;
    margin: 0 4px;
    background: var(--border);
  }

  .overlap {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--warn-bg);
    color: var(--warn);
    font-size: 13px;
  }

  .overlap span {
    flex: 1;
  }

  /* Section names are what the list is grouped by, so they read stronger than their rows. */
  .section-head :global(.editable.section) {
    font-size: 14px;
    font-weight: 700;
    color: var(--text);
  }

  .section-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 2px;
    border-radius: var(--radius-md);
  }

  .section-head.drop,
  .dropzone.drop {
    background: var(--accent-soft);
    outline: 1.5px dashed var(--accent);
  }

  .handle {
    flex: none;
    width: 24px;
    height: 28px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    cursor: grab;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .handle:hover {
    background: var(--surface-sunken);
  }

  .handle circle {
    fill: currentColor;
  }

  .dropzone {
    padding: 10px 14px;
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .count {
    font-size: 12px;
    color: var(--text-muted);
  }

  .spacer {
    flex: 1;
  }

  .section-tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px;
  }

  .tools-divider {
    width: 1px;
    height: 18px;
    margin: 0 6px;
    background: var(--border);
  }

  .link {
    padding: 4px 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
  }

  .link:hover {
    background: var(--surface-sunken);
  }

  .empty-section {
    margin: 0;
    padding: 10px 2px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .adders {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 8px;
  }

  .adder {
    flex: 1 1 180px;
    height: 46px;
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .adder:hover {
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .adder svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
