<script lang="ts">
  import { tick } from 'svelte';
  import { variableHints, type HintSource, type VariableHint } from '../api/environments';
  import { notices } from '../state/notices.svelte';
  import { preferenceSettings } from '../state/preferences.svelte';
  import { variableNames } from '../state/variable-names.svelte';
  import VariableText from './VariableText.svelte';

  interface Props {
    value: string;
    label: string;
    collectionId: string | null;
    variant: 'url' | 'cell' | 'field';
    id?: string;
    placeholder?: string;
    /** Shows dots instead of the value, e.g. for tokens while presenting. */
    masked?: boolean;
    onInput?: () => void;
  }

  let {
    value = $bindable(),
    label,
    collectionId,
    variant,
    id,
    placeholder = '',
    masked = false,
    onInput,
  }: Props = $props();

  const MAX_SUGGESTIONS = 8;
  const OPEN_PATTERN = /\{\{\s*(\$?[\w.-]*)$/;
  const SCOPE_LABELS = new Map<HintSource, string>([
    ['global', 'Globals'],
    ['collection', 'Collection'],
    ['environment', 'Environment'],
    ['dynamic', 'Dynamic'],
  ]);
  const listId = `variables-${crypto.randomUUID()}`;

  let input: HTMLInputElement;
  let hints: VariableHint[] = $state([]);
  let typed: string | null = $state(null);
  let highlighted = $state(0);
  let position = $state({ top: 0, left: 0, width: 0 });

  const matches = $derived(typed === null ? [] : rank(hints, typed).slice(0, MAX_SUGGESTIONS));
  const activeHint = $derived(matches.at(highlighted));
  // A plain input cannot colour part of its text, so references are drawn on a copy behind it.
  const painted = $derived(!masked && value.includes('{{'));
  let scrollLeft = $state(0);

  $effect(() => {
    if (painted) {
      variableNames.ensure(collectionId);
    }
  });

  function syncScroll(): void {
    scrollLeft = input.scrollLeft;
  }

  function rank(all: VariableHint[], text: string): VariableHint[] {
    const needle = text.toLowerCase();
    const starts = all.filter((hint) => hint.name.toLowerCase().startsWith(needle));
    const contains = all.filter(
      (hint) =>
        !hint.name.toLowerCase().startsWith(needle) && hint.name.toLowerCase().includes(needle),
    );
    return [...starts, ...contains];
  }

  async function loadHints(): Promise<void> {
    await notices.guard(async () => {
      hints = await variableHints(collectionId);
    });
  }

  // The list is positioned against the window so table and card overflow cannot clip it.
  // Reads the element directly: the bound `value` may not be updated yet inside `input`.
  function updateSuggestions(): void {
    const text = input.value;
    const caret = input.selectionStart ?? text.length;
    const match = preferenceSettings.editor.variableHints
      ? OPEN_PATTERN.exec(text.slice(0, caret))
      : null;
    typed = match ? (match[1] ?? '') : null;
    highlighted = 0;
    const box = input.getBoundingClientRect();
    position = { top: box.bottom + 4, left: box.left, width: box.width };
  }

  async function choose(hint: VariableHint): Promise<void> {
    const text = input.value;
    const caret = input.selectionStart ?? text.length;
    const before = text.slice(0, caret);
    const after = text.slice(caret);
    const start = before.lastIndexOf('{{');
    const closing = after.startsWith('}}') ? '' : '}}';
    value = `${before.slice(0, start)}{{${hint.name}${closing}${after}`;
    const nextCaret = start + hint.name.length + 4;
    typed = null;
    onInput?.();
    await tick();
    input.setSelectionRange(nextCaret, nextCaret);
  }

  function handleKey(event: KeyboardEvent): void {
    if (matches.length === 0) {
      return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      highlighted = (highlighted + step + matches.length) % matches.length;
    } else if (
      (event.key === 'Enter' || event.key === 'Tab') &&
      activeHint &&
      !event.ctrlKey &&
      !event.metaKey
    ) {
      // Ctrl+Enter belongs to the card (send), so it never also inserts a suggestion.
      event.preventDefault();
      event.stopPropagation();
      void choose(activeHint);
    } else if (event.key === 'Escape') {
      // Only closes the list; without this, Escape would also leave focus mode.
      event.preventDefault();
      typed = null;
    }
  }
</script>

<span class="wrap" class:url={variant === 'url'} class:field={variant === 'field'}>
  {#if painted}
    <span class="mirror {variant}" aria-hidden="true">
      <span class="mirror-text" style:transform="translateX(-{scrollLeft}px)"
        ><VariableText text={value} known={variableNames.known(collectionId)} /></span
      >
    </span>
  {/if}
  <input
    bind:this={input}
    bind:value
    type={masked ? 'password' : 'text'}
    {id}
    class="input {variant}"
    class:painted
    {placeholder}
    aria-label={label}
    role="combobox"
    aria-autocomplete="list"
    aria-expanded={matches.length > 0}
    aria-controls={listId}
    aria-activedescendant={matches.length > 0 ? `${listId}-${String(highlighted)}` : undefined}
    spellcheck="false"
    autocomplete="off"
    onfocus={() => void loadHints()}
    oninput={() => {
      onInput?.();
      updateSuggestions();
      syncScroll();
    }}
    onscroll={syncScroll}
    onkeyup={syncScroll}
    onclick={syncScroll}
    onselect={syncScroll}
    onkeydown={handleKey}
    onblur={() => (typed = null)}
  />
  {#if matches.length > 0}
    <ul
      id={listId}
      role="listbox"
      aria-label="Variables"
      style:top="{position.top}px"
      style:left="{position.left}px"
      style:min-width="{Math.max(position.width, 280)}px"
    >
      {#each matches as hint, index (hint.name)}
        <li
          id="{listId}-{index}"
          role="option"
          tabindex="-1"
          aria-selected={index === highlighted}
          class:on={index === highlighted}
          onmousedown={(event) => {
            event.preventDefault();
            void choose(hint);
          }}
        >
          <span class="name">{hint.name}</span>
          <span class="meta">{hint.preview || 'empty'} · {SCOPE_LABELS.get(hint.scope)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</span>

<style>
  .wrap {
    position: relative;
    display: block;
    min-width: 0;
  }

  .wrap.url {
    flex: 1;
    display: flex;
  }

  .input,
  .mirror {
    width: 100%;
    font-family: var(--font-code);
    font-size: var(--font-size-code);
    font-variant-ligatures: var(--code-ligatures);
    color: var(--text);
  }

  /* The copy lies on top without a background, so the input's own background, caret and
     selection still show through it. */
  .input.painted {
    color: transparent;
    caret-color: var(--text);
  }

  .input::placeholder {
    color: var(--text-muted);
  }

  .mirror {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    overflow: hidden;
    white-space: pre;
    pointer-events: none;
    border: 1px solid transparent;
  }

  .mirror.url {
    padding: 0 12px;
    border: 0;
  }

  .mirror.cell {
    padding: 0 8px;
  }

  .mirror.field {
    padding: 0 12px;
  }

  .mirror-text {
    display: inline-block;
  }

  /* Only the input: padding on the wrapper too would push its text away from the copy. */
  .input.url {
    padding: 0 12px;
    border: 0;
    background: transparent;
    outline: none;
  }

  .cell {
    height: 30px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
  }

  .cell:hover {
    border-color: var(--border);
  }

  .wrap.field {
    flex: 1;
  }

  .input.field {
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .input.field:focus {
    border-color: var(--focus);
    outline: none;
  }

  .cell:focus {
    border-color: var(--focus);
    background: var(--surface);
    outline: none;
  }

  ul {
    position: fixed;
    z-index: 30;
    max-width: 420px;
    margin: 0;
    padding: 6px;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  li {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  li.on {
    background: var(--accent-soft);
  }

  .name {
    font-family: var(--font-code);
    font-size: var(--font-size-code);
  }

  .meta {
    margin-left: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
