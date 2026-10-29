import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { CollectionSummary } from '../api/collections';
import { toAppError } from '../api/http';
import * as api from '../api/team';
import { accessGroups, grantNotice } from './access.svelte';
import type { Environments } from './environments.svelte';
import { variableRevision } from './variable-revision.svelte';
import type { Workspace } from './workspace.svelte';

export interface SyncOutcome {
  report: api.SyncReport | null;
  error: string | null;
  at: number;
}

/**
 * Keeps shared collections in step with their repositories when the person presses Sync now.
 * Nothing syncs on its own, so a slow or unreachable Git host never holds up the work.
 */
export class TeamSync {
  setup = $state<api.TeamSetup | null>(null);
  readonly links = new SvelteMap<string, api.TeamStatus>();
  readonly outcomes = new SvelteMap<string, SyncOutcome>();
  readonly syncing = new SvelteSet<string>();
  readonly signingIn = new SvelteSet<string>();
  /** Shared collections with saved changes that are not in the repository yet. */
  readonly unsent = new SvelteSet<string>();

  /** Called after a sync let this device into an access group, with the news to show. */
  onAccessGranted: ((collectionId: string, message: string) => void) | null = null;

  readonly #workspace: Workspace;
  readonly #environments: Environments;
  /** The latest sync of each collection; a new one waits for it. */
  readonly #queue = new SvelteMap<string, Promise<api.SyncReport | null>>();

  constructor(workspace: Workspace, environments: Environments) {
    this.#workspace = workspace;
    this.#environments = environments;
  }

  async load(): Promise<void> {
    const [setup, links] = await Promise.all([api.teamSetup(), api.listTeamLinks()]);
    this.setup = setup;
    this.links.clear();
    for (const link of links) {
      this.links.set(link.collectionId, link);
    }
    this.unsent.clear();
    if (links.length > 0) {
      // Changes saved before the last restart are read from the clones, without the network.
      for (const repo of await api.gitChangesOverview()) {
        if (repo.localEdits > 0 || repo.toPush > 0) {
          this.unsent.add(repo.collectionId);
        }
      }
      // Requests name the group behind a value this device cannot read.
      await Promise.all(links.map((link) => accessGroups.load(link.collectionId)));
    }
  }

  /** Marks shared collections as unsent when their content changes; returns the stop function. */
  start(): () => void {
    return this.#workspace.onChange((collectionId) => {
      if (this.links.has(collectionId)) {
        this.unsent.add(collectionId);
      }
    });
  }

  /** Syncs now, after any sync already running. `decisions` answer the last sync's conflicts. */
  sync(collectionId: string, decisions: api.Decision[] = []): Promise<api.SyncReport | null> {
    const previous = this.#queue.get(collectionId) ?? Promise.resolve(null);
    const run = previous.then(() => this.#run(collectionId, decisions));
    this.#queue.set(collectionId, run);
    void run.then(() => {
      if (this.#queue.get(collectionId) === run) {
        this.#queue.delete(collectionId);
      }
    });
    return run;
  }

  async share(
    collectionId: string,
    remote: string,
    account: api.Account,
    replaceHead: string | null,
  ): Promise<void> {
    // Edits that cannot be saved yet stay unsent until the next Sync now.
    await this.#workspace.flushSave();
    const status = await api.shareCollection(collectionId, remote, account, replaceHead);
    this.links.set(collectionId, status);
    this.outcomes.set(collectionId, { report: null, error: null, at: Date.now() });
    this.unsent.delete(collectionId);
    this.#rememberAuthor(account.author);
  }

  async join(remote: string, account: api.Account): Promise<CollectionSummary> {
    const summary = await api.joinTeamCollection(remote, account);
    this.#rememberAuthor(account.author);
    this.links.clear();
    for (const link of await api.listTeamLinks()) {
      this.links.set(link.collectionId, link);
    }
    return summary;
  }

  /** Signs in for one collection and syncs it when that worked. */
  async signIn(collectionId: string): Promise<api.AccountCheck> {
    this.signingIn.add(collectionId);
    try {
      const check = await api.signInTeamCollection(collectionId);
      if (check.status === 'signedIn') {
        await this.sync(collectionId);
      }
      return check;
    } finally {
      this.signingIn.delete(collectionId);
    }
  }

  async setAccount(collectionId: string, account: api.Account): Promise<void> {
    this.links.set(collectionId, await api.setTeamAccount(collectionId, account));
  }

  async unlink(collectionId: string): Promise<void> {
    await api.unlinkTeamCollection(collectionId);
    this.links.delete(collectionId);
    accessGroups.forget(collectionId);
    this.outcomes.delete(collectionId);
    this.unsent.delete(collectionId);
  }

  async #run(collectionId: string, decisions: api.Decision[]): Promise<api.SyncReport | null> {
    if (!this.links.has(collectionId)) {
      return null;
    }
    this.syncing.add(collectionId);
    try {
      // The sync shares what is saved, so the open card's pending edits are saved first.
      if (this.#workspace.collection?.id === collectionId) {
        await this.#workspace.flushSave();
      }
      const groupsBefore = accessGroups.groups(collectionId);
      const report = await api.syncTeamCollection(collectionId, decisions);
      this.#record(collectionId, report);
      if (report.collectionChanged || report.environmentsChanged || report.accessChanged) {
        variableRevision.bump();
      }
      if (report.accessChanged || accessGroups.overviews.has(collectionId)) {
        await accessGroups.load(collectionId);
        const granted = grantNotice(groupsBefore, accessGroups.groups(collectionId));
        if (granted !== null) {
          this.onAccessGranted?.(collectionId, granted);
        }
      }
      // A grant changes which values open here without changing any environment file.
      if (report.environmentsChanged || report.accessChanged) {
        await this.#environments.load();
      }
      if (report.changed.length > 0 || report.collectionChanged) {
        await this.#workspace.applyRemoteChanges(collectionId, report.changed);
      }
      return report;
    } catch (error) {
      const failure = toAppError(error);
      // The collection was deleted on this device, which also ended its link.
      if (failure.code === 'not_found') {
        this.links.delete(collectionId);
        this.outcomes.delete(collectionId);
        this.unsent.delete(collectionId);
        return null;
      }
      this.outcomes.set(collectionId, { report: null, error: failure.message, at: Date.now() });
      return null;
    } finally {
      this.syncing.delete(collectionId);
    }
  }

  #record(collectionId: string, report: api.SyncReport): void {
    this.outcomes.set(collectionId, { report, error: null, at: Date.now() });
    const link = this.links.get(collectionId);
    if (link) {
      this.links.set(collectionId, {
        ...link,
        conflicts: report.conflicts,
        syncedAt: report.syncedAt ?? link.syncedAt,
      });
    }
    if (report.status === 'synced') {
      this.unsent.delete(collectionId);
    }
  }

  #rememberAuthor(author: api.Author): void {
    const trimmed = { name: author.name.trim(), email: author.email.trim() };
    this.setup = { gitVersion: this.setup?.gitVersion ?? null, author: trimmed };
  }
}
