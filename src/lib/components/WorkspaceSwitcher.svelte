<script lang="ts">
  import { tick } from 'svelte';
  import type { Workspace } from '../api/workspaces';
  import { plural } from '../format/plural';
  import { workspaces } from '../state/workspaces.svelte';
  import WorkspaceTile from './WorkspaceTile.svelte';

  interface Props {
    onNew: () => void;
    onManage: () => void;
  }

  let { onNew, onManage }: Props = $props();

  type Entry = { kind: 'workspace'; workspace: Workspace } | { kind: 'new' } | { kind: 'manage' };

  const GAP_PX = 6;
  const EDGE_PX = 8;
  const menuId = `workspaces-${crypto.randomUUID()}`;

  let trigger: HTMLButtonElement | undefined = $state();
  let menu: HTMLUListElement | undefined = $state();
  let open = $state(false);
  let active = $state(0);
  let position = $state({ top: 0, left: 0 });

  const current = $derived(workspaces.current);
  const entries = $derived<Entry[]>([
    ...workspaces.list.map((workspace): Entry => ({ kind: 'workspace', workspace })),
    { kind: 'new' },
    { kind: 'manage' },
  ]);
  const firstAction = $derived(workspaces.list.length);

  async function show(): Promise<void> {
    if (open || !trigger) {
      return;
    }
    active = Math.max(
      0,
      workspaces.list.findIndex((item) => item.id === workspaces.currentId),
    );
    open = true;
    await tick();
    if (!menu) {
      return;
    }
    const box = trigger.getBoundingClientRect();
    const width = menu.offsetWidth;
    const centered = box.left + box.width / 2 - width / 2;
    position = {
      top: box.bottom + GAP_PX,
      left: Math.min(Math.max(EDGE_PX, centered), window.innerWidth - width - EDGE_PX),
    };
    menu.focus({ preventScroll: true });
  }

  function hide(refocus: boolean): void {
    open = false;
    if (refocus) {
      trigger?.focus();
    }
  }

  function choose(index: number): void {
    const entry = entries.at(index);
    hide(true);
    if (entry?.kind === 'workspace') {
      void workspaces.open(entry.workspace.id);
    } else if (entry?.kind === 'new') {
      onNew();
    } else if (entry?.kind === 'manage') {
      onManage();
    }
  }

  function move(to: number): void {
    active = Math.min(entries.length - 1, Math.max(0, to));
    menu
      ?.querySelector<HTMLElement>(`#${menuId}-${String(active)}`)
      ?.scrollIntoView({ block: 'nearest' });
  }

  function onTriggerKey(event: KeyboardEvent): void {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
      event.preventDefault();
      void show();
    }
  }

  function onMenuKey(event: KeyboardEvent): void {
    const keys = new Map<string, () => void>([
      [
        'ArrowDown',
        () => {
          move(active + 1);
        },
      ],
      [
        'ArrowUp',
        () => {
          move(active - 1);
        },
      ],
      [
        'Home',
        () => {
          move(0);
        },
      ],
      [
        'End',
        () => {
          move(entries.length - 1);
        },
      ],
      [
        'Enter',
        () => {
          choose(active);
        },
      ],
      [
        ' ',
        () => {
          choose(active);
        },
      ],
      // Escape belongs to the menu: it must not also leave focus mode or close a dialog.
      [
        'Escape',
        () => {
          hide(true);
        },
      ],
      [
        'Tab',
        () => {
          hide(true);
        },
      ],
    ]);
    const action = keys.get(event.key);
    if (action) {
      event.preventDefault();
      event.stopPropagation();
      action();
    }
  }

  function onOutside(event: Event): void {
    const target = event.target;
    if (!(target instanceof Node) || trigger?.contains(target) || menu?.contains(target)) {
      return;
    }
    hide(false);
  }

  $effect(() => {
    if (!open) {
      return;
    }
    window.addEventListener('pointerdown', onOutside, true);
    window.addEventListener('resize', onOutside);
    return () => {
      window.removeEventListener('pointerdown', onOutside, true);
      window.removeEventListener('resize', onOutside);
    };
  });
</script>

{#if current}
  <button
    bind:this={trigger}
    type="button"
    class="trigger"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={open ? menuId : undefined}
    aria-label="Workspace: {current.name}"
    onclick={() => {
      if (open) {
        hide(true);
      } else {
        void show();
      }
    }}
    onkeydown={onTriggerKey}
  >
    <WorkspaceTile name={current.name} hue={current.hue} icon={current.icon} size="sm" />
    <span class="name">{current.name}</span>
    <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M6 9l6 6 6-6" />
    </svg>
  </button>
{/if}

{#if open}
  <ul
    bind:this={menu}
    id={menuId}
    role="menu"
    tabindex="-1"
    aria-label="Workspaces"
    aria-activedescendant="{menuId}-{active}"
    style:top="{position.top}px"
    style:left="{position.left}px"
    onkeydown={onMenuKey}
  >
    {#each entries as entry, index (entry.kind === 'workspace' ? entry.workspace.id : entry.kind)}
      {#if index === firstAction}
        <li class="separator" role="separator"></li>
      {/if}
      <li
        id="{menuId}-{index}"
        role={entry.kind === 'workspace' ? 'menuitemradio' : 'menuitem'}
        aria-checked={entry.kind === 'workspace'
          ? entry.workspace.id === workspaces.currentId
          : undefined}
        class:active={index === active}
        onmousemove={() => (active = index)}
        onmousedown={(event) => {
          event.preventDefault();
          choose(index);
        }}
      >
        {#if entry.kind === 'workspace'}
          {@const item = entry.workspace}
          <WorkspaceTile name={item.name} hue={item.hue} icon={item.icon} />
          <span class="label">{item.name}</span>
          <span class="count" aria-hidden="true">{item.collectionCount}</span>
          <span class="visually-hidden">, {plural(item.collectionCount, 'collection')}</span>
          {#if item.id === workspaces.currentId}
            <svg class="check" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M5 12l5 5 9-10" />
            </svg>
          {/if}
        {:else if entry.kind === 'new'}
          <svg class="icon" width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 5v14M5 12h14" />
          </svg>
          <span class="label">New workspace</span>
        {:else}
          <svg class="icon" width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 7h10M18 7h2M4 17h4M12 17h8M14 5v4M8 15v4" />
          </svg>
          <span class="label">Manage workspaces</span>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  /* Centered in the title bar, so it stays in view when the sidebar folds to the rail. */
  .trigger {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: min(260px, calc(100vw - 360px));
    height: 26px;
    padding: 0 6px 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
  }

  .trigger:hover {
    border-color: var(--text-muted);
  }

  .trigger:active,
  .trigger[aria-expanded='true'] {
    border-color: var(--text);
    background: var(--accent-soft);
  }

  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    flex: none;
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.15s;
  }

  .trigger[aria-expanded='true'] .chevron {
    transform: rotate(180deg);
  }

  ul {
    position: fixed;
    z-index: 60;
    width: min(280px, calc(100vw - 16px));
    max-height: calc(100vh - var(--titlebar-height) - 24px);
    margin: 0;
    padding: 4px;
    overflow: auto;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    outline: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    font-size: 13px;
    cursor: pointer;
  }

  li.active {
    background: var(--accent-soft);
    color: var(--text);
  }

  li[aria-checked='true'] {
    color: var(--text);
    font-weight: 600;
  }

  .separator {
    height: 1px;
    margin: 4px 2px;
    padding: 0;
    background: var(--border-soft);
    cursor: default;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    color: var(--text-muted);
    font-family: var(--font-code);
    font-size: 11px;
    font-weight: 400;
  }

  .check,
  .icon {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .check {
    stroke: var(--focus);
    stroke-width: 2.2;
  }

  /* Lines the action icons up with the workspace tiles above them. */
  .icon {
    width: 22px;
  }
</style>
