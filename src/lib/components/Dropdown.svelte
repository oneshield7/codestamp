<script module lang="ts">
  export interface DropdownOption<T extends string> {
    value: T;
    label: string;
    /** Text colour, for example `var(--method-get)`. */
    color?: string;
    /** A status dot before the label, for example `var(--dot-ok)`. */
    dot?: string;
  }

  export type DropdownVariant = 'field' | 'compact' | 'cell' | 'method' | 'chip';
</script>

<script lang="ts" generics="T extends string">
  import { tick } from 'svelte';

  interface Props {
    value: T;
    options: DropdownOption<T>[];
    /** The accessible name of the control. */
    label: string;
    onChange: (value: T) => void;
    id?: string;
    variant?: DropdownVariant;
    disabled?: boolean;
    mono?: boolean;
    /** Background of the closed control, for example a method's tint. */
    tint?: string;
  }

  let {
    value,
    options,
    label,
    onChange,
    id,
    variant = 'compact',
    disabled = false,
    mono = false,
    tint,
  }: Props = $props();

  const GAP_PX = 4;
  const TYPEAHEAD_RESET_MS = 600;
  const listId = `dropdown-${crypto.randomUUID()}`;

  let trigger: HTMLButtonElement;
  let list: HTMLUListElement | undefined = $state();
  let open = $state(false);
  let active = $state(0);
  let position = $state({ top: 0, left: 0, width: 0 });
  let typed = '';
  let typedAt = 0;

  const selected = $derived(options.find((option) => option.value === value));

  async function show(): Promise<void> {
    if (disabled || open) {
      return;
    }
    active = Math.max(
      0,
      options.findIndex((option) => option.value === value),
    );
    // A dialog still sliding in would carry the list along and leave it beside its trigger.
    const holder = trigger.closest('dialog');
    await Promise.allSettled(holder?.getAnimations().map((animation) => animation.finished) ?? []);
    const box = trigger.getBoundingClientRect();
    position = { top: box.bottom + GAP_PX, left: box.left, width: box.width };
    open = true;
    await tick();
    if (!list) {
      return;
    }
    // Opens upwards when the list would run off the bottom of the window.
    const height = list.offsetHeight;
    if (box.bottom + GAP_PX + height > window.innerHeight && box.top - GAP_PX - height > 0) {
      position = { ...position, top: box.top - GAP_PX - height };
    }
    list.focus({ preventScroll: true });
    reveal(list.querySelector<HTMLElement>('[aria-selected="true"]'));
  }

  // Focusing or scrollIntoView would also scroll whatever holds the dropdown, such as a panel
  // dialog, and that scroll closes the list; only the list itself moves here.
  function reveal(option: HTMLElement | null | undefined): void {
    if (!list || !option) {
      return;
    }
    const top = option.offsetTop;
    const bottom = top + option.offsetHeight;
    if (top < list.scrollTop) {
      list.scrollTop = top;
    } else if (bottom > list.scrollTop + list.clientHeight) {
      list.scrollTop = bottom - list.clientHeight;
    }
  }

  function hide(refocus: boolean): void {
    open = false;
    if (refocus) {
      trigger.focus();
    }
  }

  function choose(index: number): void {
    const option = options.at(index);
    hide(true);
    if (option && option.value !== value) {
      onChange(option.value);
    }
  }

  function move(to: number): void {
    active = Math.min(options.length - 1, Math.max(0, to));
    reveal(list?.querySelector<HTMLElement>(`#${listId}-${String(active)}`));
  }

  /** Jumps to the next option starting with what was typed, like a native select. */
  function typeahead(key: string): void {
    const now = Date.now();
    typed = now - typedAt > TYPEAHEAD_RESET_MS ? key : typed + key;
    typedAt = now;
    const wanted = typed.toLowerCase();
    const start = typed.length === 1 ? active + 1 : active;
    const ordered = [...options.slice(start), ...options.slice(0, start)];
    const match = ordered.find((option) => option.label.toLowerCase().startsWith(wanted));
    if (match) {
      move(options.indexOf(match));
    }
  }

  function onTriggerKey(event: KeyboardEvent): void {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
      event.preventDefault();
      void show();
    }
  }

  function onListKey(event: KeyboardEvent): void {
    const keys = new Map<string, () => void>([
      [
        'ArrowDown',
        () => {
          move(active + 1);
        },
      ],
      [
        'ArrowUp',
        () => {
          move(active - 1);
        },
      ],
      [
        'Home',
        () => {
          move(0);
        },
      ],
      [
        'End',
        () => {
          move(options.length - 1);
        },
      ],
      [
        'Enter',
        () => {
          choose(active);
        },
      ],
      [
        ' ',
        () => {
          choose(active);
        },
      ],
      // Escape belongs to the list: it must not also close a dialog or leave focus mode.
      [
        'Escape',
        () => {
          hide(true);
        },
      ],
      [
        'Tab',
        () => {
          hide(true);
        },
      ],
    ]);
    const action = keys.get(event.key);
    if (action) {
      event.preventDefault();
      event.stopPropagation();
      action();
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      typeahead(event.key);
    }
  }

  function onOutside(event: Event): void {
    const target = event.target;
    if (!(target instanceof Node) || trigger.contains(target) || list?.contains(target)) {
      return;
    }
    hide(false);
  }

  $effect(() => {
    if (!open) {
      return;
    }
    // The list is placed against the window, so it closes rather than drift when the page moves.
    window.addEventListener('pointerdown', onOutside, true);
    window.addEventListener('scroll', onOutside, true);
    window.addEventListener('resize', onOutside);
    return () => {
      window.removeEventListener('pointerdown', onOutside, true);
      window.removeEventListener('scroll', onOutside, true);
      window.removeEventListener('resize', onOutside);
    };
  });
</script>

<div class="dropdown {variant}">
  <button
    bind:this={trigger}
    type="button"
    class="trigger"
    {id}
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={listId}
    aria-label={label}
    style:background={tint}
    onclick={() => {
      if (open) {
        hide(true);
      } else {
        void show();
      }
    }}
    onkeydown={onTriggerKey}
  >
    {#if selected?.dot}
      <span class="dot" style:background={selected.dot}></span>
    {/if}
    <span class="text" class:mono style:color={selected?.color}>{selected?.label ?? ''}</span>
    <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M6 9l6 6 6-6" />
    </svg>
  </button>
  {#if open}
    <ul
      bind:this={list}
      id={listId}
      role="listbox"
      tabindex="-1"
      aria-label={label}
      aria-activedescendant="{listId}-{active}"
      style:top="{position.top}px"
      style:left="{position.left}px"
      style:min-width="{position.width}px"
      onkeydown={onListKey}
    >
      {#each options as option, index (option.value)}
        <li
          id="{listId}-{index}"
          role="option"
          aria-selected={option.value === value}
          class:active={index === active}
          onmousemove={() => (active = index)}
          onmousedown={(event) => {
            event.preventDefault();
            choose(index);
          }}
        >
          {#if option.dot}
            <span class="dot" style:background={option.dot}></span>
          {/if}
          <span class="label" class:mono style:color={option.color}>{option.label}</span>
          {#if option.value === value}
            <svg class="check" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M5 12l5 5 9-10" />
            </svg>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .dropdown {
    display: inline-flex;
    min-width: 0;
  }

  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 34px;
    padding: 0 8px 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    text-align: left;
  }

  .trigger:hover:not(:disabled) {
    border-color: var(--text-muted);
  }

  /* The blue edge shows where the keyboard is and which list is open. */
  .trigger:focus-visible,
  .trigger[aria-expanded='true'] {
    border-color: var(--focus);
    box-shadow: 0 0 0 1px var(--focus);
    outline: none;
  }

  .trigger:disabled {
    opacity: 0.55;
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    flex: none;
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.15s;
  }

  .trigger[aria-expanded='true'] .chevron {
    transform: rotate(180deg);
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .field {
    display: flex;
    width: 100%;
  }

  .field .trigger {
    width: 100%;
    height: 40px;
    font-size: 14px;
  }

  .cell .trigger {
    height: 30px;
    padding: 0 6px 0 8px;
    border-color: transparent;
    border-radius: var(--radius-sm);
    background: transparent;
  }

  .cell .trigger:hover:not(:disabled) {
    border-color: var(--border);
  }

  .chip .trigger {
    height: 30px;
    max-width: 180px;
    padding: 0 6px 0 10px;
    border-color: var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-2);
    font-size: 12px;
  }

  /* Sits at the start of the URL field, which draws the outer border. */
  .method {
    align-self: stretch;
  }

  .method .trigger {
    height: 100%;
    padding: 0 8px 0 12px;
    border: 0;
    border-right: 1px solid var(--border);
    border-radius: 0;
    font-weight: 600;
  }

  .method .trigger:focus-visible,
  .method .trigger[aria-expanded='true'] {
    box-shadow: inset 0 0 0 2px var(--focus);
  }

  ul {
    position: fixed;
    z-index: 50;
    max-height: 320px;
    max-width: 360px;
    margin: 0;
    padding: 4px;
    overflow: auto;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    outline: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
  }

  li.active {
    background: var(--accent-soft);
  }

  li[aria-selected='true'] {
    color: var(--text);
    font-weight: 600;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .check {
    flex: none;
    fill: none;
    stroke: var(--focus);
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .mono {
    font-family: var(--font-code);
    font-size: var(--font-size-code);
    font-variant-ligatures: var(--code-ligatures);
  }

  .method .mono,
  .method li .mono {
    font-weight: 600;
  }
</style>
