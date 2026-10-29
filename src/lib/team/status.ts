import type { ChangedRequest, TeamStatus } from '../api/team';
import { plural } from '../format/plural';
import { sentence } from '../format/sentence';
import { formatSince } from '../format/time';
import { usesSignIn } from './account';
import type { SyncOutcome } from '../state/team.svelte';

export type SyncTone = 'ok' | 'busy' | 'warn' | 'err';

export interface SyncSummary {
  tone: SyncTone;
  /** For the collection header, after "Collaboration · ". */
  short: string;
  detail: string;
}

/** `unsent`: saved changes are not in the repository yet; only Sync now sends them. */
export function summarizeSync(
  link: TeamStatus,
  outcome: SyncOutcome | undefined,
  syncing: boolean,
  unsent: boolean,
  now: number = Date.now(),
): SyncSummary {
  if (syncing) {
    return { tone: 'busy', short: 'syncing', detail: 'Syncing…' };
  }
  const overlaps = link.conflicts.length;
  if (overlaps > 0) {
    return {
      tone: 'warn',
      short: `${String(overlaps)} to resolve`,
      detail: `${overlaps === 1 ? 'One file overlaps' : `${String(overlaps)} files overlap`} with a collaborator's changes. Choose what to keep.`,
    };
  }
  if (outcome?.error) {
    return { tone: 'err', short: 'error', detail: outcome.error };
  }
  const report = outcome?.report;
  const waiting =
    report && report.ahead > 0 ? ` ${plural(report.ahead, 'change')} waiting to be sent.` : '';
  switch (report?.status) {
    case 'offline':
      return {
        tone: 'warn',
        short: 'offline',
        detail: `Could not reach the repository; your work is saved here.${waiting}`,
      };
    case 'signIn':
      return {
        tone: 'err',
        short: 'sign in',
        detail: `${sentence(report.message ?? 'Git could not sign in.')} ${
          usesSignIn(link.remote)
            ? "Sign in opens Git's sign-in window."
            : 'Check your SSH key, then use Sync now.'
        }`,
      };
    case 'readOnly':
      return {
        tone: 'warn',
        short: 'read-only',
        detail: `You receive your collaborators' changes, but your account cannot send yours.${waiting}`,
      };
    case 'error':
      return { tone: 'err', short: 'error', detail: report.message ?? 'The last sync failed.' };
    default:
      if (unsent) {
        return {
          tone: 'warn',
          short: 'not synced',
          detail:
            "Your saved changes are not in the repository yet. Sync now sends them and brings in your collaborators' changes.",
        };
      }
      return {
        tone: 'ok',
        short: 'synced',
        detail:
          link.syncedAt === null
            ? 'Not synced yet'
            : `Up to date · synced ${formatSince(link.syncedAt, now)}`,
      };
  }
}

/** "2 updated, 1 added" */
export function describeChanges(changed: ChangedRequest[]): string {
  const count = (change: ChangedRequest['change']): number =>
    changed.filter((item) => item.change === change).length;
  const parts = [
    [count('updated'), 'updated'],
    [count('added'), 'added'],
    [count('removed'), 'removed'],
  ] as const;
  return parts
    .filter(([total]) => total > 0)
    .map(([total, label]) => `${plural(total, 'request')} ${label}`)
    .join(', ');
}
