<script lang="ts">
  import { onMount } from 'svelte';
  import { aboutApp, networkProblem } from '../../api/preferences';
  import { notices } from '../../state/notices.svelte';
  import { preferenceSettings } from '../../state/preferences.svelte';
  import NumberField from '../NumberField.svelte';

  interface Props {
    onOpenTls: () => void;
  }

  let { onOpenTls }: Props = $props();

  const requests = $derived(preferenceSettings.preferences.requests);
  let defaultAgent = $state('Codestamp');
  let problem: string | null = $state(null);

  onMount(() => {
    void notices.guard(async () => {
      const [about, current] = await Promise.all([aboutApp(), networkProblem()]);
      defaultAgent = `Codestamp/${about.version}`;
      problem = current;
    });
  });
</script>

{#if problem}
  <p class="error" role="alert">
    Saved network settings cannot be used, so requests go out with the defaults: {problem}
  </p>
{/if}

<section class="panel" aria-labelledby="sending-title">
  <h2 id="sending-title">Sending</h2>
  <div class="fields">
    <NumberField
      label="Default timeout (seconds)"
      value={requests.timeoutSeconds}
      range={{ min: 1, max: 300 }}
      onChange={(timeoutSeconds: number) => {
        preferenceSettings.updateRequests({ timeoutSeconds });
      }}
    />
    <NumberField
      label="Largest response (MB, 0 = any size)"
      value={requests.maxResponseMb}
      range={{ min: 0, max: 1024 }}
      onChange={(maxResponseMb: number) => {
        preferenceSettings.updateRequests({ maxResponseMb });
      }}
    />
  </div>
  <label class="field">
    User-Agent
    <input
      class="input mono"
      value={requests.userAgent}
      placeholder={defaultAgent}
      maxlength="200"
      spellcheck="false"
      oninput={(event) => {
        preferenceSettings.updateRequests({ userAgent: event.currentTarget.value });
      }}
    />
  </label>
  <p class="hint">A User-Agent header on a request still wins.</p>
</section>

<section class="panel" aria-labelledby="behaviour-title">
  <h2 id="behaviour-title">Behaviour</h2>
  <label class="check">
    <input
      type="checkbox"
      checked={requests.followRedirects}
      onchange={(event) => {
        preferenceSettings.updateRequests({ followRedirects: event.currentTarget.checked });
      }}
    />
    Follow redirects on the same site
  </label>
  <p class="hint indent">
    Redirects to another host are never followed, so tokens filled in from variables stay put.
  </p>
  <label class="check">
    <input
      type="checkbox"
      checked={requests.cookies}
      onchange={(event) => {
        preferenceSettings.updateRequests({ cookies: event.currentTarget.checked });
      }}
    />
    Keep and send cookies (the cookie jar)
  </label>
  <label class="check">
    <input
      type="checkbox"
      checked={requests.noCache}
      onchange={(event) => {
        preferenceSettings.updateRequests({ noCache: event.currentTarget.checked });
      }}
    />
    Send <span class="mono">Cache-Control: no-cache</span>
  </label>
</section>

<section class="panel" aria-labelledby="tls-title">
  <h2 id="tls-title">Certificate checks</h2>
  <p class="hint">
    Server certificates are always checked. The TLS version and the certificates Codestamp trusts or
    sends are set in SSL/TLS.
  </p>
  <div>
    <button type="button" class="button" onclick={onOpenTls}>Open SSL/TLS</button>
  </div>
</section>

{#if preferenceSettings.saveError}
  <p class="error" role="alert">{preferenceSettings.saveError}</p>
{/if}
<div>
  <button
    type="button"
    class="button"
    onclick={() => {
      preferenceSettings.resetRequests();
    }}
  >
    Reset request settings
  </button>
</div>

<style>
  .indent {
    margin-left: 28px;
  }
</style>
