<script lang="ts">
  import { tick } from 'svelte';
  import { formatClock } from '../format/time';
  import { formatBytes } from '../format/units';
  import { maskAny } from '../privacy/secrets';
  import { LOG_MARKS, STATUS_LABELS } from '../request/websocket';
  import type { LogEntry, WebSocketSession } from '../state/websocket.svelte';

  interface Props {
    session: WebSocketSession;
    /** Hides tokens in JSON messages on screen (focus mode). */
    maskSecrets?: boolean;
  }

  let { session, maskSecrets = false }: Props = $props();

  function shownText(entry: LogEntry): string {
    return maskSecrets ? maskAny(entry.text, 0).text : entry.text;
  }

  type Filter = 'all' | 'sent' | 'received';
  const FILTERS: { filter: Filter; label: string }[] = [
    { filter: 'all', label: 'All' },
    { filter: 'sent', label: 'Sent' },
    { filter: 'received', label: 'Received' },
  ];
  // Within this distance of the bottom, new messages keep the list scrolled down.
  const STICK_THRESHOLD_PX = 40;

  let filter: Filter = $state('all');
  let list: HTMLOListElement | undefined = $state();

  const entries = $derived(
    filter === 'all' ? session.log : session.log.filter((entry) => entry.kind === filter),
  );

  function countLabel(target: Filter): string {
    if (target === 'all') {
      return '';
    }
    return ` · ${String(session.log.filter((entry) => entry.kind === target).length)}`;
  }

  $effect.pre(() => {
    const count = entries.length;
    const element = list;
    if (!element || count === 0) {
      return;
    }
    const nearBottom =
      element.scrollHeight - element.scrollTop - element.clientHeight < STICK_THRESHOLD_PX;
    if (nearBottom) {
      void tick().then(() => {
        element.scrollTop = element.scrollHeight;
      });
    }
  });
</script>

<section class="response" aria-label="Messages">
  <div class="summary">
    <span class="label">Messages</span>
    <span
      class="chip"
      class:ok={session.status === 'open'}
      class:busy={session.status === 'connecting' || session.status === 'closing'}
      role="status"
    >
      {STATUS_LABELS.get(session.status)}
    </span>
    {#if session.protocol}
      <span class="muted mono">{session.protocol}</span>
    {/if}
    <span class="spacer"></span>
    <div class="segmented" role="group" aria-label="Show messages">
      {#each FILTERS as option (option.filter)}
        <button
          type="button"
          class:on={filter === option.filter}
          aria-pressed={filter === option.filter}
          onclick={() => (filter = option.filter)}
        >
          {option.label}{countLabel(option.filter)}
        </button>
      {/each}
    </div>
    <button
      type="button"
      class="ghost"
      disabled={session.log.length === 0}
      onclick={() => {
        session.clear();
      }}>Clear</button
    >
  </div>

  {#if entries.length === 0}
    <p class="empty">
      {session.log.length === 0
        ? 'Press Connect, then send a message. Everything sent and received shows up here.'
        : 'Nothing matches this filter.'}
    </p>
  {:else}
    <ol class="log" bind:this={list} aria-live="polite">
      {#each entries as entry (entry.id)}
        <li class={entry.kind}>
          <span class="mark" aria-label={entry.kind}>{LOG_MARKS.get(entry.kind)}</span>
          <time class="time mono" datetime={new Date(entry.at).toISOString()}>
            {formatClock(entry.at, true)}
          </time>
          <div class="body">
            {#if entry.binary}
              <span class="muted">Binary message</span>
            {:else}
              <pre class="text mono">{shownText(entry)}</pre>
            {/if}
            {#if entry.truncated}
              <span class="note">Only the first 64 KB is shown.</span>
            {/if}
          </div>
          {#if entry.size !== null}
            <span class="size">{formatBytes(entry.size)}</span>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .response {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface-muted);
  }

  .summary {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
  }

  .label {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
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

  .chip.busy {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .muted {
    font-size: 13px;
    color: var(--text-muted);
  }

  .spacer {
    flex: 1;
  }

  .segmented {
    display: inline-flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .segmented button {
    padding: 4px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 12px;
  }

  .segmented button.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 500;
    box-shadow: var(--shadow-segment);
  }

  .ghost {
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 12px;
  }

  .empty {
    margin: 0;
    padding: 4px 16px 18px;
    color: var(--text-muted);
    font-size: 13px;
  }

  /* The log scrolls inside this box; drag the corner to resize it. */
  .log {
    height: 320px;
    min-height: 120px;
    max-height: 80vh;
    resize: vertical;
    overflow: auto;
    margin: 0;
    padding: 0 16px 12px;
    list-style: none;
  }

  .log li {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border-soft);
  }

  .mark {
    flex: none;
    width: 14px;
    font-weight: 700;
    text-align: center;
  }

  .sent .mark {
    color: var(--accent-text);
  }

  .received .mark {
    color: var(--ok);
  }

  .info .mark,
  .info .text {
    color: var(--text-muted);
  }

  .error .mark,
  .error .text {
    color: var(--err);
  }

  .time {
    flex: none;
    color: var(--text-muted);
    font-size: 12px;
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .text {
    margin: 0;
    max-height: 200px;
    overflow: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .note {
    font-size: 12px;
    color: var(--warn);
  }

  .size {
    flex: none;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
