<script lang="ts">
  import type { CheckOp, CheckSubject, ExtractSource, FlowStep, OnFailure } from '../api/flows';
  import { prettyJson } from '../format/json';
  import type { ValueState } from '../flow/lineage';
  import { stepMark, stepResult } from '../flow/outcome';
  import {
    ON_FAILURE_OPTIONS,
    SOURCE_OPTIONS,
    SUBJECT_OPTIONS,
    hasPath,
    newCheck,
    newExtract,
    opOptions,
    pathPlaceholder,
    setSubject,
    valuePlaceholder,
  } from '../flow/rules';
  import { isSensitiveName, maskSecret } from '../privacy/secrets';
  import { stampLabel } from '../request/kinds';
  import { stampTone } from '../request/methods';
  import type { StepRun } from '../state/flows.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import Dropdown from './Dropdown.svelte';
  import FlowStepMark from './FlowStepMark.svelte';
  import FlowValueChip from './FlowValueChip.svelte';
  import IconButton from './IconButton.svelte';
  import MethodStamp from './MethodStamp.svelte';

  interface Props {
    step: FlowStep;
    /** Counted from 1. */
    number: number;
    total: number;
    run: StepRun | undefined;
    /** The `{{names}}` this step sends that earlier steps pass on, with the step (from 1). */
    received: { name: string; from: number; state: ValueState }[];
    disabled: boolean;
    onChange: () => void;
    onMove: (delta: -1 | 1) => void;
    onRemove: () => void;
    onReplace: () => void;
    onOpenRequest: () => void;
  }

  let {
    step,
    number,
    total,
    run,
    received,
    disabled,
    onChange,
    onMove,
    onRemove,
    onReplace,
    onOpenRequest,
  }: Props = $props();

  type StepTab = 'checks' | 'extracts' | 'response';
  let tab = $state<StepTab>('checks');

  const done = $derived(run?.state === 'done' ? run : null);
  const mark = $derived(stepMark(run));
  const shownTab = $derived(tab === 'response' && !done ? 'checks' : tab);
  const responseBody = $derived(done ? (prettyJson(done.body) ?? done.body) : '');

  function shownValue(name: string, value: string): string {
    return isSensitiveName(name) ? maskSecret(value) : value;
  }

  function addCheck(): void {
    step.checks.push(newCheck());
    tab = 'checks';
    onChange();
  }

  function addExtract(): void {
    step.extracts.push(newExtract());
    tab = 'extracts';
    onChange();
  }
</script>

<article class="step {mark}" aria-label="Step {number}: {step.request?.name ?? 'missing request'}">
  <header class="head">
    <FlowStepMark {mark} />
    <span class="number mono">{String(number).padStart(2, '0')}</span>
    {#if step.request}
      <MethodStamp
        label={stampLabel(step.request.kind, step.request.method)}
        tone={stampTone(step.request.kind, step.request.method)}
      />
      <span class="text">
        <span class="name">{step.request.name}</span>
        <span class="place mono"
          >{step.request.collectionName} · {step.request.url || 'No URL yet'}</span
        >
      </span>
    {:else}
      <span class="text">
        <span class="name missing">Request deleted</span>
        <span class="place">Choose another request or remove this step</span>
      </span>
    {/if}
    <span class="result" class:bad={done?.status === 'failed'}>{stepResult(run, 'full')}</span>
    <div class="tools">
      <IconButton
        label="Move left"
        disabled={disabled || number === 1}
        onclick={() => {
          onMove(-1);
        }}
      >
        <svg width="16" height="16" viewBox="0 0 24 24"><path d="M15 6l-6 6 6 6" /></svg>
      </IconButton>
      <IconButton
        label="Move right"
        disabled={disabled || number === total}
        onclick={() => {
          onMove(1);
        }}
      >
        <svg width="16" height="16" viewBox="0 0 24 24"><path d="M9 6l6 6-6 6" /></svg>
      </IconButton>
      <IconButton label="Remove step" danger align="end" {disabled} onclick={onRemove}>
        <svg width="16" height="16" viewBox="0 0 24 24">
          <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
        </svg>
      </IconButton>
    </div>
  </header>

  <div class="body">
    {#if received.length > 0}
      <p class="uses">
        <span class="label">Uses</span>
        {#each received as value (value.name)}
          <span class="use"
            ><FlowValueChip name={value.name} state={value.state} /> from step {value.from}</span
          >
        {/each}
      </p>
    {/if}
    <div class="settings">
      <span class="label">If this step fails</span>
      <Dropdown
        label="If this step fails"
        value={step.onFailure}
        options={ON_FAILURE_OPTIONS}
        {disabled}
        onChange={(next: OnFailure) => {
          step.onFailure = next;
          onChange();
        }}
      />
      <span class="spacer"></span>
      <button type="button" class="link" {disabled} onclick={onReplace}>Change request</button>
      {#if step.request}
        <button type="button" class="link" onclick={onOpenRequest}>Open request</button>
      {/if}
    </div>

    <div class="tabs" role="tablist" aria-label="Step {number} sections">
      <button
        type="button"
        role="tab"
        aria-selected={shownTab === 'checks'}
        class:on={shownTab === 'checks'}
        onclick={() => (tab = 'checks')}
        >Checks{step.checks.length > 0 ? ` · ${String(step.checks.length)}` : ''}</button
      >
      <button
        type="button"
        role="tab"
        aria-selected={shownTab === 'extracts'}
        class:on={shownTab === 'extracts'}
        onclick={() => (tab = 'extracts')}
        >Pass on{step.extracts.length > 0 ? ` · ${String(step.extracts.length)}` : ''}</button
      >
      {#if done && done.status !== 'skipped'}
        <button
          type="button"
          role="tab"
          aria-selected={shownTab === 'response'}
          class:on={shownTab === 'response'}
          onclick={() => (tab = 'response')}>Response</button
        >
      {/if}
    </div>

    {#if shownTab === 'checks'}
      <div class="rules">
        {#each step.checks as check, index (index)}
          <div class="rule">
            <Dropdown
              label="Check {index + 1}: what"
              value={check.subject}
              options={SUBJECT_OPTIONS}
              {disabled}
              onChange={(next: CheckSubject) => {
                setSubject(check, next);
                onChange();
              }}
            />
            {#if hasPath(check.subject)}
              <input
                class="input mono"
                aria-label="Check {index + 1}: {check.subject === 'header'
                  ? 'header name'
                  : 'JSON path'}"
                placeholder={check.subject === 'body'
                  ? `${pathPlaceholder('body')} (empty: whole body)`
                  : pathPlaceholder('header')}
                spellcheck="false"
                {disabled}
                bind:value={check.path}
                oninput={onChange}
              />
            {/if}
            <Dropdown
              label="Check {index + 1}: how"
              value={check.op}
              options={opOptions(check.subject)}
              {disabled}
              onChange={(next: CheckOp) => {
                check.op = next;
                onChange();
              }}
            />
            {#if check.op !== 'exists'}
              <input
                class="input mono value"
                aria-label="Check {index + 1}: expected value"
                placeholder={valuePlaceholder(check.subject)}
                spellcheck="false"
                {disabled}
                bind:value={check.value}
                oninput={onChange}
              />
            {/if}
            <IconButton
              label="Remove check {index + 1}"
              danger
              align="end"
              {disabled}
              onclick={() => {
                step.checks.splice(index, 1);
                onChange();
              }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg
              >
            </IconButton>
          </div>
        {/each}
        <button type="button" class="add" {disabled} onclick={addCheck}>+ Add check</button>
        {#if step.checks.length === 0}
          <p class="hint">
            Without checks, the step passes when the status is below 400 and the request's own tests
            pass.
          </p>
        {/if}
      </div>
    {:else if shownTab === 'extracts'}
      <div class="rules">
        {#each step.extracts as extract, index (index)}
          <div class="rule">
            <input
              class="input mono name-input"
              aria-label="Value {index + 1}: variable name"
              placeholder="token"
              spellcheck="false"
              {disabled}
              bind:value={extract.name}
              oninput={onChange}
            />
            <span class="from">from</span>
            <Dropdown
              label="Value {index + 1}: from"
              value={extract.source}
              options={SOURCE_OPTIONS}
              {disabled}
              onChange={(next: ExtractSource) => {
                extract.source = next;
                onChange();
              }}
            />
            <input
              class="input mono"
              aria-label="Value {index + 1}: {extract.source === 'header'
                ? 'header name'
                : 'JSON path'}"
              placeholder={pathPlaceholder(extract.source)}
              spellcheck="false"
              {disabled}
              bind:value={extract.path}
              oninput={onChange}
            />
            <IconButton
              label="Remove value {index + 1}"
              danger
              align="end"
              {disabled}
              onclick={() => {
                step.extracts.splice(index, 1);
                onChange();
              }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg
              >
            </IconButton>
          </div>
        {/each}
        <button type="button" class="add" {disabled} onclick={addExtract}
          >+ Add value to pass on</button
        >
        <p class="hint">
          Steps below use it as <span class="mono">{'{{name}}'}</span>. For this run it wins over
          environment values, and it is not saved anywhere.
        </p>
      </div>
    {:else if done}
      <div class="response">
        <dl class="headers mono">
          {#each done.headers as header, index (index)}
            <dt>{header.name}</dt>
            <dd>{isSensitiveName(header.name) ? maskSecret(header.value) : header.value}</dd>
          {/each}
        </dl>
        <div class="viewer">
          <CodeEditor
            value={responseBody}
            label="Step {number} response body"
            language={prettyJson(done.body) === null ? 'text' : 'json'}
            readOnly
          />
        </div>
      </div>
    {/if}

    {#if done && done.status !== 'skipped' && shownTab !== 'response'}
      <section class="last" aria-label="Last run">
        <h3>Last run</h3>
        <ul>
          {#each done.checks as result, index (index)}
            <li class:bad={!result.passed}>
              <span class="tick">{result.passed ? '✓' : '✗'}</span>
              {result.label}{result.detail ? `: ${result.detail}` : ''}
            </li>
          {/each}
          {#each done.tests as test, index (index)}
            <li class:bad={!test.passed}>
              <span class="tick">{test.passed ? '✓' : '✗'}</span>
              {test.name}{test.error ? `: ${test.error}` : ''}
            </li>
          {/each}
          {#each done.passedOn as value (value.name)}
            <li class="value mono">
              <span class="tick">→</span>{value.name} = {shownValue(value.name, value.value)}
            </li>
          {/each}
          {#each done.errors as error, index (index)}
            <li class="bad"><span class="tick">!</span>{error}</li>
          {/each}
          {#if done.note}
            <li class="muted">{done.note}</li>
          {/if}
        </ul>
      </section>
    {/if}
  </div>
</article>

<style>
  .step {
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .step.failed {
    border-color: var(--err-line);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 8px 11px 12px;
  }

  .number {
    font-size: 12px;
    color: var(--text-muted);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 500;
  }

  .name.missing {
    color: var(--err);
  }

  .place {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }

  .result {
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }

  .result.bad {
    color: var(--err);
  }

  .tools {
    display: flex;
    gap: 2px;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 10px 16px 16px;
    border-top: 1px solid var(--border-soft);
  }

  .settings {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
  }

  .label,
  .from {
    font-size: 13px;
    color: var(--text-2);
  }

  .uses {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin: 0;
  }

  .use {
    font-size: 12px;
    color: var(--text-muted);
  }

  .spacer {
    flex: 1;
  }

  .link {
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
  }

  .link:hover:not(:disabled) {
    text-decoration: underline;
  }

  .tabs {
    display: flex;
    gap: 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .tabs button {
    padding: 8px 2px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
  }

  .tabs button.on {
    color: var(--text);
    border-bottom-color: var(--text);
    font-weight: 500;
  }

  .rules {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .rule {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .input {
    flex: 1 1 160px;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .input:focus {
    border-color: var(--focus);
    outline: none;
  }

  .input.value {
    flex: 0 1 180px;
  }

  .input.name-input {
    flex: 0 1 150px;
  }

  .add {
    align-self: flex-start;
    height: 30px;
    padding: 0 12px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
  }

  .add:hover:not(:disabled) {
    border-color: var(--text-muted);
    color: var(--text);
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .last {
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--surface-muted);
  }

  .last h3 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .last ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  .last li {
    display: flex;
    gap: 8px;
    overflow-wrap: anywhere;
    color: var(--text-2);
  }

  .last li.bad {
    color: var(--err);
  }

  .last li.muted {
    color: var(--text-muted);
  }

  .last li.value {
    font-size: 12px;
  }

  .tick {
    flex: none;
    width: 14px;
    color: var(--ok);
  }

  .last li.bad .tick {
    color: var(--err);
  }

  .last li.value .tick {
    color: var(--text-muted);
  }

  .response {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .headers {
    display: grid;
    grid-template-columns: minmax(120px, max-content) 1fr;
    gap: 4px 14px;
    margin: 0;
    max-height: 160px;
    overflow: auto;
    font-size: 12px;
  }

  .headers dt {
    color: var(--text-muted);
  }

  .headers dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .viewer {
    height: 260px;
    min-height: 120px;
    resize: vertical;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    padding: 4px;
  }
</style>
