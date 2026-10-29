<script lang="ts">
  import type { CatalogDetail, Health } from '../../api/catalog';
  import {
    HEALTH_LABELS,
    RUN_LABELS,
    RUN_TONES,
    formatPercent,
    healthTone,
    trendText,
  } from '../../catalog/present';
  import { plural } from '../../format/plural';
  import { formatDateTime, formatSince } from '../../format/time';
  import { formatDuration } from '../../format/units';
  import { ADDABLE_KINDS } from '../../request/kind-icons';
  import { stampLabel } from '../../request/kinds';
  import { stampTone } from '../../request/methods';
  import type { Catalog, DetailTab } from '../../state/catalog.svelte';
  import { navLabel } from '../../state/navigation';
  import MethodStamp from '../MethodStamp.svelte';
  import HealthChip from './HealthChip.svelte';
  import InfoForm from './InfoForm.svelte';
  import RuleList from './RuleList.svelte';
  import StatTile from './StatTile.svelte';
  import TrafficChart from './TrafficChart.svelte';

  interface Props {
    catalog: Catalog;
    detail: CatalogDetail;
    onBack: () => void;
    onOpenRequest: (collectionId: string, requestId: string) => void;
    onOpenCollection: (collectionId: string) => void;
    onOpenFlow: (flowId: string) => void;
    onRunCollection: (collectionId: string) => void;
  }

  let {
    catalog,
    detail,
    onBack,
    onOpenRequest,
    onOpenCollection,
    onOpenFlow,
    onRunCollection,
  }: Props = $props();

  let editing = $state(false);

  const entry = $derived(detail.entry);
  const latest = $derived(detail.tests.latest);
  const failingRules = $derived(detail.rules.filter((rule) => rule.findings.length > 0).length);
  const tabs = $derived<{ key: DetailTab; label: string }[]>([
    { key: 'overview', label: 'Overview' },
    { key: 'runtime', label: 'Runtime' },
    { key: 'tests', label: 'Tests' },
    { key: 'quality', label: failingRules > 0 ? `Quality · ${String(failingRules)}` : 'Quality' },
  ]);
  const parts = $derived<{ label: string; weight: string; health: Health; empty: string }[]>([
    {
      label: 'Runtime',
      weight: '40%',
      health: detail.scores.runtime,
      empty: 'Nothing sent in the last 7 days',
    },
    {
      label: 'Tests',
      weight: '35%',
      health: detail.scores.tests,
      empty: 'No request that Run all sends',
    },
    { label: 'Quality', weight: '25%', health: detail.scores.quality, empty: 'No requests yet' },
  ]);

  function openRequest(requestId: string): void {
    onOpenRequest(entry.id, requestId);
  }

  function runLength(startedAt: number, finishedAt: number): string {
    return formatDuration(Math.max(0, finishedAt - startedAt));
  }

  function sendsOf(endpoint: { passed: number; failed: number; skipped: number }): number {
    return endpoint.passed + endpoint.failed + endpoint.skipped;
  }
</script>

<div class="view">
  <button type="button" class="back" onclick={onBack}>
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
      ><path d="M15 6l-6 6 6 6" /></svg
    >
    All APIs
  </button>

  <header class="top">
    <span class="icon emoji" aria-hidden="true">{entry.icon}</span>
    <div class="title">
      <h1>{entry.name}</h1>
      <div class="meta">
        <HealthChip health={entry.health} />
        <span>{entry.owner ? `Owner: ${entry.owner}` : 'No owner yet'}</span>
        {#each entry.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
      </div>
    </div>
    <div class="actions">
      <button
        type="button"
        class="quiet"
        aria-expanded={editing}
        onclick={() => (editing = !editing)}>Edit owner and tags</button
      >
      <button
        type="button"
        class="quiet"
        disabled={detail.tests.runnable === 0}
        title={detail.tests.runnable === 0 ? 'Run all · add an HTTP request first' : undefined}
        onclick={() => {
          onRunCollection(entry.id);
        }}>Run all</button
      >
      <button
        type="button"
        class="quiet"
        onclick={() => {
          onOpenCollection(entry.id);
        }}>Open collection</button
      >
    </div>
  </header>

  {#if editing}
    <InfoForm
      owner={entry.owner}
      tags={entry.tags}
      onSave={(owner: string, tags: string[]) => catalog.saveInfo(owner, tags)}
      onClose={() => (editing = false)}
    />
  {/if}

  <div class="tabs" role="tablist" aria-label="API details">
    {#each tabs as tab (tab.key)}
      <button
        type="button"
        role="tab"
        id={`catalog-tab-${tab.key}`}
        aria-selected={catalog.tab === tab.key}
        aria-controls="catalog-panel"
        class:on={catalog.tab === tab.key}
        onclick={() => (catalog.tab = tab.key)}>{tab.label}</button
      >
    {/each}
  </div>

  <div
    id="catalog-panel"
    class="panel"
    role="tabpanel"
    aria-labelledby={`catalog-tab-${catalog.tab}`}
  >
    {#if catalog.tab === 'overview'}
      <div class="columns">
        <section class="card score-card" aria-labelledby="score-title">
          <h2 id="score-title">Health score</h2>
          <div class="score-line">
            <span class="score">{entry.health.score ?? 'No data'}</span>
            {#if entry.health.score !== null}<span class="out-of">/ 100</span>{/if}
            <HealthChip health={entry.health} />
          </div>
          <ul class="parts">
            {#each parts as part (part.label)}
              <li>
                <span class="part-name">{part.label} <span class="weight">{part.weight}</span></span
                >
                <span class="meter" aria-hidden="true">
                  <span
                    class="fill {healthTone(part.health.status)}"
                    style:width={`${String(part.health.score ?? 0)}%`}
                  ></span>
                </span>
                <span class="part-value">
                  {#if part.health.score === null}
                    <span class="muted">{part.empty}</span>
                  {:else}
                    {part.health.score} · {HEALTH_LABELS[part.health.status]}
                  {/if}
                </span>
              </li>
            {/each}
          </ul>
          <p class="explain">
            Runtime counts most (40%), then Tests (35%) and Quality (25%); a part with no data is
            left out. 80 or more is healthy, 50 to 79 needs attention, below 50 is critical.
          </p>
        </section>

        <section class="card" aria-labelledby="assets-title">
          <h2 id="assets-title">What it holds</h2>
          <dl class="assets">
            <dt>Requests</dt>
            <dd>
              {#if detail.assets.kinds.length === 0}
                None
              {:else}
                {#each detail.assets.kinds as kind (kind.kind)}
                  <span
                    >{ADDABLE_KINDS.find((option) => option.kind === kind.kind)?.name ?? kind.kind}
                    <strong>{kind.count}</strong></span
                  >
                {/each}
              {/if}
            </dd>
            <dt>Sections</dt>
            <dd>{detail.assets.sections}</dd>
            <dt>Saved examples</dt>
            <dd>{detail.assets.examples}</dd>
            <dt>Collection variables</dt>
            <dd>{detail.assets.variables}</dd>
            <dt>Flows using it</dt>
            <dd>{detail.assets.flows}</dd>
          </dl>
        </section>
      </div>

      <section class="card" aria-labelledby="flows-title">
        <h2 id="flows-title">Used in {navLabel('flows')}</h2>
        {#if detail.flows.length === 0}
          <p class="muted">No flow runs a request from this API yet.</p>
        {:else}
          <ul class="flows">
            {#each detail.flows as flow (flow.id)}
              <li>
                <button
                  type="button"
                  class="target"
                  onclick={() => {
                    onOpenFlow(flow.id);
                  }}>{flow.name}</button
                >
                <span class="muted"
                  >{flow.stepsHere} of {plural(flow.stepsTotal, 'step')} from this API</span
                >
                {#if flow.otherApis.length > 0}
                  <span class="muted">· Also calls {flow.otherApis.join(', ')}</span>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {:else if catalog.tab === 'runtime'}
      {@const runtime = detail.runtime}
      <div class="tiles">
        <StatTile
          label="Sent in the last 7 days"
          value={runtime.requests.current.toLocaleString('en')}
          note={trendText(runtime.requests)}
        />
        <StatTile
          label="Error rate"
          value={runtime.errorRate === null ? 'No traffic' : formatPercent(runtime.errorRate)}
          note={`${plural(runtime.errors.current, 'error')}: no answer or a 5xx`}
          noteTone={runtime.errors.current > 0 ? 'err' : 'neutral'}
        />
        <StatTile
          label="p95 response time"
          value={runtime.p95Ms === null ? 'No traffic' : formatDuration(runtime.p95Ms)}
          note={runtime.avgMs === null ? undefined : `Average ${formatDuration(runtime.avgMs)}`}
        />
        <StatTile
          label="Client errors (4xx)"
          value={runtime.clientErrors.toLocaleString('en')}
          note="Not counted as errors"
        />
      </div>
      <TrafficChart days={runtime.days} />
      <section class="card" aria-labelledby="endpoints-title">
        <h2 id="endpoints-title">Requests by error rate</h2>
        {#if runtime.endpoints.length === 0}
          <p class="muted">No request of this API was sent in the last 7 days.</p>
        {:else}
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th scope="col">Request</th>
                  <th scope="col" class="num">Sent</th>
                  <th scope="col" class="num">Errors</th>
                  <th scope="col" class="num">Error rate</th>
                  <th scope="col" class="num">p95</th>
                </tr>
              </thead>
              <tbody>
                {#each runtime.endpoints as endpoint (endpoint.requestId)}
                  <tr>
                    <td>
                      <span class="request">
                        <MethodStamp
                          label={stampLabel(endpoint.kind, endpoint.method)}
                          tone={stampTone(endpoint.kind, endpoint.method)}
                          size="sm"
                        />
                        <button
                          type="button"
                          class="target"
                          onclick={() => {
                            openRequest(endpoint.requestId);
                          }}>{endpoint.name}</button
                        >
                      </span>
                    </td>
                    <td class="num">{endpoint.requests}</td>
                    <td class="num">{endpoint.errors}</td>
                    <td class="num">{formatPercent(endpoint.errorRate)}</td>
                    <td class="num"
                      >{endpoint.p95Ms === null ? 'None' : formatDuration(endpoint.p95Ms)}</td
                    >
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </section>
      <p class="footnote">
        Codestamp counts the requests you send from their cards, as History keeps them. Run all,
        {navLabel('flows')} and codestamp-cli runs are not counted here; Run all results are under Tests.
      </p>
    {:else if catalog.tab === 'tests'}
      {@const tests = detail.tests}
      <div class="tiles">
        <StatTile
          label="Test coverage"
          value={tests.coverage === null ? 'None' : formatPercent(tests.coverage)}
          note={`${String(tests.tested)} of ${plural(tests.runnable, 'request')} have a test script`}
        />
        <StatTile
          label="Pass rate, latest run"
          value={latest && latest.passed + latest.failed > 0
            ? formatPercent((latest.passed * 100) / (latest.passed + latest.failed))
            : 'Not run'}
          note={latest
            ? `${String(latest.passed)} of ${plural(latest.passed + latest.failed, 'send')} passed`
            : undefined}
          noteTone={latest && latest.failed > 0 ? 'err' : 'neutral'}
        />
        <StatTile
          label="Tests, latest run"
          value={latest
            ? `${String(latest.testsPassed)} of ${String(latest.testsTotal)}`
            : 'Not run'}
          note={latest ? 'Tests passed' : undefined}
        />
        <StatTile
          label="Runs kept"
          value={String(tests.runs.length)}
          note="The newest 20 are kept"
        />
      </div>

      {#if latest}
        <section class="card" aria-labelledby="latest-title">
          <div class="card-head">
            <h2 id="latest-title">Latest run</h2>
            <span class="state {RUN_TONES[latest.outcome]}">{RUN_LABELS[latest.outcome]}</span>
            <span class="muted"
              >{formatSince(latest.finishedAt)} · {latest.environment ?? 'No environment'} · {runLength(
                latest.startedAt,
                latest.finishedAt,
              )}</span
            >
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th scope="col">Request</th>
                  <th scope="col" class="num">Tests</th>
                  <th scope="col" class="num">Average time</th>
                  <th scope="col">Result</th>
                </tr>
              </thead>
              <tbody>
                {#each latest.endpoints as endpoint (endpoint.requestId)}
                  <tr>
                    <td>
                      <span class="request">
                        <MethodStamp
                          label={stampLabel(endpoint.kind, endpoint.method)}
                          tone={stampTone(endpoint.kind, endpoint.method)}
                          size="sm"
                        />
                        <button
                          type="button"
                          class="target"
                          onclick={() => {
                            openRequest(endpoint.requestId);
                          }}>{endpoint.name}</button
                        >
                      </span>
                    </td>
                    <td class="num"
                      >{endpoint.testsTotal === 0
                        ? 'None'
                        : `${String(endpoint.testsPassed)} of ${String(endpoint.testsTotal)}`}</td
                    >
                    <td class="num"
                      >{endpoint.avgMs === null ? 'None' : formatDuration(endpoint.avgMs)}</td
                    >
                    <td>
                      {#if endpoint.failed > 0}
                        <span class="state err"
                          >Failed{sendsOf(endpoint) > 1
                            ? ` ${String(endpoint.failed)} of ${String(sendsOf(endpoint))}`
                            : ''}</span
                        >
                      {:else if endpoint.passed > 0}
                        <span class="state ok">Passed</span>
                      {:else}
                        <span class="muted">Skipped</span>
                      {/if}
                      {#if endpoint.lastCode !== null}<span class="muted mono code"
                          >{endpoint.lastCode}</span
                        >{/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </section>

        <section class="card" aria-labelledby="runs-title">
          <h2 id="runs-title">Recent runs</h2>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th scope="col">Finished</th>
                  <th scope="col">Environment</th>
                  <th scope="col">Result</th>
                  <th scope="col" class="num">Sends passed</th>
                  <th scope="col" class="num">Tests passed</th>
                  <th scope="col" class="num">Took</th>
                </tr>
              </thead>
              <tbody>
                {#each tests.runs as run (run.id)}
                  <tr>
                    <td>{formatDateTime(run.finishedAt)}</td>
                    <td>{run.environment ?? 'None'}</td>
                    <td
                      ><span class="state {RUN_TONES[run.outcome]}">{RUN_LABELS[run.outcome]}</span
                      ></td
                    >
                    <td class="num">{run.passed} of {run.passed + run.failed}</td>
                    <td class="num">{run.testsPassed} of {run.testsTotal}</td>
                    <td class="num">{runLength(run.startedAt, run.finishedAt)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </section>
      {:else}
        <section class="card">
          <h2>No runs yet</h2>
          <p class="muted">
            Run all on this collection sends each HTTP, GraphQL and SOAP request with its tests, and
            its results are kept here.
          </p>
        </section>
      {/if}
    {:else}
      <p class="lead">
        {#if detail.scores.quality.score === null}
          The rules have no requests to check yet.
        {:else}
          Quality score {detail.scores.quality.score}.
          {failingRules === 0
            ? 'Every rule passes.'
            : `${String(failingRules)} of ${plural(detail.rules.length, 'rule')} found something to fix.`}
        {/if}
      </p>
      <RuleList
        rules={detail.rules}
        onOpenRequest={openRequest}
        onOpenCollection={() => {
          onOpenCollection(entry.id);
        }}
      />
    {/if}
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .back {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px 4px 4px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
  }

  .back:hover {
    background: var(--surface-sunken);
  }

  .back svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 14px;
  }

  .icon {
    flex: none;
    width: 44px;
    height: 44px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
  }

  .title {
    flex: 1 1 300px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
    overflow-wrap: anywhere;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .tag {
    padding: 1px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-2);
    font-size: 12px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
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

  .quiet[aria-expanded='true'] {
    border-color: var(--text);
    background: var(--accent-soft);
    color: var(--text);
  }

  .quiet:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .tabs {
    display: flex;
    gap: 16px;
    overflow-x: auto;
    border-bottom: 1px solid var(--border-soft);
  }

  .tabs button {
    padding: 10px 2px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    white-space: nowrap;
  }

  .tabs button:hover {
    color: var(--text-2);
  }

  .tabs button.on {
    border-bottom-color: var(--text);
    color: var(--text);
    font-weight: 500;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .columns {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
  }

  .card {
    min-width: 0;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  /* Only cards side by side share the row; stacked cards keep their own height. */
  .columns > .card {
    flex: 1 1 320px;
  }

  .columns > .score-card {
    flex-basis: 420px;
  }

  h2 {
    margin: 0 0 10px;
    font-size: 15px;
    font-weight: 600;
  }

  .card-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px 12px;
    margin-bottom: 10px;
  }

  .card-head h2 {
    margin: 0;
  }

  .score-line {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 8px;
  }

  .score {
    font-size: 48px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: -0.02em;
  }

  .out-of {
    color: var(--text-muted);
    font-size: 15px;
  }

  .parts {
    margin: 16px 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .parts li {
    display: grid;
    grid-template-columns: 120px minmax(80px, 1fr) minmax(0, auto);
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }

  .part-name {
    font-weight: 500;
  }

  .weight {
    color: var(--text-muted);
    font-weight: 400;
  }

  /* The fill carries the grade; the track is the same for every part. */
  .meter {
    height: 8px;
    border-radius: 999px;
    background: var(--surface-sunken);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: 999px;
  }

  .fill.ok {
    background: var(--ok);
  }

  .fill.warn {
    background: var(--warn);
  }

  .fill.err {
    background: var(--err);
  }

  .part-value {
    color: var(--text-2);
  }

  .explain,
  .footnote {
    margin: 14px 0 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .footnote {
    margin: 0;
  }

  .muted {
    color: var(--text-muted);
  }

  .card > .muted {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }

  .assets {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px 16px;
    margin: 0;
    font-size: 13px;
  }

  .assets dt {
    color: var(--text-muted);
  }

  .assets dd {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
  }

  .flows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .flows li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
    padding: 8px 0;
    border-top: 1px solid var(--border-soft);
    font-size: 13px;
  }

  .flows li:first-child {
    border-top: 0;
  }

  .target {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-weight: 500;
    text-align: left;
  }

  .target:hover {
    text-decoration: underline;
  }

  .tiles {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .table-wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 460px;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    padding: 7px 10px;
    border-bottom: 1px solid var(--border-soft);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  td {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-soft);
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .request {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .state {
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

  .code {
    margin-left: 8px;
    font-size: 12px;
  }

  .lead {
    margin: 0;
    color: var(--text-2);
  }
</style>
