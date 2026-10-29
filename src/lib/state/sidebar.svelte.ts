import { RememberedIds } from './remembered-ids.svelte';

const STORAGE_KEY = 'codestamp.sidebarCollapsed';
/** Below this width the full sidebar does not fit, so it is always the icon rail. */
const NARROW_QUERY = '(max-width: 899px)';

/**
 * Whether the sidebar is the narrow icon rail. Collapsing is a viewing preference, remembered on
 * this device only; a narrow window shows the rail whatever the preference.
 */
class SidebarState {
  collapsed = $state(readCollapsed());
  narrow = $state(false);
  readonly rail = $derived(this.collapsed || this.narrow);
  /** Groups (Tools, Settings) folded to their heading. The icon rail always shows every item. */
  readonly foldedGroups = new RememberedIds('codestamp.foldedNavGroups');
  #persist = true;

  constructor() {
    const query = window.matchMedia(NARROW_QUERY);
    this.narrow = query.matches;
    query.addEventListener('change', (event) => {
      this.narrow = event.matches;
    });
  }

  toggle(): void {
    if (this.narrow) {
      return;
    }
    this.collapsed = !this.collapsed;
    if (!this.#persist) {
      return;
    }
    try {
      localStorage.setItem(STORAGE_KEY, this.collapsed ? '1' : '0');
    } catch {
      // Storage refused (full or blocked): the toggle still works, it is just not remembered.
      this.#persist = false;
    }
  }
}

function readCollapsed(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

export const sidebar = new SidebarState();
