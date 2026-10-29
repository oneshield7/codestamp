<script lang="ts">
  import type { RuleResult } from '../../api/catalog';
  import { stampLabel } from '../../request/kinds';
  import { stampTone } from '../../request/methods';
  import MethodStamp from '../MethodStamp.svelte';

  interface Props {
    rules: RuleResult[];
    onOpenRequest: (requestId: string) => void;
    onOpenCollection: () => void;
  }

  let { rules, onOpenRequest, onOpenCollection }: Props = $props();

  /** Findings past this many stay folded until asked for, so one rule cannot fill the page. */
  const SHOWN_FINDINGS = 5;

  let expanded = $state<string[]>([]);

  function failing(rule: RuleResult): number {
    return new Set(rule.findings.map((finding) => finding.request?.id ?? 'collection')).size;
  }
</script>

<ul class="rules">
  {#each rules as rule (rule.key)}
    {@const failed = failing(rule)}
    {@const open = expanded.includes(rule.key)}
    {@const shown = open ? rule.findings : rule.findings.slice(0, SHOWN_FINDINGS)}
    <li class="rule">
      <div class="head">
        <span class="mark" class:ok={failed === 0} class:bad={failed > 0} aria-hidden="true">
          <svg width="14" height="14" viewBox="0 0 24 24">
            <path d={failed === 0 ? 'M5 12l5 5 9-10' : 'M6 6l12 12M18 6L6 18'} />
          </svg>
        </span>
        <div class="text">
          <h3>
            {rule.title}
            <span class="severity {rule.severity}"
              >{rule.severity === 'error' ? 'Error' : 'Warning'}</span
            >
          </h3>
          <p>{rule.description}</p>
        </div>
        <span class="count">
          {#if rule.checked === 0}
            Nothing to check
          {:else if failed === 0}
            All {rule.checked} pass
          {:else}
            {failed} of {rule.checked} fail
          {/if}
        </span>
      </div>
      {#if rule.findings.length > 0}
        <ul class="findings">
          {#each shown as finding, index (`${finding.request?.id ?? 'collection'}-${String(index)}`)}
            <li>
              {#if finding.request}
                {@const request = finding.request}
                <MethodStamp
                  label={stampLabel(request.kind, request.method)}
                  tone={stampTone(request.kind, request.method)}
                  size="sm"
                />
                <button
                  type="button"
                  class="target"
                  onclick={() => {
                    onOpenRequest(request.id);
                  }}>{request.name}</button
                >
              {:else}
                <button type="button" class="target" onclick={onOpenCollection}
                  >Collection auth</button
                >
              {/if}
              <span class="detail">{finding.detail}</span>
            </li>
          {/each}
        </ul>
        {#if rule.findings.length > SHOWN_FINDINGS}
          <button
            type="button"
            class="more"
            aria-expanded={open}
            onclick={() => {
              expanded = open
                ? expanded.filter((key) => key !== rule.key)
                : [...expanded, rule.key];
            }}
          >
            {open ? 'Show fewer' : `Show all ${String(rule.findings.length)}`}
          </button>
        {/if}
      {/if}
    </li>
  {/each}
</ul>

<style>
  .rules {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .rule {
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 12px;
  }

  .mark {
    flex: none;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .mark svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .mark.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .mark.bad {
    background: var(--err-bg);
    color: var(--err);
  }

  .text {
    flex: 1 1 320px;
    min-width: 0;
  }

  h3 {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .text p {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .severity {
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
  }

  .severity.error {
    background: var(--err-bg);
    color: var(--err);
  }

  .severity.warning {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .count {
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }

  .findings {
    margin: 12px 0 0 36px;
    padding: 0;
    list-style: none;
    border-top: 1px solid var(--border-soft);
  }

  .findings li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-soft);
    font-size: 13px;
  }

  .target {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-weight: 500;
    text-align: left;
  }

  .target:hover {
    text-decoration: underline;
  }

  .detail {
    flex: 1 1 220px;
    min-width: 0;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .more {
    margin: 8px 0 0 36px;
    padding: 4px 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 500;
  }

  .more:hover {
    background: var(--surface-sunken);
  }
</style>
