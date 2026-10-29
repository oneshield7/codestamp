<script lang="ts">
  import { getCollectionVariableSet } from '../api/environments';
  import { accessGroups } from '../state/access.svelte';
  import { notices } from '../state/notices.svelte';
  import { VariableEditor } from '../state/variable-editor.svelte';
  import { variableNames } from '../state/variable-names.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import CollectionPanel from './CollectionPanel.svelte';
  import VariablesTable from './VariablesTable.svelte';

  interface Props {
    workspace: Workspace;
    collectionId: string;
    /** The active environment, whose variables replace collection ones of the same name. */
    environmentName: string | null;
    /** Shared through Collaboration, so values can be restricted to an access group. */
    shared: boolean;
    onClose: () => void;
  }

  let { workspace, collectionId, environmentName, shared, onClose }: Props = $props();

  const editor = new VariableEditor();

  $effect(() => {
    variableNames.ensure(collectionId);
  });

  async function save(): Promise<void> {
    if ((await editor.save()) !== null) {
      workspace.notifyChanged(collectionId);
    }
  }

  $effect(() => {
    const id = collectionId;
    void notices.guard(async () => {
      await editor.load(await getCollectionVariableSet(id));
    });
  });

  $effect(() => {
    if (shared) {
      const id = collectionId;
      void notices.guard(() => accessGroups.load(id));
    }
  });
</script>

<CollectionPanel
  {workspace}
  title="Collection variables"
  description="Used by every request here. They override Globals; the active environment overrides them."
  dirty={editor.dirty}
  saving={editor.saving}
  onSave={() => void save()}
  {onClose}
>
  <VariablesTable
    {editor}
    groups={shared ? accessGroups.groups(collectionId) : []}
    replacedBy={environmentName
      ? { environment: environmentName, keys: variableNames.fromEnvironment(collectionId) }
      : null}
  />
</CollectionPanel>
