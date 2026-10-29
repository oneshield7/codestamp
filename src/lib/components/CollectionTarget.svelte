<script lang="ts">
  import { getCollection, type CollectionSummary, type SectionInfo } from '../api/collections';
  import { notices } from '../state/notices.svelte';
  import Dropdown from './Dropdown.svelte';

  interface Props {
    collections: CollectionSummary[];
    collectionId: string;
    sectionId: string | null;
  }

  let { collections, collectionId = $bindable(), sectionId = $bindable() }: Props = $props();

  const NO_SECTION = '';
  let sections: SectionInfo[] = $state([]);

  $effect(() => {
    if (collectionId === '' && collections.length > 0) {
      collectionId = collections.at(0)?.id ?? '';
    }
  });

  $effect(() => {
    const id = collectionId;
    sectionId = null;
    sections = [];
    if (id === '') {
      return;
    }
    const run = { cancelled: false };
    void notices.guard(async () => {
      const loaded = (await getCollection(id)).sections;
      if (!run.cancelled) {
        sections = loaded;
      }
    });
    return () => {
      run.cancelled = true;
    };
  });

  const collectionOptions = $derived(
    collections.map((collection) => ({ value: collection.id, label: collection.name })),
  );
  const sectionOptions = $derived([
    { value: NO_SECTION, label: 'No section' },
    ...sections.map((section) => ({ value: section.id, label: section.name })),
  ]);
</script>

{#if collections.length === 0}
  <p class="empty">Create a collection first.</p>
{:else}
  <div class="target">
    <div class="pick">
      <span>Collection</span>
      <Dropdown
        variant="field"
        label="Collection"
        value={collectionId}
        options={collectionOptions}
        onChange={(id: string) => (collectionId = id)}
      />
    </div>
    <div class="pick">
      <span>Section</span>
      <Dropdown
        variant="field"
        label="Section"
        value={sectionId ?? NO_SECTION}
        options={sectionOptions}
        onChange={(id: string) => (sectionId = id === NO_SECTION ? null : id)}
      />
    </div>
  </div>
{/if}

<style>
  .target {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .pick {
    flex: 1 1 180px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
  }
</style>
