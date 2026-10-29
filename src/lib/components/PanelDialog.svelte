<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  interface Props {
    /** The accessible name; the panel inside shows it as its heading. */
    title: string;
    /** Asked to close by Escape or a click on the dimmed backdrop; it may refuse (unsaved edits). */
    onClose: () => void;
    children: Snippet;
  }

  let { title, onClose, children }: Props = $props();

  let dialog: HTMLDialogElement;

  onMount(() => {
    dialog.showModal();
  });
</script>

<!-- A collection panel opened on top of the page, like quick search: the page behind stays put. -->
<dialog
  bind:this={dialog}
  class="panel-dialog"
  aria-label={title}
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
  onclick={(event) => {
    // A click on the dimmed backdrop lands on the dialog itself.
    if (event.target === dialog) {
      onClose();
    }
  }}
>
  {@render children()}
</dialog>

<style>
  .panel-dialog {
    width: min(880px, calc(100vw - 32px));
    /* Fits below its top margin on short windows too; the panel scrolls inside. */
    max-height: calc(90vh - 16px);
    margin: 10vh auto auto;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  /* Only opacity and transform move, as in quick search, so the opening stays smooth. */
  .panel-dialog[open] {
    animation: rise-in 240ms cubic-bezier(0.2, 0.7, 0.2, 1);
  }

  .panel-dialog::backdrop {
    background: var(--backdrop);
    animation: fade-in 200ms ease-out;
  }

  @keyframes rise-in {
    from {
      opacity: 0;
      transform: translateY(-10px) scale(0.98);
    }
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .panel-dialog[open],
    .panel-dialog::backdrop {
      animation: none;
    }
  }

  /* The dialog is the card now: the panel drops its own frame, and its heading stays in view
     while a long panel scrolls. */
  /* section.panel outranks each panel's own .panel rule, whichever stylesheet loads last. */
  .panel-dialog > :global(section.panel) {
    border: 0;
    border-radius: 0;
    overflow: visible;
  }

  .panel-dialog > :global(section.panel > .head) {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--surface);
  }
</style>
