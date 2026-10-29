<script lang="ts">
  import Dropdown from './Dropdown.svelte';
  import NumberField from './NumberField.svelte';
  import { toAppError } from '../api/http';
  import { statusTone } from '../format/status';
  import { formatClock } from '../format/time';
  import { formatDuration } from '../format/units';
  import { stampTone } from '../request/methods';
  import { notices } from '../state/notices.svelte';
  import { stampLabel } from '../request/kinds';
  import { ITERATION_RANGE, RUN_DELAYS, type RunResult, type Runner } from '../state/runner.svelte';

  interface Props {
    runner: Runner;
    onRunAgain: () => void;
    onOpenRequest: (collectionId: string, requestId: string) => void;
  }

  let { runner, onRunAgain, onOpenRequest }: Props = $props();

  const MARKS = new Map<RunResult['status'], string>([
    ['waiting', '·'],
    ['running', '…'],
    ['passed', '✓'],
    ['failed', '✗'],
    ['skipped', '-'],
  ]);
  const LABELS = new Map<RunResult['status'], string>([
    ['waiting', 'Waiting'],
    ['running', 'Running'],
    ['passed', 'Passed'],
    ['failed', 'Failed'],
    ['skipped', 'Skipped'],
  ]);

  let exportStatus = $state<string | null>(null);

  const summary = $derived(runner.summary);
  const counted = $derived(summary.passed + summary.failed);
  const subtitle = $derived.by(() => {
    const environment = runner.environmentName ?? 'No environment';
    if (runner.running) {
      return `Run all · ${environment} · running…`;
    }
    const finished =
      runner.finishedAt === null ? '' : ` · finished ${formatClock(runner.finishedAt)}`;
    return `Run all · ${environment}${finished}`;
  });

  function testsLabel(result: RunResult): string {
    if (result.tests.length === 0) {
      return 'No tests';
    }
    const passed = result.tests.filter((test) => test.passed).length;
    return `${String(passed)}/${String(result.tests.length)} tests`;
  }

  async function exportReport(): Promise<void> {
    exportStatus = null;
    try {
      const path = await runner.exportReport();
      exportStatus = path === null ? null : `Saved to ${path}`;
    } catch (caught) {
      notices.report(toAppError(caught));
    }
  }

  async function chooseData(): Promise<void> {
    await notices.guard(() => runner.chooseData());
  }

  /** The first few values of a pass's data row, to tell passes apart. */
  function dataSummary(iteration: number): string {
    return runner
      .dataFor(iteration)
      .slice(0, DATA_SUMMARY_COLUMNS)
      .map((pair) => `${pair.name}=${pair.value}`)
      .join(' · ');
  }

  const DATA_SUMMARY_COLUMNS = 3;

  const DELAY_OPTIONS = RUN_DELAYS.map((delay) => ({
    value: String(delay),
    label: delay < 1000 ? `${String(delay)} ms` : '1 s',
  }));
</script>

<div class="view">
  <header class="top">
    <div class="title">
      <h1>{runner.collectionName}</h1>
      <p>{subtitle}</p>
    </div>
    <button
      type="button"
      class="quiet"
      disabled={runner.running || runner.results.length === 0}
      onclick={() => void exportReport()}
    >
      Export report
    </button>
    {#if runner.running}
      <button
        type="button"
        class="primary"
        onclick={() => {
          runner.stop();
        }}>Stop</button
      >
    {:else}
      <button type="button" class="primary" onclick={onRunAgain}>Run again</button>
    {/if}
  </header>

  {#if exportStatus}
    <p class="saved" role="status">{exportStatus}</p>
  {/if}

  <section class="card summary" aria-label="Run summary">
    <div class="headline">
      <span class="big" role="status">
        {#if runner.results.length === 0}
          Nothing to run
        {:else}
          {summary.passed} of {counted || summary.total} passed
        {/if}
      </span>
      <span class="meta">
        {summary.tests}
        {summary.tests === 1 ? 'test' : 'tests'} · {summary.failed} failed{summary.skipped > 0
          ? ` · ${String(summary.skipped)} skipped`
          : ''} · {formatDuration(runner.durationMs())} total
      </span>
    </div>
    <div class="bar" aria-hidden="true">
      <span class="passed" style:flex={summary.passed}></span>
      <span class="failed" style:flex={summary.failed}></span>
      <span class="skipped" style:flex={summary.skipped}></span>
      <span
        class="waiting"
        style:flex={summary.total - summary.passed - summary.failed - summary.skipped}
      ></span>
    </div>
    <div class="options">
      <label>
        <input type="checkbox" bind:checked={runner.stopOnFailure} disabled={runner.running} />
        Stop on first failure
      </label>
      <span class="delay">
        Delay between requests
        <Dropdown
          label="Delay between requests"
          value={String(runner.delayMs)}
          options={DELAY_OPTIONS}
          disabled={runner.running}
          onChange={(value: string) => {
            runner.delayMs = RUN_DELAYS.find((delay) => String(delay) === value) ?? 0;
          }}
        />
      </span>
    </div>
    <div class="options data">
      {#if runner.data}
        <span class="data-file">
          <span class="data-name">{runner.data.name}</span>
          <span class="meta">
            {runner.data.rows.length}
            {runner.data.rows.length === 1 ? 'row' : 'rows'} · {runner.data.columns.join(', ')}
          </span>
        </span>
        <button
          type="button"
          class="link"
          disabled={runner.running}
          onclick={() => (runner.data = null)}>Remove data file</button
        >
      {:else}
        <span class="iterations">
          <NumberField
            label="Iterations"
            value={runner.iterations}
            range={ITERATION_RANGE}
            onChange={(value: number) => (runner.iterations = value)}
          />
        </span>
        <button
          type="button"
          class="quiet"
          disabled={runner.running}
          onclick={() => void chooseData()}>Use a data file…</button
        >
        <span class="meta">CSV or JSON; each row runs the collection once with its values.</span>
      {/if}
    </div>
    {#if runner.limitNote}
      <p class="limit" role="status">{runner.limitNote}</p>
    {/if}
  </section>

  {#each runner.results as result, index (`${String(result.iteration)}:${result.requestId}`)}
    {#if runner.passCount > 1 && runner.results[index - 1]?.iteration !== result.iteration}
      <h2 class="iteration-head">
        Iteration {result.iteration + 1} of {runner.passCount}
        {#if runner.data}<span class="meta mono">{dataSummary(result.iteration)}</span>{/if}
      </h2>
    {/if}
    <article
      class="card result {result.status}"
      aria-label="{result.name} {LABELS.get(result.status)?.toLowerCase()}"
    >
      <div class="row">
        <span class="mark {result.status}" aria-label={LABELS.get(result.status)}>
          {MARKS.get(result.status)}
        </span>
        <span
          class="method mono"
          style:color="var(--method-{stampTone(result.kind, result.method)})"
          >{stampLabel(result.kind, result.method)}</span
        >
        <span class="name">{result.name}</span>
        {#if result.note}
          <span class="meta">{result.note}</span>
        {:else if result.status !== 'waiting' && result.status !== 'running'}
          <span class="meta">{testsLabel(result)}</span>
          {#if result.code !== null}
            <span class="chip {statusTone(result.code)}">{result.code}</span>
          {/if}
          {#if result.elapsedMs !== null}
            <span class="meta">{formatDuration(result.elapsedMs)}</span>
          {/if}
        {/if}
      </div>
      {#if result.status === 'failed'}
        <div class="details">
          {#each result.tests as test, index (index)}
            <div class:pass={test.passed} class:fail={!test.passed}>
              {test.passed ? '✓' : '✗'}
              {test.name}{test.error ? `: ${test.error}` : ''}
            </div>
          {/each}
          {#each result.errors as message, index (index)}
            <div class="fail">{message}</div>
          {/each}
          {#if result.bodySnippet}
            <pre class="snippet mono">{result.bodySnippet}</pre>
          {/if}
          <div class="links">
            {#if runner.collectionId}
              {@const collectionId = runner.collectionId}
              <button
                type="button"
                class="link"
                onclick={() => {
                  onOpenRequest(collectionId, result.requestId);
                }}>Open request</button
              >
            {/if}
            <button
              type="button"
              class="link"
              disabled={runner.running}
              onclick={() => void runner.runOne(result)}>Run only this</button
            >
          </div>
        </div>
      {/if}
    </article>
  {/each}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 4px;
  }

  .title {
    flex: 1 1 280px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .title p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .data {
    align-items: flex-end;
    margin-top: 10px;
  }

  .iterations {
    width: 120px;
  }

  .data-file {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .data-name {
    font-weight: 500;
  }

  .data .quiet {
    height: 40px;
  }

  .limit {
    margin: 10px 0 0;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: var(--warn-bg);
    color: var(--warn);
    font-size: 13px;
  }

  .iteration-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 12px;
    margin: 14px 0 2px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }

  .iteration-head .meta {
    font-weight: 400;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quiet,
  .primary {
    height: 34px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }

  .quiet {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-2);
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .quiet:disabled {
    opacity: 0.6;
  }

  .saved {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .summary {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
  }

  .headline {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 18px;
  }

  .big {
    font-size: 22px;
    font-weight: 600;
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .headline .meta {
    font-size: 13px;
  }

  .bar {
    display: flex;
    height: 10px;
    border-radius: 5px;
    overflow: hidden;
    background: var(--surface-sunken);
  }

  .bar .passed {
    background: var(--dot-ok);
  }

  .bar .failed {
    background: var(--dot-err);
  }

  .bar .skipped {
    background: var(--border-strong);
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    font-size: 13px;
    color: var(--text-2);
  }

  .options label,
  .delay {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .options input {
    accent-color: var(--accent);
  }

  .result.failed {
    border-color: var(--err-line);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 11px 16px;
  }

  .mark {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 700;
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .mark.passed {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .mark.failed {
    background: var(--err-bg);
    color: var(--err);
  }

  .mark.running {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .method {
    flex: none;
    width: 52px;
    font-size: 11px;
    font-weight: 600;
  }

  .name {
    flex: 1 1 200px;
    min-width: 0;
    font-weight: 500;
  }

  .chip {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: var(--surface-sunken);
    color: var(--text-2);
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .details {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 16px 14px 54px;
    font-size: 13px;
  }

  .pass {
    color: var(--ok);
  }

  .fail {
    color: var(--err);
    overflow-wrap: anywhere;
  }

  .snippet {
    margin: 0;
    max-height: 160px;
    overflow: auto;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-muted);
    color: var(--text-2);
    font-size: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .links {
    display: flex;
    gap: 14px;
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
  }

  .link:disabled {
    color: var(--text-muted);
  }
</style>
