import type { FlowStep } from '../api/flows';
import type { StepRun } from '../state/flows.svelte';

export type ValueState = 'planned' | 'passed' | 'missing';

/** A value a step sends as `{{name}}`, and the earlier step it comes from. */
export interface IncomingValue {
  name: string;
  from: number;
}

/** A value a step passes on, and the later steps that send it. */
export interface OutgoingValue {
  name: string;
  to: number[];
}

/** The names a step passes on, once each. */
export function passedNames(step: FlowStep): string[] {
  return [
    ...new Set(step.extracts.map((extract) => extract.name.trim()).filter((name) => name !== '')),
  ];
}

/** The latest step before `index` that passes `name` on, as in a run; -1 when none does. */
function source(steps: FlowStep[], index: number, name: string): number {
  return steps.findLastIndex((step, at) => at < index && passedNames(step).includes(name));
}

export function incomingValues(steps: FlowStep[], index: number): IncomingValue[] {
  const sent = steps.at(index)?.request?.variables ?? [];
  return sent
    .map((name) => ({ name, from: source(steps, index, name) }))
    .filter((value) => value.from >= 0);
}

export function outgoingValues(steps: FlowStep[], index: number): OutgoingValue[] {
  const step = steps.at(index);
  if (!step) {
    return [];
  }
  return passedNames(step).map((name) => ({
    name,
    to: steps.flatMap((later, at) =>
      at > index &&
      (later.request?.variables.includes(name) ?? false) &&
      source(steps, at, name) === index
        ? [at]
        : [],
    ),
  }));
}

/** Whether the step that passes `name` on did so in the last run. */
export function valueState(run: StepRun | undefined, name: string): ValueState {
  if (run?.state !== 'done' || run.status === 'skipped') {
    return 'planned';
  }
  return run.passedOn.some((value) => value.name === name) ? 'passed' : 'missing';
}
