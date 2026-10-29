<script lang="ts">
  import { PhysicalPosition } from '@tauri-apps/api/dpi';
  import type { Menu } from '@tauri-apps/api/menu';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { notices } from '../state/notices.svelte';
  import type { AppMenuActions } from '../titlebar/app-menu';
  import IconButton from './IconButton.svelte';

  interface Props {
    /** The menu's state and actions, read each time it opens. */
    menuActions: () => Omit<AppMenuActions, 'exit'>;
    /** The sidebar shows as the icon rail. */
    sidebarCollapsed: boolean;
    /** Focus mode or a narrow window: the toggle waits until the full sidebar can show. */
    sidebarLocked: boolean;
    /** A window too narrow for the full sidebar, said in the toggle's tooltip. */
    sidebarNarrow: boolean;
    onToggleSidebar: () => void;
  }

  let { menuActions, sidebarCollapsed, sidebarLocked, sidebarNarrow, onToggleSidebar }: Props =
    $props();

  const toggleLabel = $derived.by(() => {
    if (sidebarNarrow) {
      return 'Widen the window to expand the sidebar';
    }
    return sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar';
  });

  const appWindow = getCurrentWindow();
  let bar: HTMLElement;
  let menuAnchor: HTMLElement;
  let maximized = $state(false);
  let menuOpen = $state(false);
  let menu: Menu | null = null;
  // Where the keyboard was before the title bar took it, so Edit commands reach that field.
  let lastFocused: HTMLElement | null = null;

  function syncMaximized(): void {
    void appWindow.isMaximized().then((value) => {
      maximized = value;
    });
  }

  onMount(() => {
    syncMaximized();
    const track = (event: FocusEvent): void => {
      if (event.target instanceof HTMLElement && !bar.contains(event.target)) {
        lastFocused = event.target;
      }
    };
    document.addEventListener('focusin', track);
    let stopListening: (() => void) | undefined;
    let unmounted = false;
    void appWindow.onResized(syncMaximized).then((stop) => {
      if (unmounted) {
        stop();
      } else {
        stopListening = stop;
      }
    });
    return () => {
      unmounted = true;
      document.removeEventListener('focusin', track);
      stopListening?.();
      void menu?.close();
    };
  });

  function closeWindow(): void {
    void notices.guard(() => appWindow.close());
  }

  async function openMenu(): Promise<void> {
    if (menuOpen) {
      return;
    }
    menuOpen = true;
    if (bar.contains(document.activeElement) && lastFocused?.isConnected) {
      lastFocused.focus();
    }
    await notices.guard(async () => {
      // The previous menu is released only now: its item events have long been delivered.
      await menu?.close();
      // The menu code loads on first use, keeping it out of the startup bundle.
      const { buildAppMenu } = await import('../titlebar/app-menu');
      menu = await buildAppMenu({ ...menuActions(), exit: closeWindow });
      const anchor = menuAnchor.getBoundingClientRect();
      // Popup positions are physical pixels; devicePixelRatio already includes the UI zoom.
      const scale = window.devicePixelRatio;
      await menu.popup(
        new PhysicalPosition(Math.round(anchor.left * scale), Math.round(anchor.bottom * scale)),
      );
    });
    menuOpen = false;
  }
</script>

<header class="titlebar" data-tauri-drag-region bind:this={bar}>
  <div class="tools">
    <span class="anchor" bind:this={menuAnchor}>
      <IconButton
        label="Menu"
        quiet
        keepFocus
        align="start"
        popup="menu"
        expanded={menuOpen}
        onclick={() => void openMenu()}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 6h16M4 12h16M4 18h11" />
        </svg>
      </IconButton>
    </span>
    <IconButton
      label={toggleLabel}
      quiet
      keepFocus
      align="start"
      disabled={sidebarLocked}
      onclick={onToggleSidebar}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 5h16v14H4zM9 5v14" />
      </svg>
    </IconButton>
  </div>
  <div class="drag" data-tauri-drag-region></div>
  <div class="controls" role="group" aria-label="Window">
    <button
      type="button"
      class="control"
      aria-label="Minimize"
      title="Minimize"
      onclick={() => void notices.guard(() => appWindow.minimize())}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <path d="M0 5.5h10" />
      </svg>
    </button>
    <button
      type="button"
      class="control"
      aria-label={maximized ? 'Restore' : 'Maximize'}
      title={maximized ? 'Restore' : 'Maximize'}
      onclick={() => void notices.guard(() => appWindow.toggleMaximize())}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        {#if maximized}
          <path d="M2.5 2.5V0.5h7v7h-2M0.5 2.5h7v7h-7z" />
        {:else}
          <path d="M0.5 0.5h9v9h-9z" />
        {/if}
      </svg>
    </button>
    <button
      type="button"
      class="control close"
      aria-label="Close"
      title="Close"
      onclick={closeWindow}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <path d="M0.5 0.5l9 9M9.5 0.5l-9 9" />
      </svg>
    </button>
  </div>
</header>

<style>
  /* Sits above the scrolling content, never in it; the empty middle drags the window. */
  .titlebar {
    flex: none;
    position: relative;
    z-index: 40;
    display: flex;
    align-items: center;
    height: var(--titlebar-height);
    padding-left: 6px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    user-select: none;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .anchor {
    display: inline-flex;
  }

  .tools svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .drag {
    flex: 1;
    align-self: stretch;
  }

  .controls {
    display: flex;
    align-self: stretch;
  }

  /* Sized and drawn like the Windows 11 caption buttons. */
  .control {
    width: 46px;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .control svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
  }

  .control:hover {
    background: var(--accent-soft);
    color: var(--text);
  }

  .control.close:hover {
    background: var(--window-close-bg);
    color: var(--window-close-text);
  }

  .control:focus-visible {
    outline-offset: -2px;
  }
</style>
