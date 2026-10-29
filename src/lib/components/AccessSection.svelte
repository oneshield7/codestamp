<script lang="ts">
  import { toAppError } from '../api/http';
  import {
    createAccessGroup,
    declineGroupAccess,
    grantGroupAccess,
    removeGroupMember,
    repairAccessGroup,
    requestGroupAccess,
    type AccessGroup,
    type AccessPerson,
    type RestrictedUse,
  } from '../api/team';
  import { accessGroups } from '../state/access.svelte';
  import type { TeamSync } from '../state/team.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';

  interface Props {
    team: TeamSync;
    collectionId: string;
  }

  let { team, collectionId }: Props = $props();

  const COPIED_MS = 1500;

  const overview = $derived(accessGroups.overviews.get(collectionId));
  let busy = $state(false);
  let error = $state<string | null>(null);
  let newName = $state('');
  let copied = $state(false);
  let removing = $state<{ group: AccessGroup; person: AccessPerson } | null>(null);
  let removed = $state<{ group: string; person: string; uses: RestrictedUse[] } | null>(null);

  $effect(() => {
    const id = collectionId;
    error = null;
    void accessGroups.load(id).catch((failure: unknown) => {
      error = toAppError(failure).message;
    });
  });

  /** Runs one change, then syncs so the team gets it right away. */
  async function act(work: () => Promise<unknown>): Promise<boolean> {
    busy = true;
    error = null;
    try {
      await work();
      await team.sync(collectionId);
      await accessGroups.load(collectionId);
      return true;
    } catch (failure) {
      error = toAppError(failure).message;
      return false;
    } finally {
      busy = false;
    }
  }

  async function create(): Promise<void> {
    const name = newName.trim();
    if (await act(() => createAccessGroup(collectionId, name))) {
      newName = '';
    }
  }

  async function remove(): Promise<void> {
    const target = removing;
    removing = null;
    if (!target) {
      return;
    }
    let uses: RestrictedUse[] = [];
    const done = await act(async () => {
      uses = await removeGroupMember(collectionId, target.group.id, target.person.fingerprint);
    });
    if (done && !target.person.thisDevice) {
      removed = { group: target.group.name, person: target.person.name, uses };
    }
  }

  async function copyFingerprint(fingerprint: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(fingerprint);
      copied = true;
      setTimeout(() => (copied = false), COPIED_MS);
    } catch {
      // The fingerprint stays selectable on screen; there is nothing else to fall back to.
      copied = false;
    }
  }

  function useLabel(use: RestrictedUse): string {
    return use.environment === null ? use.key : `${use.key} (${use.environment})`;
  }

  function badgeOf(group: AccessGroup): { tone: string; text: string } {
    if (group.problem !== null) {
      return { tone: 'warn', text: 'Locked, needs a check' };
    }
    return group.access
      ? { tone: 'ok', text: 'You are a member' }
      : { tone: 'none', text: 'No access' };
  }

  function removeMessage(target: { group: AccessGroup; person: AccessPerson }): string {
    const group = target.group.name;
    if (target.person.thisDevice) {
      return `This device stops receiving ${group} values after the next sync. A member can grant access again later.`;
    }
    return `${target.person.name} gets no new ${group} values after the next sync, and the group gets a new key. ${target.person.name} has already seen the current values, so change them where they are issued afterwards.`;
  }
</script>

<section class="access" aria-labelledby="access-title">
  <div>
    <h3 id="access-title">Access groups</h3>
    <p class="muted">
      A value restricted to a group travels encrypted, and only its members' devices can read it.
      Everyone else with access to the repository sees the variable's name only.
    </p>
  </div>

  {#if overview}
    {#if overview.fingerprint !== null}
      {@const fingerprint = overview.fingerprint}
      <dl class="facts">
        <dt>This device</dt>
        <dd>
          <span class="mono">{fingerprint}</span>
          <button type="button" class="link" onclick={() => void copyFingerprint(fingerprint)}>
            {copied ? 'Copied' : 'Copy'}
          </button>
        </dd>
      </dl>
    {/if}

    {#each overview.groups as group (group.id)}
      {@const badge = badgeOf(group)}
      {@const ownRequest = group.requests.find((request) => request.thisDevice)}
      <article class="group" aria-labelledby="group-{group.id}">
        <div class="group-head">
          <h4 id="group-{group.id}">{group.name}</h4>
          <span class="chip {badge.tone}">{badge.text}</span>
        </div>

        {#if group.problem !== null}
          <div class="problem" role="status">
            <p>
              A change to {group.name} could not be checked: {group.problem}. Its values stay locked
              on this device.
            </p>
            {#if group.canRepair}
              <p>Repair gives the members this device checked last a new key.</p>
              <button
                type="button"
                class="secondary"
                disabled={busy}
                onclick={() => void act(() => repairAccessGroup(collectionId, group.id))}
              >
                Repair group
              </button>
            {/if}
          </div>
        {/if}

        <p class="muted small">
          {group.variables.length > 0
            ? `Holds ${group.variables.map(useLabel).join(', ')}.`
            : `No variables yet. Pick “Restricted: ${group.name}” as a variable's type.`}
        </p>

        <ul class="people" aria-label="Members of {group.name}">
          {#each group.members as member (member.fingerprint)}
            <li>
              <span class="who">
                {member.name}{#if member.thisDevice}<span class="muted"> (this device)</span>{/if}
                <span class="muted email">{member.email}</span>
              </span>
              <span class="mono print">{member.fingerprint}</span>
              {#if group.access && group.problem === null && group.members.length > 1}
                <button
                  type="button"
                  class="quiet"
                  disabled={busy}
                  onclick={() => (removing = { group, person: member })}
                >
                  {member.thisDevice ? 'Leave' : 'Remove'}
                </button>
              {/if}
            </li>
          {/each}
        </ul>

        {#if group.requests.length > 0}
          <h5>Asking for access</h5>
          {#if group.access && group.requests.some((request) => !request.thisDevice)}
            <p class="muted small">
              Before granting, ask them to read the key shown under This device in their own
              Collaboration panel. Grant only when it matches the one here.
            </p>
          {/if}
          <ul class="people" aria-label="Asking for access to {group.name}">
            {#each group.requests as request (request.fingerprint)}
              <li>
                <span class="who">
                  {request.name}<span class="muted email">{request.email}</span>
                </span>
                <span class="mono print">{request.fingerprint}</span>
                {#if request.thisDevice}
                  <button
                    type="button"
                    class="quiet"
                    disabled={busy}
                    onclick={() =>
                      void act(() =>
                        declineGroupAccess(collectionId, group.id, request.fingerprint),
                      )}
                  >
                    Withdraw
                  </button>
                {:else if group.access && group.problem === null}
                  <span class="row-actions">
                    <button
                      type="button"
                      class="quiet"
                      disabled={busy}
                      onclick={() =>
                        void act(() =>
                          declineGroupAccess(collectionId, group.id, request.fingerprint),
                        )}
                    >
                      Decline
                    </button>
                    <button
                      type="button"
                      class="secondary"
                      disabled={busy}
                      onclick={() =>
                        void act(() =>
                          grantGroupAccess(collectionId, group.id, request.fingerprint),
                        )}
                    >
                      Grant
                    </button>
                  </span>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}

        {#if !group.access && group.problem === null}
          {#if ownRequest}
            <p class="muted small">
              Waiting for a member to grant access. They will ask you for the key under This device.
            </p>
          {:else}
            <div class="actions">
              <button
                type="button"
                class="secondary"
                disabled={busy}
                onclick={() => void act(() => requestGroupAccess(collectionId, group.id))}
              >
                Request access
              </button>
            </div>
          {/if}
        {/if}
      </article>
    {/each}

    {#if removed}
      <div class="done" role="status">
        <p>
          {removed.person} was removed from {removed.group}.
          {removed.uses.length > 0
            ? `Change ${removed.uses.map(useLabel).join(', ')} where they are issued, since ${removed.person} has seen them.`
            : ''}
        </p>
        <button type="button" class="link" onclick={() => (removed = null)}>Dismiss</button>
      </div>
    {/if}

    <form
      class="new-group"
      onsubmit={(event) => {
        event.preventDefault();
        void create();
      }}
    >
      <label class="field">
        <span>New group</span>
        <input bind:value={newName} placeholder="e.g. Production" maxlength="120" />
      </label>
      <button type="submit" class="secondary" disabled={busy || newName.trim() === ''}>
        Create group
      </button>
    </form>
  {:else if !error}
    <p class="muted">Loading access groups…</p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<ConfirmDialog
  open={removing !== null}
  title={removing?.person.thisDevice
    ? `Leave ${removing.group.name}?`
    : `Remove ${removing?.person.name ?? ''}?`}
  message={removing ? removeMessage(removing) : ''}
  confirmLabel={removing?.person.thisDevice ? 'Leave group' : 'Remove'}
  onConfirm={() => void remove()}
  onCancel={() => (removing = null)}
/>

<style>
  .access {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 14px;
    border-top: 1px solid var(--border-soft);
  }

  h3,
  h4,
  h5 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }

  h5 {
    color: var(--text-2);
  }

  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .muted.small {
    font-size: 12px;
  }

  h3 + .muted {
    margin-top: 4px;
  }

  .mono {
    font-family: var(--font-code);
    font-size: 12px;
  }

  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 16px;
    margin: 0;
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

  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
  }

  .group-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .chip {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.warn {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .chip.none {
    background: var(--surface-sunken);
    color: var(--text-2);
  }

  .people {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .people li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    padding: 8px 0;
    border-top: 1px solid var(--border-soft);
    font-size: 13px;
  }

  .people li:first-child {
    border-top: 0;
  }

  .who {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 8px;
    flex: 1 1 200px;
    min-width: 0;
  }

  .email {
    font-size: 12px;
  }

  .print {
    color: var(--text-2);
  }

  .row-actions {
    display: flex;
    gap: 8px;
  }

  .problem,
  .done {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }

  .problem {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .done {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .problem p,
  .done p {
    margin: 0;
    line-height: 1.5;
  }

  .new-group {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1 1 220px;
    font-size: 13px;
    color: var(--text-2);
  }

  .field input {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .actions {
    display: flex;
    gap: 10px;
  }

  .secondary,
  .quiet {
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
  }

  .quiet {
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
  }

  .secondary:disabled,
  .quiet:disabled {
    opacity: 0.6;
  }

  .link {
    margin-left: 6px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
  }

  .done .link {
    margin: 0;
    color: inherit;
    text-decoration: underline;
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }
</style>
