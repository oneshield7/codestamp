import { invoke } from '@tauri-apps/api/core';
import type { RequestKind, RequestMethod } from './collections';

export type HealthStatus = 'healthy' | 'needsAttention' | 'critical' | 'noData';

export interface Health {
  /** 0 to 100. */
  score: number | null;
  status: HealthStatus;
}

/** Each part of the health score, graded on the same bands as the whole. */
export interface CategoryHealth {
  runtime: Health;
  tests: Health;
  quality: Health;
}

export type RunOutcome = 'passed' | 'failed' | 'stopped';
export type RunStepStatus = 'passed' | 'failed' | 'skipped';

export interface RunBrief {
  id: string;
  environment: string | null;
  startedAt: number;
  finishedAt: number;
  outcome: RunOutcome;
  /** Request sends that passed and failed; each iteration counts. */
  passed: number;
  failed: number;
  testsPassed: number;
  testsTotal: number;
}

/** One request's results in a run, added up over its iterations. */
export interface RunEndpoint {
  requestId: string;
  name: string;
  method: RequestMethod;
  kind: RequestKind;
  passed: number;
  failed: number;
  skipped: number;
  testsPassed: number;
  testsTotal: number;
  avgMs: number | null;
  lastCode: number | null;
}

export interface RunRecord extends RunBrief {
  endpoints: RunEndpoint[];
}

export interface Trend {
  current: number;
  previous: number;
}

export interface CatalogEntry {
  id: string;
  name: string;
  icon: string;
  owner: string;
  tags: string[];
  requestCount: number;
  runnableCount: number;
  testedCount: number;
  exampleCount: number;
  health: Health;
  lastRun: RunBrief | null;
  /** Percent of the last 7 days' sends that got no answer or a 5xx. */
  errorRate: number | null;
  p95Ms: number | null;
  lastActivity: number | null;
}

export interface CatalogOverview {
  summary: { apis: number; endpoints: number; requests: Trend; errors: Trend };
  apis: CatalogEntry[];
  tags: string[];
}

export interface DayTraffic {
  start: number;
  requests: number;
  errors: number;
}

export interface EndpointTraffic {
  requestId: string;
  name: string;
  method: RequestMethod;
  kind: RequestKind;
  requests: number;
  errors: number;
  errorRate: number;
  p95Ms: number | null;
}

export interface Runtime {
  requests: Trend;
  errors: Trend;
  clientErrors: number;
  errorRate: number | null;
  p95Ms: number | null;
  avgMs: number | null;
  days: DayTraffic[];
  endpoints: EndpointTraffic[];
}

export interface Tests {
  runnable: number;
  tested: number;
  coverage: number | null;
  latest: RunRecord | null;
  runs: RunBrief[];
}

export type Severity = 'error' | 'warning';

/** Where a rule failed; `request` is null for the collection's own auth. */
export interface Finding {
  request: { id: string; name: string; method: RequestMethod; kind: RequestKind } | null;
  detail: string;
}

export interface RuleResult {
  key: string;
  title: string;
  description: string;
  severity: Severity;
  checked: number;
  findings: Finding[];
}

export interface FlowUse {
  id: string;
  name: string;
  stepsHere: number;
  stepsTotal: number;
  otherApis: string[];
}

export interface CatalogDetail {
  entry: CatalogEntry;
  scores: CategoryHealth;
  assets: {
    kinds: { kind: RequestKind; count: number }[];
    sections: number;
    examples: number;
    variables: number;
    flows: number;
  };
  runtime: Runtime;
  tests: Tests;
  rules: RuleResult[];
  flows: FlowUse[];
}

export interface CatalogInfo {
  owner: string;
  tags: string[];
}

export interface RunStepInput {
  requestId: string;
  status: RunStepStatus;
  code: number | null;
  elapsedMs: number | null;
  testsPassed: number;
  testsTotal: number;
}

export interface RunInput {
  collectionId: string;
  environment: string | null;
  startedAt: number;
  finishedAt: number;
  stopped: boolean;
  steps: RunStepInput[];
}

/** How far this device's clock is ahead of UTC, so the 7 days follow the local calendar. */
function utcOffsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

export function catalogOverview(): Promise<CatalogOverview> {
  return invoke('catalog_overview', { utcOffsetMinutes: utcOffsetMinutes() });
}

export function catalogDetail(collectionId: string): Promise<CatalogDetail> {
  return invoke('catalog_detail', { collectionId, utcOffsetMinutes: utcOffsetMinutes() });
}

export function saveCatalogInfo(
  collectionId: string,
  owner: string,
  tags: string[],
): Promise<CatalogInfo> {
  return invoke('save_catalog_info', { collectionId, owner, tags });
}

export function recordCollectionRun(run: RunInput): Promise<void> {
  return invoke('record_collection_run', { run });
}
