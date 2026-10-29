<script lang="ts">
  import type { RequestDocs } from '../../api/docs';
  import { stampLabel } from '../../request/kinds';
  import { stampTone } from '../../request/methods';
  import MethodStamp from '../MethodStamp.svelte';

  interface Props {
    request: RequestDocs;
    /** Inside a section, so indented under its name. */
    nested: boolean;
    selected: boolean;
    onSelect: () => void;
  }

  let { request, nested, selected, onSelect }: Props = $props();
</script>

<button
  type="button"
  class="entry"
  class:nested
  class:on={selected}
  aria-current={selected ? 'page' : undefined}
  title={request.name}
  onclick={onSelect}
>
  <MethodStamp
    label={stampLabel(request.kind, request.method)}
    tone={stampTone(request.kind, request.method)}
    size="sm"
  />
  <span class="name">{request.name}</span>
</button>

<style>
  .entry {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 4px 8px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }

  .entry:hover {
    background: var(--surface-sunken);
  }

  .entry.on {
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text);
    font-weight: 500;
  }

  .entry.nested {
    padding-left: 18px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
