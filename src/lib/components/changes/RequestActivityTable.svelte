<script lang="ts">
  import type { RequestActivity } from '../../api/team';
  import { formatSince } from '../../format/time';
  import { stampLabel } from '../../request/kinds';
  import { stampTone } from '../../request/methods';
  import MethodStamp from '../MethodStamp.svelte';

  interface Props {
    activity: RequestActivity[];
    onHistory: (request: RequestActivity) => void;
    onOpen: (requestId: string) => void;
  }

  let { activity, onHistory, onOpen }: Props = $props();

  const SECONDS_TO_MS = 1000;

  /** Most recently changed first; requests never committed yet go last, in collection order. */
  const sorted = $derived(activity.toSorted((a, b) => (b.lastTime ?? -1) - (a.lastTime ?? -1)));
</script>

{#if activity.length === 0}
  <p class="muted">This collection has no requests yet.</p>
{:else}
  <div class="wrap">
    <table>
      <thead>
        <tr>
          <th scope="col">Request</th>
          <th scope="col">Last change</th>
          <th scope="col" class="num">Versions</th>
          <th scope="col"><span class="visually-hidden">Actions</span></th>
        </tr>
      </thead>
      <tbody>
        {#each sorted as request (request.id)}
          <tr>
            <td>
              <span class="request">
                <MethodStamp
                  label={stampLabel(request.kind, request.method)}
                  tone={stampTone(request.kind, request.method)}
                  size="sm"
                />
                <span class="name">{request.name}</span>
              </span>
            </td>
            <td class="muted">
              {request.lastAuthor && request.lastTime !== null
                ? `${request.lastAuthor} · ${formatSince(request.lastTime * SECONDS_TO_MS)}`
                : 'Not synced yet'}
            </td>
            <td class="num">{request.versions}</td>
            <td>
              <span class="actions">
                <button
                  type="button"
                  class="link"
                  disabled={request.versions === 0}
                  aria-label="History of {request.name}"
                  onclick={() => {
                    onHistory(request);
                  }}>History</button
                >
                <button
                  type="button"
                  class="link"
                  aria-label="Open {request.name}"
                  onclick={() => {
                    onOpen(request.id);
                  }}>Open</button
                >
              </span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  td {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-soft);
    vertical-align: middle;
  }

  tbody tr:hover {
    background: var(--surface-sunken);
  }

  .request {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .name {
    overflow-wrap: anywhere;
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .muted {
    color: var(--text-muted);
  }

  p.muted {
    margin: 0;
  }

  .actions {
    display: inline-flex;
    gap: 12px;
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
  }

  .link:disabled {
    color: var(--text-muted);
  }
</style>
