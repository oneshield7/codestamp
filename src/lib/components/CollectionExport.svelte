<script lang="ts">
  import { exportDocs } from '../api/docs';
  import { exportCollection, type CollectionFormat, type ExportOutcome } from '../api/export';
  import { toAppError } from '../api/http';
  import type { Workspace } from '../state/workspace.svelte';
  import Dropdown from './Dropdown.svelte';
  import ExportOutcomeBox from './ExportOutcome.svelte';
  import ViewPanel from './ViewPanel.svelte';

  interface Props {
    workspace: Workspace;
    collectionId: string;
    onClose: () => void;
  }

  let { workspace, collectionId, onClose }: Props = $props();

  type Format = CollectionFormat | 'docs';

  const FORMATS: readonly { value: Format; label: string; description: string }[] = [
    {
      value: 'codestamp',
      label: 'Codestamp',
      description:
        'For another Codestamp. Every request type, examples, Docs and the collection’s environments come along.',
    },
    {
      value: 'postman',
      label: 'Postman Collection v2.1',
      description:
        'For Postman and other API tools that read its files, such as Insomnia and Bruno. WebSocket, gRPC and MCP requests stay out.',
    },
    {
      value: 'docs',
      label: 'Docs as Markdown',
      description: 'The collection’s Docs in one Markdown file, for a wiki or a README.',
    },
  ];

  let format = $state<Format>('codestamp');
  let exporting = $state(false);
  let outcome = $state<ExportOutcome | null>(null);

  const description = $derived(FORMATS.find((option) => option.value === format)?.description);

  // Exports what the user sees, so pending edits are saved first.
  async function save(): Promise<void> {
    exporting = true;
    outcome = null;
    try {
      if (!(await workspace.flushSave())) {
        return;
      }
      if (format === 'docs') {
        const path = await exportDocs(collectionId);
        outcome = path === null ? null : { path, notes: [] };
      } else {
        outcome = await exportCollection(collectionId, format);
      }
    } catch (caught) {
      outcome = { error: toAppError(caught).message };
    } finally {
      exporting = false;
    }
  }
</script>

<ViewPanel
  title="Export"
  description="Saves this collection as a file where you choose. Local file paths and secret values never leave this device."
  {onClose}
>
  <div class="field">
    <span>Format</span>
    <Dropdown
      variant="field"
      label="Export format"
      value={format}
      options={FORMATS.map((option) => ({ value: option.value, label: option.label }))}
      onChange={(value: Format) => {
        format = value;
        outcome = null;
      }}
    />
    <p class="hint">{description}</p>
  </div>
  <ExportOutcomeBox {outcome} />
  <div class="actions">
    <button type="button" class="primary" disabled={exporting} onclick={() => void save()}>
      {exporting ? 'Exporting…' : 'Export…'}
    </button>
  </div>
</ViewPanel>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 520px;
    font-size: 13px;
    color: var(--text-2);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
  }

  .primary {
    height: 36px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
  }

  .primary:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
  }
</style>
