<script lang="ts">
  import { untrack } from 'svelte';
  import type { FlowDetail, FlowStep } from '../api/flows';
  import type { SearchEntry } from '../api/search';
  import { incomingValues, valueState } from '../flow/lineage';
  import { formatDuration } from '../format/units';
  import type { Environments } from '../state/environments.svelte';
  import type { Flows } from '../state/flows.svelte';
  import { notices } from '../state/notices.svelte';
  import type { SaveState } from '../state/workspace.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import EditableText from './EditableText.svelte';
  import EnvironmentSelect from './EnvironmentSelect.svelte';
  import FlowMap from './FlowMap.svelte';
  import FlowStepCard from './FlowStepCard.svelte';
  import FlowStepPicker from './FlowStepPicker.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    flows: Flows;
    flow: FlowDetail;
    environments: Environments;
    onOpenRequest: (collectionId: string, requestId: string) => void;
  }

  let { flows, flow, environments, onOpenRequest }: Props = $props();

  type Picking = { mode: 'add' } | { mode: 'replace'; stepId: string };

  let selectedId = $state<string | null>(null);
  let picking = $state<Picking | null>(null);
  let confirmingDelete = $state(false);

  const SAVE_LABELS = new Map<SaveState, string>([
    ['saved', 'Saved'],
    ['unsaved', 'Unsaved changes'],
    ['saving', 'Saving…'],
    ['failed', 'Not saved'],
  ]);

  // A removed or unknown step falls back to the first one, so a flow with steps always shows one.
  const selectedIndex = $derived(
    Math.max(
      flow.steps.findIndex((step) => step.id === selectedId),
      0,
    ),
  );
  const selected = $derived(flow.steps.at(selectedIndex) ?? null);

  const received = $derived(
    incomingValues(flow.steps, selectedIndex).map((value) => ({
      name: value.name,
      from: value.from + 1,
      state: valueState(flows.runs.get(flow.steps[value.from]?.id ?? ''), value.name),
    })),
  );

  // After a run, the step that stopped it (or the first failure) is shown to explain why. Only a
  // new result moves the selection; reordering steps afterwards must not.
  $effect(() => {
    const summary = flows.summary;
    if (!summary || summary.failed === 0) {
      return;
    }
    untrack(() => {
      const failed = flow.steps.find((step) => {
        const run = flows.runs.get(step.id);
        return run?.state === 'done' && run.status === 'failed';
      });
      selectedId = summary.stoppedAt ?? failed?.id ?? selectedId;
    });
  });

  const runningNumber = $derived(
    flow.steps.findIndex((step) => flows.runs.get(step.id)?.state === 'running') + 1,
  );

  const endText = $derived.by(() => {
    if (flows.running) {
      return runningNumber > 0
        ? `Running step ${String(runningNumber)} of ${String(flow.steps.length)}…`
        : 'Starting…';
    }
    if (flows.runError) {
      return flows.runError;
    }
    const summary = flows.summary;
    if (!summary) {
      return flow.steps.length === 0
        ? 'Add the first step, then run the flow'
        : 'Run the flow to see the result here';
    }
    const parts = [`${String(summary.passed)} passed`, `${String(summary.failed)} failed`];
    if (summary.skipped > 0) {
      parts.push(`${String(summary.skipped)} skipped`);
    }
    parts.push(formatDuration(summary.durationMs));
    return parts.join(' · ');
  });

  const stopNote = $derived.by(() => {
    const summary = flows.summary;
    if (!summary || flows.running) {
      return null;
    }
    if (summary.cancelled) {
      return 'Stopped before the end';
    }
    const index = flow.steps.findIndex((step) => step.id === summary.stoppedAt);
    const stopped = flow.steps.at(index);
    if (index < 0 || !stopped) {
      return null;
    }
    return `Stopped at step ${String(index + 1)}: ${stopped.request?.name ?? 'missing request'}`;
  });

  const outcome = $derived.by((): 'passed' | 'failed' | null => {
    const summary = flows.summary;
    if (flows.running || !summary) {
      return flows.runError ? 'failed' : null;
    }
    return summary.failed > 0 || summary.cancelled ? 'failed' : 'passed';
  });

  function pick(entry: SearchEntry): void {
    const current = picking;
    picking = null;
    if (!current) {
      return;
    }
    if (current.mode === 'add') {
      flows.addStep(entry);
      selectedId = flow.steps.at(-1)?.id ?? null;
    } else {
      flows.replaceRequest(current.stepId, entry);
    }
  }

  function removeStep(step: FlowStep): void {
    const index = flow.steps.indexOf(step);
    flows.removeStep(step.id);
    selectedId = (flow.steps.at(index) ?? flow.steps.at(index - 1))?.id ?? null;
  }

  async function openRequest(step: FlowStep): Promise<void> {
    const request = step.request;
    if (request && (await flows.flushSave())) {
      onOpenRequest(request.collectionId, request.id);
    }
  }

  function handleShortcut(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      event.preventDefault();
      void flows.run();
    }
  }
</script>

<section class="editor" aria-label="Flow {flow.name}" onkeydowncapture={handleShortcut}>
  <header class="head">
    <div class="name">
      <EditableText
        value={flow.name}
        label="Flow name"
        variant="title"
        onCommit={(next: string) => {
          flow.name = next;
          flows.markEdited();
        }}
      />
      <span class="save {flows.saveState}" role="status" title={flows.saveError ?? undefined}>
        {SAVE_LABELS.get(flows.saveState)}{flows.saveState === 'failed' && flows.saveError
          ? `: ${flows.saveError}`
          : ''}
      </span>
    </div>
    {#if flows.running}
      <button type="button" class="run stop" onclick={() => void flows.stop()}>Stop</button>
    {:else}
      <button
        type="button"
        class="run"
        disabled={flow.steps.length === 0}
        title={flow.steps.length === 0 ? 'Add a step first' : undefined}
        onclick={() => void flows.run()}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
          ><path d="M7 5l12 7-12 7z" /></svg
        >
        Run flow
      </button>
    {/if}
    <IconButton
      label="Delete flow"
      danger
      align="end"
      popup="dialog"
      expanded={confirmingDelete}
      disabled={flows.running}
      onclick={() => (confirmingDelete = true)}
    >
      <svg width="16" height="16" viewBox="0 0 24 24">
        <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
      </svg>
    </IconButton>
  </header>

  <section class="map-card" aria-label="Flow map">
    <div class="bar">
      <span class="bar-label">Environment</span>
      <EnvironmentSelect {environments} />
      <span class="end-text" role="status">
        {endText}
        {#if stopNote}<span class="stop-note">{stopNote}</span>{/if}
      </span>
    </div>
    <FlowMap
      steps={flow.steps}
      runs={flows.runs}
      selectedId={selected?.id ?? null}
      stoppedAt={flows.running ? null : (flows.summary?.stoppedAt ?? null)}
      {outcome}
      adding={picking?.mode === 'add'}
      disabled={flows.running}
      onSelect={(stepId: string) => (selectedId = stepId)}
      onAdd={() => (picking = { mode: 'add' })}
    />
  </section>

  {#if selected}
    {@const step = selected}
    <FlowStepCard
      {step}
      number={selectedIndex + 1}
      total={flow.steps.length}
      run={flows.runs.get(step.id)}
      {received}
      disabled={flows.running}
      onChange={() => {
        flows.markEdited();
      }}
      onMove={(delta: -1 | 1) => {
        flows.moveStep(step.id, delta);
      }}
      onRemove={() => {
        removeStep(step);
      }}
      onReplace={() => (picking = { mode: 'replace', stepId: step.id })}
      onOpenRequest={() => void openRequest(step)}
    />
  {/if}
</section>

{#if picking}
  <FlowStepPicker
    title={picking.mode === 'add' ? 'Add a step' : 'Choose another request'}
    onPick={pick}
    onClose={() => (picking = null)}
  />
{/if}

<ConfirmDialog
  open={confirmingDelete}
  title="Delete “{flow.name}”?"
  message="The flow and its steps are removed. The requests it calls stay in their collections."
  confirmLabel="Delete"
  onConfirm={() => {
    confirmingDelete = false;
    void notices.guard(() => flows.remove(flow.id));
  }}
  onCancel={() => (confirmingDelete = false)}
/>

<style>
  .editor {
    flex: 1 1 520px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .name {
    flex: 1 1 260px;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .save {
    padding-left: 8px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .save.failed {
    color: var(--err);
  }

  .run {
    height: 38px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 14px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .run svg {
    fill: currentColor;
  }

  .run:disabled {
    opacity: 0.6;
  }

  .run.stop {
    background: var(--surface);
    color: var(--err);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .map-card {
    padding: 12px 14px 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
  }

  .bar-label {
    font-size: 13px;
    color: var(--text-muted);
  }

  .end-text {
    margin-left: auto;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    font-size: 13px;
    color: var(--text-2);
    text-align: right;
  }

  .stop-note {
    font-size: 12px;
    color: var(--err);
  }
</style>
