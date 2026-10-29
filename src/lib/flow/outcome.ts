import type { StepStatus } from '../api/flows';
import { formatDuration } from '../format/units';
import type { StepRun } from '../state/flows.svelte';

export type StepMark = 'idle' | 'running' | StepStatus;

export const MARK_LABELS = new Map<StepMark, string>([
  ['idle', 'Not run yet'],
  ['running', 'Running'],
  ['passed', 'Passed'],
  ['failed', 'Failed'],
  ['skipped', 'Skipped'],
]);

export function stepMark(run: StepRun | undefined): StepMark {
  if (run?.state === 'running') {
    return 'running';
  }
  return run?.status ?? 'idle';
}

/** The step's last result: `full` adds the status text and skip reason, `short` fits a map node. */
export function stepResult(run: StepRun | undefined, detail: 'full' | 'short'): string {
  if (run?.state === 'running') {
    return 'Running…';
  }
  if (!run) {
    return 'Not run yet';
  }
  if (run.status === 'skipped') {
    return detail === 'full' ? (run.note ?? 'Skipped') : 'Skipped';
  }
  if (run.code === null) {
    return detail === 'full' ? (run.errors.at(0) ?? 'No response') : 'No response';
  }
  const status = detail === 'full' ? `${String(run.code)} ${run.statusText}` : String(run.code);
  return run.elapsedMs === null ? status : `${status} · ${formatDuration(run.elapsedMs)}`;
}
