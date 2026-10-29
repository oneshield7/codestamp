<script lang="ts">
  import { ADDABLE_KINDS } from '../request/kind-icons';
  import type { RequestKind } from '../api/collections';
  import { getCollectionVariableSet, getEnvironmentVariables } from '../api/environments';
  import { revealCard } from '../dom/reveal';
  import type { Workspace } from '../state/workspace.svelte';

  interface Props {
    workspace: Workspace;
    collectionId: string;
    onOpenPanel: (panel: 'variables' | 'team') => void;
    onPasteCurl: () => void;
  }

  let { workspace, collectionId, onOpenPanel, onPasteCurl }: Props = $props();

  async function addFirst(kind: RequestKind): Promise<void> {
    await workspace.addRequest(null, kind);
    await revealCard(workspace.openRequestId);
  }

  const BASE_URL_KEY = 'baseUrl';
  const BASE_URL_TOKEN = `{{${BASE_URL_KEY}}}`;

  let baseUrl = $state<string | null>(null);

  // Only the variable card depends on this, so a failed read leaves that card out.
  $effect(() => {
    const id = collectionId;
    const run = { cancelled: false };
    baseUrl = null;
    getCollectionVariableSet(id)
      .then(getEnvironmentVariables)
      .then(
        (variables) => {
          const found = variables.find(
            (variable) => variable.key === BASE_URL_KEY && !variable.isSecret && variable.enabled,
          );
          if (!run.cancelled) {
            baseUrl = found?.value ?? null;
          }
        },
        () => {
          if (!run.cancelled) {
            baseUrl = null;
          }
        },
      );
    return () => {
      run.cancelled = true;
    };
  });
</script>

<section class="start" aria-labelledby="start-title">
  <div>
    <h2 id="start-title">Add your first request</h2>
    <p>Requests run top to bottom, like steps in a notebook. Each one saves as you type.</p>
  </div>
  <div class="tiles">
    {#each ADDABLE_KINDS as option (option.kind)}
      <button type="button" class="tile" onclick={() => void addFirst(option.kind)}>
        <svg width="20" height="20" viewBox="0 0 24 24" aria-hidden="true" data-hue={option.hue}>
          <path d={option.icon} />
        </svg>
        <span class="tile-title">{option.name}</span>
        <span class="tile-text">{option.description}</span>
      </button>
    {/each}
    <button type="button" class="tile" onclick={onPasteCurl}>
      <svg width="20" height="20" viewBox="0 0 24 24" aria-hidden="true" data-hue="violet">
        <path d="M4 17l6-5-6-5M12 19h8" />
      </svg>
      <span class="tile-title">Paste a cURL command</span>
      <span class="tile-text"
        >From browser dev tools or API docs. Codestamp fills in the request.</span
      >
    </button>
  </div>
</section>

<section class="ready" aria-labelledby="ready-title">
  <h2 id="ready-title">Also ready in this collection</h2>
  <div class="cards">
    {#if baseUrl !== null}
      <div class="card">
        <h3>Variable <span class="mono">{BASE_URL_TOKEN}</span></h3>
        <p>
          Set to <span class="mono value">{baseUrl}</span>. Start each URL with it, and change it
          once for every request.
        </p>
        <button
          type="button"
          class="go"
          onclick={() => {
            onOpenPanel('variables');
          }}>Edit variables →</button
        >
      </div>
    {/if}
    <div class="card">
      <h3>Sections</h3>
      <p>Group related requests, like Auth or Customers, and drag them into the order they run.</p>
      <button type="button" class="go" onclick={() => void workspace.addSection()}
        >Add a section →</button
      >
    </div>
    <div class="card">
      <h3>Collaborate on it</h3>
      <p>Share this collection through a Git repository, and collaborators get every change.</p>
      <button
        type="button"
        class="go"
        onclick={() => {
          onOpenPanel('team');
        }}>Open Collaboration →</button
      >
    </div>
  </div>
</section>

<style>
  .start {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 28px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.015em;
  }

  .start p {
    margin: 6px 0 0;
    line-height: 1.6;
    color: var(--text-muted);
  }

  .tiles,
  .cards {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .tile {
    flex: 1 1 240px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-muted);
    text-align: left;
  }

  .tile:hover {
    border-color: var(--border-strong);
  }

  .tile:active {
    background: var(--surface-sunken);
  }

  .tile-title {
    font-size: 15px;
    font-weight: 600;
  }

  .tile-text {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .ready {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 12px;
  }

  .ready h2 {
    font-size: 14px;
    letter-spacing: 0;
  }

  .card {
    flex: 1 1 280px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .card p {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .value {
    color: var(--text-2);
  }

  .go {
    align-self: flex-start;
    margin-top: auto;
    padding: 4px 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .go:hover {
    text-decoration: underline;
  }
</style>
