<script lang="ts">
  import {
    revealSecret,
    setVariableValue,
    variableDetail,
    type VariableDetail,
  } from '../api/environments';
  import { toAppError } from '../api/http';
  import { focusMode } from '../state/focus.svelte';
  import type { HoveredVariable, VariableHover } from '../state/variable-hover.svelte';
  import { variableRevision } from '../state/variable-revision.svelte';

  interface Props {
    shown: HoveredVariable;
    collectionId: string | null;
    hover: VariableHover;
    /** Escape inside the popover: the field it belongs to takes focus again. */
    onDismiss: () => void;
  }

  let { shown, collectionId, hover, onDismiss }: Props = $props();

  const WIDTH = 320;
  const GAP = 6;
  const EDGE = 8;
  /** Room the popover needs below the reference before it opens above it instead. */
  const ROOM_BELOW = 180;

  const inputId = `variable-value-${crypto.randomUUID()}`;
  let root: HTMLDivElement;
  let detail = $state<VariableDetail | null>(null);
  let problem = $state<string | null>(null);
  let draft = $state('');
  let saved = $state('');
  let revealed = $state(false);
  let busy = $state(false);
  let status = $state('');

  $effect(() => {
    void load(shown.name, collectionId);
  });

  async function load(name: string, collection: string | null): Promise<void> {
    detail = null;
    problem = null;
    status = '';
    revealed = false;
    try {
      const next = await variableDetail(collection, name);
      if (name === shown.name) {
        detail = next;
        draft = next.value;
        saved = next.value;
      }
    } catch (error) {
      problem = toAppError(error).message;
    }
  }

  const where = $derived.by(() => {
    switch (detail?.source) {
      case 'global':
        return 'Globals';
      case 'collection':
        return 'Collection';
      case 'environment':
        return detail.setName === null ? 'Environment' : `Environment · ${detail.setName}`;
      case 'dynamic':
        return 'Dynamic';
      case null:
        return 'Not defined';
      default:
        return '';
    }
  });
  const editable = $derived(detail !== null && detail.id !== null && !detail.locked);
  const canReveal = $derived(
    detail?.isSecret === true && !(focusMode.active && focusMode.hideSecrets),
  );

  const left = $derived(
    Math.max(EDGE, Math.min(shown.anchor.left, window.innerWidth - WIDTH - EDGE)),
  );
  const below = $derived(shown.anchor.bottom + GAP + ROOM_BELOW <= window.innerHeight);

  async function save(): Promise<void> {
    const id = detail?.id;
    if (!id || busy || draft === saved) {
      return;
    }
    busy = true;
    problem = null;
    try {
      await setVariableValue(id, draft);
      saved = draft;
      status = 'Saved';
      variableRevision.bump();
      if (detail?.sharedBy) {
        variableRevision.sharedChanged(detail.sharedBy);
      }
    } catch (error) {
      problem = toAppError(error).message;
    } finally {
      busy = false;
    }
  }

  // A secret's value is only read when asked for, and only when nothing was typed over it yet.
  async function toggleSecret(): Promise<void> {
    const id = detail?.id;
    if (revealed || !id) {
      revealed = false;
      return;
    }
    if (draft === saved) {
      try {
        const value = await revealSecret(id);
        draft = value;
        saved = value;
      } catch (error) {
        problem = toAppError(error).message;
        return;
      }
    }
    revealed = true;
  }

  function onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      // Only closes the popover; without this, Escape would also leave focus mode.
      event.preventDefault();
      event.stopPropagation();
      hover.close();
      onDismiss();
    } else if (event.key === 'Enter' && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      event.stopPropagation();
      void save();
    }
  }
</script>

<div
  bind:this={root}
  class="popover"
  role="dialog"
  tabindex="-1"
  aria-label="Variable {shown.name}"
  style:left="{left}px"
  style:top={below ? `${String(shown.anchor.bottom + GAP)}px` : undefined}
  style:bottom={below ? undefined : `${String(window.innerHeight - shown.anchor.top + GAP)}px`}
  onpointerenter={() => {
    hover.pointerOnPopover(true);
  }}
  onpointerleave={() => {
    hover.pointerOnPopover(false);
  }}
  onfocusin={() => {
    hover.focusInPopover(true);
  }}
  onfocusout={(event) => {
    if (!(event.relatedTarget instanceof Node && root.contains(event.relatedTarget))) {
      hover.focusInPopover(false);
    }
  }}
  onkeydown={onKey}
>
  <div class="head">
    <span class="name">{`{{${shown.name}}}`}</span>
    <span class="where">{where}</span>
  </div>

  {#if problem !== null}
    <p class="problem" role="alert">{problem}</p>
  {/if}

  {#if detail === null}
    {#if problem === null}
      <p class="note">Reading the value…</p>
    {/if}
  {:else if detail.source === null}
    <p class="note">Not defined in Globals, this collection, or its active environment.</p>
  {:else if detail.source === 'dynamic'}
    <p class="text">{detail.value}</p>
    <p class="note">Codestamp makes a new value every time the request is sent.</p>
  {:else if detail.locked}
    <p class="note">
      Restricted to an access group this device is not in. Only its members can read or change it.
    </p>
  {:else if editable}
    <label class="visually-hidden" for={inputId}>Value of {shown.name}</label>
    <div class="field">
      <input
        id={inputId}
        type={detail.isSecret && !revealed ? 'password' : 'text'}
        bind:value={draft}
        placeholder={detail.isSecret && !revealed ? 'Hidden · type to replace it' : 'Empty'}
        spellcheck="false"
        autocomplete="off"
        oninput={() => (status = '')}
      />
      {#if canReveal}
        <button type="button" class="reveal" onclick={() => void toggleSecret()}
          >{revealed ? 'Hide' : 'Show'}</button
        >
      {/if}
    </div>
    <div class="actions">
      <span class="status" role="status">{status}</span>
      <button
        type="button"
        class="primary"
        disabled={busy || draft === saved}
        onclick={() => void save()}>{busy ? 'Saving…' : 'Save'}</button
      >
    </div>
  {/if}
</div>

<style>
  .popover {
    position: fixed;
    z-index: 31;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 320px;
    max-width: calc(100vw - 16px);
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    font-size: 13px;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-code);
    font-size: var(--font-size-code);
    color: var(--var);
  }

  .where {
    margin-left: auto;
    flex: none;
    font-size: 12px;
    color: var(--text-muted);
  }

  .note,
  .text,
  .problem {
    margin: 0;
  }

  .note {
    color: var(--text-muted);
  }

  .text {
    color: var(--text);
  }

  .problem {
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: var(--err-bg);
    color: var(--err);
  }

  .field {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .field input {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font-family: var(--font-code);
    font-size: var(--font-size-code);
  }

  .field input:focus {
    border-color: var(--focus);
    outline: none;
  }

  .reveal {
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
  }

  .reveal:hover {
    border-color: var(--border-strong);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .status {
    margin-right: auto;
    font-size: 12px;
    color: var(--ok);
  }

  .primary {
    height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
  }

  .primary:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }
</style>
