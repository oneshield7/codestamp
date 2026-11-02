<script lang="ts">
  import type { WorkspaceHue, WorkspaceIcon } from '../api/workspaces';
  import { initial } from '../format/names';

  interface Props {
    name: string;
    hue: WorkspaceHue;
    icon?: WorkspaceIcon;
    size?: 'sm' | 'md' | 'lg' | 'xl';
  }

  let { name, hue, icon = { kind: 'letter' }, size = 'md' }: Props = $props();
</script>

{#if icon.kind === 'image'}
  <span class="tile image {size}" aria-hidden="true"><img src={icon.dataUrl} alt="" /></span>
{:else}
  <span class="tile {size}" class:emoji={icon.kind === 'emoji'} data-hue={hue} aria-hidden="true"
    >{icon.kind === 'emoji' ? icon.emoji : initial(name)}</span
  >
{/if}

<style>
  .tile {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: var(--hue-bg);
    font-weight: 600;
    line-height: 1;
  }

  /* Logos are drawn on white whatever the theme, the way their makers draw them. */
  .image {
    border: 1px solid var(--border);
    background: #ffffff;
    overflow: hidden;
  }

  .image img {
    width: 76%;
    height: 76%;
    object-fit: contain;
  }

  .sm {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    font-size: 11px;
  }

  .md {
    width: 22px;
    height: 22px;
    font-size: 12px;
  }

  .lg {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    font-size: 15px;
  }

  .xl {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-lg);
    font-size: 22px;
  }

  .emoji.sm {
    font-size: 12px;
  }

  .emoji.md {
    font-size: 14px;
  }

  .emoji.lg {
    font-size: 18px;
  }

  .emoji.xl {
    font-size: 26px;
  }
</style>
