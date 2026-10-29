import type { CatalogEntry, HealthStatus, RunOutcome, Trend } from '../api/catalog';

export type Tone = 'ok' | 'warn' | 'err' | 'neutral';

export const HEALTH_LABELS: Readonly<Record<HealthStatus, string>> = {
  healthy: 'Healthy',
  needsAttention: 'Needs attention',
  critical: 'Critical',
  noData: 'No data',
};

/** Worst first, so sorting by health puts the APIs that need work on top. */
const HEALTH_RANK: Readonly<Record<HealthStatus, number>> = {
  critical: 0,
  needsAttention: 1,
  healthy: 2,
  noData: 3,
};

export const RUN_LABELS: Readonly<Record<RunOutcome, string>> = {
  passed: 'Passed',
  failed: 'Failed',
  stopped: 'Stopped',
};

export const RUN_TONES: Readonly<Record<RunOutcome, Tone>> = {
  passed: 'ok',
  failed: 'err',
  stopped: 'warn',
};

export function healthTone(status: HealthStatus): Tone {
  switch (status) {
    case 'healthy':
      return 'ok';
    case 'needsAttention':
      return 'warn';
    case 'critical':
      return 'err';
    case 'noData':
      return 'neutral';
  }
}

/** "4.5%" under 10%, whole numbers above, so small rates keep their detail. */
export function formatPercent(value: number): string {
  const shown = value > 0 && value < 10 ? value.toFixed(1) : String(Math.round(value));
  return `${shown}%`;
}

/** How this week compares with the one before, in words. */
export function trendText(trend: Trend): string {
  if (trend.previous === 0) {
    return trend.current === 0 ? 'None the week before either' : 'Up from none the week before';
  }
  const change = Math.round(((trend.current - trend.previous) * 100) / trend.previous);
  if (change === 0) {
    return 'Same as the week before';
  }
  return `${change > 0 ? 'Up' : 'Down'} ${String(Math.abs(change))}% on the week before`;
}

export type SortKey = 'name' | 'health' | 'errorRate' | 'p95' | 'activity';

export interface ListFilter {
  query: string;
  tag: string | null;
  health: HealthStatus | null;
}

export function filterApis(apis: readonly CatalogEntry[], filter: ListFilter): CatalogEntry[] {
  const words = filter.query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const tag = filter.tag?.toLocaleLowerCase() ?? null;
  return apis.filter((api) => {
    const text = [api.name, api.owner, ...api.tags].join(' ').toLocaleLowerCase();
    return (
      words.every((word) => text.includes(word)) &&
      (tag === null || api.tags.some((own) => own.toLocaleLowerCase() === tag)) &&
      (filter.health === null || api.health.status === filter.health)
    );
  });
}

/** Missing values (no traffic, never active) always go last, whichever way the list runs. */
export function sortApis(
  apis: readonly CatalogEntry[],
  key: SortKey,
  descending: boolean,
): CatalogEntry[] {
  const direction = descending ? -1 : 1;
  const byName = (a: CatalogEntry, b: CatalogEntry): number =>
    a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
  const value = (api: CatalogEntry): number | null => {
    switch (key) {
      case 'name':
        return null;
      case 'health':
        return HEALTH_RANK[api.health.status] * 1000 + (api.health.score ?? 0);
      case 'errorRate':
        return api.errorRate;
      case 'p95':
        return api.p95Ms;
      case 'activity':
        return api.lastActivity;
    }
  };
  return [...apis].sort((a, b) => {
    if (key === 'name') {
      return direction * byName(a, b);
    }
    const [left, right] = [value(a), value(b)];
    if (left === null || right === null) {
      return left === right ? byName(a, b) : left === null ? 1 : -1;
    }
    return left === right ? byName(a, b) : direction * (left - right);
  });
}

/** Tags typed as "public, payments". The core checks each one and says what is wrong. */
export function parseTags(text: string): string[] {
  return text
    .split(',')
    .map((tag) => tag.trim())
    .filter((tag) => tag !== '');
}
