import { SvelteMap } from 'svelte/reactivity';
import * as api from '../api/environments';
import { notices } from './notices.svelte';
import { variableRevision } from './variable-revision.svelte';
import { VariableEditor } from './variable-editor.svelte';

const NEW_ENVIRONMENT_NAME = 'New environment';

/**
 * Globals and named environments, the environment each collection uses, and the editor for the
 * selected one.
 */
export class Environments {
  list = $state<api.EnvironmentSummary[]>([]);
  /** The active environment of each collection, by collection id. */
  readonly activeIds = new SvelteMap<string, string>();
  selectedId = $state<string | null>(null);
  readonly editor = new VariableEditor();

  readonly selected = $derived(this.list.find((env) => env.id === this.selectedId) ?? null);
  readonly named = $derived(this.list.filter((env) => !env.isGlobal));

  /**
   * The environments a collection can use: its own and the ones kept on this device. Another
   * collection's environment never fills in values here.
   */
  usableIn(collectionId: string): api.EnvironmentSummary[] {
    return this.named.filter(
      (env) => env.collectionId === null || env.collectionId === collectionId,
    );
  }

  /** The environment this collection uses, if any. */
  activeIn(collectionId: string): api.EnvironmentSummary | null {
    const id = this.activeIds.get(collectionId);
    return this.usableIn(collectionId).find((env) => env.id === id) ?? null;
  }

  /** The collections that use this environment. */
  usedBy(environmentId: string): string[] {
    return [...this.activeIds.keys()].filter(
      (collectionId) => this.activeIn(collectionId)?.id === environmentId,
    );
  }

  /** Also after a sync brought environments in: the open one is re-read unless it has edits. */
  async load(): Promise<void> {
    await notices.guard(async () => {
      const result = await api.listEnvironments();
      this.list = result.environments;
      this.activeIds.clear();
      for (const [collectionId, id] of Object.entries(result.activeIds)) {
        this.activeIds.set(collectionId, id);
      }
      if (this.selectedId === null || !this.list.some((env) => env.id === this.selectedId)) {
        await this.select(this.list.at(0)?.id ?? null);
      } else if (!this.editor.dirty) {
        await this.editor.load(this.selectedId);
      }
    });
  }

  /** Gives the open environment to a collection (shared with it) or back to this device. */
  async setCollection(collectionId: string | null): Promise<void> {
    const current = this.selected;
    if (!current || current.isGlobal || current.collectionId === collectionId) {
      return;
    }
    await notices.guard(async () => {
      await api.setEnvironmentCollection(current.id, collectionId);
      const previous = current.collectionId;
      current.collectionId = collectionId;
      // Once it belongs to one collection, the others stop using it.
      if (collectionId !== null) {
        this.#forget(current.id, collectionId);
      }
      for (const owner of [previous, collectionId]) {
        if (owner !== null) {
          variableRevision.sharedChanged(owner);
        }
      }
    });
  }

  async select(id: string | null): Promise<void> {
    this.selectedId = id;
    await this.editor.load(id);
  }

  async create(): Promise<void> {
    await notices.guard(async () => {
      const created = await api.createEnvironment(NEW_ENVIRONMENT_NAME);
      this.list.push(created);
      await this.select(created.id);
    });
  }

  async rename(name: string): Promise<void> {
    const current = this.selected;
    if (!current || current.isGlobal) {
      return;
    }
    await notices.guard(async () => {
      await api.renameEnvironment(current.id, name);
      current.name = name.trim();
      this.#tellOwner(current);
    });
  }

  async remove(): Promise<void> {
    const current = this.selected;
    if (!current || current.isGlobal) {
      return;
    }
    await notices.guard(async () => {
      await api.deleteEnvironment(current.id);
      this.#tellOwner(current);
      this.list = this.list.filter((env) => env.id !== current.id);
      this.#forget(current.id, null);
      await this.select(this.list.at(0)?.id ?? null);
    });
  }

  /** `id: null` leaves the collection without an environment. */
  async setActive(collectionId: string, id: string | null): Promise<void> {
    await notices.guard(async () => {
      await api.setActiveEnvironment(collectionId, id);
      if (id === null) {
        this.activeIds.delete(collectionId);
      } else {
        this.activeIds.set(collectionId, id);
      }
      variableRevision.bump();
    });
  }

  async save(): Promise<void> {
    const current = this.selected;
    const count = await this.editor.save();
    if (current && count !== null) {
      current.variableCount = count;
      this.#tellOwner(current);
    }
  }

  /** Drops an environment from every collection that uses it, except `keep`. */
  #forget(environmentId: string, keep: string | null): void {
    for (const [collectionId, id] of [...this.activeIds]) {
      if (id === environmentId && collectionId !== keep) {
        this.activeIds.delete(collectionId);
      }
    }
  }

  #tellOwner(environment: api.EnvironmentSummary): void {
    if (environment.collectionId !== null) {
      variableRevision.sharedChanged(environment.collectionId);
    }
  }
}
