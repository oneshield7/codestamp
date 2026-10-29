<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** The tooltip, and the button's accessible name. */
    label: string;
    /** For a button that opens a panel or dialog: drawn pressed while it is open. */
    expanded?: boolean;
    popup?: 'dialog' | 'menu';
    disabled?: boolean;
    /** Turns red on hover, for destructive actions. */
    danger?: boolean;
    /** A small status mark in the corner (sync state, auth set). */
    dot?: 'ok' | 'busy' | 'warn' | 'err' | 'on' | null;
    /** `start` and `end` keep the tooltip inside the window for buttons at its edges. */
    align?: 'start' | 'center' | 'end';
    /** `above` for buttons at the bottom of the window, such as the status bar's. */
    placement?: 'below' | 'above';
    /** For a button that opens a page: drawn pressed while that page is open. */
    current?: boolean;
    /** A larger circle, for buttons beside a pill-shaped main button. */
    round?: boolean;
    /** Borderless and smaller, for the title bar. */
    quiet?: boolean;
    /**
     * A mouse click leaves the keyboard focus where it was, so a command from the button (Paste
     * from the menu, say) still reaches the field being edited.
     */
    keepFocus?: boolean;
    onclick: () => void;
    children: Snippet;
  }

  let {
    label,
    expanded,
    popup,
    disabled = false,
    danger = false,
    dot = null,
    align = 'center',
    placement = 'below',
    current = false,
    round = false,
    quiet = false,
    keepFocus = false,
    onclick,
    children,
  }: Props = $props();
</script>

<span class="wrap">
  <button
    type="button"
    class="button"
    class:danger
    class:round
    class:quiet
    aria-label={label}
    aria-current={current ? 'page' : undefined}
    aria-expanded={expanded}
    aria-haspopup={popup}
    {disabled}
    {onclick}
    onmousedown={(event) => {
      if (keepFocus) {
        event.preventDefault();
      }
    }}
  >
    {@render children()}
    {#if dot}
      <span class="dot {dot}" aria-hidden="true"></span>
    {/if}
  </button>
  <!-- The label is already the button's name, so screen readers skip this copy. -->
  <span
    class="tip"
    class:at-start={align === 'start'}
    class:at-end={align === 'end'}
    class:above={placement === 'above'}
    aria-hidden="true">{label}</span
  >
</span>

<style>
  .wrap {
    position: relative;
    display: inline-flex;
  }

  .button {
    position: relative;
    width: 34px;
    height: 34px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .button.round {
    width: 40px;
    height: 40px;
    border-radius: 50%;
  }

  .button.quiet {
    width: 32px;
    height: 28px;
    border-color: transparent;
    border-radius: var(--radius-sm);
    background: transparent;
  }

  /* --surface-sunken is darker than the page in dark themes, so a hover would barely show. */
  .button.quiet:hover {
    border-color: transparent;
    background: var(--accent-soft);
  }

  .button.quiet[aria-expanded='true'],
  .button.quiet[aria-current='page'] {
    border-color: transparent;
    background: var(--accent-soft);
    color: var(--text);
  }

  .button:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .button:active:not(:disabled) {
    background: var(--surface-sunken);
  }

  .button[aria-expanded='true'] {
    border-color: var(--text);
    background: var(--accent-soft);
    color: var(--text);
  }

  .button.danger:hover {
    border-color: var(--err);
    color: var(--err);
  }

  .button:disabled {
    border-color: var(--border);
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .button :global(svg) {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .dot {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--surface);
  }

  .dot.ok {
    background: var(--ok);
  }

  .dot.busy {
    background: var(--focus);
  }

  .dot.warn {
    background: var(--warn);
  }

  .dot.err {
    background: var(--err);
  }

  .dot.on {
    background: var(--text-2);
  }

  .tip {
    position: absolute;
    top: calc(100% + 6px);
    left: 50%;
    z-index: 30;
    transform: translateX(-50%);
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    pointer-events: none;
    visibility: hidden;
  }

  .tip.at-start {
    left: 0;
    transform: none;
  }

  .tip.at-end {
    left: auto;
    right: 0;
    transform: none;
  }

  .tip.above {
    top: auto;
    bottom: calc(100% + 6px);
  }

  /* Hover, or keyboard focus; a mouse click alone does not leave the tooltip up. */
  .wrap:hover .tip,
  .button:focus-visible + .tip {
    visibility: visible;
  }
</style>
