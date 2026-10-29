<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../../api/http';
  import {
    getProxy,
    saveProxy,
    type ProxyMode,
    type ProxyScheme,
    type ProxySettings,
  } from '../../api/preferences';
  import { notices } from '../../state/notices.svelte';
  import Dropdown from '../Dropdown.svelte';
  import NumberField from '../NumberField.svelte';

  const MODES: { key: ProxyMode; label: string; hint: string }[] = [
    { key: 'system', label: 'Use the system proxy', hint: 'The proxy set in Windows, if any' },
    { key: 'off', label: 'No proxy', hint: 'Connect directly, even if Windows has a proxy' },
    { key: 'custom', label: 'Custom proxy', hint: 'A proxy only Codestamp uses' },
  ];
  const SCHEMES: { value: ProxyScheme; label: string }[] = [
    { value: 'http', label: 'HTTP' },
    { value: 'https', label: 'HTTPS' },
    { value: 'socks5', label: 'SOCKS5' },
  ];

  let settings: ProxySettings = $state({
    mode: 'system',
    scheme: 'http',
    host: '',
    port: 8080,
    username: '',
    bypass: '',
  });
  let hasPassword = $state(false);
  let password = $state('');
  let passwordTouched = $state(false);
  let saving = $state(false);
  let saved = $state(false);
  let error: string | null = $state(null);

  onMount(() => {
    void notices.guard(async () => {
      const view = await getProxy();
      settings = view.settings.port === 0 ? { ...view.settings, port: 8080 } : view.settings;
      hasPassword = view.hasPassword;
    });
  });

  function changed(): void {
    saved = false;
    error = null;
  }

  async function save(): Promise<void> {
    saving = true;
    try {
      const view = await saveProxy(
        $state.snapshot(settings),
        passwordTouched ? password : undefined,
      );
      settings = view.settings;
      hasPassword = view.hasPassword;
      password = '';
      passwordTouched = false;
      saved = true;
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      saving = false;
    }
  }
</script>

<section class="panel" aria-labelledby="proxy-title">
  <h2 id="proxy-title">Proxy</h2>
  <p class="hint">
    Used by requests, scripts (<span class="mono">pm.sendRequest</span>) and MCP servers over HTTP.
    WebSocket connections always connect directly.
  </p>
  <div class="modes" role="radiogroup" aria-label="Proxy">
    {#each MODES as mode (mode.key)}
      <label class="mode" class:on={settings.mode === mode.key}>
        <input
          type="radio"
          name="proxy-mode"
          checked={settings.mode === mode.key}
          onchange={() => {
            settings.mode = mode.key;
            changed();
          }}
        />
        <span>
          <span class="mode-label">{mode.label}</span>
          <span class="mode-hint">{mode.hint}</span>
        </span>
      </label>
    {/each}
  </div>

  {#if settings.mode === 'custom'}
    <div class="fields">
      <div class="field">
        Type
        <Dropdown
          variant="field"
          label="Proxy type"
          value={settings.scheme}
          options={SCHEMES}
          onChange={(scheme: ProxyScheme) => {
            settings.scheme = scheme;
            changed();
          }}
        />
      </div>
      <label class="field wide">
        Host
        <input
          id="proxy-host"
          class="input mono"
          bind:value={settings.host}
          oninput={changed}
          placeholder="proxy.example.com"
          spellcheck="false"
          maxlength="253"
        />
      </label>
      <NumberField
        label="Port"
        value={settings.port}
        range={{ min: 1, max: 65535 }}
        onChange={(port: number) => {
          settings.port = port;
          changed();
        }}
      />
    </div>
    <div class="fields">
      <label class="field wide">
        Username (optional)
        <input
          class="input"
          bind:value={settings.username}
          oninput={changed}
          autocomplete="off"
          spellcheck="false"
          maxlength="200"
        />
      </label>
      <label class="field wide">
        Password
        <input
          class="input"
          type="password"
          value={password}
          placeholder={hasPassword ? 'Saved · type to replace' : ''}
          autocomplete="new-password"
          maxlength="500"
          oninput={(event) => {
            password = event.currentTarget.value;
            passwordTouched = true;
            changed();
          }}
        />
      </label>
    </div>
    {#if hasPassword && !passwordTouched}
      <div>
        <button
          type="button"
          class="button"
          onclick={() => {
            password = '';
            passwordTouched = true;
            changed();
          }}
        >
          Remove saved password
        </button>
      </div>
    {/if}
    <label class="field">
      Skip the proxy for
      <input
        class="input mono"
        bind:value={settings.bypass}
        oninput={changed}
        placeholder="localhost, 127.0.0.1, .internal.example.com"
        spellcheck="false"
        maxlength="2000"
      />
    </label>
    <p class="hint">The password is stored encrypted, like secret variables.</p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  <div class="actions">
    <button type="button" class="primary" disabled={saving} onclick={() => void save()}>
      {saving ? 'Saving…' : 'Save proxy'}
    </button>
    {#if saved}
      <span class="status" role="status">Saved · new requests use it</span>
    {/if}
  </div>
</section>

<style>
  .modes {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mode {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .mode.on {
    border-color: var(--text);
    background: var(--accent-soft);
  }

  .mode input {
    margin-top: 3px;
    accent-color: var(--accent);
  }

  .mode-label {
    display: block;
    font-weight: 500;
  }

  .mode-hint {
    display: block;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
