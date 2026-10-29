<script lang="ts">
  import type { Snippet } from 'svelte';
  import PanelDialog from './PanelDialog.svelte';

  interface Props {
    title: string;
    description: string;
    onClose: () => void;
    /** Buttons beside Close, such as Clear. */
    actions?: Snippet;
    children: Snippet;
  }

  let { title, description, onClose, actions, children }: Props = $props();
</script>

<PanelDialog {title} {onClose}>
  <section class="panel" aria-label={title}>
    <div class="head">
      <div class="title">
        <h2>{title}</h2>
        <p>{description}</p>
      </div>
      {@render actions?.()}
      <button type="button" class="close" onclick={onClose}>Close</button>
    </div>
    <div class="body">
      {@render children()}
    </div>
  </section>
</PanelDialog>

<style>
  .panel {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  .title {
    flex: 1 1 260px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .title p {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .close {
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 13px;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
  }
</style>
