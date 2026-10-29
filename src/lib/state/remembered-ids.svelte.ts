import { SvelteSet } from 'svelte/reactivity';

/** Old ids (of deleted sections, say) are dropped first once a list grows this long. */
const MAX_REMEMBERED = 500;

/**
 * A set of ids remembered on this device, such as folded sections or open groups. A viewing
 * preference: kept here, never shared.
 */
export class RememberedIds {
  readonly #key: string;
  readonly #ids: SvelteSet<string>;
  #persist = true;

  constructor(storageKey: string) {
    this.#key = storageKey;
    this.#ids = new SvelteSet<string>(readStored(storageKey));
  }

  has(id: string): boolean {
    return this.#ids.has(id);
  }

  toggle(id: string): void {
    if (!this.#ids.delete(id)) {
      this.#ids.add(id);
    }
    this.#store();
  }

  add(id: string): void {
    if (!this.#ids.has(id)) {
      this.#ids.add(id);
      this.#store();
    }
  }

  delete(id: string): void {
    if (this.#ids.delete(id)) {
      this.#store();
    }
  }

  #store(): void {
    if (!this.#persist) {
      return;
    }
    try {
      localStorage.setItem(this.#key, JSON.stringify([...this.#ids].slice(-MAX_REMEMBERED)));
    } catch {
      // Storage refused (full or blocked): folding still works, it is just not remembered.
      this.#persist = false;
    }
  }
}

function readStored(storageKey: string): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(storageKey) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === 'string') : [];
  } catch {
    return [];
  }
}
