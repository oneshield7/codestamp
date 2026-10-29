<script lang="ts">
  import type { SoapVersion } from '../api/collections';
  import { pickWsdlFile, type WsdlDefinition } from '../api/soap';
  import { fileName } from '../request/body';
  import { newRequestName, type RequestDraft } from '../request/draft';
  import { notices } from '../state/notices.svelte';
  import type { WsdlLoader } from '../state/soap.svelte';
  import Dropdown from './Dropdown.svelte';

  interface Props {
    draft: RequestDraft;
    loader: WsdlLoader;
  }

  let { draft = $bindable(), loader }: Props = $props();

  const VERSIONS: SoapVersion[] = ['1.1', '1.2'];
  const NO_OPERATION = '';
  /** Warnings past this many are summed up, so a WSDL with many broken imports stays readable. */
  const SHOWN_WARNINGS = 3;

  // A WSDL loaded earlier from the same place lists its operations again.
  $effect(() => {
    loader.recall(draft.soap.wsdl);
  });

  const definition = $derived(loader.definition);
  const isFile = $derived(
    draft.soap.wsdl.trim() !== '' && !/^https?:\/\//i.test(draft.soap.wsdl.trim()),
  );
  const chosen = $derived(loader.find(draft.soap.binding, draft.soap.operation));
  const chosenKey = $derived(
    draft.soap.operation === '' ? NO_OPERATION : `${draft.soap.binding}/${draft.soap.operation}`,
  );
  const operationOptions = $derived.by(() => {
    const options = (definition?.interfaces ?? []).flatMap((binding) =>
      binding.operations.map((operation) => ({
        value: `${binding.name}/${operation.name}`,
        label: `${operation.name} · ${binding.name} · SOAP ${binding.version}`,
      })),
    );
    if (!options.some((option) => option.value === chosenKey)) {
      options.unshift({
        value: chosenKey,
        label: draft.soap.operation === '' ? 'Choose an operation' : draft.soap.operation,
      });
    }
    return options;
  });
  const warnings = $derived(definition?.warnings ?? []);

  // Binding and operation names are XML names, which cannot contain "/".
  function choose(key: string): void {
    const [binding = '', operation = ''] = key.split('/');
    const next = loader.find(binding, operation);
    if (!next || key === chosenKey) {
      return;
    }
    // An envelope or name the user wrote is kept; an empty one or the last sample is replaced.
    const previous = chosen;
    const sampleUntouched = draft.body.trim() === '' || draft.body === previous?.operation.envelope;
    const nameUntouched =
      draft.name === newRequestName('soap') || draft.name === previous?.operation.name;
    draft.soap = {
      ...draft.soap,
      version: next.binding.version,
      binding,
      operation,
      action: next.operation.action,
    };
    if (sampleUntouched) {
      draft.body = next.operation.envelope;
    }
    if (nameUntouched) {
      draft.name = next.operation.name;
    }
    const endpoint = next.binding.endpoints.at(0);
    if (draft.url.trim() === '' && endpoint) {
      draft.url = endpoint;
    }
  }

  /** The first operation of a freshly loaded WSDL is chosen, so its sample shows right away. */
  function chooseFirst(loaded: WsdlDefinition): void {
    const binding = loaded.interfaces.at(0);
    const operation = binding?.operations.at(0);
    if (draft.soap.operation === '' && binding && operation) {
      choose(`${binding.name}/${operation.name}`);
    }
  }

  async function load(): Promise<void> {
    const loaded = await loader.load(draft.soap.wsdl);
    if (loaded) {
      chooseFirst(loaded);
    }
  }

  async function chooseFile(): Promise<void> {
    await notices.guard(async () => {
      const picked = await pickWsdlFile();
      if (picked) {
        draft.soap.wsdl = picked.path;
        await load();
      }
    });
  }

  function useSample(): void {
    if (chosen) {
      draft.body = chosen.operation.envelope;
    }
  }
</script>

<div class="soap">
  <div class="wsdl">
    {#if isFile}
      <span class="file mono" title="WSDL file on this device">
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 3h8l4 4v14H6zM14 3v4h4" />
        </svg>
        {fileName(draft.soap.wsdl)}
      </span>
      <button type="button" class="link" onclick={() => (draft.soap.wsdl = '')}>Use a URL</button>
    {:else}
      <label class="url">
        <span class="visually-hidden">WSDL URL</span>
        <input
          class="mono"
          bind:value={draft.soap.wsdl}
          placeholder="WSDL URL, e.g. https://…/Service.asmx?wsdl"
          spellcheck="false"
          onkeydown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              void load();
            }
          }}
        />
      </label>
    {/if}
    <button type="button" class="ghost" onclick={() => void chooseFile()}>Choose file…</button>
    <button
      type="button"
      class="ghost"
      disabled={loader.loading || draft.soap.wsdl.trim() === ''}
      title={draft.soap.wsdl.trim() === ''
        ? 'Enter the WSDL URL or choose a file first'
        : undefined}
      onclick={() => void load()}
    >
      {loader.loading ? 'Loading…' : definition ? 'Reload WSDL' : 'Load WSDL'}
    </button>
  </div>

  {#if loader.error}
    <p class="error" role="alert">{loader.error}</p>
  {/if}
  {#if warnings.length > 0}
    <ul class="warnings">
      {#each warnings.slice(0, SHOWN_WARNINGS) as warning, index (index)}
        <li>{warning}</li>
      {/each}
      {#if warnings.length > SHOWN_WARNINGS}
        <li>…and {warnings.length - SHOWN_WARNINGS} more</li>
      {/if}
    </ul>
  {/if}

  <div class="row">
    <div class="operation">
      <span class="label">Operation</span>
      <Dropdown
        variant="field"
        label="Operation"
        value={chosenKey}
        options={operationOptions}
        disabled={!definition}
        onChange={choose}
      />
    </div>
    <div class="version">
      <span class="label">SOAP version</span>
      <div class="segmented" role="group" aria-label="SOAP version">
        {#each VERSIONS as version (version)}
          <button
            type="button"
            class:on={draft.soap.version === version}
            aria-pressed={draft.soap.version === version}
            onclick={() => (draft.soap.version = version)}>{version}</button
          >
        {/each}
      </div>
    </div>
  </div>
  <label class="action">
    <span class="label">SOAPAction</span>
    <input class="mono" bind:value={draft.soap.action} spellcheck="false" placeholder="(none)" />
  </label>
  <p class="hint">
    {#if chosen?.operation.documentation}
      {chosen.operation.documentation}
    {:else if !definition && draft.soap.operation !== ''}
      Load the WSDL to switch operations or get a fresh sample envelope.
    {:else}
      SOAP 1.1 sends the action as a SOAPAction header, SOAP 1.2 inside the Content-Type.
    {/if}
  </p>
  {#if chosen}
    <button type="button" class="link reset" onclick={useSample}>
      Reset the envelope to the sample
    </button>
  {/if}
</div>

<style>
  .soap {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 14px;
    border-bottom: 1px dashed var(--border);
  }

  .wsdl,
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px 12px;
  }

  .url {
    flex: 1 1 280px;
    min-width: 0;
  }

  input {
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
  }

  input:focus {
    border-color: var(--focus);
    outline: none;
  }

  .file {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 320px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .ghost {
    height: 34px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
    white-space: nowrap;
  }

  .ghost:hover:not(:disabled) {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .ghost:disabled {
    color: var(--text-muted);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .reset {
    align-self: flex-start;
  }

  .operation {
    flex: 1 1 320px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .version,
  .action {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    font-size: 13px;
    color: var(--text-2);
  }

  .segmented {
    display: inline-flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .segmented button {
    padding: 5px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
  }

  .segmented button.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 500;
    box-shadow: var(--shadow-segment);
  }

  .error {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .warnings {
    margin: 0;
    padding: 8px 12px 8px 28px;
    border-radius: var(--radius-md);
    background: var(--warn-bg);
    color: var(--warn);
    font-size: 13px;
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }
</style>
