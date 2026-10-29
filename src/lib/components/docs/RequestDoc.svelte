<script lang="ts">
  import type { Auth } from '../../api/collections';
  import type { RequestDocs } from '../../api/docs';
  import { bodyOf, documentedRows, formRows, requestFacts } from '../../docs/request';
  import { statusTone } from '../../format/status';
  import { stampLabel } from '../../request/kinds';
  import { stampTone } from '../../request/methods';
  import type { Docs } from '../../state/docs.svelte';
  import MethodStamp from '../MethodStamp.svelte';
  import DescriptionBlock from './DescriptionBlock.svelte';
  import DocTable from './DocTable.svelte';

  interface Props {
    docs: Docs;
    request: RequestDocs;
    collectionAuth: Auth;
    onOpenRequest: (requestId: string) => void;
  }

  let { docs, request, collectionAuth, onOpenRequest }: Props = $props();

  const facts = $derived(requestFacts(request, collectionAuth));
  const body = $derived(bodyOf(request));
  const pathVariables = $derived(
    request.pathVariables.map((variable) => ({ name: variable.key, value: variable.value })),
  );
</script>

<article
  class="request"
  id={`doc-${request.id}`}
  tabindex="-1"
  aria-labelledby={`doc-title-${request.id}`}
>
  <header>
    <MethodStamp
      label={stampLabel(request.kind, request.method)}
      tone={stampTone(request.kind, request.method)}
    />
    <h4 id={`doc-title-${request.id}`}>{request.name}</h4>
    <button
      type="button"
      class="open"
      onclick={() => {
        onOpenRequest(request.id);
      }}>Open request <span class="visually-hidden">{request.name}</span></button
    >
  </header>
  {#if request.url.trim() !== ''}
    <p class="url mono">{request.url}</p>
  {/if}

  <DescriptionBlock
    {docs}
    item="request"
    id={request.id}
    text={request.description}
    subject="this request"
    headingOffset={4}
  />

  {#if facts.length > 0}
    <dl class="facts">
      {#each facts as fact (fact.name)}
        <div>
          <dt>{fact.name}</dt>
          <dd>{fact.value}</dd>
        </div>
      {/each}
    </dl>
  {/if}

  <DocTable title="Path variables" rows={pathVariables} />
  <DocTable title="Query parameters" rows={documentedRows(request.params)} />
  <DocTable
    title={request.kind === 'grpc' ? 'Metadata' : 'Headers'}
    rows={documentedRows(request.headers)}
  />
  <DocTable title="Body" rows={formRows(request)} />

  {#if body}
    <section class="block" aria-label={body.label}>
      <h5>{body.label}</h5>
      <pre class="mono">{body.text}</pre>
    </section>
  {/if}
  {#if request.kind === 'graphql' && request.graphqlVariables.trim() !== ''}
    <section class="block" aria-label="Variables">
      <h5>Variables</h5>
      <pre class="mono">{request.graphqlVariables}</pre>
    </section>
  {/if}

  {#if request.examples.length > 0}
    <section class="block" aria-label="Examples">
      <h5>Examples</h5>
      {#each request.examples as example, index (index)}
        <details>
          <summary>
            <span class="chip {statusTone(example.status)}">{example.status}</span>
            {example.name}
          </summary>
          {#if example.body === ''}
            <p class="muted">No body.</p>
          {:else}
            <pre class="mono">{example.body}</pre>
            {#if example.cut}
              <p class="muted">The rest of this example is left out here; the request keeps it.</p>
            {/if}
          {/if}
        </details>
      {/each}
    </section>
  {/if}
</article>

<style>
  .request {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    scroll-margin-top: 16px;
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
  }

  h4 {
    flex: 1 1 200px;
    min-width: 0;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .open {
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
  }

  .open:hover {
    border-color: var(--border-strong);
  }

  .url {
    margin: 0;
    color: var(--text-2);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 20px;
    margin: 0;
    font-size: 13px;
  }

  .facts div {
    display: flex;
    gap: 6px;
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h5 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  pre {
    max-height: 320px;
    margin: 0;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    color: var(--text);
    overflow: auto;
    white-space: pre;
  }

  details {
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
  }

  summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    color: var(--text-2);
    font-size: 13px;
    cursor: pointer;
  }

  details[open] summary {
    border-bottom: 1px solid var(--border-soft);
  }

  details pre {
    margin: 10px;
  }

  .chip {
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--surface-sunken);
    color: var(--text-2);
    font-family: var(--font-code);
    font-size: 12px;
    font-weight: 600;
  }

  .chip.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .chip.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .muted {
    margin: 10px;
    color: var(--text-muted);
    font-size: 12px;
  }
</style>
