import type { Check, CheckOp, CheckSubject, Extract, ExtractSource, OnFailure } from '../api/flows';
import type { DropdownOption } from '../components/Dropdown.svelte';

export const SUBJECT_OPTIONS: DropdownOption<CheckSubject>[] = [
  { value: 'status', label: 'Status' },
  { value: 'body', label: 'Body' },
  { value: 'header', label: 'Header' },
  { value: 'time', label: 'Time' },
];

const OP_LABELS = new Map<CheckOp, string>([
  ['equals', 'equals'],
  ['notEquals', 'is not'],
  ['contains', 'contains'],
  ['exists', 'exists'],
  ['lessThan', 'is less than'],
]);

/** The comparisons each subject allows; the Rust core checks the same pairs. */
const OPS = new Map<CheckSubject, CheckOp[]>([
  ['status', ['equals', 'notEquals', 'lessThan']],
  ['time', ['lessThan']],
  ['header', ['equals', 'notEquals', 'contains', 'exists']],
  ['body', ['equals', 'notEquals', 'contains', 'exists', 'lessThan']],
]);

export const SOURCE_OPTIONS: DropdownOption<ExtractSource>[] = [
  { value: 'body', label: 'Response body' },
  { value: 'header', label: 'Response header' },
];

export const ON_FAILURE_OPTIONS: DropdownOption<OnFailure>[] = [
  { value: 'stop', label: 'Stop the flow' },
  { value: 'continue', label: 'Continue with the next step' },
];

export function opOptions(subject: CheckSubject): DropdownOption<CheckOp>[] {
  return (OPS.get(subject) ?? []).map((op) => ({ value: op, label: OP_LABELS.get(op) ?? op }));
}

export function hasPath(subject: CheckSubject): boolean {
  return subject === 'body' || subject === 'header';
}

export function newCheck(): Check {
  return { subject: 'status', path: '', op: 'equals', value: '200' };
}

export function newExtract(): Extract {
  return { name: '', source: 'body', path: '' };
}

/** Switches what a check looks at, keeping only the parts that still make sense. */
export function setSubject(check: Check, subject: CheckSubject): void {
  check.subject = subject;
  const allowed = OPS.get(subject) ?? [];
  if (!allowed.includes(check.op)) {
    check.op = allowed[0] ?? 'equals';
  }
  if (!hasPath(subject)) {
    check.path = '';
  }
  if (subject === 'time') {
    check.value = '1000';
  } else if (subject === 'status') {
    check.value = '200';
  }
}

export function pathPlaceholder(subject: CheckSubject | ExtractSource): string {
  return subject === 'header' ? 'Content-Type' : 'data.items[0].id';
}

export function valuePlaceholder(subject: CheckSubject): string {
  switch (subject) {
    case 'status':
      return '201';
    case 'time':
      return 'ms, such as 500';
    case 'header':
      return 'application/json';
    case 'body':
      return 'Expected value';
  }
}
