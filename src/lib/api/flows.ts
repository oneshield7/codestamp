import { Channel, invoke } from '@tauri-apps/api/core';
import type { RequestKind, RequestMethod } from './collections';
import type { NameValue, TestResult } from './http';

export interface FlowSummary {
  id: string;
  name: string;
  stepCount: number;
}

export type OnFailure = 'stop' | 'continue';
export type ExtractSource = 'body' | 'header';
export type CheckSubject = 'status' | 'body' | 'header' | 'time';
export type CheckOp = 'equals' | 'notEquals' | 'contains' | 'exists' | 'lessThan';

/** A value taken from a step's response and passed on as `{{name}}` to the steps after it. */
export interface Extract {
  name: string;
  source: ExtractSource;
  /** A JSON path such as `data.items[0].id`, or a header name. */
  path: string;
}

/** A condition the step's response must meet. */
export interface Check {
  subject: CheckSubject;
  /** The JSON path (body) or header name; empty for status and time. */
  path: string;
  op: CheckOp;
  value: string;
}

export interface StepRequest {
  id: string;
  collectionId: string;
  collectionName: string;
  kind: RequestKind;
  name: string;
  method: RequestMethod;
  url: string;
  /** Every `{{name}}` the request sends; filled in when the flow is loaded or saved. */
  variables: string[];
}

export interface FlowStep {
  id: string;
  /** Null once the request was deleted. */
  request: StepRequest | null;
  extracts: Extract[];
  checks: Check[];
  onFailure: OnFailure;
}

/** A flow always runs from Start to End; only the steps between them are stored. */
export interface FlowDetail {
  id: string;
  name: string;
  steps: FlowStep[];
}

export interface FlowStepInput {
  id: string;
  requestId: string | null;
  extracts: Extract[];
  checks: Check[];
  onFailure: OnFailure;
}

export interface FlowInput {
  id: string;
  name: string;
  steps: FlowStepInput[];
}

export type StepStatus = 'passed' | 'failed' | 'skipped';

export interface CheckResult {
  label: string;
  passed: boolean;
  detail: string | null;
}

export interface StepOutcome {
  stepId: string;
  status: StepStatus;
  code: number | null;
  statusText: string;
  elapsedMs: number | null;
  checks: CheckResult[];
  tests: TestResult[];
  passedOn: NameValue[];
  errors: string[];
  note: string | null;
  headers: NameValue[];
  /** The first 64 KB of the body. */
  body: string;
}

export type FlowEvent =
  { type: 'stepStarted'; stepId: string } | ({ type: 'stepFinished' } & StepOutcome);

export interface FlowRunSummary {
  passed: number;
  failed: number;
  skipped: number;
  durationMs: number;
  /** The step that failed and stopped the run. */
  stoppedAt: string | null;
  cancelled: boolean;
}

export function listFlows(): Promise<FlowSummary[]> {
  return invoke('list_flows');
}

export function createFlow(name: string): Promise<FlowSummary> {
  return invoke('create_flow', { name });
}

export function getFlow(id: string): Promise<FlowDetail> {
  return invoke('get_flow', { id });
}

export function saveFlow(flow: FlowInput): Promise<FlowDetail> {
  return invoke('save_flow', { flow });
}

export function deleteFlow(id: string): Promise<void> {
  return invoke('delete_flow', { id });
}

/** Runs the saved flow; each step's start and result arrive on `onEvent` as they happen. */
export function runFlow(
  id: string,
  runId: string,
  onEvent: (event: FlowEvent) => void,
): Promise<FlowRunSummary> {
  const channel = new Channel<FlowEvent>();
  channel.onmessage = onEvent;
  return invoke('run_flow', { id, runId, onEvent: channel });
}

export function stopFlow(runId: string): Promise<void> {
  return invoke('stop_flow', { runId });
}
