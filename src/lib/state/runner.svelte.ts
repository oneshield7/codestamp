import {
  getRequest,
  type CollectionDetail,
  type RequestKind,
  type RequestMethod,
} from '../api/collections';
import { recordCollectionRun, type RunStepInput } from '../api/catalog';
import { saveReport } from '../api/export';
import {
  sendHttpRequest,
  toAppError,
  type HttpResponseOutput,
  type NameValue,
  type TestResult,
} from '../api/http';
import { pickRunData, type RunData } from '../api/runs';
import { stopEventStream } from '../api/streams';
import { clipUtf8 } from '../format/clip';
import { slugify } from '../format/slug';
import { formatIso } from '../format/time';
import { buildRequestInput, draftFromSaved } from '../request/draft';
import { SKIPPED_IN_RUNS, stampLabel } from '../request/kinds';
import { orderedRequests } from '../request/order';
import { notices } from './notices.svelte';

export type RunStatus = 'waiting' | 'running' | 'passed' | 'failed' | 'skipped';

export interface RunResult {
  requestId: string;
  /** Which pass of the run, counted from 0. */
  iteration: number;
  name: string;
  method: RequestMethod;
  kind: RequestKind;
  url: string;
  status: RunStatus;
  code: number | null;
  statusText: string;
  elapsedMs: number | null;
  tests: TestResult[];
  /** Request errors and script errors. */
  errors: string[];
  bodySnippet: string;
  note: string | null;
}

export interface RunSummary {
  total: number;
  passed: number;
  failed: number;
  skipped: number;
  tests: number;
  failedTests: number;
}

export const RUN_DELAYS = [0, 250, 1000] as const;
export type RunDelay = (typeof RUN_DELAYS)[number];
export const ITERATION_RANGE = { min: 1, max: 100 } as const;
/** Requests times iterations; past this a run would flood the page with rows. */
const MAX_RUN_STEPS = 10_000;

const SNIPPET_BYTES = 600;
const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Runs every HTTP request of a collection in notebook order, one at a time, with its scripts.
 * Variables set by one request's scripts are used by the next. Runs do not add to history.
 */
export class Runner {
  collectionId = $state<string | null>(null);
  collectionName = $state('');
  environmentName = $state<string | null>(null);
  results = $state<RunResult[]>([]);
  running = $state(false);
  startedAt = $state<number | null>(null);
  finishedAt = $state<number | null>(null);
  stopOnFailure = $state(false);
  delayMs = $state<RunDelay>(0);
  /** Passes over the collection when there is no data file. */
  iterations = $state(1);
  /** With a data file, there is one pass per row. */
  data = $state.raw<RunData | null>(null);
  /** Set when a run was cut short to keep it within the step limit. */
  limitNote = $state<string | null>(null);
  readonly iterationCount = $derived(this.data?.rows.length ?? this.iterations);
  /** Passes in the current results, which can be fewer than asked for (see `limitNote`). */
  readonly passCount = $derived(
    this.results.reduce((most, result) => Math.max(most, result.iteration + 1), 0),
  );

  readonly summary = $derived.by((): RunSummary => {
    const count = (status: RunStatus): number =>
      this.results.filter((result) => result.status === status).length;
    const tests = this.results.flatMap((result) => result.tests);
    return {
      total: this.results.length,
      passed: count('passed'),
      failed: count('failed'),
      skipped: count('skipped'),
      tests: tests.length,
      failedTests: tests.filter((test) => !test.passed).length,
    };
  });

  #stopRequested = false;
  #onResult: (requestId: string, response: HttpResponseOutput) => void = () => undefined;

  async start(
    collection: CollectionDetail,
    environmentName: string | null,
    onResult: (requestId: string, response: HttpResponseOutput) => void,
  ): Promise<void> {
    if (this.running) {
      return;
    }
    this.collectionId = collection.id;
    this.collectionName = collection.name;
    this.environmentName = environmentName;
    this.#onResult = onResult;
    const requests = orderedRequests(collection);
    const wanted = this.iterationCount;
    const passes = Math.min(
      wanted,
      Math.max(1, Math.floor(MAX_RUN_STEPS / Math.max(1, requests.length))),
    );
    this.limitNote =
      passes < wanted
        ? `Only the first ${String(passes)} of ${String(wanted)} iterations ran: a run has at most ${MAX_RUN_STEPS.toLocaleString('en')} steps.`
        : null;
    this.results = Array.from({ length: passes }, (_, iteration) =>
      requests.map((request): RunResult => ({
        requestId: request.id,
        iteration,
        name: request.name,
        method: request.method,
        kind: request.kind,
        url: request.url,
        status: 'waiting',
        code: null,
        statusText: '',
        elapsedMs: null,
        tests: [],
        errors: [],
        bodySnippet: '',
        note: null,
      })),
    ).flat();
    await this.#runAll();
  }

  /** Lets the user pick a data file; the next run makes one pass per row. */
  async chooseData(): Promise<void> {
    const picked = await pickRunData();
    if (picked) {
      this.data = picked;
    }
  }

  /** The values a pass gets from the data file, as `{{column}}` variables. */
  dataFor(iteration: number): NameValue[] {
    const row = this.data?.rows.at(iteration);
    if (!this.data || !row) {
      return [];
    }
    return this.data.columns.map((name, index) => ({ name, value: row.at(index) ?? '' }));
  }

  /** Runs one step again and updates its row ("Run only this"). */
  async runOne(row: RunResult): Promise<void> {
    if (!this.results.includes(row) || this.running) {
      return;
    }
    this.running = true;
    await this.#run(row);
    this.running = false;
  }

  stop(): void {
    this.#stopRequested = true;
  }

  /** Saves the results as JSON. Bodies are left out so responses with secrets are not copied. */
  async exportReport(): Promise<string | null> {
    const report = {
      app: 'Codestamp',
      collection: this.collectionName,
      environment: this.environmentName,
      dataFile: this.data?.name ?? null,
      iterations: this.passCount,
      startedAt: this.startedAt === null ? null : formatIso(this.startedAt),
      finishedAt: this.finishedAt === null ? null : formatIso(this.finishedAt),
      durationMs: this.durationMs(),
      summary: this.summary,
      results: this.results.map((result) => ({
        iteration: result.iteration + 1,
        name: result.name,
        method: stampLabel(result.kind, result.method),
        url: result.url,
        status: result.status,
        code: result.code,
        elapsedMs: result.elapsedMs,
        tests: result.tests,
        errors: result.errors,
        note: result.note,
      })),
    };
    const slug = slugify(this.collectionName) || 'collection';
    return saveReport(JSON.stringify(report, null, 2), `codestamp-run-${slug}`);
  }

  durationMs(): number {
    if (this.startedAt === null) {
      return 0;
    }
    return (this.finishedAt ?? Date.now()) - this.startedAt;
  }

  async #runAll(): Promise<void> {
    this.running = true;
    this.#stopRequested = false;
    const startedAt = Date.now();
    this.startedAt = startedAt;
    this.finishedAt = null;
    let first = true;
    for (const [index, row] of this.results.entries()) {
      if (this.#stopWanted()) {
        this.#skipFrom(index, 'Stopped before this request');
        break;
      }
      const skipped = SKIPPED_IN_RUNS.get(row.kind);
      if (skipped) {
        row.status = 'skipped';
        row.note = skipped;
        continue;
      }
      if (!first && this.delayMs > 0) {
        await sleep(this.delayMs);
      }
      first = false;
      await this.#run(row);
      if (row.status === 'failed' && this.stopOnFailure) {
        this.#skipFrom(index + 1, 'Skipped after a failure');
        break;
      }
    }
    const finishedAt = Date.now();
    this.finishedAt = finishedAt;
    this.running = false;
    await this.#record(startedAt, finishedAt);
  }

  /** Keeps the run for Catalog's Tests numbers. */
  async #record(startedAt: number, finishedAt: number): Promise<void> {
    const collectionId = this.collectionId;
    if (collectionId === null) {
      return;
    }
    const steps: RunStepInput[] = this.results.flatMap((row) =>
      row.status === 'passed' || row.status === 'failed' || row.status === 'skipped'
        ? [
            {
              requestId: row.requestId,
              status: row.status,
              code: row.code,
              elapsedMs: row.elapsedMs,
              testsPassed: row.tests.filter((test) => test.passed).length,
              testsTotal: row.tests.length,
            },
          ]
        : [],
    );
    await notices.guard(() =>
      recordCollectionRun({
        collectionId,
        environment: this.environmentName,
        startedAt,
        finishedAt,
        stopped: this.#stopRequested,
        steps,
      }),
    );
  }

  async #run(row: RunResult): Promise<void> {
    const collectionId = this.collectionId;
    row.status = 'running';
    row.note = null;
    try {
      const saved = await getRequest(row.requestId);
      const input = buildRequestInput(draftFromSaved(saved), {
        collectionId,
        requestId: row.requestId,
      });
      const response = await sendHttpRequest({
        ...input,
        recordHistory: false,
        iterationData: this.dataFor(row.iteration),
        iteration: { index: row.iteration, count: this.passCount },
      });
      if (response.streamId !== null) {
        // A run does not wait for a stream that may never end.
        await stopEventStream(response.streamId);
        row.note = 'Event stream: stopped once its headers arrived';
      }
      this.#onResult(row.requestId, response);
      row.name = saved.name;
      row.code = response.status;
      row.statusText = response.statusText;
      row.elapsedMs = response.elapsedMs;
      row.tests = response.script?.tests ?? [];
      row.errors = response.script?.errors ?? [];
      row.bodySnippet = clipUtf8(response.bodyPreview, SNIPPET_BYTES).text;
      row.status = passed(row) ? 'passed' : 'failed';
    } catch (caught) {
      row.code = null;
      row.elapsedMs = null;
      row.tests = [];
      row.bodySnippet = '';
      row.errors = [toAppError(caught).message];
      row.status = 'failed';
    }
  }

  // A method rather than a field read: Stop is pressed while the loop awaits.
  #stopWanted(): boolean {
    return this.#stopRequested;
  }

  #skipFrom(index: number, note: string): void {
    for (const row of this.results.slice(index)) {
      if (row.status === 'waiting') {
        row.status = 'skipped';
        row.note = note;
      }
    }
  }
}

/**
 * A request passes when it got a response, its scripts ran cleanly and every test passed.
 * Without tests, an error status (4xx/5xx) counts as a failure.
 */
function passed(row: RunResult): boolean {
  if (row.code === null || row.errors.length > 0 || row.tests.some((test) => !test.passed)) {
    return false;
  }
  return row.tests.length > 0 || row.code < 400;
}
