<script lang="ts">
  import type { ListItem, RootContent } from 'mdast';
  import { isWebLink } from '../../docs/markdown';
  import Self from './MarkdownNodes.svelte';

  interface Props {
    nodes: RootContent[];
    /** Added to a Markdown heading's depth, so `#` sits below the page's own headings. */
    headingOffset: number;
    onLink: (url: string) => void;
  }

  let { nodes, headingOffset, onLink }: Props = $props();

  function headingTag(depth: number): string {
    return `h${String(Math.min(6, depth + headingOffset))}`;
  }

  /** A tight list keeps each item's text on one line instead of a paragraph each. */
  function itemContent(item: ListItem): RootContent[] {
    if (item.spread) {
      return item.children;
    }
    return item.children.flatMap((child): RootContent[] =>
      child.type === 'paragraph' ? child.children : [child],
    );
  }
</script>

{#each nodes as node, index (index)}
  {#if node.type === 'paragraph'}
    <p><Self nodes={node.children} {headingOffset} {onLink} /></p>
  {:else if node.type === 'heading'}
    <svelte:element this={headingTag(node.depth)} class="md-heading md-h{node.depth}">
      <Self nodes={node.children} {headingOffset} {onLink} />
    </svelte:element>
  {:else if node.type === 'text'}
    {node.value}
  {:else if node.type === 'emphasis'}
    <em><Self nodes={node.children} {headingOffset} {onLink} /></em>
  {:else if node.type === 'strong'}
    <strong><Self nodes={node.children} {headingOffset} {onLink} /></strong>
  {:else if node.type === 'delete'}
    <del><Self nodes={node.children} {headingOffset} {onLink} /></del>
  {:else if node.type === 'inlineCode'}
    <code>{node.value}</code>
  {:else if node.type === 'code'}
    <pre><code>{node.value}</code></pre>
  {:else if node.type === 'break'}
    <br />
  {:else if node.type === 'thematicBreak'}
    <hr />
  {:else if node.type === 'blockquote'}
    <blockquote><Self nodes={node.children} {headingOffset} {onLink} /></blockquote>
  {:else if node.type === 'list'}
    {#if node.ordered}
      <ol start={node.start ?? 1}><Self nodes={node.children} {headingOffset} {onLink} /></ol>
    {:else}
      <ul><Self nodes={node.children} {headingOffset} {onLink} /></ul>
    {/if}
  {:else if node.type === 'listItem'}
    {@const task = typeof node.checked === 'boolean'}
    <li class:task>
      {#if task}
        <input type="checkbox" checked={node.checked === true} disabled aria-label="Task" />
      {/if}
      <Self nodes={itemContent(node)} {headingOffset} {onLink} />
    </li>
  {:else if node.type === 'link'}
    {#if isWebLink(node.url)}
      <a
        href={node.url}
        title={node.url}
        onclick={(event) => {
          event.preventDefault();
          onLink(node.url);
        }}><Self nodes={node.children} {headingOffset} {onLink} /></a
      >
    {:else}
      <span class="plain-link" title={node.url}
        ><Self nodes={node.children} {headingOffset} {onLink} /></span
      >
    {/if}
  {:else if node.type === 'image'}
    <span class="image">Image: {node.alt ?? node.url}</span>
  {:else if node.type === 'table'}
    <div class="table-wrap">
      <table>
        <thead>
          {#each node.children.slice(0, 1) as row, rowIndex (rowIndex)}
            <tr>
              {#each row.children as cell, cellIndex (cellIndex)}
                <th scope="col" style:text-align={node.align?.at(cellIndex) ?? undefined}
                  ><Self nodes={cell.children} {headingOffset} {onLink} /></th
                >
              {/each}
            </tr>
          {/each}
        </thead>
        <tbody>
          {#each node.children.slice(1) as row, rowIndex (rowIndex)}
            <tr>
              {#each row.children as cell, cellIndex (cellIndex)}
                <td style:text-align={node.align?.at(cellIndex) ?? undefined}
                  ><Self nodes={cell.children} {headingOffset} {onLink} /></td
                >
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if node.type === 'html'}
    <!-- Raw HTML in a description is shown as the text it is, never as markup. -->
    <code class="raw">{node.value}</code>
  {:else if 'children' in node}
    <Self nodes={node.children} {headingOffset} {onLink} />
  {:else if 'value' in node}
    {node.value}
  {/if}
{/each}
