import * as api from '../api/history';
import type { SavedRequest } from '../api/collections';
import { notices } from './notices.svelte';

/** One collection's history and its filters. Entries load on demand and are not kept elsewhere. */
export class History {
  entries = $state<api.HistoryEntry[]>([]);
  query = $state('');
  status = $state<api.StatusFilter>('all');
  loaded = $state(false);

  readonly #collectionId: string;
  #loadGeneration = 0;

  constructor(collectionId: string) {
    this.#collectionId = collectionId;
  }

  /** Only the newest search is shown, even if an older one answers later. */
  async load(): Promise<void> {
    const generation = ++this.#loadGeneration;
    await notices.guard(async () => {
      const query = this.query.trim();
      const entries = await api.listHistory({
        query: query === '' ? null : query,
        status: this.status,
        collectionId: this.#collectionId,
      });
      if (generation === this.#loadGeneration) {
        this.entries = entries;
      }
    });
    this.loaded = true;
  }

  async remove(id: string): Promise<void> {
    await notices.guard(async () => {
      await api.deleteHistoryEntry(id);
      this.entries = this.entries.filter((entry) => entry.id !== id);
    });
  }

  async clear(): Promise<void> {
    await notices.guard(async () => {
      await api.clearHistory(this.#collectionId);
      this.entries = [];
    });
  }

  async saveTo(
    id: string,
    collectionId: string,
    sectionId: string | null,
  ): Promise<SavedRequest | null> {
    let saved: SavedRequest | null = null;
    await notices.guard(async () => {
      saved = await api.saveHistoryToCollection(id, collectionId, sectionId);
    });
    return saved;
  }
}
