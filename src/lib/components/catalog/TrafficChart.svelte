<script lang="ts">
  import type { DayTraffic } from '../../api/catalog';
  import { plural } from '../../format/plural';

  interface Props {
    days: DayTraffic[];
  }

  let { days }: Props = $props();

  const NICE_STEPS = [1, 2, 5] as const;

  let asTable = $state(false);

  /** The smallest 1, 2 or 5 times a power of ten that holds the busiest day. */
  const top = $derived.by(() => {
    const busiest = Math.max(1, ...days.map((day) => day.requests));
    for (let power = 1; ; power *= 10) {
      const step = NICE_STEPS.find((base) => base * power >= busiest);
      if (step !== undefined) {
        return step * power;
      }
    }
  });
  const totals = $derived({
    requests: days.reduce((sum, day) => sum + day.requests, 0),
    errors: days.reduce((sum, day) => sum + day.errors, 0),
  });

  function dayName(start: number, index: number): string {
    if (index === days.length - 1) {
      return 'Today';
    }
    return new Date(start).toLocaleDateString([], { weekday: 'short', day: 'numeric' });
  }

  function fullDay(start: number): string {
    return new Date(start).toLocaleDateString([], {
      weekday: 'long',
      day: 'numeric',
      month: 'long',
    });
  }

  /** A segment's height; two stacked segments each give up 1px for the 2px gap between them. */
  function share(value: number, stacked: boolean): string {
    const percent = `${String((value / top) * 100)}%`;
    return stacked ? `calc(${percent} - 1px)` : percent;
  }
</script>

<section class="chart" aria-labelledby="traffic-title">
  <header>
    <div>
      <h3 id="traffic-title">Requests per day</h3>
      <p>{plural(totals.requests, 'request')} sent, {plural(totals.errors, 'error')}</p>
    </div>
    <div class="legend" aria-hidden={asTable}>
      <span><i class="swatch answered"></i>Answered</span>
      <span><i class="swatch errors"></i>Errors</span>
    </div>
    <button type="button" class="toggle" aria-pressed={asTable} onclick={() => (asTable = !asTable)}
      >{asTable ? 'Show as chart' : 'Show as table'}</button
    >
  </header>

  {#if asTable}
    <table>
      <thead>
        <tr><th scope="col">Day</th><th scope="col">Sent</th><th scope="col">Errors</th></tr>
      </thead>
      <tbody>
        {#each days as day (day.start)}
          <tr>
            <th scope="row">{fullDay(day.start)}</th>
            <td>{day.requests.toLocaleString('en')}</td>
            <td>{day.errors.toLocaleString('en')}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <div
      class="plot"
      role="img"
      aria-label={`Requests per day for the last 7 days: ${plural(totals.requests, 'request')}, ${plural(totals.errors, 'error')}. Show as table lists each day.`}
    >
      <div class="grid" aria-hidden="true">
        <span class="line top"><span class="tick">{top.toLocaleString('en')}</span></span>
        <span class="line mid"><span class="tick">{(top / 2).toLocaleString('en')}</span></span>
        <span class="line base"><span class="tick">0</span></span>
      </div>
      <ol class="days" aria-hidden="true">
        {#each days as day, index (day.start)}
          {@const answered = day.requests - day.errors}
          {@const stacked = day.errors > 0 && answered > 0}
          <li class="slot">
            <div class="column">
              {#if day.errors > 0}
                <span class="segment errors" style:height={share(day.errors, stacked)}></span>
              {/if}
              {#if answered > 0}
                <span class="segment answered" style:height={share(answered, stacked)}></span>
              {/if}
            </div>
            <span class="day">{dayName(day.start, index)}</span>
            <span class="tip">
              <strong>{fullDay(day.start)}</strong>
              {plural(day.requests, 'request')} · {plural(day.errors, 'error')}
            </span>
          </li>
        {/each}
      </ol>
    </div>
  {/if}
</section>

<style>
  .chart {
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 10px 18px;
    margin-bottom: 14px;
  }

  header > div:first-child {
    flex: 1 1 200px;
  }

  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  header p {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .legend {
    display: flex;
    gap: 14px;
    align-items: center;
    font-size: 12px;
    color: var(--text-2);
  }

  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }

  /* Grey for every answered request and the status red for errors: the app's colour for
     meaning, kept apart by the 2px gap and their fixed place in the stack. */
  .answered {
    background: var(--text-muted);
  }

  .errors {
    background: var(--err);
  }

  .toggle {
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
  }

  .toggle:hover {
    border-color: var(--border-strong);
  }

  .plot {
    position: relative;
    height: 180px;
    padding-left: 40px;
  }

  .grid {
    position: absolute;
    inset: 0 0 24px 40px;
  }

  .line {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid var(--border-soft);
  }

  .line.top {
    top: 0;
  }

  .line.mid {
    top: 50%;
  }

  .line.base {
    bottom: 0;
    border-top-color: var(--border);
  }

  .tick {
    position: absolute;
    right: calc(100% + 8px);
    top: -8px;
    font-family: var(--font-code);
    font-size: 11px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .days {
    position: relative;
    display: flex;
    height: 100%;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .slot {
    position: relative;
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .slot:hover {
    background: var(--surface-muted);
  }

  .column {
    flex: 1;
    width: 24px;
    max-width: 60%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 2px;
  }

  .segment {
    display: block;
  }

  /* Rounded where the data ends, square on the baseline. */
  .segment:first-child {
    border-radius: 4px 4px 0 0;
  }

  .day {
    height: 24px;
    padding-top: 6px;
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .tip {
    position: absolute;
    bottom: calc(100% - 20px);
    left: 50%;
    z-index: 2;
    display: none;
    width: max-content;
    max-width: 200px;
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    color: var(--text-2);
    font-size: 12px;
    translate: -50% 0;
  }

  .tip strong {
    display: block;
    color: var(--text);
    font-weight: 600;
  }

  .slot:hover .tip {
    display: block;
  }

  /* The outer days open their tip inwards so it stays inside the chart. */
  .slot:first-child .tip {
    left: 0;
    translate: none;
  }

  .slot:last-child .tip {
    left: auto;
    right: 0;
    translate: none;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th,
  td {
    padding: 7px 10px;
    border-bottom: 1px solid var(--border-soft);
    text-align: left;
  }

  thead th {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
  }

  tbody th {
    font-weight: 400;
  }

  td {
    font-variant-numeric: tabular-nums;
  }
</style>
