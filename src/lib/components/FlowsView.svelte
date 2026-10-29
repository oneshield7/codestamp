<script lang="ts">
  import { onMount } from 'svelte';
  import type { Environments } from '../state/environments.svelte';
  import type { Flows } from '../state/flows.svelte';
  import { navLabel } from '../state/navigation';
  import { notices } from '../state/notices.svelte';
  import FlowEditor from './FlowEditor.svelte';
  import NavIcon from './NavIcon.svelte';

  interface Props {
    flows: Flows;
    environments: Environments;
    onOpenRequest: (collectionId: string, requestId: string) => void;
  }

  let { flows, environments, onOpenRequest }: Props = $props();

  // Requests may have been renamed or deleted in their collections since the page was open.
  onMount(() => {
    void notices.guard(() => flows.refresh());
  });
</script>

<div class="view">
  <header class="top">
    <div class="title">
      <h1 class="page-title"><NavIcon name="flows" size="lg" />{navLabel('flows')}</h1>
      <p>
        Run requests one after another from Start to End. Each step can check its response and pass
        values, such as a token, to the steps after it. Use a flow to test a sequence like register
        to checkout before you hand the API over.
      </p>
    </div>
    <button
      type="button"
      class="new"
      disabled={flows.running}
      onclick={() => void notices.guard(() => flows.create())}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
        ><path d="M12 5v14M5 12h14" /></svg
      >
      New flow
    </button>
  </header>

  {#if flows.loaded && flows.list.length === 0}
    <section class="empty" aria-label="No flows yet">
      <h2>Build your first flow</h2>
      <p>
        Add requests in the order they run, such as Register, Login, Add to cart and Checkout. Pass
        the token from Login on, check each response, then run it from Start to End.
      </p>
      <button type="button" class="new" onclick={() => void notices.guard(() => flows.create())}
        >Create a flow</button
      >
    </section>
  {:else}
    <div class="layout">
      <nav class="list" aria-label="Flows">
        {#each flows.list as item (item.id)}
          {@const active = flows.current?.id === item.id}
          <button
            type="button"
            class="item"
            class:on={active}
            aria-current={active ? 'page' : undefined}
            disabled={flows.running && !active}
            onclick={() => void notices.guard(() => flows.select(item.id))}
          >
            <span class="name">{item.name}</span>
            <span class="count">{item.stepCount} {item.stepCount === 1 ? 'step' : 'steps'}</span>
          </button>
        {/each}
      </nav>
      {#if flows.current}
        {#key flows.current.id}
          <FlowEditor flow={flows.current} {flows} {environments} {onOpenRequest} />
        {/key}
      {/if}
    </div>
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 16px;
  }

  .title {
    flex: 1 1 360px;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .title p {
    margin: 4px 0 0;
    max-width: 680px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .new {
    height: 36px;
    padding: 0 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .new:hover:not(:disabled) {
    border-color: var(--text-muted);
  }

  .new svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 28px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
  }

  .empty h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .empty p {
    margin: 0;
    max-width: 560px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .layout {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 20px;
  }

  .list {
    flex: 0 1 200px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    text-align: left;
  }

  .item:hover:not(:disabled) {
    background: var(--surface-sunken);
  }

  .item.on {
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .item .name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .count {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
