<script lang="ts">
  import { SvelteMap } from 'svelte/reactivity';
  import type { Auth, AuthType } from '../api/collections';
  import { authOptions, authPreview, blankAuth, effectiveAuth } from '../request/auth';
  import Dropdown from './Dropdown.svelte';
  import OAuthFields from './OAuthFields.svelte';
  import SecretInput from './SecretInput.svelte';
  import VariableInput from './VariableInput.svelte';

  interface Props {
    auth: Auth;
    /** Requests may inherit the collection's auth; a collection cannot. */
    allowInherit: boolean;
    collectionAuth: Auth | null;
    collectionId: string;
    /** Where an OAuth 2.0 token is kept: the request, or the collection for its own auth. */
    ownerId: string | null;
    /** Types this kind of request cannot use. */
    hidden?: AuthType[];
  }

  let {
    auth = $bindable(),
    allowInherit,
    collectionAuth,
    collectionId,
    ownerId,
    hidden = [],
  }: Props = $props();

  // Switching type and back keeps what was typed for the earlier type.
  const remembered = new SvelteMap<AuthType, Auth>();

  const options = $derived(
    authOptions(allowInherit, collectionAuth).filter((option) => !hidden.includes(option.type)),
  );
  const typeOptions = $derived(
    options.map((option) => ({ value: option.type, label: option.label })),
  );
  const chosen = $derived(options.find((option) => option.type === auth.type));
  const preview = $derived(authPreview(effectiveAuth(auth, collectionAuth)));

  function choose(type: AuthType): void {
    if (type === auth.type) {
      return;
    }
    remembered.set(auth.type, auth);
    auth = remembered.get(type) ?? blankAuth(type);
  }
</script>

<div class="auth">
  <div class="type">
    <span class="label">Auth type</span>
    <div class="type-row">
      <div class="type-select">
        <Dropdown
          variant="field"
          label="Auth type"
          value={auth.type}
          options={typeOptions}
          onChange={(type: AuthType) => {
            choose(type);
          }}
        />
      </div>
      {#if chosen}
        <span class="type-hint">{chosen.hint}</span>
      {/if}
    </div>
  </div>

  <div class="fields">
    {#if auth.type === 'bearer'}
      <div class="field">
        <span class="label">Token</span>
        <SecretInput
          bind:value={auth.token}
          label="Token"
          placeholder={'{{authToken}}'}
          {collectionId}
        />
      </div>
      <label class="field narrow">
        Header prefix
        <input class="input mono" bind:value={auth.prefix} maxlength="40" spellcheck="false" />
      </label>
    {:else if auth.type === 'basic'}
      <div class="field">
        <span class="label">Username</span>
        <VariableInput bind:value={auth.username} label="Username" variant="field" {collectionId} />
      </div>
      <div class="field">
        <span class="label">Password</span>
        <SecretInput bind:value={auth.password} label="Password" {collectionId} />
      </div>
    {:else if auth.type === 'apiKey'}
      <label class="field narrow">
        Key
        <input class="input mono" bind:value={auth.key} spellcheck="false" />
      </label>
      <div class="field">
        <span class="label">Value</span>
        <SecretInput bind:value={auth.value} label="API key value" {collectionId} />
      </div>
      <div class="field">
        <span class="label">Add to</span>
        <div class="segmented" role="group" aria-label="Add the API key to">
          <button
            type="button"
            class:on={auth.location === 'header'}
            aria-pressed={auth.location === 'header'}
            onclick={() => {
              if (auth.type === 'apiKey') {
                auth.location = 'header';
              }
            }}>Header</button
          >
          <button
            type="button"
            class:on={auth.location === 'query'}
            aria-pressed={auth.location === 'query'}
            onclick={() => {
              if (auth.type === 'apiKey') {
                auth.location = 'query';
              }
            }}>Query param</button
          >
        </div>
      </div>
    {:else if auth.type === 'digest'}
      <div class="field">
        <span class="label">Username</span>
        <VariableInput
          bind:value={auth.username}
          label="Digest username"
          variant="field"
          {collectionId}
        />
      </div>
      <div class="field">
        <span class="label">Password</span>
        <SecretInput bind:value={auth.password} label="Digest password" {collectionId} />
      </div>
      <p class="hint">
        The request is sent once without credentials; when the server answers 401 with a Digest
        challenge, it is sent again with the answer.
      </p>
    {:else if auth.type === 'awsSigV4'}
      <div class="field">
        <span class="label">Access key</span>
        <VariableInput
          bind:value={auth.accessKey}
          label="AWS access key"
          variant="field"
          placeholder={'{{awsAccessKey}}'}
          {collectionId}
        />
      </div>
      <div class="field">
        <span class="label">Secret key</span>
        <SecretInput bind:value={auth.secretKey} label="AWS secret key" {collectionId} />
      </div>
      <div class="pair">
        <div class="field">
          <span class="label">Region</span>
          <VariableInput
            bind:value={auth.region}
            label="AWS region"
            variant="field"
            placeholder="us-east-1"
            {collectionId}
          />
        </div>
        <div class="field">
          <span class="label">Service</span>
          <VariableInput
            bind:value={auth.service}
            label="AWS service"
            variant="field"
            placeholder="execute-api"
            {collectionId}
          />
        </div>
      </div>
      <div class="field">
        <span class="label">Session token <small>(temporary credentials only)</small></span>
        <SecretInput bind:value={auth.sessionToken} label="AWS session token" {collectionId} />
      </div>
    {:else if auth.type === 'oauth2'}
      <OAuthFields bind:auth {ownerId} {collectionId} />
    {:else if auth.type === 'inherit'}
      <p class="hint">
        This request uses the collection’s auth. Change it with <strong>Auth</strong> in the collection
        header.
      </p>
    {:else}
      <p class="hint">Nothing is added to this request.</p>
    {/if}

    {#if auth.type !== 'inherit' && auth.type !== 'none'}
      <div class="tip">
        <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" />
        </svg>
        <span>
          Put secrets in a secret variable such as <span class="mono">{'{{authToken}}'}</span>.
          Secret variables are encrypted, and their key stays in the system credential store.
        </span>
      </div>
    {/if}
    <p class="preview mono">Preview → {preview}</p>
  </div>
</div>

<style>
  .auth {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .type {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .type .label {
    font-weight: 600;
    color: var(--text-2);
  }

  .type-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
  }

  .type-select {
    flex: 0 1 280px;
    min-width: 200px;
  }

  .type-hint {
    font-size: 13px;
    color: var(--text-muted);
  }

  .fields {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .label {
    font-size: 13px;
  }

  .label small {
    color: var(--text-muted);
  }

  .pair {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .pair .field {
    flex: 1 1 160px;
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

  .tip svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .segmented {
    display: inline-flex;
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

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .tip {
    display: flex;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text-2);
    font-size: 13px;
  }

  .tip svg {
    flex: none;
    margin-top: 1px;
  }

  .preview {
    margin: 0;
    padding: 10px 12px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text-2);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
</style>
