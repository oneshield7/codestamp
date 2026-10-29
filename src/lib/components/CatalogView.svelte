<script lang="ts">
  import { onMount } from 'svelte';
  import type { Catalog } from '../state/catalog.svelte';
  import CatalogApi from './catalog/CatalogApi.svelte';
  import CatalogList from './catalog/CatalogList.svelte';

  interface Props {
    catalog: Catalog;
    onOpenRequest: (collectionId: string, requestId: string) => void;
    onOpenCollection: (collectionId: string) => void;
    onOpenFlow: (flowId: string) => void;
    onRunCollection: (collectionId: string) => void;
    onNewCollection: () => void;
  }

  let {
    catalog,
    onOpenRequest,
    onOpenCollection,
    onOpenFlow,
    onRunCollection,
    onNewCollection,
  }: Props = $props();

  // History, runs and requests change elsewhere, so every visit reads them again.
  onMount(() => {
    const openId = catalog.openId;
    if (openId === null) {
      void catalog.load();
    } else {
      void catalog.open(openId, catalog.tab);
    }
  });
</script>

{#if catalog.openId === null}
  <CatalogList {catalog} onOpen={(id: string) => void catalog.open(id)} {onNewCollection} />
{:else if catalog.detail}
  {#key catalog.detail.entry.id}
    <CatalogApi
      {catalog}
      detail={catalog.detail}
      onBack={() => void catalog.close()}
      {onOpenRequest}
      {onOpenCollection}
      {onOpenFlow}
      {onRunCollection}
    />
  {/key}
{:else if catalog.detailError}
  <div class="problem" role="alert">
    <span>This API could not load: {catalog.detailError}</span>
    <button type="button" onclick={() => void catalog.close()}>Back to all APIs</button>
  </div>
{:else}
  <p class="loading" role="status">Loading the API…</p>
{/if}

<style>
  .problem {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid var(--err-line);
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .problem span {
    flex: 1 1 240px;
  }

  .problem button {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--err-line);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
  }

  .loading {
    margin: 12px 0;
    color: var(--text-muted);
  }
</style>
