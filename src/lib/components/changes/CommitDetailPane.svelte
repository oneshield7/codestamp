<script lang="ts">
  import type { Change, CommitDetail, FileChange } from '../../api/team';
  import { formatDateTime } from '../../format/time';
  import { plural } from '../../format/plural';
  import FieldChanges from '../FieldChanges.svelte';

  interface Props {
    detail: CommitDetail;
    /** Requests that still exist, so only those offer Open. */
    present: ReadonlySet<string>;
    onOpenRequest: (requestId: string) => void;
  }

  let { detail, present, onOpenRequest }: Props = $props();

  const SECONDS_TO_MS = 1000;
  const SHORT_HASH = 7;
  const COPIED_MS = 1500;
  const CHANGE_LABELS = new Map<Change, string>([
    ['added', 'Added'],
    ['updated', 'Updated'],
    ['removed', 'Removed'],
  ]);

  const subject = $derived(detail.message.split('\n', 1)[0] ?? '');
  const body = $derived(detail.message.slice(subject.length).trim());
  let copied = $state(false);

  function fileTitle(file: FileChange): string {
    switch (file.kind) {
      case 'collection':
        return 'Collection settings';
      case 'environment':
        return `Environment “${file.name}”`;
      case 'accessGroup':
        return `Access group “${file.name}”`;
      case 'accessRequest':
        return `Request for access from ${file.name}`;
      case 'request':
        return file.name || '(unnamed request)';
    }
  }

  async function copyHash(): Promise<void> {
    try {
      await navigator.clipboard.writeText(detail.commit);
      copied = true;
      setTimeout(() => (copied = false), COPIED_MS);
    } catch {
      // The hash stays selectable on screen; there is nothing else to fall back to.
      copied = false;
    }
  }
</script>

<section class="detail" aria-label="Commit {detail.commit.slice(0, SHORT_HASH)}">
  <h3>{subject}</h3>
  {#if body}
    <p class="body">{body}</p>
  {/if}
  <p class="meta">
    {detail.author}
    <span class="muted">&lt;{detail.email}&gt;</span>
    · {formatDateTime(detail.time * SECONDS_TO_MS)}
    · <span class="mono">{detail.commit.slice(0, SHORT_HASH)}</span>
    <button type="button" class="link" onclick={() => void copyHash()}>
      {copied ? 'Copied' : 'Copy hash'}
    </button>
  </p>
  {#if detail.parents.length > 1}
    <p class="muted">A merge: shown against the first line of work it joined.</p>
  {/if}

  {#if detail.files.length === 0}
    <p class="muted">No request or collection changes in this commit.</p>
  {:else}
    <ul class="files" aria-label="Changes in this commit">
      {#each detail.files as file, index (index)}
        <li class="file">
          <div class="file-head">
            <span class="chip {file.change}">{CHANGE_LABELS.get(file.change)}</span>
            <span class="name">
              {fileTitle(file)}
            </span>
            {#if file.requestId !== null && present.has(file.requestId)}
              {@const id = file.requestId}
              <button
                type="button"
                class="link"
                onclick={() => {
                  onOpenRequest(id);
                }}>Open</button
              >
            {/if}
          </div>
          {#if file.fields.length > 0}
            <FieldChanges changes={file.fields} thenLabel="Before" nowLabel="After" />
          {/if}
        </li>
      {/each}
    </ul>
    {#if detail.hidden > 0}
      <p class="muted">
        {plural(detail.hidden, 'more file')} changed in this commit.
      </p>
    {/if}
  {/if}
</section>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }

  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .body {
    margin: 0;
    font-size: 13px;
    color: var(--text-2);
    white-space: pre-wrap;
  }

  .meta {
    margin: 0;
    font-size: 13px;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }

  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .files {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }

  .file {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border-soft);
  }

  .file-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .name {
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .chip {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 600;
  }

  .chip.added {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.updated {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .chip.removed {
    background: var(--err-bg);
    color: var(--err);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
  }
</style>
