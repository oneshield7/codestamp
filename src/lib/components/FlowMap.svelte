<script lang="ts">
  import type { FlowStep } from '../api/flows';
  import { incomingValues, outgoingValues, valueState, type ValueState } from '../flow/lineage';
  import { stepMark, stepResult } from '../flow/outcome';
  import { stampLabel } from '../request/kinds';
  import { stampTone } from '../request/methods';
  import type { StepRun } from '../state/flows.svelte';
  import FlowStepMark from './FlowStepMark.svelte';
  import FlowValueChip from './FlowValueChip.svelte';
  import IconButton from './IconButton.svelte';
  import MethodStamp from './MethodStamp.svelte';

  interface Props {
    steps: FlowStep[];
    runs: ReadonlyMap<string, StepRun>;
    selectedId: string | null;
    /** The step that failed and stopped the last run. */
    stoppedAt: string | null;
    outcome: 'passed' | 'failed' | null;
    adding: boolean;
    disabled: boolean;
    onSelect: (stepId: string) => void;
    onAdd: () => void;
  }

  let { steps, runs, selectedId, stoppedAt, outcome, adding, disabled, onSelect, onAdd }: Props =
    $props();

  /** A value line between two steps (indexes); `stops` are the steps that send the value. */
  interface ValueLine {
    name: string;
    from: number;
    to: number;
    stops: number[];
    state: ValueState;
  }

  let scroller: HTMLDivElement;

  const selected = $derived(steps.findIndex((step) => step.id === selectedId));
  const stopped = $derived(steps.findIndex((step) => step.id === stoppedAt));
  // Start, Add step and End get fixed columns and the steps share the rest equally, so a line can
  // find a step's centre from the columns it spans.
  const template = $derived(
    [
      'var(--edge)',
      ...(steps.length > 0
        ? [`repeat(${String(steps.length)}, minmax(var(--node-min), 1fr))`]
        : []),
      'var(--add)',
      'var(--edge)',
    ].join(' '),
  );

  const lines = $derived.by((): ValueLine[] => {
    const current = steps.at(selected);
    if (!current) {
      return [];
    }
    const received = incomingValues(steps, selected).map((value) => ({
      name: value.name,
      from: value.from,
      to: selected,
      stops: [selected],
      state: valueState(runs.get(steps[value.from]?.id ?? ''), value.name),
    }));
    const passed = outgoingValues(steps, selected).flatMap((value) => {
      const last = value.to.at(-1);
      return last === undefined
        ? []
        : [
            {
              name: value.name,
              from: selected,
              to: last,
              stops: value.to,
              state: valueState(runs.get(current.id), value.name),
            },
          ];
    });
    return [...received, ...passed];
  });

  function column(index: number): number {
    return index + 2;
  }

  function stopAt(line: ValueLine, stop: number): string {
    return `${String(((stop - line.from) / (line.to - line.from)) * 100)}%`;
  }

  // Only the map scrolls sideways to show the selected step; the page stays where it is.
  $effect(() => {
    const node = selectedId === null ? null : scroller.querySelector('[aria-current="step"]');
    if (!node) {
      return;
    }
    const box = node.getBoundingClientRect();
    const view = scroller.getBoundingClientRect();
    if (box.left < view.left) {
      scroller.scrollLeft -= view.left - box.left + 16;
    } else if (box.right > view.right) {
      scroller.scrollLeft += box.right - view.right + 16;
    }
  });

  /** Left and right move from the focused step, which may differ from the selected one. */
  function moveSelection(event: KeyboardEvent, from: number): void {
    const delta = new Map([
      ['ArrowLeft', -1],
      ['ArrowRight', 1],
    ]).get(event.key);
    const next = delta === undefined ? undefined : steps[from + delta];
    if (!next) {
      return;
    }
    event.preventDefault();
    onSelect(next.id);
    scroller.querySelector<HTMLElement>(`[data-step="${next.id}"]`)?.focus();
  }
</script>

<div class="map" bind:this={scroller}>
  <div class="track" style:grid-template-columns={template} style:--steps={steps.length}>
    {#each lines as line, row (`${line.name}:${String(line.from)}`)}
      <div
        class="line {line.state}"
        style:grid-row={row + 1}
        style:grid-column="{column(line.from)} / {column(line.to) + 1}"
        style:--span={line.to - line.from + 1}
        aria-hidden="true"
      >
        <span class="label"><FlowValueChip name={line.name} state={line.state} /></span>
        {#each line.stops as stop (stop)}
          <svg class="tip" style:left={stopAt(line, stop)} width="12" height="8" viewBox="0 0 12 8">
            <path d="M1 1l5 6 5-6" />
          </svg>
        {/each}
      </div>
    {/each}

    <ol class="nodes" style:grid-row={lines.length + 1} aria-label="Steps from start to end">
      <li class="edge">
        <span class="pill start">
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
            ><path d="M7 5l12 7-12 7z" /></svg
          >
          Start
        </span>
      </li>
      {#each steps as step, index (step.id)}
        {@const run = runs.get(step.id)}
        {@const mark = stepMark(run)}
        <li>
          <button
            type="button"
            class="node {mark}"
            class:current={step.id === selectedId}
            aria-current={step.id === selectedId ? 'step' : undefined}
            data-step={step.id}
            onclick={() => {
              onSelect(step.id);
            }}
            onkeydown={(event) => {
              moveSelection(event, index);
            }}
          >
            <span class="top">
              <FlowStepMark {mark} size={14} />
              <span class="number mono">{String(index + 1).padStart(2, '0')}</span>
              {#if step.request}
                <MethodStamp
                  label={stampLabel(step.request.kind, step.request.method)}
                  tone={stampTone(step.request.kind, step.request.method)}
                  size="sm"
                />
              {/if}
            </span>
            <span class="name" class:missing={!step.request}
              >{step.request?.name ?? 'Request deleted'}</span
            >
            <span class="result mono">{stepResult(run, 'short')}</span>
          </button>
        </li>
      {/each}
      <li class="add">
        <IconButton
          label="Add step"
          round
          popup="dialog"
          expanded={adding}
          {disabled}
          onclick={onAdd}
        >
          <svg width="16" height="16" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
        </IconButton>
      </li>
      <li class="edge">
        <span class="pill end {outcome ?? ''}">
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M6 21V4M6 4h11l-2 4 2 4H6" />
          </svg>
          End
        </span>
      </li>
    </ol>

    {#if stopped >= 0}
      <div
        class="stopped"
        style:grid-row={lines.length + 2}
        style:grid-column="{column(stopped)} / -1"
        style:--span={steps.length - stopped}
        aria-hidden="true"
      >
        <span class="label">Stopped at step {stopped + 1}</span>
      </div>
    {/if}
  </div>
</div>

<style>
  /* The bottom room keeps the Add step tooltip inside the map, so it never adds a scrollbar. */
  .map {
    overflow-x: auto;
    padding: 4px 0 16px;
  }

  .track {
    --edge: 80px;
    --add: 52px;
    --node-min: 120px;
    display: grid;
    min-width: calc(2 * var(--edge) + var(--add) + var(--steps) * var(--node-min));
  }

  /* A value line runs from the centre of one step's column to the centre of another's. */
  .line {
    position: relative;
    margin: 0 calc(100% / (2 * var(--span)));
    height: 14px;
    margin-top: 22px;
    border: 1.5px solid var(--text-muted);
    border-bottom: 0;
    border-radius: 10px 10px 0 0;
  }

  .line.passed {
    border-color: var(--ok);
  }

  .line.missing {
    border-color: var(--err);
    border-style: dashed;
  }

  .line .label {
    position: absolute;
    top: -12px;
    left: 50%;
    transform: translateX(-50%);
    padding: 0 4px;
    background: var(--surface);
  }

  .tip {
    position: absolute;
    bottom: -6px;
    transform: translateX(-50%);
  }

  .tip path {
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .passed .tip path {
    stroke: var(--ok);
  }

  .missing .tip path {
    stroke: var(--err);
  }

  .nodes {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
    position: relative;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
  }

  /* The shaft from Start to End; nodes sit on it with their own background. */
  .nodes::before {
    content: '';
    position: absolute;
    top: 50%;
    left: calc(var(--edge) / 2);
    right: calc(var(--edge) / 2);
    border-top: 1.5px solid var(--border-strong);
  }

  .nodes li {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 6px;
  }

  .nodes li.edge {
    padding: 0 2px;
  }

  /* An arrowhead where the shaft enters each node after Start. */
  .start-point {
    justify-content: flex-end;
  }

  .end-point {
    justify-content: flex-start;
  }

  .nodes li + li::before {
    content: '';
    position: absolute;
    left: -1px;
    top: 50%;
    width: 6px;
    height: 6px;
    border-top: 1.5px solid var(--border-strong);
    border-right: 1.5px solid var(--border-strong);
    transform: translateY(-50%) rotate(45deg);
  }

  .node {
    position: relative;
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    text-align: left;
  }

  .node:hover {
    border-color: var(--border-strong);
  }

  .node.failed {
    border-color: var(--err-line);
  }

  .node.skipped {
    border-style: dashed;
  }

  .node.current {
    border-color: var(--text);
    box-shadow: var(--shadow-raise);
  }

  .top {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .number {
    font-size: 11px;
    color: var(--text-muted);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 500;
  }

  .name.missing {
    color: var(--err);
  }

  .skipped .name {
    color: var(--text-muted);
  }

  .result {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: var(--text-muted);
  }

  .passed .result {
    color: var(--ok);
  }

  .failed .result {
    color: var(--err);
  }

  .pill {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 10px 0 9px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--surface);
    font-size: 13px;
    font-weight: 600;
  }

  .start svg {
    fill: var(--ok);
  }

  .end svg {
    fill: none;
    stroke: var(--text-2);
    stroke-width: 2;
    stroke-linejoin: round;
  }

  /* The tint is see-through in the dark themes, so it sits on the surface to hide the shaft. */
  .end.passed {
    border-color: var(--ok-line);
    background: linear-gradient(var(--ok-bg), var(--ok-bg)) var(--surface);
    color: var(--ok);
  }

  .end.failed {
    border-color: var(--err-line);
    background: linear-gradient(var(--err-bg), var(--err-bg)) var(--surface);
    color: var(--err);
  }

  .end.passed svg {
    stroke: var(--ok);
  }

  .end.failed svg {
    stroke: var(--err);
  }

  /* From the centre of the step that stopped the run to the centre of End. */
  .stopped {
    position: relative;
    margin-left: calc((100% - var(--add) - var(--edge)) / (2 * var(--span)));
    margin-right: calc(var(--edge) / 2);
    height: 12px;
    margin-top: 8px;
    margin-bottom: 16px;
    border: 1.5px dashed var(--err);
    border-top: 0;
    border-radius: 0 0 10px 10px;
  }

  .stopped .label {
    position: absolute;
    bottom: -11px;
    left: 50%;
    transform: translateX(-50%);
    padding: 1px 8px;
    border: 1px solid var(--err-line);
    border-radius: 999px;
    background: var(--surface);
    color: var(--err);
    font-size: 12px;
    white-space: nowrap;
  }
</style>
