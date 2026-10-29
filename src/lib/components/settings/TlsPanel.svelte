<script lang="ts">
  import type { TlsVersion } from '../../api/preferences';
  import { preferenceSettings } from '../../state/preferences.svelte';
  import CertificatesPanel from './CertificatesPanel.svelte';

  const VERSIONS: { key: TlsVersion; label: string }[] = [
    { key: '1.2', label: 'TLS 1.2 or newer' },
    { key: '1.3', label: 'TLS 1.3 only' },
  ];

  const minimum = $derived(preferenceSettings.preferences.requests.minTlsVersion);
</script>

<section class="panel" aria-labelledby="version-title">
  <h2 id="version-title">TLS version</h2>
  <p class="hint">
    The oldest version a server may answer with. TLS 1.3 only refuses servers that still use 1.2.
    Applies to HTTP, GraphQL, SOAP, event streams and WebSocket requests, OAuth token requests and
    imports from a URL. gRPC always accepts TLS 1.2 or newer.
  </p>
  <div class="segmented" role="radiogroup" aria-label="Minimum TLS version">
    {#each VERSIONS as option (option.key)}
      <label class:on={minimum === option.key}>
        <input
          type="radio"
          name="tls-version"
          class="visually-hidden"
          checked={minimum === option.key}
          onchange={() => {
            preferenceSettings.updateRequests({ minTlsVersion: option.key });
          }}
        />
        {option.label}
      </label>
    {/each}
  </div>
  {#if preferenceSettings.saveError}
    <p class="error" role="alert">{preferenceSettings.saveError}</p>
  {/if}
</section>

<section class="panel" aria-labelledby="checks-title">
  <h2 id="checks-title">Certificate checks</h2>
  <p class="hint">
    Codestamp always checks the server's certificate and cannot turn the check off. For a server
    with an internal or self-signed certificate, add the CA certificate that signed it below.
  </p>
</section>

<CertificatesPanel />
