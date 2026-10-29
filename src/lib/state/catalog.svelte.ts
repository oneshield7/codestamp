import * as api from '../api/catalog';
import { toAppError } from '../api/http';
import { filterApis, sortApis, type SortKey } from '../catalog/present';

export type DetailTab = 'overview' | 'runtime' | 'tests' | 'quality';

/** Catalog: the list of APIs (one per collection) and the one opened from it. */
export class Catalog {
  overview = $state<api.CatalogOverview | null>(null);
  loading = $state(false);
  /** Why the list could not load; the page shows it with a way to try again. */
  error = $state<string | null>(null);

  query = $state('');
  tag = $state<string | null>(null);
  health = $state<api.HealthStatus | null>(null);
  sortKey = $state<SortKey>('name');
  descending = $state(false);

  openId = $state<string | null>(null);
  detail = $state<api.CatalogDetail | null>(null);
  detailError = $state<string | null>(null);
  tab = $state<DetailTab>('overview');

  readonly visible = $derived(
    sortApis(
      filterApis(this.overview?.apis ?? [], {
        query: this.query,
        tag: this.tag,
        health: this.health,
      }),
      this.sortKey,
      this.descending,
    ),
  );

  #listGeneration = 0;
  #detailGeneration = 0;

  async load(): Promise<void> {
    const generation = ++this.#listGeneration;
    this.loading = true;
    try {
      const overview = await api.catalogOverview();
      if (generation === this.#listGeneration) {
        this.overview = overview;
        this.error = null;
        // A tag nobody uses any more would leave an empty list behind.
        if (this.tag !== null && !overview.tags.includes(this.tag)) {
          this.tag = null;
        }
      }
    } catch (caught) {
      if (generation === this.#listGeneration) {
        this.error = toAppError(caught).message;
      }
    } finally {
      if (generation === this.#listGeneration) {
        this.loading = false;
      }
    }
  }

  /** Clicking the column that already sorts the list turns it around. */
  sortBy(key: SortKey): void {
    if (this.sortKey === key) {
      this.descending = !this.descending;
      return;
    }
    this.sortKey = key;
    // Names start at A and health at the worst; the rest start with the highest or newest.
    this.descending = key === 'errorRate' || key === 'p95' || key === 'activity';
  }

  async open(id: string, tab: DetailTab = 'overview'): Promise<void> {
    const generation = ++this.#detailGeneration;
    this.openId = id;
    this.tab = tab;
    this.detail = null;
    this.detailError = null;
    try {
      const detail = await api.catalogDetail(id);
      if (generation === this.#detailGeneration) {
        this.detail = detail;
      }
    } catch (caught) {
      if (generation === this.#detailGeneration) {
        this.detailError = toAppError(caught).message;
      }
    }
  }

  /** Back to the list, which reloads so an edited owner or tag shows there too. */
  async close(): Promise<void> {
    this.openId = null;
    this.detail = null;
    this.detailError = null;
    await this.load();
  }

  /** Saves the open API's owner and tags; returns why it failed, or null. */
  async saveInfo(owner: string, tags: string[]): Promise<string | null> {
    const detail = this.detail;
    if (!detail) {
      return null;
    }
    try {
      const saved = await api.saveCatalogInfo(detail.entry.id, owner, tags);
      detail.entry.owner = saved.owner;
      detail.entry.tags = saved.tags;
      return null;
    } catch (caught) {
      return toAppError(caught).message;
    }
  }
}
