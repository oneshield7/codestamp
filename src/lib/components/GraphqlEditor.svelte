<script lang="ts">
  import type { GraphQLSchema } from 'graphql';
  import { graphqlSchema, toAppError } from '../api/http';
  import { buildRequestInput, type RequestDraft } from '../request/draft';
  import { knownSchema, rememberSchema } from '../state/graphql-schemas';
  import CodeEditor from './CodeEditor.svelte';

  interface Props {
    draft: RequestDraft;
    collectionId: string;
    requestId: string;
  }

  let { draft = $bindable(), collectionId, requestId }: Props = $props();

  let schema = $state.raw<GraphQLSchema | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // A schema fetched earlier for this endpoint is used again.
  $effect(() => {
    schema = knownSchema(collectionId, draft.url);
  });

  const typeCount = $derived(
    schema ? Object.keys(schema.getTypeMap()).filter((name) => !name.startsWith('__')).length : 0,
  );

  async function fetchSchema(): Promise<void> {
    loading = true;
    error = null;
    try {
      const { buildClientSchema, getIntrospectionQuery } = await import('graphql');
      const input = buildRequestInput(draft, { collectionId, requestId });
      const text = await graphqlSchema({
        ...input,
        body: { mode: 'graphql', query: getIntrospectionQuery(), variables: '' },
      });
      const answer = JSON.parse(text) as {
        data?: Parameters<typeof buildClientSchema>[0];
        errors?: { message?: string }[];
      };
      if (!answer.data) {
        error = answer.errors?.[0]?.message ?? 'The answer holds no schema';
        return;
      }
      const built = buildClientSchema(answer.data);
      rememberSchema(collectionId, draft.url, built);
      schema = built;
    } catch (caught) {
      error =
        caught instanceof SyntaxError
          ? 'The answer is not JSON'
          : toAppError(caught).message || 'The schema could not be read';
    } finally {
      loading = false;
    }
  }
</script>

<div class="graphql">
  <div class="schema-bar">
    <span class="schema-status" role="status">
      {#if schema}
        Schema loaded · {typeCount} types · completion and checks are on
      {:else}
        No schema yet · fetch it for completion and checks as you type
      {/if}
    </span>
    <button
      type="button"
      class="quiet"
      disabled={loading || draft.url.trim() === ''}
      onclick={() => void fetchSchema()}
    >
      {loading ? 'Fetching…' : schema ? 'Fetch again' : 'Fetch schema'}
    </button>
  </div>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  <div class="editor query">
    <CodeEditor
      value={draft.body}
      label="GraphQL query"
      language="graphql"
      {schema}
      variablesOf={collectionId}
      onChange={(next: string) => (draft.body = next)}
    />
  </div>
  <span class="label">Variables (JSON)</span>
  <div class="editor variables">
    <CodeEditor
      value={draft.graphqlVariables}
      label="GraphQL variables"
      language="json"
      variablesOf={collectionId}
      onChange={(next: string) => (draft.graphqlVariables = next)}
    />
  </div>
</div>

<style>
  .graphql {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .schema-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
  }

  .schema-status {
    flex: 1 1 240px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .quiet {
    height: 28px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
  }

  .editor {
    min-height: 100px;
    resize: vertical;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    padding: 4px;
  }

  .query {
    height: 260px;
  }

  .variables {
    height: 120px;
  }

  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }

  .error {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }
</style>
