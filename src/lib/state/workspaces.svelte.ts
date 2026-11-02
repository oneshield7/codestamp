import * as api from '../api/workspaces';
import { showOnLaunch } from '../dom/launch';
import { notices } from './notices.svelte';

/**
 * The workspaces and the open one. Opening another reloads Codestamp, the way a backup restore
 * does, so every view reads the new workspace from the start.
 */
class Workspaces {
  list = $state.raw<api.Workspace[]>([]);
  currentId = $state<string | null>(null);
  readonly current = $derived(this.list.find((item) => item.id === this.currentId) ?? null);
  /** Set by the app: saves pending edits and names what still keeps the app from reloading. */
  readyToLeave: () => Promise<string | null> = () => Promise.resolve(null);

  async load(): Promise<void> {
    await notices.guard(async () => {
      this.#apply(await api.listWorkspaces());
    });
  }

  /** What stops opening another workspace, or failures, show in the banner. */
  async open(id: string): Promise<void> {
    if (id === this.currentId || !(await this.#mayLeave())) {
      return;
    }
    await notices.guard(async () => {
      await api.openWorkspace(id);
      this.#reloadInto(id);
    });
  }

  /** Creates a workspace and opens it. Errors are thrown so the dialog can show them. */
  async create(look: api.WorkspaceLook): Promise<void> {
    const created = await api.createWorkspace(look);
    this.list = [...this.list, created];
    await this.open(created.id);
  }

  /** Errors are thrown so the form can show them. */
  async update(id: string, look: api.WorkspaceLook): Promise<void> {
    const updated = await api.updateWorkspace(id, look);
    this.list = this.list.map((item) => (item.id === id ? updated : item));
  }

  /** Deleting the open workspace opens another one. Errors are thrown so the list can show them. */
  async remove(id: string): Promise<void> {
    if (id === this.currentId && !(await this.#mayLeave())) {
      return;
    }
    const left = await api.deleteWorkspace(id);
    const reopened = left.currentId !== this.currentId;
    this.#apply(left);
    if (reopened) {
      this.#reloadInto(left.currentId);
    }
  }

  /** Starts the page over in workspace `id`, with its icon on the launch screen. */
  #reloadInto(id: string): void {
    const opening = this.list.find((item) => item.id === id);
    if (opening) {
      showOnLaunch(opening);
    }
    location.reload();
  }

  async #mayLeave(): Promise<boolean> {
    const blocker = await this.readyToLeave();
    if (blocker) {
      notices.report({ code: 'unsaved', message: blocker });
    }
    return blocker === null;
  }

  #apply(list: api.WorkspaceList): void {
    this.list = list.workspaces;
    this.currentId = list.currentId;
  }
}

export const workspaces = new Workspaces();
