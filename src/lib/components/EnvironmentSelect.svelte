<script lang="ts">
  import type { Environments } from '../state/environments.svelte';
  import Dropdown from './Dropdown.svelte';

  interface Props {
    environments: Environments;
    /** The collection whose environment this picks: its own ones and those on this device. */
    collectionId: string;
    /** Names the collection, where the picker is not on that collection's own page. */
    label?: string;
  }

  let { environments, collectionId, label = 'Active environment' }: Props = $props();

  const NO_ENVIRONMENT = '';

  const value = $derived(environments.activeIn(collectionId)?.id ?? NO_ENVIRONMENT);

  // A green dot marks the one environment whose variables are used; the others stay grey.
  const options = $derived([
    { value: NO_ENVIRONMENT, label: 'No environment', dot: 'var(--dot-off)' },
    ...environments.usableIn(collectionId).map((env) => ({
      value: env.id,
      label: env.collectionId === null ? `${env.name} · This device` : env.name,
      dot: env.id === value ? 'var(--dot-ok)' : 'var(--dot-off)',
    })),
  ]);
</script>

<Dropdown
  {label}
  {value}
  {options}
  onChange={(id: string) =>
    void environments.setActive(collectionId, id === NO_ENVIRONMENT ? null : id)}
/>
