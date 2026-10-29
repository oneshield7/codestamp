<script lang="ts">
  import type { CollectionSummary } from '../api/collections';
  import type { Environments } from '../state/environments.svelte';
  import Dropdown from './Dropdown.svelte';

  interface Props {
    environments: Environments;
    /** Names the collection an environment belongs to, next to its own name. */
    collections?: readonly CollectionSummary[];
  }

  let { environments, collections = [] }: Props = $props();

  const NO_ENVIRONMENT = '';

  // A green dot marks the one environment whose variables are used; the others stay grey.
  const options = $derived([
    { value: NO_ENVIRONMENT, label: 'No environment', dot: 'var(--dot-off)' },
    ...environments.named.map((env) => {
      const owner = collections.find((collection) => collection.id === env.collectionId);
      return {
        value: env.id,
        label: owner ? `${env.name} · ${owner.name}` : env.name,
        dot: env.id === environments.activeId ? 'var(--dot-ok)' : 'var(--dot-off)',
      };
    }),
  ]);
</script>

<Dropdown
  label="Active environment"
  value={environments.activeId ?? NO_ENVIRONMENT}
  {options}
  onChange={(id: string) => void environments.setActive(id === NO_ENVIRONMENT ? null : id)}
/>
