import * as api from '../api/environments';
import { notices } from './notices.svelte';
import { variableRevision } from './variable-revision.svelte';

export interface VariableRow {
  rowId: string;
  id: string | null;
  key: string;
  value: string;
  isSecret: boolean;
  enabled: boolean;
  /** The access group whose members can read the value; always a secret here. */
  restricted: string | null;
  /** Restricted to a group this device is not in: shown, but not editable. */
  locked: boolean;
  /** A secret already stored on disk whose value has not been loaded into the editor. */
  hiddenSecret: boolean;
  visible: boolean;
}

/** `text`, `secret`, or `restricted:<group id>`. */
export type VariableType = string;

export const RESTRICTED_PREFIX = 'restricted:';

export function typeOf(row: VariableRow): VariableType {
  if (row.restricted !== null) {
    return `${RESTRICTED_PREFIX}${row.restricted}`;
  }
  return row.isSecret ? 'secret' : 'text';
}

export function createVariableRow(): VariableRow {
  return {
    rowId: crypto.randomUUID(),
    id: null,
    key: '',
    value: '',
    isSecret: false,
    enabled: true,
    restricted: null,
    locked: false,
    hiddenSecret: false,
    visible: false,
  };
}

/**
 * Edits one variable set (an environment, Globals, or a collection's variables). Changes are
 * saved explicitly, so a half-typed secret never reaches storage.
 */
export class VariableEditor {
  setId = $state<string | null>(null);
  rows = $state<VariableRow[]>([]);
  dirty = $state(false);
  saving = $state(false);

  #loadGeneration = 0;

  // Rows and set id change together, and only for the newest load, so a slow answer for an
  // earlier selection can never put one set's rows under another set's id.
  async load(setId: string | null): Promise<void> {
    const generation = ++this.#loadGeneration;
    await notices.guard(async () => {
      const rows = setId === null ? [] : toRows(await api.getEnvironmentVariables(setId));
      if (generation !== this.#loadGeneration) {
        return;
      }
      this.setId = setId;
      this.rows = rows;
      this.dirty = false;
    });
  }

  markDirty(): void {
    this.dirty = true;
  }

  async toggleVisible(row: VariableRow): Promise<void> {
    if (row.hiddenSecret) {
      await this.#loadSecret(row);
    }
    row.visible = !row.visible;
  }

  async setType(row: VariableRow, type: VariableType): Promise<void> {
    const isSecret = type !== 'text';
    // Turning a stored secret into plain text needs its real value first, or it would be lost.
    if (!isSecret && row.hiddenSecret) {
      await this.#loadSecret(row);
    }
    row.isSecret = isSecret;
    row.restricted = type.startsWith(RESTRICTED_PREFIX)
      ? type.slice(RESTRICTED_PREFIX.length)
      : null;
    row.visible = !isSecret;
    this.markDirty();
  }

  /** Returns the number of saved variables, or null when nothing was saved. */
  async save(): Promise<number | null> {
    const setId = this.setId;
    if (setId === null || this.saving) {
      return null;
    }
    this.saving = true;
    let savedCount: number | null = null;
    await notices.guard(async () => {
      const inputs = this.rows.filter((row) => row.key.trim() !== '').map(toInput);
      const saved = await api.saveEnvironmentVariables(setId, inputs);
      variableRevision.bump();
      savedCount = saved.length;
      // Another set may have been opened while saving; its rows must stay as they are.
      if (this.setId === setId) {
        this.rows = toRows(saved);
        this.dirty = false;
      }
    });
    this.saving = false;
    return savedCount;
  }

  async #loadSecret(row: VariableRow): Promise<void> {
    const id = row.id;
    if (id === null) {
      return;
    }
    await notices.guard(async () => {
      row.value = await api.revealSecret(id);
      row.hiddenSecret = false;
    });
  }
}

function toRows(views: api.VariableView[]): VariableRow[] {
  return [
    ...views.map((view) => ({
      rowId: crypto.randomUUID(),
      id: view.id,
      key: view.key,
      value: view.value,
      isSecret: view.isSecret,
      enabled: view.enabled,
      restricted: view.restricted,
      locked: view.locked,
      hiddenSecret: view.isSecret,
      visible: false,
    })),
    createVariableRow(),
  ];
}

function toInput(row: VariableRow): api.VariableInput {
  return {
    id: row.id,
    key: row.key.trim(),
    value: row.hiddenSecret ? null : row.value,
    isSecret: row.isSecret,
    enabled: row.enabled,
    restricted: row.restricted,
  };
}
