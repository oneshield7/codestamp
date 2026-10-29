<script lang="ts">
  import { NAV_ICON_HUES, NAV_ICON_PATHS, type NavIconName } from '../state/navigation';

  interface Props {
    name: NavIconName;
    size?: 'sm' | 'lg';
  }

  let { name, size = 'sm' }: Props = $props();

  const pixels = $derived(size === 'lg' ? 18 : 14);
</script>

<span class="nav-icon {size}" data-hue={NAV_ICON_HUES.get(name)} aria-hidden="true">
  <svg width={pixels} height={pixels} viewBox="0 0 24 24">
    <path d={NAV_ICON_PATHS.get(name)} />
  </svg>
</span>

<style>
  .nav-icon {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: var(--hue-bg);
  }

  .sm {
    width: 22px;
    height: 22px;
  }

  .lg {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
