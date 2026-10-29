<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { parseMarkdown } from '../../docs/markdown';
  import { notices } from '../../state/notices.svelte';
  import MarkdownNodes from './MarkdownNodes.svelte';

  interface Props {
    text: string;
    headingOffset: number;
  }

  let { text, headingOffset }: Props = $props();

  const tree = $derived(parseMarkdown(text));

  function open(url: string): void {
    void notices.guard(() => openUrl(url));
  }
</script>

<div class="markdown">
  <MarkdownNodes nodes={tree.children} {headingOffset} onLink={open} />
</div>

<style>
  .markdown {
    color: var(--text-2);
    font-size: 14px;
    line-height: 1.6;
    overflow-wrap: anywhere;
  }

  .markdown > :global(:first-child) {
    margin-top: 0;
  }

  .markdown > :global(:last-child) {
    margin-bottom: 0;
  }

  .markdown :global(p),
  .markdown :global(ul),
  .markdown :global(ol),
  .markdown :global(blockquote),
  .markdown :global(pre),
  .markdown :global(.table-wrap) {
    margin: 0 0 10px;
  }

  .markdown :global(.md-heading) {
    margin: 16px 0 6px;
    color: var(--text);
    font-weight: 600;
    line-height: 1.3;
  }

  .markdown :global(.md-h1) {
    font-size: 18px;
  }

  .markdown :global(.md-h2) {
    font-size: 16px;
  }

  .markdown :global(.md-h3),
  .markdown :global(.md-h4),
  .markdown :global(.md-h5),
  .markdown :global(.md-h6) {
    font-size: 14px;
  }

  .markdown :global(ul),
  .markdown :global(ol) {
    padding-left: 22px;
  }

  .markdown :global(li.task) {
    list-style: none;
    margin-left: -20px;
  }

  .markdown :global(li.task input) {
    margin: 0 6px 0 0;
    vertical-align: -2px;
  }

  .markdown :global(code) {
    padding: 1px 5px;
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
    color: var(--text);
    font-family: var(--font-code);
    font-size: 0.92em;
  }

  .markdown :global(pre) {
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    overflow-x: auto;
  }

  .markdown :global(pre code) {
    padding: 0;
    background: transparent;
    font-size: var(--font-size-code);
    white-space: pre;
  }

  .markdown :global(blockquote) {
    padding: 2px 0 2px 12px;
    border-left: 3px solid var(--border-strong);
    color: var(--text-muted);
  }

  .markdown :global(a) {
    color: var(--accent-text);
    font-weight: 500;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .markdown :global(.plain-link) {
    text-decoration: underline dotted;
  }

  .markdown :global(.image) {
    color: var(--text-muted);
    font-style: italic;
  }

  .markdown :global(hr) {
    margin: 14px 0;
    border: 0;
    border-top: 1px solid var(--border);
  }

  .markdown :global(.table-wrap) {
    overflow-x: auto;
  }

  .markdown :global(table) {
    border-collapse: collapse;
    font-size: 13px;
  }

  .markdown :global(th),
  .markdown :global(td) {
    padding: 6px 10px;
    border: 1px solid var(--border);
    text-align: left;
  }

  .markdown :global(th) {
    background: var(--surface-muted);
    color: var(--text);
    font-weight: 600;
  }
</style>
