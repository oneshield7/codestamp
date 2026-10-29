<script lang="ts">
  const SHORTCUTS: { keys: string[]; action: string }[] = [
    { keys: ['Ctrl', 'Space'], action: 'Search requests, paths, collections and settings' },
    { keys: ['Tab'], action: 'Take the suggested path in search' },
    { keys: ['Ctrl', 'B'], action: 'Collapse or expand the sidebar' },
    { keys: ['Ctrl', 'Enter'], action: 'Send the open request, or save a description' },
    { keys: ['Ctrl', 'F'], action: 'Find in the open request or in Docs' },
    { keys: ['Ctrl', 'S'], action: 'Save now (changes also save on their own)' },
    { keys: ['Alt', '↑ ↓'], action: 'Move a request or section' },
    { keys: ['Esc'], action: 'Leave focus mode' },
    { keys: ['Ctrl', 'Shift', 'C'], action: 'Copy a captured image' },
  ];
</script>

<dl>
  {#each SHORTCUTS as shortcut (shortcut.action)}
    <dt>
      {#each shortcut.keys as key (key)}<kbd>{key}</kbd>{/each}
    </dt>
    <dd>{shortcut.action}</dd>
  {/each}
</dl>

<style>
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 10px 18px;
    font-size: 13px;
  }

  dt {
    display: flex;
    gap: 4px;
  }

  dd {
    margin: 0;
    color: var(--text-2);
  }

  kbd {
    padding: 1px 7px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    background: var(--surface-muted);
    font-family: var(--font-code);
    font-size: 12px;
  }
</style>
