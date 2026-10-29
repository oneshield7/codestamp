<script lang="ts">
  import type { CertificateInfo } from '../../api/preferences';

  interface Props {
    item: CertificateInfo;
    /** The confirmation for this certificate is open. */
    removing: boolean;
    onRemove: (item: CertificateInfo) => void;
  }

  let { item, removing, onRemove }: Props = $props();

  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });
</script>

<li class="cert">
  <span class="text">
    <span class="name">{item.host ?? item.name}</span>
    <span class="meta">
      {item.host ? `${item.name} · ` : ''}Added {dateFormat.format(item.createdAt)}
    </span>
  </span>
  <button
    type="button"
    class="button"
    aria-haspopup="dialog"
    aria-expanded={removing}
    onclick={() => {
      onRemove(item);
    }}
  >
    Remove
  </button>
</li>

<style>
  .cert {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--bg);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
