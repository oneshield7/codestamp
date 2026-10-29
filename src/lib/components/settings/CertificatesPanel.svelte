<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../../api/http';
  import {
    addCaCertificate,
    addClientCertificate,
    deleteCertificate,
    listCertificates,
    type CertificateInfo,
  } from '../../api/preferences';
  import { notices } from '../../state/notices.svelte';
  import ConfirmDialog from '../ConfirmDialog.svelte';
  import CertificateRow from './CertificateRow.svelte';

  let certificates: CertificateInfo[] = $state([]);
  let host = $state('');
  let busy = $state(false);
  let error: string | null = $state(null);
  let pendingRemove: CertificateInfo | null = $state(null);

  const authorities = $derived(certificates.filter((item) => item.kind === 'ca'));
  const clients = $derived(certificates.filter((item) => item.kind === 'client'));

  onMount(() => {
    void notices.guard(async () => {
      certificates = await listCertificates();
    });
  });

  async function add(task: () => Promise<CertificateInfo | null>): Promise<void> {
    busy = true;
    error = null;
    try {
      const added = await task();
      if (added) {
        certificates = await listCertificates();
        host = '';
      }
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }

  function ask(item: CertificateInfo): void {
    pendingRemove = item;
  }

  async function remove(): Promise<void> {
    const target = pendingRemove;
    pendingRemove = null;
    if (!target) {
      return;
    }
    await notices.guard(async () => {
      await deleteCertificate(target.id);
      certificates = certificates.filter((item) => item.id !== target.id);
    });
  }
</script>

<section class="panel" aria-labelledby="ca-title">
  <h2 id="ca-title">CA certificates</h2>
  <p class="hint">
    Trusted on top of the certificates Windows trusts, for servers with an internal or self-signed
    certificate.
  </p>
  {#if authorities.length > 0}
    <ul class="list">
      {#each authorities as item (item.id)}
        <CertificateRow {item} removing={pendingRemove?.id === item.id} onRemove={ask} />
      {/each}
    </ul>
  {/if}
  <div>
    <button type="button" class="button" disabled={busy} onclick={() => void add(addCaCertificate)}>
      Add CA certificate…
    </button>
  </div>
</section>

<section class="panel" aria-labelledby="client-title">
  <h2 id="client-title">Client certificates</h2>
  <p class="hint">
    Sent to one host for mutual TLS. Choose a PEM certificate and its key (or one file with both).
    The key is stored encrypted; a .pfx file must be converted to PEM first.
  </p>
  {#if clients.length > 0}
    <ul class="list">
      {#each clients as item (item.id)}
        <CertificateRow {item} removing={pendingRemove?.id === item.id} onRemove={ask} />
      {/each}
    </ul>
  {/if}
  <form
    class="fields"
    onsubmit={(event) => {
      event.preventDefault();
      void add(() => addClientCertificate(host));
    }}
  >
    <label class="field wide">
      Host
      <input
        id="client-cert-host"
        class="input mono"
        bind:value={host}
        placeholder="api.example.com:8443"
        spellcheck="false"
        maxlength="260"
      />
    </label>
    <button type="submit" class="button add" disabled={busy || host.trim() === ''}>
      Choose certificate…
    </button>
  </form>
</section>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<ConfirmDialog
  open={pendingRemove !== null}
  title="Remove certificate?"
  message={pendingRemove?.kind === 'client'
    ? `Requests to ${pendingRemove.host ?? ''} will no longer send this certificate.`
    : `Servers signed by “${pendingRemove?.name ?? ''}” will no longer be trusted.`}
  confirmLabel="Remove"
  onConfirm={() => void remove()}
  onCancel={() => (pendingRemove = null)}
/>

<style>
  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .add {
    align-self: flex-end;
    height: 40px;
  }
</style>
