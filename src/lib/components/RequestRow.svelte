<script lang="ts">
  import type { RequestSummary } from '../api/collections';
  import { statusTone } from '../format/status';
  import { formatDuration } from '../format/units';
  import { SKIPPED_IN_RUNS, stampLabel } from '../request/kinds';
  import { stampTone } from '../request/methods';
  import { variableNames } from '../state/variable-names.svelte';
  import type { LastResult } from '../state/workspace.svelte';
  import MethodStamp from './MethodStamp.svelte';
  import VariableText from './VariableText.svelte';

  interface Props {
    step: number;
    request: RequestSummary;
    collectionId: string;
    lastResult: LastResult | undefined;
    dropTarget: boolean;
    onOpen: () => void;
    onDragStart: (event: DragEvent) => void;
    onDragEnd: () => void;
    onDragOver: (event: DragEvent) => void;
    onDrop: (event: DragEvent) => void;
    onNudge: (step: -1 | 1) => void;
  }

  let {
    step,
    request,
    collectionId,
    lastResult,
    dropTarget,
    onOpen,
    onDragStart,
    onDragEnd,
    onDragOver,
    onDrop,
    onNudge,
  }: Props = $props();

  const tone = $derived(lastResult === undefined ? 'idle' : statusTone(lastResult.status));
  const stamp = $derived(stampTone(request.kind, request.method));
  const stampText = $derived(stampLabel(request.kind, request.method));

  $effect(() => {
    if (request.url.includes('{{')) {
      variableNames.ensure(collectionId);
    }
  });

  // Alt + arrow keys reorder without a mouse.
  function handleKey(event: KeyboardEvent): void {
    if (event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
      event.preventDefault();
      onNudge(event.key === 'ArrowUp' ? -1 : 1);
    }
  }
</script>

<button
  type="button"
  class="row"
  data-request-id={request.id}
  class:drop={dropTarget}
  draggable="true"
  aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
  onclick={onOpen}
  onkeydown={handleKey}
  ondragstart={onDragStart}
  ondragend={onDragEnd}
  ondragover={onDragOver}
  ondrop={onDrop}
>
  <span class="step mono">{String(step).padStart(2, '0')}</span>
  <MethodStamp label={stampText} tone={stamp} />
  <span class="text">
    <span class="name">{request.name}</span>
    <span class="url mono"
      >{#if request.url}<VariableText
          text={request.url}
          known={variableNames.known(collectionId)}
        />{:else}No URL yet{/if}</span
    >
  </span>
  {#if lastResult}
    <span class="result">
      <span class="time mono">{formatDuration(lastResult.elapsedMs)}</span>
      <span class="chip mono {tone}">{lastResult.status}</span>
    </span>
  {:else if !SKIPPED_IN_RUNS.has(request.kind)}
    <span class="chip idle">Not run yet</span>
  {/if}
</button>

<style>
  .row {
    width: 100%;
    display: grid;
    grid-template-columns: 24px 64px minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    text-align: left;
  }

  .row.drop {
    box-shadow: 0 -3px 0 var(--accent);
  }

  .row:hover {
    border-color: var(--border-strong);
  }

  .step {
    font-size: 12px;
    color: var(--text-muted);
  }

  .result {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .time {
    font-size: 12px;
    color: var(--text-muted);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 500;
  }

  .url {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }

  .chip {
    flex: none;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: var(--surface-sunken);
    color: var(--text-2);
  }

  .chip.idle {
    color: var(--text-muted);
    font-weight: 500;
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.err {
    background: var(--err-bg);
    color: var(--err);
  }
</style>
