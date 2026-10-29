<script lang="ts">
  import { untrack } from 'svelte';
  import { toAppError } from '../api/http';
  import { previewShare, type SharePreview, type TeamStatus } from '../api/team';
  import { plural } from '../format/plural';
  import { notices } from '../state/notices.svelte';
  import type { TeamSync } from '../state/team.svelte';
  import { accountFor, usesSignIn } from '../team/account';
  import { describeChanges, summarizeSync } from '../team/status';
  import AccessSection from './AccessSection.svelte';
  import AccountFields from './AccountFields.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import PanelDialog from './PanelDialog.svelte';

  interface Props {
    team: TeamSync;
    collectionId: string;
    collectionName: string;
    onResolve: () => void;
    onClose: () => void;
  }

  let { team, collectionId, collectionName, onResolve, onClose }: Props = $props();

  const COPIED_MS = 1500;

  const link = $derived(team.links.get(collectionId));
  const outcome = $derived(team.outcomes.get(collectionId));
  const syncing = $derived(team.syncing.has(collectionId));
  const signingIn = $derived(team.signingIn.has(collectionId));
  const summary = $derived(
    link ? summarizeSync(link, outcome, syncing, team.unsent.has(collectionId)) : null,
  );
  const incoming = $derived.by(() => {
    const report = outcome?.report;
    return report && report.authors.length > 0 ? report : null;
  });

  let remote = $state('');
  let username = $state('');
  let name = $state('');
  let email = $state('');
  let busy = $state(false);
  let checking = $state(false);
  /** A repository with files, shown for review before sharing replaces them. */
  let review = $state<SharePreview | null>(null);
  let error = $state<string | null>(null);
  let editingAccount = $state(false);
  let confirmUnlink = $state(false);
  let copied = $state(false);

  // The saved profile (or Git's own user.name / user.email) fills the form once it is known.
  $effect(() => {
    const author = team.setup?.author;
    if (author && untrack(() => name === '' && email === '')) {
      name = author.name;
      email = author.email;
    }
  });

  async function attempt(task: () => Promise<void>): Promise<boolean> {
    busy = true;
    error = null;
    try {
      await task();
      return true;
    } catch (caught) {
      error = toAppError(caught).message;
      return false;
    } finally {
      busy = false;
    }
  }

  async function signIn(): Promise<void> {
    await attempt(async () => {
      const check = await team.signIn(collectionId);
      if (check.status !== 'signedIn') {
        error = check.message ?? 'Git could not sign in.';
      }
    });
  }

  async function share(): Promise<void> {
    const account = accountFor(remote, name, email, username);
    checking = true;
    await attempt(async () => {
      const preview = await previewShare(remote, account.username);
      checking = false;
      if (preview.empty) {
        await team.share(collectionId, remote, account, null);
      } else {
        review = preview;
      }
    });
    checking = false;
  }

  async function replaceAndShare(): Promise<void> {
    const head = review?.head;
    if (!head) {
      return;
    }
    const account = accountFor(remote, name, email, username);
    if (await attempt(() => team.share(collectionId, remote, account, head))) {
      review = null;
    }
  }

  function editAccount(shared: TeamStatus): void {
    username = shared.username ?? '';
    name = shared.author?.name ?? '';
    email = shared.author?.email ?? '';
    error = null;
    editingAccount = true;
  }

  async function saveAccount(shared: TeamStatus): Promise<void> {
    const account = accountFor(shared.remote, name, email, username);
    if (await attempt(() => team.setAccount(collectionId, account))) {
      editingAccount = false;
    }
  }

  async function unlink(): Promise<void> {
    confirmUnlink = false;
    await notices.guard(() => team.unlink(collectionId));
  }

  async function copyRemote(address: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(address);
      copied = true;
      setTimeout(() => (copied = false), COPIED_MS);
    } catch {
      error = 'Could not copy here. Select the address and copy it instead.';
    }
  }
</script>

<PanelDialog title="Collaboration" {onClose}>
  <section class="panel" aria-label="Collaboration">
    <div class="head">
      <div class="title">
        <h2>Collaboration</h2>
        <p>
          {link
            ? 'Shared through a Git repository. Sync now sends your changes and brings in theirs.'
            : 'Work on this collection together through a Git repository you control.'}
        </p>
      </div>
      {#if link}
        <button
          type="button"
          class="primary"
          disabled={syncing}
          onclick={() => void team.sync(collectionId)}
        >
          {syncing ? 'Syncing…' : 'Sync now'}
        </button>
      {/if}
      <button type="button" class="quiet" onclick={onClose}>Close</button>
    </div>

    <div class="body">
      {#if !team.setup}
        <p class="muted">Checking Git…</p>
      {:else if team.setup.gitVersion === null}
        <div class="notice">
          <p>
            Git is not installed on this device. Codestamp syncs through Git and leaves signing in
            to it, so no password or token is stored here.
          </p>
          <p>Install Git for Windows from git-scm.com, then check again.</p>
          <button
            type="button"
            class="secondary"
            onclick={() => void notices.guard(() => team.load())}
          >
            Check again
          </button>
        </div>
      {:else if link && summary}
        <div class="status {summary.tone}" role="status">
          <span class="dot" aria-hidden="true"></span>
          <span class="status-text">{summary.detail}</span>
          {#if link.conflicts.length > 0}
            <button type="button" class="secondary" onclick={onResolve}>Resolve</button>
          {:else if outcome?.report?.status === 'signIn' && usesSignIn(link.remote)}
            <button
              type="button"
              class="secondary"
              disabled={signingIn || syncing}
              onclick={() => void signIn()}
            >
              {signingIn ? 'Signing in…' : 'Sign in'}
            </button>
          {/if}
        </div>
        {#if incoming}
          <p class="muted">
            Last update from {incoming.authors.join(', ')}{incoming.changed.length > 0
              ? ` · ${describeChanges(incoming.changed)}`
              : ''}.
          </p>
        {/if}

        <dl class="facts">
          <dt>Repository</dt>
          <dd>
            <span class="mono">{link.remote}</span>
            <button type="button" class="link" onclick={() => void copyRemote(link.remote)}>
              {copied ? 'Copied' : 'Copy'}
            </button>
          </dd>
          <dt>Branch</dt>
          <dd class="mono">{link.branch}</dd>
          {#if !editingAccount}
            {#if usesSignIn(link.remote)}
              <dt>Signs in as</dt>
              <dd>
                {#if link.username}
                  <span class="mono">{link.username}</span>
                {:else}
                  <span class="muted">Git's default account for this host</span>
                {/if}
              </dd>
            {/if}
            <dt>Your changes as</dt>
            <dd>
              {link.author?.name ?? 'Not set'}
              {#if link.author}<span class="muted">· {link.author.email}</span>{/if}
            </dd>
          {/if}
        </dl>
        {#if editingAccount}
          <div class="account-form" role="group" aria-labelledby="account-title">
            <h3 id="account-title">Account for “{collectionName}”</h3>
            <AccountFields remote={link.remote} bind:username bind:name bind:email />
            <div class="actions">
              <button type="button" class="secondary" onclick={() => (editingAccount = false)}>
                Cancel
              </button>
              <button
                type="button"
                class="primary"
                disabled={busy || name.trim() === '' || email.trim() === ''}
                onclick={() => void saveAccount(link)}
              >
                Save
              </button>
            </div>
          </div>
        {:else}
          <div class="actions start">
            <button
              type="button"
              class="secondary"
              onclick={() => {
                editAccount(link);
              }}
            >
              Change account
            </button>
          </div>
        {/if}

        {#if error}
          <p class="error" role="alert">{error}</p>
        {/if}

        <p class="muted">
          To invite someone, add them to the repository on your Git host. They choose Import › Join
          a shared collection and paste the repository address. People with read-only access receive
          changes but cannot send them.
        </p>
        <AccessSection {team} {collectionId} />
        <div class="actions">
          <button type="button" class="quiet danger" onclick={() => (confirmUnlink = true)}>
            Stop syncing
          </button>
        </div>
      {:else}
        <ol class="steps">
          <li>
            Create a repository on GitHub, GitLab, Bitbucket or Azure DevOps, or a bare repository
            in a shared folder. One that already has files works too: Codestamp lists them and asks
            before replacing them.
          </li>
          <li>Paste its address here. Git asks you to sign in the first time.</li>
          <li>Add collaborators to the repository. They join from Import.</li>
        </ol>
        <label class="field">
          Repository address
          <input
            bind:value={remote}
            class="mono"
            placeholder="https://github.com/acme/shop-api.git"
            spellcheck="false"
            autocomplete="off"
            oninput={() => (review = null)}
          />
        </label>
        <AccountFields {remote} bind:username bind:name bind:email />
        <p class="muted">
          Shared: requests, sections, collection auth and collection variables. Kept on this device:
          secret values, cookies, history and the files picked for uploads. Once shared, values can
          be restricted to an access group and travel encrypted for its members.
        </p>
        {#if error}
          <p class="error" role="alert">{error}</p>
        {/if}
        {#if review}
          <div class="replace" role="group" aria-labelledby="replace-title">
            <h3 id="replace-title">This repository already has files</h3>
            <p>
              Sharing “{collectionName}” removes them and puts the collection in their place on
              branch
              <span class="mono">{review.branch}</span>, as one ordinary commit. Nothing is
              force-pushed, so the old files stay in the repository's history.
            </p>
            {#if review.codestamp}
              <p>
                It holds a collection shared from Codestamp. People who joined that one need to join
                again.
              </p>
            {/if}
            <p class="list-title">
              {plural(review.files.length + review.more, 'file')} to remove
            </p>
            <ul class="files" aria-label="Files to remove">
              {#each review.files as file (file)}
                <li class="mono">{file}</li>
              {/each}
            </ul>
            {#if review.more > 0}
              <p class="muted">and {plural(review.more, 'more file')} not listed.</p>
            {/if}
            <div class="actions">
              {#if busy}
                <span class="muted">Git may open a sign-in window.</span>
              {/if}
              <button
                type="button"
                class="secondary"
                disabled={busy}
                onclick={() => (review = null)}
              >
                Cancel
              </button>
              <button
                type="button"
                class="replace-button"
                disabled={busy}
                onclick={() => void replaceAndShare()}
              >
                {busy ? 'Replacing…' : 'Replace files and share'}
              </button>
            </div>
          </div>
        {:else}
          <div class="actions">
            {#if busy}
              <span class="muted">Git may open a sign-in window.</span>
            {/if}
            <button
              type="button"
              class="primary"
              disabled={busy || remote.trim() === '' || name.trim() === '' || email.trim() === ''}
              onclick={() => void share()}
            >
              {checking
                ? 'Checking the repository…'
                : busy
                  ? 'Sharing…'
                  : `Share “${collectionName}”`}
            </button>
          </div>
        {/if}
      {/if}
    </div>
  </section>
</PanelDialog>

<ConfirmDialog
  open={confirmUnlink}
  title="Stop syncing?"
  message="“{collectionName}” stays on this device as a regular collection. The repository and your collaborators' copies are not changed."
  confirmLabel="Stop syncing"
  onConfirm={() => void unlink()}
  onCancel={() => (confirmUnlink = false)}
/>

<style>
  .panel {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .title {
    flex: 1 1 260px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .title p {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
  }

  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    background: var(--warn-bg);
    color: var(--warn);
    font-size: 13px;
  }

  .notice p {
    margin: 0;
    line-height: 1.5;
  }

  .status {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font-size: 13px;
  }

  .status-text {
    flex: 1 1 240px;
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .status.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .status.ok .dot {
    background: var(--ok);
  }

  .status.warn {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .status.warn .dot {
    background: var(--warn);
  }

  .status.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .status.err .dot {
    background: var(--err);
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

  .account-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
  }

  h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }

  .actions.start {
    justify-content: flex-start;
  }

  .steps {
    margin: 0;
    padding-left: 20px;
    font-size: 13px;
    color: var(--text-2);
    line-height: 1.6;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .field input {
    height: 38px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }

  .primary,
  .secondary,
  .quiet {
    height: 32px;
    padding: 0 14px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .primary:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .secondary,
  .quiet {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-2);
  }

  .replace {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--warn-line);
    border-radius: var(--radius-md);
    background: var(--warn-bg);
    color: var(--text);
    font-size: 13px;
  }

  .replace p {
    margin: 0;
    line-height: 1.5;
  }

  .list-title {
    font-weight: 600;
  }

  .files {
    max-height: 180px;
    overflow: auto;
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    list-style: none;
    font-size: 12px;
  }

  .files li {
    padding: 2px 0;
    overflow-wrap: anywhere;
  }

  .replace-button {
    height: 32px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--err);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
  }

  .replace-button:disabled {
    opacity: 0.6;
  }

  .quiet.danger:hover {
    border-color: var(--err);
    color: var(--err);
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
</style>
