<script lang="ts">
  import NavIcon from './NavIcon.svelte';
  import { exportEnvironment, type ExportResult } from '../api/export';
  import { dialogIsOpen, isSaveShortcut } from '../dom/keys';
  import type { EnvironmentSummary } from '../api/environments';
  import { accessGroups } from '../state/access.svelte';
  import type { Environments } from '../state/environments.svelte';
  import { notices } from '../state/notices.svelte';
  import type { TeamSync } from '../state/team.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import Dropdown from './Dropdown.svelte';
  import EditableText from './EditableText.svelte';
  import VariablesTable from './VariablesTable.svelte';

  interface Props {
    environments: Environments;
    workspace: Workspace;
    team: TeamSync;
  }

  let { environments, workspace, team }: Props = $props();

  const THIS_DEVICE = '';

  /** This device's environments first, then each collection's own, under its name. */
  const groups = $derived.by(() => {
    const local = environments.list.filter((env) => env.collectionId === null);
    const owned = workspace.collections
      .map((collection) => ({
        id: collection.id,
        title: `${collection.icon} ${collection.name}`,
        items: environments.list.filter((env) => env.collectionId === collection.id),
      }))
      .filter((group) => group.items.length > 0);
    return [{ id: THIS_DEVICE, title: 'On this device', items: local }, ...owned];
  });
  const owner = $derived(
    workspace.collections.find((collection) => collection.id === selected?.collectionId) ?? null,
  );
  const sharedOwner = $derived(owner !== null && team.links.has(owner.id) ? owner.id : null);

  $effect(() => {
    if (sharedOwner !== null) {
      const id = sharedOwner;
      void notices.guard(() => accessGroups.load(id));
    }
  });
  const ownerOptions = $derived([
    { value: THIS_DEVICE, label: 'This device only' },
    ...workspace.collections.map((collection) => ({
      value: collection.id,
      label: `${collection.icon} ${collection.name}`,
    })),
  ]);

  function meta(env: EnvironmentSummary): string {
    return env.id === environments.activeId ? 'Active' : String(env.variableCount);
  }

  type Pending = { kind: 'delete' } | { kind: 'switch'; id: string };
  let pending: Pending | null = $state(null);

  const selected = $derived(environments.selected);
  const isActive = $derived(selected !== null && selected.id === environments.activeId);
  let exported: (ExportResult & { environmentId: string }) | null = $state(null);

  async function exportSelected(id: string): Promise<void> {
    await notices.guard(async () => {
      const result = await exportEnvironment(id);
      exported = result ? { ...result, environmentId: id } : null;
    });
  }

  function requestSwitch(id: string): void {
    if (id === environments.selectedId) {
      return;
    }
    if (environments.editor.dirty) {
      pending = { kind: 'switch', id };
      return;
    }
    void environments.select(id);
  }

  function saveShortcut(event: KeyboardEvent): void {
    if (!isSaveShortcut(event) || dialogIsOpen()) {
      return;
    }
    event.preventDefault();
    if (environments.editor.dirty && !environments.editor.saving) {
      void environments.save();
    }
  }

  async function confirmPending(): Promise<void> {
    const action = pending;
    pending = null;
    if (action?.kind === 'delete') {
      await environments.remove();
    } else if (action) {
      await environments.select(action.id);
    }
  }
</script>

<svelte:window onkeydown={saveShortcut} />

<div class="view">
  <header>
    <h1 class="page-title"><NavIcon name="environments" size="lg" />Environments</h1>
    <p>
      Use <span class="mono">{'{{name}}'}</span> in URLs, params, headers and bodies. The active environment
      overrides Globals and collection variables that have the same name.
    </p>
  </header>

  <div class="layout">
    <nav class="list" aria-label="Environments">
      {#each groups as group (group.id)}
        <h2 class="group-title">{group.title}</h2>
        {#each group.items as env (env.id)}
          <button
            type="button"
            class="env"
            class:on={env.id === environments.selectedId}
            aria-current={env.id === environments.selectedId ? 'true' : undefined}
            onclick={() => {
              requestSwitch(env.id);
            }}
          >
            <span
              class="dot"
              class:global={env.isGlobal}
              class:active={env.id === environments.activeId}
            ></span>
            <span class="name">{env.name}</span>
            <span class="meta">{meta(env)}</span>
          </button>
        {/each}
      {/each}
      <button type="button" class="env add" onclick={() => void environments.create()}>
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 5v14M5 12h14" />
        </svg>
        New environment
      </button>
    </nav>

    {#if selected}
      <section class="panel" aria-label="{selected.name} variables">
        <div class="panel-head">
          {#if selected.isGlobal}
            <h2>Globals</h2>
            <span class="hint">Available in every environment</span>
          {:else}
            <EditableText
              value={selected.name}
              label="Environment name"
              onCommit={(name: string) => void environments.rename(name)}
            />
            {#if isActive}
              <span class="badge">Active</span>
              <button type="button" class="quiet" onclick={() => void environments.setActive(null)}>
                Deactivate
              </button>
            {:else}
              <button
                type="button"
                class="quiet"
                onclick={() => void environments.setActive(selected.id)}
              >
                Set active
              </button>
            {/if}
          {/if}
          <span class="spacer"></span>
          <button
            type="button"
            class="quiet"
            disabled={environments.editor.dirty}
            title={environments.editor.dirty
              ? 'Save your changes first'
              : 'Save as an environment file'}
            onclick={() => void exportSelected(selected.id)}
          >
            Export
          </button>
          {#if !selected.isGlobal}
            <button
              type="button"
              class="quiet danger"
              onclick={() => (pending = { kind: 'delete' })}
            >
              Delete
            </button>
          {/if}
          <button
            type="button"
            class="primary"
            disabled={!environments.editor.dirty || environments.editor.saving}
            onclick={() => void environments.save()}
          >
            {environments.editor.saving ? 'Saving…' : environments.editor.dirty ? 'Save' : 'Saved'}
          </button>
        </div>

        {#if !selected.isGlobal}
          <div class="owner">
            <span class="owner-label">Belongs to</span>
            <div class="owner-pick">
              <Dropdown
                variant="field"
                label="Belongs to"
                value={selected.collectionId ?? THIS_DEVICE}
                options={ownerOptions}
                onChange={(id: string) =>
                  void environments.setCollection(id === THIS_DEVICE ? null : id)}
              />
            </div>
            <p class="owner-hint">
              {#if owner === null}
                Kept on this device only.
              {:else if team.links.has(owner.id)}
                Shared with everyone in “{owner.name}” through its Git repository when you press
                Sync now. Secret values stay on each device; restricted ones reach only their group.
              {:else}
                Goes with “{owner.name}” when you share it from Collaboration.
              {/if}
            </p>
          </div>
        {/if}

        {#if exported && exported.environmentId === selected.id}
          <div class="exported" role="status">
            <div>
              <p>Exported to <span class="mono">{exported.path}</span></p>
              {#each exported.notes as note, index (index)}
                <p class="muted">{note}</p>
              {/each}
            </div>
            <button type="button" class="dismiss" onclick={() => (exported = null)}>Dismiss</button>
          </div>
        {/if}

        <VariablesTable editor={environments.editor} groups={accessGroups.groups(sharedOwner)} />

        <p class="note">
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" />
          </svg>
          Secret values are encrypted on this device. The key lives in Windows Credential Manager, and
          secrets are never shown unless you choose to reveal them.
        </p>
      </section>
    {/if}
  </div>
</div>

<ConfirmDialog
  open={pending !== null}
  title={pending?.kind === 'delete' ? 'Delete environment?' : 'Discard changes?'}
  message={pending?.kind === 'delete'
    ? `“${selected?.name ?? ''}” and its variables will be deleted. This cannot be undone.`
    : 'You have unsaved variable changes. Switching environments will discard them.'}
  confirmLabel={pending?.kind === 'delete' ? 'Delete' : 'Discard'}
  onConfirm={() => void confirmPending()}
  onCancel={() => (pending = null)}
/>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  header p {
    margin: 4px 0 0;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .layout {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    align-items: flex-start;
  }

  .list {
    flex: 1 1 200px;
    max-width: 260px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .group-title {
    margin: 10px 12px 2px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .group-title:first-child {
    margin-top: 0;
  }

  .owner {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-soft);
    font-size: 13px;
  }

  .owner-label {
    color: var(--text-2);
  }

  .owner-pick {
    flex: 0 1 280px;
    min-width: 200px;
  }

  .owner-hint {
    flex: 1 1 240px;
    margin: 0;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .env {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: transparent;
    text-align: left;
  }

  .env:hover {
    background: var(--surface-sunken);
  }

  .env.on {
    border-color: var(--border);
    background: var(--surface);
    font-weight: 500;
  }

  .env.add {
    color: var(--accent-text);
  }

  .env.add svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border-strong);
  }

  .dot.global {
    background: var(--dot-off);
  }

  .dot.active {
    background: var(--dot-ok);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .panel {
    flex: 4 1 540px;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .panel-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .panel-head :global(.editable) {
    max-width: 280px;
    font-size: 17px;
  }

  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .hint {
    font-size: 13px;
    color: var(--text-muted);
  }

  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 12px;
    font-weight: 600;
  }

  .spacer {
    flex: 1;
  }

  .quiet {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .quiet.danger:hover {
    border-color: var(--err);
    color: var(--err);
  }

  .quiet:disabled {
    color: var(--text-muted);
  }

  .exported {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border-soft);
    background: var(--ok-bg);
    color: var(--ok);
    font-size: 13px;
  }

  .exported div {
    flex: 1;
    min-width: 0;
  }

  .exported p {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .exported .muted {
    margin-top: 4px;
    color: var(--text-2);
  }

  .dismiss {
    border: 0;
    background: transparent;
    color: var(--ok);
    font-size: 13px;
    font-weight: 600;
  }

  .primary {
    height: 32px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
  }

  .primary:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .note {
    display: flex;
    gap: 10px;
    margin: 0;
    padding: 12px 16px;
    background: var(--surface-muted);
    color: var(--text-2);
    font-size: 13px;
    line-height: 1.5;
  }

  .note svg {
    flex: none;
    margin-top: 2px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
