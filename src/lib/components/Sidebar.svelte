<script lang="ts">
  import logo from '../../../branding/codestamp-logo.svg';
  import FoldButton from './FoldButton.svelte';
  import NavIcon from './NavIcon.svelte';
  import { navLabel, type NavPage, type View } from '../state/navigation';
  import { sidebar } from '../state/sidebar.svelte';
  import type { Workspace } from '../state/workspace.svelte';

  interface Props {
    workspace: Workspace;
    view: View;
    onNavigate: (view: View) => void;
    onNewCollection: () => void;
    newCollectionOpen: boolean;
    onSearch: () => void;
    searchOpen: boolean;
    /** The narrow icon rail: collapsed by the user, or a window too narrow for the full list. */
    rail: boolean;
  }

  let {
    workspace,
    view,
    onNavigate,
    onNewCollection,
    newCollectionOpen,
    onSearch,
    searchOpen,
    rail,
  }: Props = $props();

  // Settings, Recently deleted, Cookies and Environments sit in the status bar at the bottom.
  const GROUPS: readonly { title: string; items: readonly NavPage[] }[] = [
    { title: 'Tools', items: ['flows', 'catalog', 'docs', 'changes'] },
  ];

  function openCollection(id: string): void {
    onNavigate('collection');
    void workspace.openCollection(id);
  }

  /**
   * Beside the hovered or keyboard-focused button that has `data-tip`. It belongs to the layout it
   * was measured in, so expanding or collapsing the sidebar under a focused button drops it.
   */
  let tip = $state<{ text: string; top: number; left: number; rail: boolean } | null>(null);

  // Fixed to the window, because the scrolling sidebar would clip a tooltip placed inside it.
  function tooltips(nav: HTMLElement): () => void {
    const show = (event: Event): void => {
      const button =
        event.target instanceof Element ? event.target.closest<HTMLElement>('[data-tip]') : null;
      if (!button?.dataset.tip) {
        tip = null;
        return;
      }
      if (event.type === 'focusin' && !button.matches(':focus-visible')) {
        return;
      }
      const box = button.getBoundingClientRect();
      tip = { text: button.dataset.tip, top: box.top + box.height / 2, left: box.right + 8, rail };
    };
    const hide = (): void => {
      tip = null;
    };
    nav.addEventListener('pointerover', show);
    nav.addEventListener('focusin', show);
    nav.addEventListener('pointerleave', hide);
    nav.addEventListener('focusout', hide);
    nav.addEventListener('scroll', hide);
    return () => {
      nav.removeEventListener('pointerover', show);
      nav.removeEventListener('focusin', show);
      nav.removeEventListener('pointerleave', hide);
      nav.removeEventListener('focusout', hide);
      nav.removeEventListener('scroll', hide);
    };
  }
</script>

<nav class="sidebar" class:rail id="sidebar" aria-label="Navigation" {@attach tooltips}>
  <div class="brand">
    <img src={logo} width="28" height="28" alt="" />
    <span class="brand-name">Codestamp</span>
  </div>
  <button
    type="button"
    class="search"
    aria-haspopup="dialog"
    aria-expanded={searchOpen}
    aria-keyshortcuts="Control+Space"
    data-tip={rail ? 'Search' : undefined}
    onclick={onSearch}
  >
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" />
    </svg>
    <span class="name">Search</span>
  </button>
  <button
    type="button"
    class="item"
    class:on={view === 'home'}
    data-tip={rail ? 'Home' : undefined}
    aria-current={view === 'home' ? 'page' : undefined}
    onclick={() => {
      onNavigate('home');
    }}
  >
    <NavIcon name="home" />
    <span class="name">Home</span>
  </button>

  <div class="group">
    <h2>Collections</h2>
    <button
      type="button"
      class="new"
      aria-haspopup="dialog"
      aria-expanded={newCollectionOpen}
      aria-label="New collection"
      data-tip="New collection"
      onclick={onNewCollection}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 5v14M5 12h14" />
      </svg>
    </button>
  </div>
  {#each workspace.collections as item (item.id)}
    {@const active =
      (view === 'collection' || view === 'run') && item.id === workspace.collection?.id}
    <button
      type="button"
      class="item"
      class:on={active}
      aria-current={active ? 'page' : undefined}
      title={rail ? undefined : item.name}
      data-tip={rail ? item.name : undefined}
      onclick={() => {
        openCollection(item.id);
      }}
    >
      <span class="icon emoji" aria-hidden="true">{item.icon}</span>
      <span class="name">{item.name}</span>
      <span class="count">{item.requestCount}</span>
    </button>
  {:else}
    {#if workspace.loaded}
      <p class="none">No collections yet.</p>
    {/if}
  {/each}

  {#each GROUPS as group (group.title)}
    {@const open = rail || !sidebar.foldedGroups.has(group.title)}
    <div class="section" role="group" aria-label={group.title}>
      <div class="group">
        <h2>
          <FoldButton
            expanded={open}
            label={group.title}
            onToggle={() => {
              sidebar.foldedGroups.toggle(group.title);
            }}
          >
            {group.title}
          </FoldButton>
        </h2>
      </div>
      {#each open ? group.items : [] as page (page)}
        <button
          type="button"
          class="item"
          class:on={view === page}
          data-tip={rail ? navLabel(page) : undefined}
          aria-current={view === page ? 'page' : undefined}
          onclick={() => {
            onNavigate(page);
          }}
        >
          <NavIcon name={page} />
          <span class="name">{navLabel(page)}</span>
        </button>
      {/each}
    </div>
  {/each}
</nav>
{#if tip?.rail === rail}
  <span class="tip" style:top="{tip.top}px" style:left="{tip.left}px" aria-hidden="true"
    >{tip.text}</span
  >
{/if}

<style>
  /* Stays in view while the content scrolls, and scrolls on its own when it holds many
     collections. */
  .sidebar {
    flex: none;
    position: sticky;
    top: 0;
    align-self: flex-start;
    height: calc(100vh - var(--titlebar-height) - var(--statusbar-height));
    overflow-x: hidden;
    overflow-y: auto;
    scrollbar-width: thin;
    width: 264px;
    padding: 20px 14px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-right: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 18px;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  /* The right padding lines the icon buttons' glyphs up with the counts in the rows below. */
  .group {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 4px 6px 8px;
  }

  /* Tools sits at the foot of the sidebar when there is room, and after the collections when
     the list is long. */
  .section {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    margin-top: auto;
    padding-top: 8px;
  }

  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  /* A folding heading keeps the heading's look; its chevron sits where the text started. */
  h2 :global(.fold) {
    height: 24px;
    margin-left: -6px;
    color: inherit;
    font-weight: inherit;
  }

  .new {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .new:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .new:active,
  .new[aria-expanded='true'] {
    background: var(--accent-soft);
    color: var(--text);
  }

  .item:active {
    background: var(--accent-soft);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    padding: 7px 8px 7px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
    font-size: 13px;
    text-align: left;
  }

  .search:hover {
    border-color: var(--border-strong);
    color: var(--text-2);
  }

  .search[aria-expanded='true'] {
    border-color: var(--focus);
    box-shadow: 0 0 0 1px var(--focus);
  }

  .search svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-2);
    text-align: left;
  }

  .item:hover {
    background: var(--surface-sunken);
  }

  .item.on {
    border-color: var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-raise);
    color: var(--text);
    font-weight: 500;
  }

  .new svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .icon {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    font-family: var(--font-code);
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .none {
    margin: 0;
    padding: 4px 10px;
    font-size: 13px;
    color: var(--text-muted);
  }

  /* The icon rail (collapsed, or a narrow window). Names stay readable to screen readers and
     show as tooltips. */
  .rail {
    width: 64px;
    padding: 16px 10px;
    align-items: center;
  }

  .rail .brand {
    padding: 0 0 14px;
  }

  .rail .brand-name,
  .rail .count,
  .rail .none,
  .rail h2 {
    display: none;
  }

  .rail .name {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .rail .search,
  .rail .item {
    width: 44px;
    justify-content: center;
    padding: 8px;
  }

  .rail .section {
    align-items: center;
  }

  .rail .group {
    justify-content: center;
    width: 100%;
    margin-top: 10px;
    padding: 10px 0 4px;
    border-top: 1px solid var(--border);
  }

  /* Drawn like the icon buttons' tooltips. A long collection name wraps instead of being cut. */
  .tip {
    position: fixed;
    z-index: 45;
    max-width: 280px;
    transform: translateY(-50%);
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    overflow-wrap: anywhere;
    pointer-events: none;
  }
</style>
