<script lang="ts">
  import type { McpLaunch } from '../api/mcp';

  interface Props {
    launch: McpLaunch;
    onApprove: () => void;
    onCancel: () => void;
  }

  let { launch, onApprove, onCancel }: Props = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="mcp-approval-title"
  oncancel={(event) => {
    event.preventDefault();
    onCancel();
  }}
>
  <h2 id="mcp-approval-title">Start this program?</h2>
  <p class="lead">
    The MCP server runs as a program on this computer, with your permissions. Start it only if you
    trust where this request came from.
  </p>
  <dl>
    <dt>Program</dt>
    <dd class="mono">{launch.program}</dd>
    {#if launch.args.length > 0}
      <dt>Arguments</dt>
      <dd>
        <ol class="args mono">
          {#each launch.args as arg, index (index)}
            <li>{arg === '' ? '""' : arg}</li>
          {/each}
        </ol>
      </dd>
    {/if}
    {#if launch.env.length > 0}
      <dt>Environment</dt>
      <dd class="mono">{launch.env.join(', ')}</dd>
    {/if}
  </dl>
  <p class="note">
    It is started directly, without a shell. Codestamp asks again whenever the program, its
    arguments or its environment change, also after a collaboration sync.
  </p>
  <div class="actions">
    <button type="button" class="cancel" onclick={onCancel}>Cancel</button>
    <button type="button" class="primary" onclick={onApprove}>Start program</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(520px, calc(100vw - 32px));
    padding: 22px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  h2 {
    margin: 0 0 6px;
    font-size: 18px;
    font-weight: 600;
  }

  .lead,
  .note {
    margin: 0;
    font-size: 13px;
    color: var(--text-2);
  }

  .note {
    color: var(--text-muted);
  }

  dl {
    margin: 16px 0;
    padding: 12px 14px;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    font-size: 13px;
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .args {
    margin: 0;
    padding-left: 18px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 18px;
  }

  .cancel,
  .primary {
    height: 36px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }

  .cancel {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
</style>
