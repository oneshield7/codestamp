import * as api from '../api/docs';
import { toAppError } from '../api/http';
import { RememberedIds } from './remembered-ids.svelte';

/** Docs: the open collection's documentation, and the one description being edited. */
export class Docs {
  /** Sections unfolded in the index; the rest stay folded so a long collection stays short. */
  readonly openSections = new RememberedIds('codestamp.openDocsSections');
  collectionId = $state<string | null>(null);
  current = $state<api.CollectionDocs | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);
  /** `item:id` of the description open for editing; one at a time. */
  editing = $state<string | null>(null);
  /** The section or request whose doc is showing; null shows the collection's overview. */
  selected = $state<string | null>(null);
  /** Set when quick search picked the item: the page scrolls to its doc and focuses it. */
  revealPending = $state(false);

  #generation = 0;

  /** Picks the collection the page loads next, and the item to show there. */
  select(collectionId: string, itemId: string | null): void {
    this.collectionId = collectionId;
    this.selected = itemId === collectionId ? null : itemId;
    this.revealPending = true;
  }

  /** Another collection starts on its overview; reopening the same one keeps the selection. */
  async open(collectionId: string): Promise<void> {
    const generation = ++this.#generation;
    if (collectionId !== this.collectionId) {
      this.selected = null;
    }
    this.collectionId = collectionId;
    this.editing = null;
    this.loading = true;
    this.error = null;
    try {
      const docs = await api.collectionDocs(collectionId);
      if (generation === this.#generation) {
        this.current = docs;
      }
    } catch (caught) {
      if (generation === this.#generation) {
        this.current = null;
        this.error = toAppError(caught).message;
      }
    } finally {
      if (generation === this.#generation) {
        this.loading = false;
      }
    }
  }

  /** Reads the open collection again, for requests changed since. */
  async reload(): Promise<void> {
    if (this.collectionId !== null) {
      await this.open(this.collectionId);
    }
  }

  /** Saves one description; resolves to why it failed, or null once it is stored. */
  async save(item: api.Described, id: string, text: string): Promise<string | null> {
    const docs = this.current;
    if (!docs) {
      return null;
    }
    try {
      const stored = await api.saveDescription(item, id, text);
      const target =
        item === 'collection'
          ? docs
          : item === 'section'
            ? docs.sections.find((section) => section.id === id)
            : docs.requests.find((request) => request.id === id);
      if (target) {
        target.description = stored;
      }
      this.editing = null;
      return null;
    } catch (caught) {
      return toAppError(caught).message;
    }
  }
}
