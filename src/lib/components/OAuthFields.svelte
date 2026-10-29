<script lang="ts">
  import type { Auth, ClientAuth, OAuthGrant } from '../api/collections';
  import { toAppError } from '../api/http';
  import { clearOAuthToken, getOAuthToken, oauthTokenStatus, type TokenStatus } from '../api/oauth';
  import { formatSince, formatUntil } from '../format/time';
  import SecretInput from './SecretInput.svelte';
  import VariableInput from './VariableInput.svelte';

  type OAuth2Auth = Extract<Auth, { type: 'oauth2' }>;

  interface Props {
    auth: OAuth2Auth;
    /** The request or collection the token belongs to; null before it is saved. */
    ownerId: string | null;
    collectionId: string;
  }

  let { auth = $bindable(), ownerId, collectionId }: Props = $props();

  const GRANTS: { value: OAuthGrant; label: string }[] = [
    { value: 'authorizationCode', label: 'Authorization code' },
    { value: 'clientCredentials', label: 'Client credentials' },
    { value: 'password', label: 'Password' },
  ];
  const CLIENT_AUTH: { value: ClientAuth; label: string }[] = [
    { value: 'header', label: 'Basic auth header' },
    { value: 'body', label: 'In the request body' },
  ];
  const CLOCK_MS = 30_000;
  /** The Rust core renews a token this long before it expires, so it never lapses in flight. */
  const EXPIRY_MARGIN_MS = 30_000;

  let status = $state<TokenStatus | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let now = $state(Date.now());

  $effect(() => {
    const owner = ownerId;
    status = null;
    if (owner === null) {
      return;
    }
    let cancelled = false;
    oauthTokenStatus(owner).then(
      (found) => {
        if (!cancelled) {
          status = found;
        }
      },
      (caught: unknown) => {
        if (!cancelled) {
          error = toAppError(caught).message;
        }
      },
    );
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), CLOCK_MS);
    return () => {
      clearInterval(timer);
    };
  });

  const statusText = $derived.by(() => {
    if (!status) {
      return 'No access token yet.';
    }
    const obtained = `Got ${formatSince(status.obtainedAt, now)}`;
    if (status.expiresAt === null) {
      return `${obtained}; the server gave no expiry.`;
    }
    const until = formatUntil(status.expiresAt - EXPIRY_MARGIN_MS, now);
    if (until !== null) {
      return `${obtained} · expires ${until}`;
    }
    return status.refreshable || auth.grant === 'clientCredentials'
      ? `${obtained} · expired; it renews on the next send`
      : `${obtained} · expired; get a new one`;
  });

  async function getToken(owner: string): Promise<void> {
    busy = true;
    error = null;
    try {
      status = await getOAuthToken(owner, collectionId, $state.snapshot(auth));
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }

  async function clearToken(owner: string): Promise<void> {
    error = null;
    try {
      await clearOAuthToken(owner);
      status = null;
    } catch (caught) {
      error = toAppError(caught).message;
    }
  }
</script>

<div class="field">
  <span class="label">Grant type</span>
  <div class="segmented" role="group" aria-label="Grant type">
    {#each GRANTS as grant (grant.value)}
      <button
        type="button"
        class:on={auth.grant === grant.value}
        aria-pressed={auth.grant === grant.value}
        onclick={() => (auth.grant = grant.value)}>{grant.label}</button
      >
    {/each}
  </div>
</div>

{#if auth.grant === 'authorizationCode'}
  <div class="field">
    <span class="label">Authorization URL</span>
    <VariableInput
      bind:value={auth.authUrl}
      label="Authorization URL"
      variant="field"
      placeholder="https://login.example.com/oauth/authorize"
      {collectionId}
    />
  </div>
{/if}
<div class="field">
  <span class="label">Token URL</span>
  <VariableInput
    bind:value={auth.tokenUrl}
    label="Token URL"
    variant="field"
    placeholder="https://login.example.com/oauth/token"
    {collectionId}
  />
</div>
<div class="field">
  <span class="label">Client ID</span>
  <VariableInput bind:value={auth.clientId} label="Client ID" variant="field" {collectionId} />
</div>
<div class="field">
  <span class="label">Client secret <small>(leave empty for public clients)</small></span>
  <SecretInput bind:value={auth.clientSecret} label="Client secret" {collectionId} />
</div>
<div class="field">
  <span class="label">Scope</span>
  <VariableInput
    bind:value={auth.scope}
    label="Scope"
    variant="field"
    placeholder="read write"
    {collectionId}
  />
</div>
{#if auth.grant === 'authorizationCode'}
  <div class="field">
    <span class="label">Callback URL</span>
    <VariableInput
      bind:value={auth.redirectUri}
      label="Callback URL"
      variant="field"
      placeholder="https://app.example.com/callback"
      {collectionId}
    />
    <small class="note">
      The URL registered for this client. Sign-in happens in its own window, and Codestamp catches
      the redirect to this URL there, so it can be a placeholder such as
      <span class="mono">http://127.0.0.1/callback</span>.
    </small>
  </div>
  <label class="check">
    <input type="checkbox" bind:checked={auth.usePkce} />
    Use PKCE (recommended)
  </label>
{:else if auth.grant === 'password'}
  <div class="field">
    <span class="label">Username</span>
    <VariableInput
      bind:value={auth.username}
      label="OAuth username"
      variant="field"
      {collectionId}
    />
  </div>
  <div class="field">
    <span class="label">Password</span>
    <SecretInput bind:value={auth.password} label="OAuth password" {collectionId} />
  </div>
{/if}
<div class="field">
  <span class="label">Send client credentials</span>
  <div class="segmented" role="group" aria-label="Send client credentials">
    {#each CLIENT_AUTH as option (option.value)}
      <button
        type="button"
        class:on={auth.clientAuth === option.value}
        aria-pressed={auth.clientAuth === option.value}
        onclick={() => (auth.clientAuth = option.value)}>{option.label}</button
      >
    {/each}
  </div>
</div>
<label class="field narrow">
  Header prefix
  <input
    class="input mono"
    bind:value={auth.headerPrefix}
    maxlength="40"
    placeholder="Bearer"
    spellcheck="false"
  />
</label>

<section class="token" aria-label="Access token">
  <div class="token-text">
    <span class="token-title">Access token</span>
    <span class="token-status" role="status">{statusText}</span>
    {#if status?.scope}
      <span class="token-scope mono">{status.scope}</span>
    {/if}
  </div>
  {#if ownerId === null}
    <span class="note">Save the request first.</span>
  {:else}
    {@const owner = ownerId}
    {#if status}
      <button type="button" class="quiet" disabled={busy} onclick={() => void clearToken(owner)}>
        Clear
      </button>
    {/if}
    <button type="button" class="primary" disabled={busy} onclick={() => void getToken(owner)}>
      {#if busy}
        {auth.grant === 'authorizationCode' ? 'Waiting for sign-in…' : 'Getting token…'}
      {:else}
        Get new access token
      {/if}
    </button>
  {/if}
</section>
{#if error}
  <p class="error" role="alert">{error}</p>
{/if}
<p class="note">
  Tokens stay encrypted on this device and are never shared with the collection. Expired tokens are
  refreshed when the server gave a refresh token.
</p>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .label small {
    color: var(--text-muted);
  }

  .narrow {
    max-width: 260px;
  }

  .input {
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .input:focus {
    border-color: var(--focus);
    outline: none;
  }

  .segmented {
    display: inline-flex;
    flex-wrap: wrap;
    align-self: flex-start;
    gap: 2px;
    padding: 3px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .segmented button {
    padding: 5px 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
  }

  .segmented button.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 500;
    box-shadow: var(--shadow-segment);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-2);
  }

  .check input {
    accent-color: var(--accent);
  }

  .token {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
  }

  .token-text {
    flex: 1 1 220px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }

  .token-title {
    font-weight: 600;
  }

  .token-status,
  .token-scope {
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .token-scope {
    font-size: 12px;
  }

  .primary,
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

  .quiet {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-2);
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
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
