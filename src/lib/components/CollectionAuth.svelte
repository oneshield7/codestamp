<script lang="ts">
  import type { Auth } from '../api/collections';
  import type { Workspace } from '../state/workspace.svelte';
  import AuthEditor from './AuthEditor.svelte';
  import CollectionPanel from './CollectionPanel.svelte';

  interface Props {
    workspace: Workspace;
    collectionId: string;
    saved: Auth;
    onClose: () => void;
  }

  let { workspace, collectionId, saved, onClose }: Props = $props();

  let auth = $state<Auth>(copySaved());
  let saving = $state(false);
  const dirty = $derived(JSON.stringify(auth) !== JSON.stringify(saved));

  function copySaved(): Auth {
    return $state.snapshot(saved);
  }

  async function save(): Promise<void> {
    saving = true;
    await workspace.setCollectionAuth($state.snapshot(auth));
    saving = false;
  }
</script>

<CollectionPanel
  {workspace}
  title="Collection auth"
  description="Requests set to “Inherit from collection” use this. A header a request sets itself still wins."
  {dirty}
  {saving}
  onSave={() => void save()}
  {onClose}
>
  <div class="body">
    <AuthEditor
      bind:auth
      allowInherit={false}
      collectionAuth={null}
      {collectionId}
      ownerId={collectionId}
    />
  </div>
</CollectionPanel>

<style>
  .body {
    padding: 16px;
  }
</style>
