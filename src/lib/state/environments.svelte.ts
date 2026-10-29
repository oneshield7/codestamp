import * as api from '../api/environments';
import { notices } from './notices.svelte';
import { variableRevision } from './variable-revision.svelte';
import { VariableEditor } from './variable-editor.svelte';

const NEW_ENVIRONMENT_NAME = 'New environment';

/** Globals and named environments, the active one, and the editor for the selected one. */
export class Environments {
  list = $state<api.EnvironmentSummary[]>([]);
  activeId = $state<string | null>(null);
  selectedId = $state<string | null>(null);
  readonly editor = new VariableEditor();

  readonly selected = $derived(this.list.find((env) => env.id === this.selectedId) ?? null);
  readonly named = $derived(this.list.filter((env) => !env.isGlobal));
  readonly active = $derived(this.list.find((env) => env.id === this.activeId) ?? null);

  /** Told when an environment that belongs to a collection changes, so the next sync shares it. */
  #sharedChanged: (collectionId: string) => void = () => undefined;

  watchShared(listener: (collectionId: string) => void): void {
    this.#sharedChanged = listener;
  }

  /** Also after a sync brought environments in: the open one is re-read unless it has edits. */
  async load(): Promise<void> {
    await notices.guard(async () => {
      const result = await api.listEnvironments();
      this.list = result.environments;
      this.activeId = result.activeId;
      if (this.selectedId === null || !this.list.some((env) => env.id === this.selectedId)) {
        await this.select(this.activeId ?? this.list.at(0)?.id ?? null);
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
      for (const owner of [previous, collectionId]) {
        if (owner !== null) {
          this.#sharedChanged(owner);
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
      if (this.activeId === current.id) {
        this.activeId = null;
      }
      await this.select(this.list.at(0)?.id ?? null);
    });
  }

  async setActive(id: string | null): Promise<void> {
    await notices.guard(async () => {
      await api.setActiveEnvironment(id);
      this.activeId = id;
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

  #tellOwner(environment: api.EnvironmentSummary): void {
    if (environment.collectionId !== null) {
      this.#sharedChanged(environment.collectionId);
    }
  }
}
