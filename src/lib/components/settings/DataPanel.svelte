<script lang="ts">
  import { toAppError } from '../../api/http';
  import { clearHistory } from '../../api/history';
  import { backupData, restoreData } from '../../api/preferences';
  import { preferenceSettings } from '../../state/preferences.svelte';
  import ConfirmDialog from '../ConfirmDialog.svelte';
  import NumberField from '../NumberField.svelte';

  type Pending = 'clear' | 'restore' | null;

  const history = $derived(preferenceSettings.preferences.history);
  let pending: Pending = $state(null);
  let busy = $state(false);
  let status: string | null = $state(null);
  let error: string | null = $state(null);

  async function run(task: () => Promise<string | null>): Promise<void> {
    busy = true;
    status = null;
    error = null;
    try {
      status = await task();
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }

  async function backup(): Promise<string | null> {
    const file = await backupData();
    return file ? `Backed up to ${file}` : null;
  }

  async function restore(): Promise<string | null> {
    if (await restoreData()) {
      // Every view holds data from before the restore; starting over is the simple, safe way.
      location.reload();
    }
    return null;
  }

  async function clear(): Promise<string | null> {
    await clearHistory(null);
    return 'History cleared';
  }

  function confirm(): void {
    const action = pending === 'clear' ? clear : restore;
    pending = null;
    void run(action);
  }
</script>

<section class="panel" aria-labelledby="history-title">
  <h2 id="history-title">History</h2>
  <p class="hint">Older entries are removed when either limit is reached.</p>
  <div class="fields">
    <NumberField
      label="Keep up to (requests)"
      value={history.maxEntries}
      range={{ min: 100, max: 10000 }}
      onChange={(maxEntries: number) => {
        preferenceSettings.updateHistory({ maxEntries });
      }}
    />
    <NumberField
      label="Keep for (days)"
      value={history.keepDays}
      range={{ min: 1, max: 365 }}
      onChange={(keepDays: number) => {
        preferenceSettings.updateHistory({ keepDays });
      }}
    />
  </div>
  {#if preferenceSettings.saveError}
    <p class="error" role="alert">{preferenceSettings.saveError}</p>
  {/if}
  <div>
    <button
      type="button"
      class="button"
      disabled={busy}
      aria-haspopup="dialog"
      aria-expanded={pending === 'clear'}
      onclick={() => (pending = 'clear')}
    >
      Clear history…
    </button>
  </div>
</section>

<section class="panel" aria-labelledby="backup-title">
  <h2 id="backup-title">Backup</h2>
  <p class="hint">
    One file with every collection, environment, history entry and setting. Secret values stay
    encrypted and only open again on this device.
  </p>
  <div class="actions">
    <button type="button" class="button" disabled={busy} onclick={() => void run(backup)}>
      Back up…
    </button>
    <button
      type="button"
      class="button"
      disabled={busy}
      aria-haspopup="dialog"
      aria-expanded={pending === 'restore'}
      onclick={() => (pending = 'restore')}
    >
      Restore from backup…
    </button>
  </div>
</section>

{#if status}
  <p class="status" role="status">{status}</p>
{/if}
{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<ConfirmDialog
  open={pending !== null}
  title={pending === 'clear' ? 'Clear history?' : 'Restore from a backup?'}
  message={pending === 'clear'
    ? 'Every request in History is removed. Collections are not touched.'
    : 'Everything in Codestamp is replaced by the backup you choose next. This cannot be undone.'}
  confirmLabel={pending === 'clear' ? 'Clear history' : 'Choose backup'}
  onConfirm={confirm}
  onCancel={() => (pending = null)}
/>
