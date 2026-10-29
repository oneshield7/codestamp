<script lang="ts">
  import type { CatalogEntry, HealthStatus } from '../../api/catalog';
  import {
    HEALTH_LABELS,
    RUN_LABELS,
    RUN_TONES,
    formatPercent,
    trendText,
    type SortKey,
  } from '../../catalog/present';
  import { plural } from '../../format/plural';
  import { formatSince } from '../../format/time';
  import { formatDuration } from '../../format/units';
  import type { Catalog } from '../../state/catalog.svelte';
  import { navLabel } from '../../state/navigation';
  import Dropdown, { type DropdownOption } from '../Dropdown.svelte';
  import NavIcon from '../NavIcon.svelte';
  import HealthChip from './HealthChip.svelte';
  import StatTile from './StatTile.svelte';

  interface Props {
    catalog: Catalog;
    onOpen: (id: string) => void;
    onNewCollection: () => void;
  }

  let { catalog, onOpen, onNewCollection }: Props = $props();

  const HEALTH_FILTERS: { value: HealthStatus | null; label: string }[] = [
    { value: null, label: 'All' },
    { value: 'critical', label: HEALTH_LABELS.critical },
    { value: 'needsAttention', label: HEALTH_LABELS.needsAttention },
    { value: 'healthy', label: HEALTH_LABELS.healthy },
    { value: 'noData', label: HEALTH_LABELS.noData },
  ];

  const COLUMNS: { key: SortKey | null; label: string; class: string }[] = [
    { key: 'name', label: 'API', class: 'col-api' },
    { key: 'health', label: 'Health', class: 'col-health' },
    { key: null, label: 'Last run', class: 'col-run' },
    { key: 'errorRate', label: 'Error rate', class: 'col-rate' },
    { key: 'p95', label: 'p95', class: 'col-p95' },
    { key: null, label: 'Owner', class: 'col-owner' },
    { key: 'activity', label: 'Last activity', class: 'col-activity' },
  ];

  const overview = $derived(catalog.overview);
  const tagOptions = $derived<DropdownOption<string>[]>([
    { value: '', label: 'All tags' },
    ...(overview?.tags ?? []).map((tag) => ({ value: tag, label: tag })),
  ]);
  const attention = $derived(
    overview?.apis.filter(
      (api) => api.health.status === 'critical' || api.health.status === 'needsAttention',
    ).length ?? 0,
  );
  const scored = $derived(
    overview?.apis.filter((api) => api.health.status !== 'noData').length ?? 0,
  );
  const filtered = $derived(
    catalog.query.trim() !== '' || catalog.tag !== null || catalog.health !== null,
  );

  function clearFilters(): void {
    catalog.query = '';
    catalog.tag = null;
    catalog.health = null;
  }

  function ariaSort(key: SortKey | null): 'ascending' | 'descending' | undefined {
    if (key === null || catalog.sortKey !== key) {
      return undefined;
    }
    return catalog.descending ? 'descending' : 'ascending';
  }

  function testsLine(api: CatalogEntry): string {
    if (api.runnableCount === 0) {
      return plural(api.requestCount, 'request');
    }
    return `${plural(api.requestCount, 'request')} · ${String(api.testedCount)} with tests`;
  }
</script>

<div class="view">
  <header class="top">
    <div class="title">
      <h1 class="page-title"><NavIcon name="catalog" size="lg" />{navLabel('catalog')}</h1>
      <p>
        Every collection is listed here as an API. Codestamp grades its health from the requests you
        sent from it in the last 7 days, its latest Run all and a set of quality rules.
      </p>
    </div>
    <button
      type="button"
      class="quiet"
      disabled={catalog.loading}
      aria-busy={catalog.loading}
      onclick={() => void catalog.load()}
    >
      {catalog.loading ? 'Refreshing…' : 'Refresh'}
    </button>
  </header>

  {#if catalog.error && !overview}
    <div class="problem" role="alert">
      <span>The catalog could not load: {catalog.error}</span>
      <button type="button" class="quiet" onclick={() => void catalog.load()}>Try again</button>
    </div>
  {:else if !overview}
    <p class="muted" role="status">Loading the catalog…</p>
  {:else if overview.apis.length === 0}
    <section class="empty" aria-label="No APIs yet">
      <h2>No APIs yet</h2>
      <p>
        Each collection shows up here as an API. Create or import one, send a few requests and run
        it, and its health appears here.
      </p>
      <button type="button" class="quiet" onclick={onNewCollection}>Create a collection</button>
    </section>
  {:else}
    {#if catalog.error}
      <p class="problem" role="alert">The catalog could not refresh: {catalog.error}</p>
    {/if}
    <div class="tiles">
      <StatTile
        label="APIs"
        value={overview.summary.apis.toLocaleString('en')}
        note={`${plural(overview.summary.endpoints, 'request')} in all`}
      />
      <StatTile
        label="Sent in the last 7 days"
        value={overview.summary.requests.current.toLocaleString('en')}
        note={trendText(overview.summary.requests)}
      />
      <StatTile
        label="Errors in the last 7 days"
        value={overview.summary.errors.current.toLocaleString('en')}
        note={trendText(overview.summary.errors)}
        noteTone={overview.summary.errors.current > overview.summary.errors.previous
          ? 'err'
          : 'neutral'}
      />
      <StatTile
        label="Need attention"
        value={attention.toLocaleString('en')}
        note={scored === 0 ? 'No API has a score yet' : `Of ${plural(scored, 'API')} with a score`}
        noteTone={attention > 0 ? 'warn' : 'neutral'}
      />
    </div>

    <div class="filters">
      <label class="search">
        <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" />
        </svg>
        <span class="visually-hidden">Search APIs</span>
        <input
          type="search"
          bind:value={catalog.query}
          placeholder="Search names, owners and tags"
          maxlength="200"
        />
      </label>
      {#if overview.tags.length > 0}
        <Dropdown
          label="Tag"
          value={catalog.tag ?? ''}
          options={tagOptions}
          onChange={(tag: string) => {
            catalog.tag = tag === '' ? null : tag;
          }}
        />
      {/if}
      <div class="pills" role="group" aria-label="Health filter">
        {#each HEALTH_FILTERS as filter (filter.label)}
          <button
            type="button"
            class:on={catalog.health === filter.value}
            aria-pressed={catalog.health === filter.value}
            onclick={() => (catalog.health = filter.value)}>{filter.label}</button
          >
        {/each}
      </div>
    </div>

    {#if catalog.visible.length === 0}
      <div class="none">
        <p>No API matches these filters.</p>
        {#if filtered}
          <button type="button" class="quiet" onclick={clearFilters}>Clear filters</button>
        {/if}
      </div>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              {#each COLUMNS as column (column.label)}
                <th scope="col" class={column.class} aria-sort={ariaSort(column.key)}>
                  {#if column.key}
                    {@const key = column.key}
                    <button
                      type="button"
                      class="sort"
                      onclick={() => {
                        catalog.sortBy(key);
                      }}
                    >
                      {column.label}
                      <svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true">
                        {#if catalog.sortKey === key}
                          <path d={catalog.descending ? 'M6 9l6 6 6-6' : 'M6 15l6-6 6 6'} />
                        {:else}
                          <path d="M8 9l4-4 4 4M8 15l4 4 4-4" />
                        {/if}
                      </svg>
                    </button>
                  {:else}
                    {column.label}
                  {/if}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each catalog.visible as api (api.id)}
              <tr>
                <td class="col-api">
                  <div class="api">
                    <span class="icon emoji" aria-hidden="true">{api.icon}</span>
                    <div class="api-text">
                      <button
                        type="button"
                        class="open"
                        onclick={() => {
                          onOpen(api.id);
                        }}>{api.name}</button
                      >
                      <span class="sub">
                        {testsLine(api)}<span class="owner-inline"
                          >{api.owner ? ` · ${api.owner}` : ''}</span
                        >
                      </span>
                      {#if api.tags.length > 0}
                        <span class="tags">
                          {#each api.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
                        </span>
                      {/if}
                    </div>
                  </div>
                </td>
                <td class="col-health"><HealthChip health={api.health} /></td>
                <td class="col-run">
                  {#if api.lastRun}
                    <span class="state {RUN_TONES[api.lastRun.outcome]}"
                      >{RUN_LABELS[api.lastRun.outcome]}</span
                    >
                    <span class="sub">{formatSince(api.lastRun.finishedAt)}</span>
                  {:else}
                    <span class="sub">Not run yet</span>
                  {/if}
                </td>
                <td class="col-rate num">
                  {#if api.errorRate === null}
                    <span class="sub">No traffic</span>
                  {:else}
                    {formatPercent(api.errorRate)}
                  {/if}
                </td>
                <td class="col-p95 num">
                  {#if api.p95Ms === null}
                    <span class="sub">No traffic</span>
                  {:else}
                    {formatDuration(api.p95Ms)}
                  {/if}
                </td>
                <td class="col-owner">
                  {#if api.owner}{api.owner}{:else}<span class="sub">No owner</span>{/if}
                </td>
                <td class="col-activity">
                  {#if api.lastActivity === null}
                    <span class="sub">None</span>
                  {:else}
                    <span class="sub">{formatSince(api.lastActivity)}</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
  }

  .title {
    flex: 1 1 360px;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .title p {
    margin: 4px 0 0;
    max-width: 680px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .quiet {
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
  }

  .quiet:hover:not(:disabled) {
    border-color: var(--border-strong);
  }

  .quiet:active:not(:disabled) {
    background: var(--surface-sunken);
  }

  .quiet:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .muted {
    margin: 12px 0;
    color: var(--text-muted);
  }

  .problem {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin: 0;
    padding: 10px 14px;
    border: 1px solid var(--err-line);
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .problem span {
    flex: 1 1 240px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 28px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
  }

  .empty h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .empty p {
    margin: 0;
    max-width: 560px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .tiles {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .search {
    flex: 1 1 260px;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
  }

  .search:focus-within {
    border-color: var(--focus);
  }

  .search svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  /* The label's border turns blue on focus instead. */
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    outline: none;
  }

  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pills button {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
  }

  .pills button:hover {
    border-color: var(--border-strong);
  }

  .pills button.on {
    border-color: var(--text);
    background: var(--text);
    color: var(--surface);
  }

  .none {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    color: var(--text-muted);
  }

  .none p {
    margin: 0;
  }

  /* Narrow content keeps the columns that rank APIs; owner moves under the name. */
  .table-wrap {
    container-type: inline-size;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    padding: 8px 14px;
    border-bottom: 1px solid var(--border-soft);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  td {
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-soft);
    vertical-align: top;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  tbody tr:hover td {
    background: var(--surface-muted);
  }

  .num {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .sort {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px;
    margin-left: -4px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    font-weight: inherit;
  }

  .sort:hover {
    background: var(--surface-sunken);
    color: var(--text);
  }

  .sort svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .api {
    display: flex;
    gap: 10px;
    min-width: 0;
  }

  .icon {
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
  }

  .api-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .open {
    max-width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-weight: 600;
    text-align: left;
    overflow-wrap: anywhere;
  }

  .open:hover {
    text-decoration: underline;
  }

  .sub {
    font-size: 12px;
    color: var(--text-muted);
  }

  .owner-inline {
    display: none;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 2px;
  }

  .tag {
    padding: 1px 7px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-2);
    font-size: 11px;
  }

  .state {
    display: block;
    font-weight: 600;
  }

  .state.ok {
    color: var(--ok);
  }

  .state.warn {
    color: var(--warn);
  }

  .state.err {
    color: var(--err);
  }

  @container (max-width: 760px) {
    .col-p95,
    .col-owner,
    .col-activity {
      display: none;
    }

    .owner-inline {
      display: inline;
    }
  }
</style>
