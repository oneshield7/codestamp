import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import * as api from '../api/collections';
import { toAppError, type HttpResponseOutput } from '../api/http';
import type { ChangedRequest } from '../api/team';
import { recentlyDeleted } from './deleted.svelte';
import { notices } from './notices.svelte';
import { contentFromDraft, createDraft, draftFromSaved, type RequestDraft } from '../request/draft';

export type SaveState = 'saved' | 'unsaved' | 'saving' | 'failed';

export interface LastResult {
  status: number;
  elapsedMs: number;
}

const AUTOSAVE_DELAY_MS = 600;
const NEW_SECTION_NAME = 'New section';

/**
 * Holds the open collection and at most one open request. Only the open request keeps its full
 * content (and an editor) in memory; every other request is a lightweight summary.
 */
export class Workspace {
  loaded = $state(false);
  collections = $state<api.CollectionSummary[]>([]);
  collection = $state<api.CollectionDetail | null>(null);
  openRequestId = $state<string | null>(null);
  draft = $state<RequestDraft | null>(null);
  saveState = $state<SaveState>('saved');
  saveError = $state<string | null>(null);
  /** The collection created in this session most recently, for its "created" note. */
  createdId = $state<string | null>(null);
  readonly lastResults = new SvelteMap<string, LastResult>();

  #saveTimer: ReturnType<typeof setTimeout> | undefined;
  #inFlight: Promise<boolean> | null = null;
  #openGeneration = 0;
  #loading: Promise<void> = Promise.resolve();
  /** Panels with unsaved edits; while any returns a title the collection is not left. */
  readonly #leaveGuards = new SvelteSet<() => string | null>();
  readonly #changeListeners = new SvelteSet<(collectionId: string) => void>();
  #editRevision = 0;

  async load(): Promise<void> {
    this.#loading = notices.guard(async () => {
      this.collections = await api.listCollections();
      const first = this.collections.at(0);
      if (first && !this.collection) {
        await this.openCollection(first.id);
      }
    });
    await this.#loading;
    this.loaded = true;
  }

  /** Creates a collection and opens it. Failures are thrown, so the dialog can show them. */
  async createCollection(input: api.NewCollection): Promise<void> {
    // Waits for the startup load, whose collection list would otherwise overwrite the new entry.
    await this.#loading;
    const created = await api.createCollection(input);
    this.collections.push(created);
    this.createdId = created.id;
    await this.openCollection(created.id);
  }

  /** Registers a panel whose edits leaving would drop: the guard returns its title while unsaved. */
  addLeaveGuard(guard: () => string | null): () => void {
    this.#leaveGuards.add(guard);
    return () => {
      this.#leaveGuards.delete(guard);
    };
  }

  /** The title of the first open panel with unsaved edits. */
  unsavedPanel(): string | null {
    for (const guard of this.#leaveGuards) {
      const title = guard();
      if (title) {
        return title;
      }
    }
    return null;
  }

  mayLeave(): boolean {
    const title = this.unsavedPanel();
    if (title) {
      notices.report({
        code: 'unsaved',
        message: `Save or close ${title.toLowerCase()} before leaving this collection.`,
      });
    }
    return title === null;
  }

  /** Registers a listener for saved changes to a collection; returns its removal. */
  onChange(listener: (collectionId: string) => void): () => void {
    this.#changeListeners.add(listener);
    return () => {
      this.#changeListeners.delete(listener);
    };
  }

  /** Tells listeners (team sync) that a collection's saved content changed. */
  notifyChanged(collectionId: string): void {
    for (const listener of this.#changeListeners) {
      listener(collectionId);
    }
  }

  /**
   * Shows what a teammate changed; it is already saved on this device. The open card is reloaded
   * when it has no unsaved edits. Otherwise those edits are kept (and shared on the next sync),
   * and the user is told so they can compare in the request's history.
   */
  async applyRemoteChanges(collectionId: string, changed: ChangedRequest[]): Promise<void> {
    await this.refresh(collectionId);
    const openId = this.openRequestId;
    const touched = changed.find((item) => item.id === openId);
    if (!openId || !touched || this.#openCollectionId() !== collectionId) {
      return;
    }
    if (touched.change === 'removed') {
      this.#dropOpenCard();
      notices.inform(`A collaborator deleted “${touched.name}”.`);
      return;
    }
    if (this.saveState !== 'saved') {
      notices.inform(
        `A collaborator changed “${touched.name}” while you were editing it. Your edits are kept and will be shared; its history shows their version.`,
      );
      return;
    }
    await notices.guard(async () => {
      const saved = await api.getRequest(openId);
      if (this.openRequestId === openId && this.saveState === 'saved') {
        this.#showRequest(saved);
      }
    });
  }

  /** Puts a request saved outside the card (a restored version) into the open card. */
  showSaved(saved: api.SavedRequest): void {
    if (this.openRequestId !== saved.id) {
      return;
    }
    clearTimeout(this.#saveTimer);
    const existing = this.collection?.requests.find((request) => request.id === saved.id);
    if (existing) {
      Object.assign(existing, {
        kind: saved.kind,
        name: saved.name,
        method: saved.method,
        url: saved.url,
      });
    }
    this.#showRequest(saved);
  }

  // Only the newest open wins: the startup load can finish after the user already created or
  // picked another collection, and must not replace it (later edits would go to the wrong one).
  async openCollection(id: string): Promise<void> {
    if (this.collection?.id !== id && !this.mayLeave()) {
      return;
    }
    const generation = ++this.#openGeneration;
    await notices.guard(async () => {
      if (!(await this.closeRequest())) {
        return;
      }
      const detail = await api.getCollection(id);
      if (generation === this.#openGeneration) {
        this.collection = detail;
      }
    });
  }

  /** Re-reads collection counts and, when it is open, the given collection. */
  async refresh(collectionId: string): Promise<void> {
    await notices.guard(async () => {
      this.collections = await api.listCollections();
      if (this.collection?.id !== collectionId) {
        return;
      }
      const detail = await api.getCollection(collectionId);
      // The user may have opened another collection meanwhile.
      if (this.#openCollectionId() === collectionId) {
        this.collection = detail;
      }
    });
  }

  /** Moves a request into a section (null = no section), before `beforeId` or at the end. */
  async moveRequest(id: string, sectionId: string | null, beforeId: string | null): Promise<void> {
    const layout = this.#layout();
    if (!layout || beforeId === id) {
      return;
    }
    for (const group of layout.groups) {
      group.requestIds = group.requestIds.filter((requestId) => requestId !== id);
    }
    const target = layout.groups.find((group) => group.sectionId === sectionId);
    if (!target) {
      return;
    }
    const index = beforeId === null ? -1 : target.requestIds.indexOf(beforeId);
    target.requestIds.splice(index < 0 ? target.requestIds.length : index, 0, id);
    await this.#applyLayout(layout);
  }

  /** One step up or down; past the edge of a section it moves into the neighbouring one. */
  async nudgeRequest(id: string, step: -1 | 1): Promise<void> {
    const layout = this.#layout();
    const groupIndex = layout?.groups.findIndex((group) => group.requestIds.includes(id)) ?? -1;
    const group = layout?.groups.at(groupIndex);
    if (!layout || !group || groupIndex < 0) {
      return;
    }
    const index = group.requestIds.indexOf(id);
    const swapWith = group.requestIds.at(index + step);
    if (index + step >= 0 && swapWith !== undefined) {
      group.requestIds.splice(index, 1);
      group.requestIds.splice(index + step, 0, id);
    } else {
      const neighbour = groupIndex + step >= 0 ? layout.groups.at(groupIndex + step) : undefined;
      if (!neighbour) {
        return;
      }
      group.requestIds.splice(index, 1);
      if (step < 0) {
        neighbour.requestIds.push(id);
      } else {
        neighbour.requestIds.unshift(id);
      }
    }
    await this.#applyLayout(layout);
  }

  async moveSection(id: string, beforeId: string | null): Promise<void> {
    const layout = this.#layout();
    if (!layout || beforeId === id) {
      return;
    }
    const sections = layout.sections.filter((sectionId) => sectionId !== id);
    const index = beforeId === null ? -1 : sections.indexOf(beforeId);
    sections.splice(index < 0 ? sections.length : index, 0, id);
    await this.#applyLayout({ ...layout, sections });
  }

  async nudgeSection(id: string, step: -1 | 1): Promise<void> {
    const layout = this.#layout();
    const index = layout?.sections.indexOf(id) ?? -1;
    if (!layout || index < 0 || index + step < 0 || index + step >= layout.sections.length) {
      return;
    }
    layout.sections.splice(index, 1);
    layout.sections.splice(index + step, 0, id);
    await this.#applyLayout(layout);
  }

  async renameCollection(name: string): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      const summary = await api.renameCollection(current.id, name);
      this.notifyChanged(current.id);
      current.name = summary.name;
      current.icon = summary.icon;
      this.#replaceSummary(summary);
    });
  }

  /** `null` lets the icon follow the name again. */
  async setCollectionIcon(icon: string | null): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      const summary = await api.setCollectionIcon(current.id, icon);
      // A chosen icon is part of a team collection's collection.json.
      this.notifyChanged(current.id);
      current.icon = summary.icon;
      current.iconCustom = icon !== null;
      this.#replaceSummary(summary);
    });
  }

  async deleteCollection(): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      // The open card belongs to this collection, so its unsaved edits go with it.
      this.#dropOpenCard();
      const deleted = await api.deleteCollection(current.id);
      this.collections = this.collections.filter((item) => item.id !== current.id);
      this.collection = null;
      const next = this.collections.at(0);
      if (next) {
        await this.openCollection(next.id);
      }
      recentlyDeleted.announce(deleted);
    });
  }

  async setCollectionAuth(auth: api.Auth): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      await api.setCollectionAuth(current.id, auth);
      this.notifyChanged(current.id);
      current.auth = auth;
    });
  }

  async addSection(): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      current.sections.push(await api.createSection(current.id, NEW_SECTION_NAME));
      this.notifyChanged(current.id);
    });
  }

  async renameSection(id: string, name: string): Promise<void> {
    const collectionId = this.collection?.id;
    await notices.guard(async () => {
      await api.renameSection(id, name);
      if (collectionId) {
        this.notifyChanged(collectionId);
      }
      const section = this.collection?.sections.find((item) => item.id === id);
      if (section) {
        section.name = name.trim();
      }
    });
  }

  async deleteSection(id: string): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      await api.deleteSection(id);
      this.notifyChanged(current.id);
      current.sections = current.sections.filter((section) => section.id !== id);
      for (const request of current.requests) {
        if (request.sectionId === id) {
          request.sectionId = null;
        }
      }
    });
  }

  async addRequest(sectionId: string | null, kind: api.RequestKind): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      if (!(await this.closeRequest())) {
        return;
      }
      const saved = await api.createRequest(
        current.id,
        sectionId,
        contentFromDraft(createDraft(kind)),
      );
      this.notifyChanged(current.id);
      current.requests.push({
        id: saved.id,
        sectionId: saved.sectionId,
        kind: saved.kind,
        name: saved.name,
        method: saved.method,
        url: saved.url,
      });
      this.#adjustCount(current.id, 1);
      this.#showRequest(saved);
    });
  }

  async openRequest(id: string): Promise<void> {
    if (this.openRequestId === id) {
      return;
    }
    await notices.guard(async () => {
      if (!(await this.closeRequest())) {
        return;
      }
      this.#showRequest(await api.getRequest(id));
    });
  }

  /**
   * Saves and closes the open card. When saving fails the card stays open and the user is told
   * why, so edits are never thrown away silently. Resolves to whether the card was closed.
   */
  async closeRequest(): Promise<boolean> {
    if (!(await this.flushSave())) {
      const name = this.draft?.name.trim() || 'The open request';
      notices.report({
        code: 'unsaved',
        message: `“${name}” could not be saved (${this.saveError ?? 'unknown error'}). Fix it, or undo the change, before leaving the card.`,
      });
      return false;
    }
    this.#dropOpenCard();
    return true;
  }

  /** Saves pending edits first so the copy matches what the user sees, then opens the copy. */
  async duplicateRequest(id: string, name: string): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      if (!(await this.closeRequest())) {
        return;
      }
      const copy = await api.duplicateRequest(id, name);
      this.notifyChanged(current.id);
      const detail = await api.getCollection(current.id);
      if (this.#openCollectionId() !== current.id) {
        return;
      }
      this.collection = detail;
      this.#adjustCount(current.id, 1);
      this.#showRequest(copy);
    });
  }

  async duplicateSection(id: string, name: string): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      if (!(await this.flushSave())) {
        return;
      }
      const detail = await api.duplicateSection(id, name);
      this.notifyChanged(current.id);
      if (this.#openCollectionId() === current.id) {
        this.#adjustCount(current.id, detail.requests.length - current.requests.length);
        this.collection = detail;
      }
    });
  }

  async duplicateCollection(name: string): Promise<void> {
    const current = this.collection;
    if (!current || !this.mayLeave()) {
      return;
    }
    await notices.guard(async () => {
      if (!(await this.flushSave())) {
        return;
      }
      const copy = await api.duplicateCollection(current.id, name);
      this.collections.push(copy);
      await this.openCollection(copy.id);
    });
  }

  async deleteRequest(id: string): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      if (this.openRequestId === id) {
        this.#dropOpenCard();
      }
      const deleted = await api.deleteRequest(id);
      this.notifyChanged(current.id);
      current.requests = current.requests.filter((request) => request.id !== id);
      this.lastResults.delete(id);
      this.#adjustCount(current.id, -1);
      recentlyDeleted.announce(deleted);
    });
  }

  markEdited(): void {
    this.#editRevision += 1;
    this.saveState = 'unsaved';
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => void this.flushSave(), AUTOSAVE_DELAY_MS);
  }

  /** Saves pending edits now (waiting for a save already in flight). Resolves to success. */
  async flushSave(): Promise<boolean> {
    clearTimeout(this.#saveTimer);
    while (this.#inFlight) {
      await this.#inFlight;
    }
    const id = this.openRequestId;
    if (!id || !this.draft || this.saveState === 'saved') {
      return true;
    }
    const attempt = this.#write(id, contentFromDraft(this.draft), this.#editRevision);
    this.#inFlight = attempt;
    const saved = await attempt;
    this.#inFlight = null;
    return saved;
  }

  async #write(id: string, content: api.RequestContent, revision: number): Promise<boolean> {
    const collectionId = this.collection?.id;
    this.saveState = 'saving';
    try {
      const summary = await api.updateRequest(id, content);
      if (collectionId) {
        this.notifyChanged(collectionId);
      }
      const existing = this.collection?.requests.find((request) => request.id === id);
      if (existing) {
        Object.assign(existing, summary);
      }
      this.saveError = null;
      // Edits made while the save was in flight stay 'unsaved' and are saved on the next round.
      if (revision === this.#editRevision) {
        this.saveState = 'saved';
      }
      return true;
    } catch (error) {
      this.saveState = 'failed';
      this.saveError = toAppError(error).message;
      return false;
    }
  }

  // A method call, so the check runs on the current value after an await.
  #openCollectionId(): string | undefined {
    return this.collection?.id;
  }

  #dropOpenCard(): void {
    clearTimeout(this.#saveTimer);
    this.openRequestId = null;
    this.draft = null;
  }

  recordResult(id: string, response: HttpResponseOutput): void {
    this.lastResults.set(id, { status: response.status, elapsedMs: response.elapsedMs });
  }

  #showRequest(saved: api.SavedRequest): void {
    this.openRequestId = saved.id;
    this.draft = draftFromSaved(saved);
    this.saveState = 'saved';
    this.saveError = null;
  }

  #layout(): { sections: string[]; groups: api.RequestGroup[] } | null {
    const current = this.collection;
    if (!current) {
      return null;
    }
    const idsIn = (sectionId: string | null): string[] =>
      current.requests
        .filter((request) => request.sectionId === sectionId)
        .map((request) => request.id);
    return {
      sections: current.sections.map((section) => section.id),
      groups: [
        { sectionId: null, requestIds: idsIn(null) },
        ...current.sections.map((section) => ({
          sectionId: section.id,
          requestIds: idsIn(section.id),
        })),
      ],
    };
  }

  async #applyLayout(layout: { sections: string[]; groups: api.RequestGroup[] }): Promise<void> {
    const current = this.collection;
    if (!current) {
      return;
    }
    await notices.guard(async () => {
      const reordered = await api.reorderCollection(current.id, layout.sections, layout.groups);
      this.notifyChanged(current.id);
      if (this.#openCollectionId() === current.id) {
        this.collection = reordered;
      }
    });
  }

  #replaceSummary(summary: api.CollectionSummary): void {
    this.collections = this.collections.map((item) => (item.id === summary.id ? summary : item));
  }

  #adjustCount(collectionId: string, delta: number): void {
    const summary = this.collections.find((item) => item.id === collectionId);
    if (summary) {
      summary.requestCount += delta;
    }
  }
}
