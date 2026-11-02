<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../../api/http';
  import type { Workspace } from '../../api/workspaces';
  import { plural } from '../../format/plural';
  import { workspaces } from '../../state/workspaces.svelte';
  import ConfirmDialog from '../ConfirmDialog.svelte';
  import WorkspaceDialog from '../WorkspaceDialog.svelte';
  import WorkspaceTile from '../WorkspaceTile.svelte';

  // `undefined`: closed; `null`: creating a new one.
  let editing = $state<Workspace | null | undefined>(undefined);
  let deleting = $state<Workspace | null>(null);
  let error = $state<string | null>(null);

  onMount(() => {
    void workspaces.load();
  });

  function contents(workspace: Workspace): string {
    const parts = [
      workspace.collectionCount > 0 ? plural(workspace.collectionCount, 'collection') : null,
      workspace.environmentCount > 0 ? plural(workspace.environmentCount, 'environment') : null,
      workspace.flowCount > 0 ? plural(workspace.flowCount, 'flow') : null,
    ].filter((part) => part !== null);
    return parts.length > 0 ? parts.join(' · ') : 'Empty';
  }

  function deletable(workspace: Workspace): boolean {
    return (
      workspaces.list.length > 1 && workspace.collectionCount === 0 && workspace.flowCount === 0
    );
  }

  async function remove(): Promise<void> {
    const workspace = deleting;
    deleting = null;
    if (!workspace) {
      return;
    }
    error = null;
    try {
      await workspaces.remove(workspace.id);
    } catch (caught) {
      error = toAppError(caught).message;
    }
  }
</script>

<section class="panel" aria-labelledby="workspaces-title">
  <div class="title-row">
    <div class="titles">
      <h2 id="workspaces-title">Workspaces</h2>
      <p class="hint">
        Each workspace keeps its own collections, environments, Globals, flows, cookies and Recently
        deleted. Settings, Git accounts and certificates are shared.
      </p>
    </div>
    <button
      type="button"
      class="button"
      aria-haspopup="dialog"
      aria-expanded={editing === null}
      onclick={() => (editing = null)}
    >
      New workspace
    </button>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <ul class="list" aria-label="Workspaces">
    {#each workspaces.list as workspace (workspace.id)}
      {@const isOpen = workspace.id === workspaces.currentId}
      <li class="row">
        <WorkspaceTile name={workspace.name} hue={workspace.hue} icon={workspace.icon} size="lg" />
        <div class="about">
          <span class="name">
            {workspace.name}
            {#if isOpen}
              <span class="chip">Open</span>
            {/if}
          </span>
          <span class="muted">{contents(workspace)}</span>
        </div>
        <div class="row-actions">
          {#if !isOpen}
            <button
              type="button"
              class="button"
              aria-label="Open {workspace.name}"
              onclick={() => void workspaces.open(workspace.id)}
            >
              Open
            </button>
          {/if}
          <button
            type="button"
            class="button"
            aria-label="Edit {workspace.name}"
            aria-haspopup="dialog"
            aria-expanded={editing?.id === workspace.id}
            onclick={() => (editing = workspace)}
          >
            Edit
          </button>
          <button
            type="button"
            class="button danger"
            aria-label="Delete {workspace.name}"
            aria-haspopup="dialog"
            aria-expanded={deleting?.id === workspace.id}
            disabled={!deletable(workspace)}
            onclick={() => (deleting = workspace)}
          >
            Delete
          </button>
        </div>
      </li>
    {/each}
  </ul>

  <p class="muted">
    Only an empty workspace can be deleted, and Codestamp always keeps one. To empty a workspace,
    move its collections to another one with Move to workspace in the collection header, and delete
    its flows in Orchestration.
  </p>
</section>

{#if editing !== undefined}
  <WorkspaceDialog {editing} onClose={() => (editing = undefined)} />
{/if}

<ConfirmDialog
  open={deleting !== null}
  title="Delete “{deleting?.name ?? ''}”?"
  message="Its environments, Globals, cookies and Recently deleted items are deleted with it. This cannot be undone."
  confirmLabel="Delete workspace"
  onConfirm={() => void remove()}
  onCancel={() => (deleting = null)}
/>

<style>
  .title-row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 12px;
  }

  .titles {
    flex: 1 1 280px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    container-type: inline-size;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 12px 0;
    border-top: 1px solid var(--border-soft);
  }

  .about {
    flex: 1 1 200px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .chip {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 12px;
    font-weight: 600;
  }

  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .row-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .danger:hover:not(:disabled) {
    border-color: var(--err);
    color: var(--err);
  }
</style>
