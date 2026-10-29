import { SvelteMap } from 'svelte/reactivity';
import * as api from '../api/flows';
import { toAppError } from '../api/http';
import type { SearchEntry } from '../api/search';
import { freeName } from '../format/names';
import { notices } from './notices.svelte';
import type { SaveState } from './workspace.svelte';

const AUTOSAVE_DELAY_MS = 600;

/** A step's place in the last run: sending, or its outcome. */
export type StepRun = { state: 'running' } | ({ state: 'done' } & api.StepOutcome);

/** Orchestration: the list of flows, the open one (edited in place and autosaved) and its last run. */
export class Flows {
  list = $state<api.FlowSummary[]>([]);
  loaded = $state(false);
  current = $state<api.FlowDetail | null>(null);
  saveState = $state<SaveState>('saved');
  saveError = $state<string | null>(null);
  running = $state(false);
  readonly runs = new SvelteMap<string, StepRun>();
  summary = $state<api.FlowRunSummary | null>(null);
  runError = $state<string | null>(null);

  #runId: string | null = null;
  #saveTimer: ReturnType<typeof setTimeout> | undefined;
  #inFlight: Promise<boolean> | null = null;
  #editRevision = 0;

  async load(): Promise<void> {
    this.list = await api.listFlows();
    this.loaded = true;
    const first = this.list.at(0);
    if (!this.current && first) {
      await this.select(first.id);
    }
  }

  /** Reloads the list, and the open flow so renamed or deleted requests show as they are now. */
  async refresh(): Promise<void> {
    if (!this.loaded) {
      await this.load();
      return;
    }
    this.list = await api.listFlows();
    const current = this.current;
    if (!current || this.running || this.saveState !== 'saved') {
      return;
    }
    if (this.list.some((flow) => flow.id === current.id)) {
      this.current = await api.getFlow(current.id);
      return;
    }
    this.current = null;
    this.#clearRun();
    const first = this.list.at(0);
    if (first) {
      await this.select(first.id);
    }
  }

  async select(id: string): Promise<void> {
    if (this.running || this.current?.id === id || !(await this.flushSave())) {
      return;
    }
    this.current = await api.getFlow(id);
    this.#clearRun();
  }

  async create(): Promise<void> {
    if (this.running || !(await this.flushSave())) {
      return;
    }
    const created = await api.createFlow(
      freeName(
        'New flow',
        this.list.map((flow) => flow.name),
      ),
    );
    this.list.push(created);
    await this.select(created.id);
  }

  async remove(id: string): Promise<void> {
    if (this.running) {
      return;
    }
    clearTimeout(this.#saveTimer);
    await api.deleteFlow(id);
    this.list = this.list.filter((flow) => flow.id !== id);
    if (this.current?.id === id) {
      this.current = null;
      this.saveState = 'saved';
      this.#clearRun();
      const next = this.list.at(0);
      if (next) {
        await this.select(next.id);
      }
    }
  }

  addStep(entry: SearchEntry): void {
    this.current?.steps.push({
      id: crypto.randomUUID(),
      request: toStepRequest(entry),
      extracts: [],
      checks: [],
      onFailure: 'stop',
    });
    this.markEdited();
  }

  /** Points a step at another request, such as after its request was deleted. */
  replaceRequest(stepId: string, entry: SearchEntry): void {
    const step = this.current?.steps.find((candidate) => candidate.id === stepId);
    if (step) {
      step.request = toStepRequest(entry);
      this.markEdited();
    }
  }

  removeStep(stepId: string): void {
    if (!this.current) {
      return;
    }
    this.current.steps = this.current.steps.filter((step) => step.id !== stepId);
    this.runs.delete(stepId);
    this.markEdited();
  }

  moveStep(stepId: string, delta: -1 | 1): void {
    const steps = this.current?.steps;
    const from = steps?.findIndex((step) => step.id === stepId) ?? -1;
    const to = from + delta;
    if (!steps || from < 0 || to < 0 || to >= steps.length) {
      return;
    }
    const [moved] = steps.splice(from, 1);
    if (moved) {
      steps.splice(to, 0, moved);
      this.markEdited();
    }
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
    const flow = this.current;
    if (!flow || this.saveState === 'saved') {
      return true;
    }
    const attempt = this.#write(toInput(flow), this.#editRevision);
    this.#inFlight = attempt;
    const saved = await attempt;
    this.#inFlight = null;
    return saved;
  }

  async run(): Promise<void> {
    const flow = this.current;
    if (!flow || this.running || !(await this.flushSave())) {
      return;
    }
    this.#clearRun();
    this.running = true;
    const runId = crypto.randomUUID();
    this.#runId = runId;
    try {
      this.summary = await api.runFlow(flow.id, runId, (event) => {
        if (event.type === 'stepStarted') {
          this.runs.set(event.stepId, { state: 'running' });
        } else {
          this.runs.set(event.stepId, { ...event, state: 'done' });
        }
      });
    } catch (error) {
      this.runError = toAppError(error).message;
    } finally {
      this.running = false;
      this.#runId = null;
    }
  }

  async stop(): Promise<void> {
    const runId = this.#runId;
    if (runId) {
      await notices.guard(() => api.stopFlow(runId));
    }
  }

  async #write(input: api.FlowInput, revision: number): Promise<boolean> {
    this.saveState = 'saving';
    try {
      const saved = await api.saveFlow(input);
      const entry = this.list.find((flow) => flow.id === saved.id);
      if (entry) {
        entry.name = saved.name;
        entry.stepCount = saved.steps.length;
      }
      this.#takeVariables(saved);
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

  /** A step added or pointed at another request learns its `{{names}}` from the saved copy. */
  #takeVariables(saved: api.FlowDetail): void {
    for (const savedStep of saved.steps) {
      const request = this.current?.steps.find((step) => step.id === savedStep.id)?.request;
      if (request && request.id === savedStep.request?.id) {
        request.variables = savedStep.request.variables;
      }
    }
  }

  #clearRun(): void {
    this.runs.clear();
    this.summary = null;
    this.runError = null;
  }
}

function toStepRequest(entry: SearchEntry): api.StepRequest {
  return {
    id: entry.requestId,
    collectionId: entry.collectionId,
    collectionName: entry.collectionName,
    kind: entry.kind,
    name: entry.name,
    method: entry.method,
    url: entry.url,
    variables: [],
  };
}

function toInput(flow: api.FlowDetail): api.FlowInput {
  return {
    id: flow.id,
    name: flow.name,
    steps: flow.steps.map((step) => ({
      id: step.id,
      requestId: step.request?.id ?? null,
      extracts: step.extracts,
      checks: step.checks,
      onFailure: step.onFailure,
    })),
  };
}
