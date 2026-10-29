import { SvelteMap } from 'svelte/reactivity';
import { variableHints } from '../api/environments';
import { variableRevision } from './variable-revision.svelte';

interface Loaded {
  revision: number;
  /** `null` when the names could not be read. */
  names: ReadonlySet<string> | null;
  /** Names whose value comes from the active environment, over Globals and the collection. */
  fromEnvironment: ReadonlySet<string>;
}

const NONE: ReadonlySet<string> = new Set();

/**
 * The variable names each collection can use (Globals, the collection, the active environment
 * and dynamic variables), so `{{name}}` can be coloured as known or unknown. Reloaded after
 * variables or the active environment change.
 */
class VariableNames {
  readonly #loaded = new SvelteMap<string, Loaded>();
  readonly #loading = new Set<string>();

  /** Every name `{{…}}` may use; `null` while unknown, when nothing should be marked missing. */
  known(collectionId: string | null): ReadonlySet<string> | null {
    return this.#loaded.get(collectionId ?? '')?.names ?? null;
  }

  fromEnvironment(collectionId: string | null): ReadonlySet<string> {
    return this.#loaded.get(collectionId ?? '')?.fromEnvironment ?? NONE;
  }

  /** Call from an effect: loads once per collection and variable revision. */
  ensure(collectionId: string | null): void {
    const key = collectionId ?? '';
    const revision = variableRevision.value;
    if (this.#loaded.get(key)?.revision === revision) {
      return;
    }
    const pending = `${key}:${String(revision)}`;
    if (this.#loading.has(pending)) {
      return;
    }
    this.#loading.add(pending);
    void variableHints(collectionId)
      .then((hints) => {
        this.#loaded.set(key, {
          revision,
          names: new Set(hints.map((hint) => hint.name)),
          fromEnvironment: new Set(
            hints.filter((hint) => hint.scope === 'environment').map((hint) => hint.name),
          ),
        });
      })
      .catch(() => {
        // Without names no reference is marked missing; sending still reports an unknown
        // variable, so nothing is lost.
        this.#loaded.set(key, { revision, names: null, fromEnvironment: NONE });
      })
      .finally(() => {
        this.#loading.delete(pending);
      });
  }
}

export const variableNames = new VariableNames();
