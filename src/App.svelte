<script lang="ts">
  import { onMount } from 'svelte';
  import CollectionView from './lib/components/CollectionView.svelte';
  import CookiesView from './lib/components/CookiesView.svelte';
  import EnvironmentsView from './lib/components/EnvironmentsView.svelte';
  import FlowsView from './lib/components/FlowsView.svelte';
  import DeletedView from './lib/components/DeletedView.svelte';
  import HomeView from './lib/components/HomeView.svelte';
  import ImportView from './lib/components/ImportView.svelte';
  import NewCollectionDialog from './lib/components/NewCollectionDialog.svelte';
  import QuickSearch from './lib/components/QuickSearch.svelte';
  import RunView from './lib/components/RunView.svelte';
  import { getCollection, type RequestKind } from './lib/api/collections';
  import SettingsView from './lib/components/SettingsView.svelte';
  import type { ImportResult } from './lib/api/import';
  import Sidebar from './lib/components/Sidebar.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import UpdateBanner from './lib/components/UpdateBanner.svelte';
  import { appearanceSettings } from './lib/state/appearance.svelte';
  import { sidebar } from './lib/state/sidebar.svelte';
  import type { AppMenuActions } from './lib/titlebar/app-menu';
  import { Catalog } from './lib/state/catalog.svelte';
  import { Docs } from './lib/state/docs.svelte';
  import { Environments } from './lib/state/environments.svelte';
  import { Flows } from './lib/state/flows.svelte';
  import { focusMode } from './lib/state/focus.svelte';
  import type { NewCollectionStart, SettingsSection, View } from './lib/state/navigation';
  import type { SearchTarget } from './lib/search/results';
  import { finishLaunch } from './lib/dom/launch';
  import { revealCard } from './lib/dom/reveal';
  import { recentlyDeleted } from './lib/state/deleted.svelte';
  import { notices } from './lib/state/notices.svelte';
  import { panelRequest } from './lib/state/panel-request.svelte';
  import { Runner } from './lib/state/runner.svelte';
  import { TeamSync } from './lib/state/team.svelte';
  import { Updates } from './lib/state/updates.svelte';
  import { Workspace } from './lib/state/workspace.svelte';

  const UPDATE_CHECK_DELAY_MS = 3000;

  const workspace = new Workspace();
  const environments = new Environments();
  const runner = new Runner();
  const flows = new Flows();
  const catalog = new Catalog();
  const docs = new Docs();
  const updates = new Updates();
  const team = new TeamSync(workspace, environments);
  // A restore can bring back a shared collection's link and environments, and flow steps.
  recentlyDeleted.onRestored = async (restored, open) => {
    await workspace.refresh(restored.collectionId);
    workspace.notifyChanged(restored.collectionId);
    await environments.load();
    await flows.refresh();
    if (restored.kind === 'collection') {
      await team.load();
    }
    if (!open) {
      return;
    }
    if (restored.requestId === null) {
      await openCollectionById(restored.collectionId);
    } else {
      await openRequestIn(restored.collectionId, restored.requestId);
    }
  };
  team.onAccessGranted = (collectionId, message) => {
    notices.inform(message, {
      label: 'Open variables',
      run: () => {
        panelRequest.open(collectionId, 'variables');
        void openCollectionById(collectionId);
      },
    });
  };
  // Edits to an environment a collection shares wait for that collection's next Sync now.
  environments.watchShared((collectionId) => {
    workspace.notifyChanged(collectionId);
  });
  let view = $state<View>('collection');
  let settingsSection = $state<SettingsSection>('appearance');
  let newCollection = $state<NewCollectionStart | null>(null);
  let searchOpen = $state(false);
  // Handed to the Import page when it opens from the New collection dialog or an empty collection.
  let importFile = $state<File | null>(null);
  let importTarget = $state<string | null>(null);

  // With no collection to show, the collection view is the Home page.
  const showingHome = $derived(
    view === 'home' || (view === 'collection' && workspace.loaded && !workspace.collection),
  );

  async function navigate(next: View): Promise<void> {
    focusMode.exit();
    // Leaving the collection closes the open card, which also flushes any pending autosave.
    if (next !== 'collection' && (!workspace.mayLeave() || !(await workspace.closeRequest()))) {
      return;
    }
    // Leaving Orchestration saves the open flow's pending edits.
    if (next !== 'flows' && !(await flows.flushSave())) {
      return;
    }
    if (next !== 'import') {
      importFile = null;
      importTarget = null;
    }
    view = next;
  }

  async function openImport(file: File | null, target: string | null): Promise<void> {
    newCollection = null;
    importFile = file;
    importTarget = target;
    await navigate('import');
  }

  /** History lives in each collection: this opens the current one with its History panel. */
  async function openHistory(): Promise<void> {
    const current = workspace.collection;
    if (!current) {
      return;
    }
    panelRequest.open(current.id, 'history');
    await openCollectionById(current.id);
  }

  // Installing an update closes Codestamp, so pending edits are saved first and open ones stop it.
  async function readyToClose(): Promise<string | null> {
    const panel = workspace.unsavedPanel();
    if (panel) {
      return `Save or close ${panel.toLowerCase()} first, then update.`;
    }
    if (environments.editor.dirty) {
      return 'Save or discard your environment changes first, then update.';
    }
    if (flows.running) {
      return 'Wait for the Orchestration run to finish, then update.';
    }
    if (!(await workspace.flushSave()) || !(await flows.flushSave())) {
      return 'Your latest edits could not be saved, so the update did not start.';
    }
    return null;
  }

  function installUpdate(): void {
    void updates.install(readyToClose);
  }

  async function openSettings(section: SettingsSection): Promise<void> {
    settingsSection = section;
    await navigate('settings');
  }

  async function addRequestFromHome(kind: RequestKind): Promise<void> {
    await navigate('collection');
    if (view === 'collection') {
      await workspace.addRequest(null, kind);
      await revealCard(workspace.openRequestId);
    }
  }

  async function runAll(): Promise<void> {
    const collection = workspace.collection;
    if (!collection || runner.running) {
      return;
    }
    focusMode.exit();
    // Closing the card saves pending edits, so the run uses what the user sees.
    if (!(await workspace.closeRequest())) {
      return;
    }
    view = 'run';
    await runner.start(collection, environments.active?.name ?? null, (id, response) => {
      workspace.recordResult(id, response);
    });
  }

  async function runAgain(): Promise<void> {
    const id = runner.collectionId;
    if (!id || runner.running) {
      return;
    }
    await notices.guard(async () => {
      const collection =
        workspace.collection?.id === id ? workspace.collection : await getCollection(id);
      await runner.start(collection, environments.active?.name ?? null, (requestId, response) => {
        workspace.recordResult(requestId, response);
      });
    });
  }

  async function openCollectionById(collectionId: string): Promise<void> {
    await navigate('collection');
    if (view === 'collection') {
      await workspace.openCollection(collectionId);
    }
  }

  async function runCollection(collectionId: string): Promise<void> {
    await openCollectionById(collectionId);
    if (workspace.collection?.id === collectionId) {
      await runAll();
    }
  }

  /** Picks the flow first, so Orchestration opens on it instead of the first one. */
  async function openFlow(flowId: string): Promise<void> {
    await notices.guard(() => flows.select(flowId));
    await navigate('flows');
  }

  /** Docs, scrolled to one collection, section or request. */
  async function openDocs(collectionId: string, itemId: string): Promise<void> {
    const alreadyOpen = view === 'docs';
    docs.select(collectionId, itemId);
    if (alreadyOpen) {
      await docs.open(collectionId);
    } else {
      // The page loads the selected collection as it opens.
      await navigate('docs');
    }
  }

  async function openRequestIn(collectionId: string, requestId: string): Promise<void> {
    if (!(await workspace.closeRequest())) {
      return;
    }
    view = 'collection';
    await workspace.openCollection(collectionId);
    await workspace.openRequest(requestId);
    await revealCard(requestId);
  }

  async function openSearched(target: SearchTarget): Promise<void> {
    searchOpen = false;
    focusMode.exit();
    switch (target.kind) {
      case 'request':
        await openRequestIn(target.collectionId, target.requestId);
        break;
      case 'collection':
        await openCollectionById(target.collectionId);
        break;
      case 'docs':
        await openDocs(target.collectionId, target.itemId);
        break;
      case 'environment':
        await navigate('environments');
        // Unsaved edits in the open environment stay open rather than being switched away.
        if (view === 'environments' && !environments.editor.dirty) {
          await environments.select(target.environmentId);
        }
        break;
      case 'page':
        await navigate(target.view);
        break;
      case 'settings':
        await openSettings(target.section);
        break;
    }
  }

  // Focus mode keeps the sidebar out of view, and a narrow window always shows the rail.
  const sidebarLocked = $derived(focusMode.active || sidebar.narrow);

  function toggleSidebar(): void {
    if (!sidebarLocked) {
      sidebar.toggle();
    }
  }

  function menuActions(): Omit<AppMenuActions, 'exit'> {
    return {
      sidebarCollapsed: sidebar.collapsed,
      sidebarLocked,
      theme: appearanceSettings.appearance.theme,
      newCollection: () => {
        newCollection = 'empty';
      },
      navigate: (next: View) => {
        void navigate(next);
      },
      openSettings: (section: SettingsSection) => {
        void openSettings(section);
      },
      search: () => {
        searchOpen = true;
      },
      toggleSidebar,
      setTheme: (theme) => {
        appearanceSettings.update({ theme });
      },
    };
  }

  /** Ctrl+B collapses or expands the sidebar, unless a dialog has the keyboard. */
  function sidebarShortcut(event: KeyboardEvent): void {
    const pressed =
      event.key.toLowerCase() === 'b' &&
      event.ctrlKey &&
      !event.altKey &&
      !event.shiftKey &&
      !event.metaKey;
    if (!pressed || event.defaultPrevented || document.querySelector('dialog[open]')) {
      return;
    }
    event.preventDefault();
    toggleSidebar();
  }

  /**
   * Ctrl+Space opens quick search from anywhere, the code editors included, so it is caught
   * before they see it. It is left alone while another dialog has the keyboard.
   */
  function toggleSearch(event: KeyboardEvent): void {
    const pressed =
      event.code === 'Space' && event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (!pressed) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (searchOpen) {
      searchOpen = false;
    } else if (!document.querySelector('dialog[open]')) {
      searchOpen = true;
    }
  }

  async function showImported(result: ImportResult): Promise<void> {
    if (result.kind === 'collection') {
      await workspace.refresh(result.id);
      await workspace.openCollection(result.id);
      view = 'collection';
    } else {
      await environments.load();
      // Unsaved edits in the open environment are kept; the new one is then picked by hand.
      if (environments.editor.dirty) {
        notices.report({
          code: 'unsaved',
          message:
            'The environment was imported. Save or discard your open changes, then select it.',
        });
      } else {
        await environments.select(result.id);
      }
      view = 'environments';
    }
  }

  onMount(() => {
    void workspace.load().finally(finishLaunch);
    void environments.load();
    window.addEventListener('keydown', toggleSearch, true);
    window.addEventListener('keydown', sidebarShortcut);
    // The first look for an update waits a moment so it does not slow down start-up.
    const updateCheck = setTimeout(() => void updates.check(true), UPDATE_CHECK_DELAY_MS);
    let stopTeam: (() => void) | undefined;
    let unmounted = false;
    void notices.guard(async () => {
      await team.load();
      if (!unmounted) {
        stopTeam = team.start();
      }
    });
    return () => {
      unmounted = true;
      clearTimeout(updateCheck);
      stopTeam?.();
      window.removeEventListener('keydown', toggleSearch, true);
      window.removeEventListener('keydown', sidebarShortcut);
    };
  });
</script>

<TitleBar
  {menuActions}
  sidebarCollapsed={sidebar.rail}
  {sidebarLocked}
  sidebarNarrow={sidebar.narrow}
  onToggleSidebar={toggleSidebar}
/>
<div class="scroller">
  <div class="shell">
    {#if !focusMode.active}
      <Sidebar
        {workspace}
        view={showingHome ? 'home' : view}
        onNavigate={(next: View) => void navigate(next)}
        onNewCollection={() => (newCollection = 'empty')}
        newCollectionOpen={newCollection !== null}
        onSearch={() => (searchOpen = true)}
        {searchOpen}
        rail={sidebar.rail}
      />
    {/if}
    <main class:focus={focusMode.active}>
      <div class="content">
        <UpdateBanner
          {updates}
          onInstall={installUpdate}
          onShowNotes={() => void openSettings('about')}
        />
        {#if notices.message}
          <div
            class="banner"
            class:info={notices.tone === 'info'}
            role={notices.tone === 'info' ? 'status' : 'alert'}
          >
            <span>{notices.message}</span>
            {#if notices.action}
              {@const action = notices.action}
              <button
                type="button"
                onclick={() => {
                  // Taken first: the notice's action is gone once it is dismissed.
                  const run = action.run;
                  notices.dismiss();
                  run();
                }}>{action.label}</button
              >
            {/if}
            <button
              type="button"
              onclick={() => {
                notices.dismiss();
              }}>Dismiss</button
            >
          </div>
        {/if}

        {#if view === 'flows'}
          <FlowsView
            {flows}
            {environments}
            onOpenRequest={(collectionId: string, requestId: string) =>
              void openRequestIn(collectionId, requestId)}
          />
        {:else if view === 'catalog'}
          <!-- Loaded on the first visit, so the catalog adds nothing to start-up. -->
          {#await import('./lib/components/CatalogView.svelte') then { default: CatalogView }}
            <CatalogView
              {catalog}
              onOpenRequest={(collectionId: string, requestId: string) =>
                void openRequestIn(collectionId, requestId)}
              onOpenCollection={(collectionId: string) => void openCollectionById(collectionId)}
              onOpenFlow={(flowId: string) => void openFlow(flowId)}
              onRunCollection={(collectionId: string) => void runCollection(collectionId)}
              onNewCollection={() => (newCollection = 'empty')}
            />
          {:catch}
            <p class="banner" role="alert">
              Catalog could not open. Restart Codestamp and try again.
            </p>
          {/await}
        {:else if view === 'docs'}
          <!-- Loaded on the first visit, with the Markdown parser it needs. -->
          {#await import('./lib/components/DocsView.svelte') then { default: DocsView }}
            <DocsView
              {docs}
              {workspace}
              onOpenRequest={(collectionId: string, requestId: string) =>
                void openRequestIn(collectionId, requestId)}
              onNewCollection={() => (newCollection = 'empty')}
            />
          {:catch}
            <p class="banner" role="alert">Docs could not open. Restart Codestamp and try again.</p>
          {/await}
        {:else if view === 'changes'}
          {#await import('./lib/components/GitChangesView.svelte') then { default: GitChangesView }}
            <GitChangesView
              {team}
              {workspace}
              onOpenRequest={(collectionId: string, requestId: string) =>
                void openRequestIn(collectionId, requestId)}
              onOpenCollection={(collectionId: string) => void openCollectionById(collectionId)}
            />
          {:catch}
            <p class="banner" role="alert">
              Git changes could not open. Restart Codestamp and try again.
            </p>
          {/await}
        {:else if view === 'environments'}
          <EnvironmentsView {environments} {workspace} {team} />
        {:else if view === 'cookies'}
          <CookiesView />
        {:else if view === 'trash'}
          <DeletedView />
        {:else if view === 'run'}
          <RunView
            {runner}
            onRunAgain={() => void runAgain()}
            onOpenRequest={(collectionId: string, requestId: string) =>
              void openRequestIn(collectionId, requestId)}
          />
        {:else if view === 'settings'}
          <SettingsView bind:section={settingsSection} {updates} onInstallUpdate={installUpdate} />
        {:else if view === 'import'}
          <ImportView
            {workspace}
            {team}
            pendingFile={importFile}
            preferredCollectionId={importTarget}
            onPendingTaken={() => (importFile = null)}
            onImported={(result: ImportResult) => void showImported(result)}
            onOpenRequest={(collectionId: string, requestId: string) =>
              void openRequestIn(collectionId, requestId)}
          />
        {:else if showingHome}
          <HomeView
            activeStart={newCollection}
            collectionName={workspace.collection?.name ?? null}
            onNewCollection={(start: NewCollectionStart) => (newCollection = start)}
            onNavigate={(next: View) => void navigate(next)}
            onOpenSettings={(section: SettingsSection) => void openSettings(section)}
            onAddRequest={(kind: RequestKind) => void addRequestFromHome(kind)}
            onRunAll={() => void runAll()}
            onOpenHistory={() => void openHistory()}
            onSearch={() => (searchOpen = true)}
          />
        {:else if workspace.collection}
          <CollectionView
            {workspace}
            {environments}
            {team}
            onRunAll={() => void runAll()}
            onPasteCurl={() => void openImport(null, workspace.collection?.id ?? null)}
          />
        {/if}
      </div>
    </main>
  </div>
</div>
{#if !focusMode.active}
  <StatusBar view={showingHome ? 'home' : view} onNavigate={(next: View) => void navigate(next)} />
{/if}

{#if searchOpen}
  <QuickSearch
    collections={workspace.collections}
    environments={environments.list}
    docsFirst={view === 'docs'}
    onPick={(target: SearchTarget) => void openSearched(target)}
    onClose={() => (searchOpen = false)}
  />
{/if}

{#if newCollection}
  <NewCollectionDialog
    {workspace}
    {team}
    start={newCollection}
    onClose={() => (newCollection = null)}
    onCreated={() => {
      newCollection = null;
      view = 'collection';
    }}
    onImportFile={(file: File) => void openImport(file, null)}
    onJoined={(collectionId: string) => {
      newCollection = null;
      void showImported({ kind: 'collection', id: collectionId });
    }}
  />
{/if}

<style>
  /* Everything below the title bar scrolls here, so the scrollbar starts under the title bar.
     `position: relative` keeps absolute children inside it, so they cannot stretch the window. */
  .scroller {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  /* The sidebar always stays beside the content; on narrow windows it becomes an icon rail. */
  .shell {
    min-height: 100%;
    display: flex;
  }

  /* The vertical padding is shared, so a view can fit itself to the window below the title bar. */
  main {
    --main-pad-top: 28px;
    --main-pad-bottom: 48px;
    flex: 1 1 0;
    min-width: 0;
    padding: var(--main-pad-top) 24px var(--main-pad-bottom);
  }

  @media (max-width: 899px) {
    main {
      --main-pad-top: 20px;
      --main-pad-bottom: 40px;
      padding: var(--main-pad-top) 16px var(--main-pad-bottom);
    }
  }

  .content {
    max-width: 1080px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  main.focus {
    --main-pad-top: 18px;
    background: var(--surface-sunken);
  }

  main.focus .content {
    max-width: 1180px;
  }

  /* Large monitors get a wider column, so request and response fit side by side. */
  @media (min-width: 1800px) {
    .content {
      max-width: 1400px;
    }

    main.focus .content {
      max-width: 1560px;
    }
  }

  @media (min-width: 2400px) {
    .content {
      max-width: 1720px;
    }

    main.focus .content {
      max-width: 1880px;
    }
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid var(--err-line);
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .banner.info {
    border-color: var(--accent-soft);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .banner span {
    flex: 1;
  }

  .banner button {
    border: 0;
    background: transparent;
    color: inherit;
    font-weight: 600;
  }
</style>
