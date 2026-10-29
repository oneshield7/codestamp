<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { toAppError } from '../api/http';
  import {
    gitChangesOverview,
    gitCommitDetail,
    gitCommits,
    gitRequestActivity,
    type CommitDetail,
    type CommitSummary,
    type RepoSummary,
    type RequestActivity,
  } from '../api/team';
  import { dayLabel, formatSince } from '../format/time';
  import { plural } from '../format/plural';
  import type { TeamSync } from '../state/team.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import CommitDetailPane from './changes/CommitDetailPane.svelte';
  import RequestActivityTable from './changes/RequestActivityTable.svelte';
  import Dropdown from './Dropdown.svelte';
  import NavIcon from './NavIcon.svelte';
  import RequestHistoryDialog from './RequestHistoryDialog.svelte';

  interface Props {
    team: TeamSync;
    workspace: Workspace;
    onOpenRequest: (collectionId: string, requestId: string) => void;
    onOpenCollection: (collectionId: string) => void;
  }

  let { team, workspace, onOpenRequest, onOpenCollection }: Props = $props();

  type Tab = 'commits' | 'requests';
  const TABS: readonly { key: Tab; label: string }[] = [
    { key: 'commits', label: 'Commits' },
    { key: 'requests', label: 'Requests' },
  ];
  const SECONDS_TO_MS = 1000;

  let repos = $state<RepoSummary[] | null>(null);
  let collectionId = $state('');
  let tab = $state<Tab>('commits');
  let commits = $state<CommitSummary[]>([]);
  let more = $state(false);
  let loadingCommits = $state(false);
  let selected = $state<string | null>(null);
  let detail = $state<CommitDetail | null>(null);
  let activity = $state<RequestActivity[] | null>(null);
  let history = $state<RequestActivity | null>(null);
  let error = $state<string | null>(null);
  // Answers for a collection or commit picked earlier are dropped when a newer pick is waiting.
  let generation = 0;
  let detailGeneration = 0;

  const repo = $derived(repos?.find((item) => item.collectionId === collectionId) ?? null);
  const present = $derived(new Set((activity ?? []).map((request) => request.id)));
  const syncing = $derived(team.syncing.has(collectionId));
  const lastOutcome = $derived(team.outcomes.get(collectionId)?.at ?? 0);

  onMount(() => {
    void loadRepos();
  });

  // A sync that finishes while the page is open brings new commits; read them again.
  $effect(() => {
    if (lastOutcome > 0) {
      untrack(() => void refresh());
    }
  });

  async function loadRepos(): Promise<void> {
    try {
      const loaded = await gitChangesOverview();
      repos = loaded;
      const open = workspace.collection?.id;
      const keep = loaded.some((item) => item.collectionId === collectionId);
      if (!keep) {
        const start = loaded.find((item) => item.collectionId === open) ?? loaded.at(0);
        await pick(start?.collectionId ?? '');
      }
    } catch (caught) {
      error = toAppError(caught).message;
    }
  }

  async function pick(id: string): Promise<void> {
    const run = ++generation;
    collectionId = id;
    commits = [];
    more = false;
    selected = null;
    detail = null;
    activity = null;
    error = null;
    if (id === '') {
      return;
    }
    loadingCommits = true;
    try {
      const [page, requests] = await Promise.all([gitCommits(id, 0), gitRequestActivity(id)]);
      if (run !== generation) {
        return;
      }
      commits = page.commits;
      more = page.more;
      activity = requests;
      const first = page.commits.at(0);
      if (first) {
        await select(first.commit);
      }
    } catch (caught) {
      if (run === generation) {
        error = toAppError(caught).message;
      }
    } finally {
      if (run === generation) {
        loadingCommits = false;
      }
    }
  }

  /** Reads the commits again after a sync; the commit picked at that moment stays picked. */
  async function refresh(): Promise<void> {
    await loadRepos();
    const run = generation;
    if (collectionId === '') {
      return;
    }
    try {
      const [page, requests] = await Promise.all([
        gitCommits(collectionId, 0),
        gitRequestActivity(collectionId),
      ]);
      if (run !== generation) {
        return;
      }
      commits = page.commits;
      more = page.more;
      activity = requests;
      const first = page.commits.at(0);
      if (first && !page.commits.some((commit) => commit.commit === selected)) {
        await select(first.commit);
      }
    } catch (caught) {
      error = toAppError(caught).message;
    }
  }

  async function loadMore(): Promise<void> {
    const run = generation;
    loadingCommits = true;
    try {
      const page = await gitCommits(collectionId, commits.length);
      if (run === generation) {
        commits = [...commits, ...page.commits];
        more = page.more;
      }
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      loadingCommits = false;
    }
  }

  async function select(commit: string): Promise<void> {
    const run = ++detailGeneration;
    selected = commit;
    try {
      const loaded = await gitCommitDetail(collectionId, commit);
      if (run === detailGeneration) {
        detail = loaded;
      }
    } catch (caught) {
      if (run === detailGeneration) {
        error = toAppError(caught).message;
      }
    }
  }

  function moveTab(event: KeyboardEvent): void {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') {
      return;
    }
    event.preventDefault();
    tab = tab === 'commits' ? 'requests' : 'commits';
    document.getElementById(`changes-tab-${tab}`)?.focus();
  }

  /** Commits grouped under Today, Yesterday or their date, newest first. */
  const days = $derived.by(() => {
    const groups: { day: string; commits: CommitSummary[] }[] = [];
    for (const commit of commits) {
      const day = dayLabel(commit.time * SECONDS_TO_MS);
      const last = groups.at(-1);
      if (last?.day === day) {
        last.commits.push(commit);
      } else {
        groups.push({ day, commits: [commit] });
      }
    }
    return groups;
  });
</script>

<div class="view" class:fill={repos !== null && repos.length > 0}>
  <header>
    <h1 class="page-title"><NavIcon name="changes" size="lg" />Git changes</h1>
    <p>
      Commits of your shared collections and who changed each request, read from this device's copy
      of the repository.
    </p>
  </header>

  {#if repos === null && !error}
    <p class="muted">Reading repositories…</p>
  {:else if repos !== null && repos.length === 0}
    <div class="empty">
      <h2>No shared collections yet</h2>
      <p>
        Share a collection from its Collaboration panel, or join one from Import. Its commits show
        up here after the first sync.
      </p>
    </div>
  {:else if repos !== null}
    <div class="bar">
      <div class="pick">
        <Dropdown
          variant="field"
          label="Shared collection"
          value={collectionId}
          options={repos.map((item) => ({ value: item.collectionId, label: item.name }))}
          onChange={(id: string) => void pick(id)}
        />
      </div>
      {#if repo}
        <span class="chips">
          {#if repo.localEdits > 0}
            <span class="chip warn">
              {plural(repo.localEdits, 'local edit')}
            </span>
          {/if}
          {#if repo.toPush > 0}
            <span class="chip warn">{repo.toPush} to push</span>
          {/if}
          <span class="synced">
            {repo.syncedAt === null ? 'Not synced yet' : `Synced ${formatSince(repo.syncedAt)}`}
          </span>
        </span>
        <span class="spacer"></span>
        <button
          type="button"
          class="secondary"
          onclick={() => {
            onOpenCollection(repo.collectionId);
          }}
        >
          Open collection
        </button>
        <button
          type="button"
          class="secondary"
          disabled={syncing}
          onclick={() => void team.sync(repo.collectionId)}
        >
          {syncing ? 'Syncing…' : 'Sync now'}
        </button>
      {/if}
    </div>

    <div class="tabs" role="tablist" aria-label="Git changes" tabindex="-1" onkeydown={moveTab}>
      {#each TABS as item (item.key)}
        <button
          type="button"
          role="tab"
          id="changes-tab-{item.key}"
          class="tab"
          class:on={tab === item.key}
          aria-selected={tab === item.key}
          aria-controls="changes-panel"
          tabindex={tab === item.key ? 0 : -1}
          onclick={() => (tab = item.key)}
        >
          {item.label}
        </button>
      {/each}
    </div>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div
      class="panel"
      id="changes-panel"
      role="tabpanel"
      aria-labelledby="changes-tab-{tab}"
      tabindex="-1"
    >
      {#if tab === 'commits'}
        {#if loadingCommits && commits.length === 0}
          <p class="muted">Reading commits…</p>
        {:else if commits.length === 0}
          <p class="muted">No commits yet. They appear after the first sync.</p>
        {:else}
          <div class="split">
            <div class="list">
              {#each days as group (group.day)}
                <h2 class="day">{group.day}</h2>
                <ol class="commits" aria-label="Commits of {group.day}">
                  {#each group.commits as commit (commit.commit)}
                    <li>
                      <button
                        type="button"
                        class="commit"
                        class:on={selected === commit.commit}
                        aria-pressed={selected === commit.commit}
                        onclick={() => void select(commit.commit)}
                      >
                        <span class="dot" class:merge={commit.merge} aria-hidden="true"></span>
                        <span class="text">
                          <span class="message">{commit.message}</span>
                          <span class="meta">
                            {commit.author} · {formatSince(commit.time * SECONDS_TO_MS)}
                            {#if commit.merge}· merge{/if}
                          </span>
                        </span>
                        {#if !commit.pushed}
                          <span class="chip warn small">Not pushed</span>
                        {/if}
                      </button>
                    </li>
                  {/each}
                </ol>
              {/each}
              {#if more}
                <button
                  type="button"
                  class="secondary more"
                  disabled={loadingCommits}
                  onclick={() => void loadMore()}
                >
                  {loadingCommits ? 'Loading…' : 'Load older commits'}
                </button>
              {/if}
            </div>
            <div class="side">
              {#if detail}
                <CommitDetailPane
                  {detail}
                  {present}
                  onOpenRequest={(requestId: string) => {
                    onOpenRequest(collectionId, requestId);
                  }}
                />
              {:else}
                <p class="muted">{selected ? 'Reading the commit…' : 'Pick a commit.'}</p>
              {/if}
            </div>
          </div>
        {/if}
      {:else if activity === null}
        <p class="muted">Reading requests…</p>
      {:else}
        <RequestActivityTable
          {activity}
          onHistory={(request: RequestActivity) => (history = request)}
          onOpen={(requestId: string) => {
            onOpenRequest(collectionId, requestId);
          }}
        />
      {/if}
    </div>
  {/if}
</div>

{#if history}
  <RequestHistoryDialog
    {workspace}
    {collectionId}
    requestId={history.id}
    requestName={history.name}
    onClose={() => {
      history = null;
    }}
  />
{/if}

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    container-type: inline-size;
  }

  header p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .empty {
    padding: 24px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .empty h2 {
    margin: 0 0 6px;
    font-size: 16px;
    font-weight: 600;
  }

  .empty p {
    margin: 0;
    color: var(--text-2);
    font-size: 14px;
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .pick {
    flex: 0 1 280px;
    min-width: 200px;
  }

  .chips {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .chip {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .chip.warn {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .synced {
    font-size: 13px;
    color: var(--text-muted);
  }

  .spacer {
    flex: 1;
  }

  .secondary {
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .secondary:disabled {
    color: var(--text-muted);
  }

  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    height: 36px;
    margin-bottom: -1px;
    padding: 0 14px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
  }

  .tab.on {
    border-bottom-color: var(--accent);
    color: var(--text);
    font-weight: 600;
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .panel {
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  /* The view fits the window, so the header, picker and tabs stay put while the panel scrolls.
     A long history then never pushes them away. */
  .view.fill {
    height: calc(
      100vh - var(--titlebar-height) - var(--statusbar-height) - var(--main-pad-top) -
        var(--main-pad-bottom)
    );
    min-height: 480px;
  }

  .fill .panel {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .split {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 20px;
  }

  @container (min-width: 760px) {
    .split {
      flex: 1;
      min-height: 0;
      grid-template-columns: minmax(260px, 2fr) minmax(0, 3fr);
      grid-template-rows: minmax(0, 1fr);
    }

    /* Each column scrolls on its own; the padding keeps focus rings clear of the clipping edge. */
    .list,
    .side {
      min-height: 0;
      margin: -4px;
      padding: 4px;
      overflow-y: auto;
    }
  }

  .day {
    margin: 12px 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .day:first-child {
    margin-top: 0;
  }

  .commits {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .commit {
    width: 100%;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
  }

  .commit:hover {
    background: var(--surface-sunken);
  }

  .commit.on {
    background: var(--accent-soft);
  }

  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    margin-top: 4px;
    border-radius: 50%;
    background: var(--accent);
  }

  .dot.merge {
    border: 2px solid var(--accent);
    background: var(--surface);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .message {
    font-size: 13px;
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  .small {
    margin-top: 2px;
  }

  .more {
    margin-top: 10px;
  }
</style>
