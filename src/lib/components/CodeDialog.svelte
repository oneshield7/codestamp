<script lang="ts">
  import { onMount } from 'svelte';
  import type { Auth } from '../api/collections';
  import { resolveRequestForCode, toAppError } from '../api/http';
  import {
    CODE_LANGUAGES,
    environmentVariables,
    filledRequest,
    generateCode,
    isCodeLanguage,
    templateRequest,
    type CodeLanguage,
    type CodeRequest,
    type ValueSource,
  } from '../request/codegen';
  import { buildRequestInput, type RequestDraft } from '../request/draft';
  import CodeEditor from './CodeEditor.svelte';
  import Dropdown, { type DropdownOption } from './Dropdown.svelte';

  interface Props {
    draft: RequestDraft;
    collectionId: string;
    collectionAuth: Auth;
    onClose: () => void;
  }

  let { draft, collectionId, collectionAuth, onClose }: Props = $props();

  const LANGUAGE_KEY = 'codestamp.codeLanguage';

  const LANGUAGE_OPTIONS: DropdownOption<CodeLanguage>[] = CODE_LANGUAGES.map((option) => ({
    value: option.language,
    label: option.label,
  }));

  const SOURCE_OPTIONS: DropdownOption<ValueSource>[] = [
    { value: 'environment', label: 'Environment variables' },
    { value: 'template', label: 'As written, {{name}}' },
    { value: 'filled', label: 'Filled in (shows secrets)' },
  ];

  let dialog: HTMLDialogElement;
  let language = $state<CodeLanguage>(rememberedLanguage());
  // Always starts on environment variables, so secrets never show without being asked for.
  let source = $state<ValueSource>('environment');
  let resolved = $state<CodeRequest | null>(null);
  let status = $state<{ tone: 'ok' | 'err'; text: string } | null>(null);

  const request = $derived(source === 'filled' ? resolved : templateRequest(draft, collectionAuth));
  const code = $derived(request ? generateCode(language, request, source) : '');
  const editor = $derived(
    CODE_LANGUAGES.find((option) => option.language === language)?.editor ?? 'text',
  );
  const hint = $derived.by(() => {
    if (source === 'filled') {
      return 'Values are filled in as they would be sent now. Pre-request scripts are not run.';
    }
    if (source === 'template') {
      return 'Variables stay as {{name}}. Pre-request scripts are not run.';
    }
    const names = request ? environmentVariables(request, source).map((item) => item.env) : [];
    return names.length === 0
      ? 'This request uses no variables. Pre-request scripts are not run.'
      : `Reads ${names.join(', ')} from environment variables. Pre-request scripts are not run.`;
  });

  function rememberedLanguage(): CodeLanguage {
    try {
      const stored = localStorage.getItem(LANGUAGE_KEY);
      return isCodeLanguage(stored) ? stored : 'curl';
    } catch {
      return 'curl';
    }
  }

  function chooseLanguage(next: CodeLanguage): void {
    language = next;
    try {
      localStorage.setItem(LANGUAGE_KEY, next);
    } catch {
      // Storage refused (full or blocked): the choice still applies, it is just not remembered.
    }
  }

  onMount(() => {
    dialog.showModal();
  });

  // Filled in by the Rust core (variables, secrets and auth), the same way a send would.
  $effect(() => {
    if (source !== 'filled') {
      resolved = null;
      return;
    }
    const input = buildRequestInput(draft, { collectionId, requestId: null });
    const run = { cancelled: false };
    resolveRequestForCode(input).then(
      (result) => {
        if (!run.cancelled) {
          resolved = filledRequest(result);
          status = null;
        }
      },
      (caught: unknown) => {
        if (!run.cancelled) {
          resolved = null;
          status = { tone: 'err', text: toAppError(caught).message };
        }
      },
    );
    return () => {
      run.cancelled = true;
    };
  });

  async function copy(): Promise<void> {
    try {
      await navigator.clipboard.writeText(code);
      status = { tone: 'ok', text: 'Copied to the clipboard.' };
    } catch {
      status = { tone: 'err', text: 'Could not copy here. Select the code and copy it instead.' };
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="code-title"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <header class="head">
    <h2 id="code-title">Copy “{draft.name}” as code</h2>
    <span class="spacer"></span>
    <button type="button" class="close" aria-label="Close" onclick={onClose}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>
  </header>

  <div class="controls">
    <div class="field">
      <span class="label">Language</span>
      <Dropdown
        label="Language"
        variant="field"
        value={language}
        options={LANGUAGE_OPTIONS}
        onChange={chooseLanguage}
      />
    </div>
    <div class="field">
      <span class="label">Values</span>
      <Dropdown
        label="Values"
        variant="field"
        value={source}
        options={SOURCE_OPTIONS}
        onChange={(next: ValueSource) => (source = next)}
      />
    </div>
  </div>

  <div class="viewer">
    <CodeEditor value={code} label="Generated code" language={editor} readOnly />
  </div>

  <footer class="foot">
    {#if status}
      <span class="status {status.tone}" role="status">{status.text}</span>
    {:else}
      <span class="hint">{hint}</span>
    {/if}
    <span class="spacer"></span>
    <button type="button" class="primary" disabled={code === ''} onclick={() => void copy()}>
      Copy
    </button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(760px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
  }

  /* On short windows the code area gives up height, so Copy stays in view. */
  dialog[open] {
    display: flex;
    flex-direction: column;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  .head,
  .controls,
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
  }

  .head {
    border-bottom: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .spacer {
    flex: 1;
  }

  .close {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .close:hover {
    background: var(--surface-sunken);
  }

  .close path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .controls {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    align-items: end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .viewer {
    flex: 0 1 340px;
    min-height: 120px;
    resize: vertical;
    overflow: hidden;
    margin: 0 18px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
  }

  .hint,
  .status {
    font-size: 13px;
    color: var(--text-muted);
  }

  .status.ok {
    color: var(--ok);
  }

  .status.err {
    color: var(--err);
  }

  .primary {
    height: 36px;
    padding: 0 18px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
  }

  .primary:disabled {
    opacity: 0.6;
  }
</style>
