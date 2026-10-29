<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { toAppError } from '../../api/http';
  import { plural } from '../../format/plural';
  import {
    addGitAccount,
    checkGitAccount,
    listGitAccounts,
    removeGitAccount,
    signInGitAccount,
    signOutGitAccount,
    type AccountCheck,
    type GitAccount,
  } from '../../api/team';
  import { accountKey, canCheck, checkTone, type CheckTone } from '../../team/account';
  import ConfirmDialog from '../ConfirmDialog.svelte';

  let accounts = $state<GitAccount[] | null>(null);
  const checks = new SvelteMap<string, AccountCheck>();
  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);
  let adding = $state(false);
  let host = $state('github.com');
  let username = $state('');
  let label = $state('');
  let leaving = $state<GitAccount | null>(null);

  onMount(() => {
    void attempt(async () => {
      accounts = await listGitAccounts();
    });
  });

  async function attempt(task: () => Promise<void>): Promise<boolean> {
    error = null;
    try {
      await task();
      return true;
    } catch (caught) {
      error = toAppError(caught).message;
      return false;
    }
  }

  async function add(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const added = await attempt(async () => {
      accounts = await addGitAccount({ host, username, label });
    });
    if (added) {
      adding = false;
      username = '';
      label = '';
    }
  }

  async function remove(account: GitAccount): Promise<void> {
    const id = account.id;
    if (id !== null) {
      await attempt(async () => {
        accounts = await removeGitAccount(id);
      });
    }
  }

  /** Checking can open the credential manager's sign-in window, so it waits on the person. */
  async function verify(account: GitAccount, signIn: boolean): Promise<void> {
    const key = accountKey(account);
    busy = key;
    await attempt(async () => {
      const result = signIn
        ? await signInGitAccount(account.host, account.username)
        : await checkGitAccount(account.host, account.username);
      checks.set(key, result);
    });
    busy = null;
  }

  async function signOut(): Promise<void> {
    const account = leaving;
    leaving = null;
    if (!account?.username) {
      return;
    }
    const name = account.username;
    await attempt(async () => {
      await signOutGitAccount(account.host, name);
      checks.delete(accountKey(account));
      accounts = await listGitAccounts();
    });
  }

  function tone(account: GitAccount): CheckTone | null {
    return checkTone(checks.get(accountKey(account)) ?? null, account.signedIn);
  }
</script>

<section class="panel" aria-labelledby="git-accounts-title">
  <div class="title-row">
    <div class="titles">
      <h2 id="git-accounts-title">Git accounts</h2>
      <p class="hint">
        The accounts shared collections sign in with. Codestamp keeps the host and username only;
        passwords and tokens stay with Git's credential manager.
      </p>
    </div>
    <button
      type="button"
      class="button"
      aria-expanded={adding}
      onclick={() => {
        adding = !adding;
        error = null;
      }}
    >
      Add account
    </button>
  </div>

  {#if adding}
    <form class="add" aria-label="Add a Git account" onsubmit={(event) => void add(event)}>
      <div class="fields">
        <label class="field">
          Host
          <input class="input mono" bind:value={host} maxlength="253" spellcheck="false" />
        </label>
        <label class="field">
          Username
          <input
            class="input mono"
            bind:value={username}
            maxlength="100"
            spellcheck="false"
            autocomplete="off"
          />
        </label>
        <label class="field">
          <span>Label <span class="optional">(optional)</span></span>
          <input class="input" bind:value={label} maxlength="60" />
        </label>
      </div>
      <div class="actions">
        <button type="button" class="button" onclick={() => (adding = false)}>Cancel</button>
        <button
          type="submit"
          class="primary"
          disabled={host.trim() === '' || username.trim() === ''}
        >
          Add account
        </button>
      </div>
    </form>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if accounts === null}
    <p class="muted">Loading accounts…</p>
  {:else if accounts.length === 0}
    <p class="muted">
      No accounts yet. Add one here, or type a Git account when you share a collection over https.
    </p>
  {:else}
    <ul class="accounts" aria-label="Git accounts">
      {#each accounts as account (accountKey(account))}
        {@const key = accountKey(account)}
        {@const state = tone(account)}
        {@const result = checks.get(key)}
        <li class="account">
          <div class="who">
            <span class="name" class:mono={account.username !== null}
              >{account.username ?? "Git's default account"}</span
            >
            <span class="muted">· {account.host}</span>
            {#if account.label}
              <span class="label">{account.label}</span>
            {/if}
          </div>
          <div class="chips">
            <span class="chip uses">
              {account.collections.length === 0
                ? 'Not used yet'
                : plural(account.collections.length, 'collection')}
            </span>
            {#if state}
              <span class="chip {state.tone}">{state.text}</span>
            {/if}
          </div>
          <div class="row-actions">
            {#if canCheck(account)}
              <button
                type="button"
                class="button"
                disabled={busy !== null}
                onclick={() => void verify(account, state?.tone === 'warn')}
              >
                {busy === key ? 'Checking…' : state?.tone === 'warn' ? 'Sign in' : 'Check'}
              </button>
            {/if}
            {#if account.username !== null}
              <button
                type="button"
                class="button"
                disabled={busy !== null}
                onclick={() => (leaving = account)}
              >
                Sign out
              </button>
            {/if}
            {#if account.id !== null && account.collections.length === 0}
              <button type="button" class="button" onclick={() => void remove(account)}>
                Remove
              </button>
            {/if}
          </div>
          {#if account.collections.length > 0}
            <p class="used">
              Used by {account.collections.map((collection) => collection.name).join(', ')}
            </p>
          {/if}
          {#if busy === key}
            <p class="used">Git may open a sign-in window.</p>
          {:else if result?.message && result.status !== 'signedIn'}
            <p class="used">{result.message}</p>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <p class="muted">
    In a collection's Collaboration panel, Change account picks from this list. SSH addresses sign
    in with your SSH key instead.
  </p>
</section>

<ConfirmDialog
  open={leaving !== null}
  title="Sign out of {leaving?.username ?? ''}?"
  message="Git's credential manager forgets the sign-in of {leaving?.username ??
    ''} on {leaving?.host ??
    ''}. Collections that use this account ask you to sign in again on their next sync."
  confirmLabel="Sign out"
  onConfirm={() => void signOut()}
  onCancel={() => (leaving = null)}
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

  .add {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
  }

  .add .field {
    flex: 1 1 180px;
  }

  .optional {
    color: var(--text-muted);
  }

  .actions {
    justify-content: flex-end;
  }

  .accounts {
    margin: 0;
    padding: 0;
    list-style: none;
    container-type: inline-size;
  }

  .account {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px 12px;
    padding: 12px 0;
    border-top: 1px solid var(--border-soft);
  }

  .who {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 6px;
    font-size: 13px;
  }

  .name {
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .label {
    flex-basis: 100%;
    font-size: 12px;
    color: var(--text-muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
  }

  .chip.uses {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.warn {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .chip.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .chip.quiet {
    background: var(--surface-sunken);
    color: var(--text-2);
  }

  .row-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .used {
    grid-column: 1 / -1;
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  @container (max-width: 560px) {
    .account {
      grid-template-columns: minmax(0, 1fr);
    }

    .row-actions {
      justify-content: flex-start;
    }
  }
</style>
