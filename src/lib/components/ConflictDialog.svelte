<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import type { Choice, ConflictView, Decision } from '../api/team';
  import type { TeamSync } from '../state/team.svelte';

  interface Props {
    team: TeamSync;
    collectionId: string;
    conflicts: ConflictView[];
    onClose: () => void;
  }

  let { team, collectionId, conflicts, onClose }: Props = $props();

  let dialog: HTMLDialogElement;
  const choices = new SvelteMap<string, Choice>();
  let applying = $state(false);
  let error = $state<string | null>(null);

  const ready = $derived(conflicts.every((conflict) => choices.has(conflict.file)));

  onMount(() => {
    dialog.showModal();
  });

  function options(conflict: ConflictView): { choice: Choice; label: string }[] {
    const list: { choice: Choice; label: string }[] = [
      { choice: 'mine', label: 'Keep mine' },
      { choice: 'theirs', label: "Take my collaborator's" },
    ];
    if (conflict.allowBoth) {
      list.push({ choice: 'both', label: 'Keep both (mine becomes a copy)' });
    }
    return list;
  }

  function chooseAll(choice: Choice): void {
    for (const conflict of conflicts) {
      choices.set(conflict.file, choice);
    }
  }

  async function apply(): Promise<void> {
    applying = true;
    error = null;
    const decisions: Decision[] = conflicts.flatMap((conflict) => {
      const choice = choices.get(conflict.file);
      return choice ? [{ file: conflict.file, choice }] : [];
    });
    const report = await team.sync(collectionId, decisions);
    applying = false;
    if (report === null) {
      error = team.outcomes.get(collectionId)?.error ?? 'The sync did not finish. Try again.';
    } else if (report.status === 'conflict') {
      // A teammate changed the same places again meanwhile; the list now shows the new overlaps.
      choices.clear();
    } else {
      onClose();
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="conflict-title"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <header class="head">
    <div>
      <h2 id="conflict-title">Choose what to keep</h2>
      <p>
        You and a collaborator changed the same {conflicts.length === 1 ? 'place' : 'places'}. Every
        other change was merged on its own.
      </p>
    </div>
    <span class="spacer"></span>
    <button type="button" class="close" aria-label="Close" onclick={onClose}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>
  </header>

  <div class="list">
    {#each conflicts as conflict (conflict.file)}
      <section class="conflict" aria-label={conflict.title}>
        <h3>
          {conflict.title}
          <span class="kind">{conflict.requestId ? 'Request' : 'Collection settings'}</span>
        </h3>
        <div class="grid">
          <span class="col">Field</span>
          <span class="col">Yours</span>
          <span class="col">Your collaborator's</span>
          {#each conflict.fields as field, index (index)}
            <span class="label">{field.label}</span>
            <pre class="value mono">{field.mine}</pre>
            <pre class="value mono">{field.theirs}</pre>
          {/each}
        </div>
        <div class="choices" role="radiogroup" aria-label="Keep which version of {conflict.title}">
          {#each options(conflict) as option (option.choice)}
            <label class="choice" class:on={choices.get(conflict.file) === option.choice}>
              <input
                type="radio"
                name="choice-{conflict.file}"
                checked={choices.get(conflict.file) === option.choice}
                onchange={() => {
                  choices.set(conflict.file, option.choice);
                }}
              />
              {option.label}
            </label>
          {/each}
        </div>
      </section>
    {/each}
  </div>

  <footer class="foot">
    {#if conflicts.length > 1}
      <button
        type="button"
        class="secondary"
        onclick={() => {
          chooseAll('mine');
        }}>All mine</button
      >
      <button
        type="button"
        class="secondary"
        onclick={() => {
          chooseAll('theirs');
        }}>All theirs</button
      >
    {/if}
    {#if error}
      <span class="error" role="alert">{error}</span>
    {/if}
    <span class="spacer"></span>
    <button type="button" class="secondary" onclick={onClose}>Later</button>
    <button
      type="button"
      class="primary"
      disabled={!ready || applying}
      onclick={() => void apply()}
    >
      {applying ? 'Syncing…' : 'Apply and sync'}
    </button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(860px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
  }

  dialog[open] {
    display: flex;
    flex-direction: column;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  .head,
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 18px;
  }

  .head {
    align-items: flex-start;
    border-bottom: 1px solid var(--border-soft);
  }

  .foot {
    border-top: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .head p {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .spacer {
    flex: 1;
  }

  .close {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .close:hover {
    background: var(--surface-sunken);
  }

  .close path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .list {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 18px;
  }

  .conflict {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .kind {
    margin-left: 8px;
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(90px, max-content) 1fr 1fr;
    gap: 6px 12px;
    align-items: start;
  }

  .col {
    font-size: 12px;
    color: var(--text-muted);
  }

  .label {
    font-size: 13px;
    color: var(--text-2);
  }

  .value {
    margin: 0;
    max-height: 180px;
    overflow: auto;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-muted);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .choice {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 13px;
    cursor: pointer;
  }

  .choice.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .choice input {
    accent-color: var(--accent);
  }

  .error {
    font-size: 13px;
    color: var(--err);
  }

  .primary,
  .secondary {
    height: 36px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .primary:disabled {
    opacity: 0.6;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }
</style>
