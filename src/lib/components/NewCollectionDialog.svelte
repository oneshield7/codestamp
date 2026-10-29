<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { suggestCollectionIcon } from '../api/collections';
  import { toAppError } from '../api/http';
  import { MAX_NAME_LENGTH } from '../format/names';
  import type { NewCollectionStart } from '../state/navigation';
  import type { TeamSync } from '../state/team.svelte';
  import type { Workspace } from '../state/workspace.svelte';
  import { accountFor } from '../team/account';
  import AccountFields from './AccountFields.svelte';
  import ChoiceCard from './ChoiceCard.svelte';
  import EmojiPicker from './EmojiPicker.svelte';

  interface Props {
    workspace: Workspace;
    team: TeamSync;
    start: NewCollectionStart;
    onClose: () => void;
    /** The new collection is open in the workspace. */
    onCreated: () => void;
    /** A Postman export to preview on the Import page. */
    onImportFile: (file: File) => void;
    onJoined: (collectionId: string) => void;
  }

  let { workspace, team, start, onClose, onCreated, onImportFile, onJoined }: Props = $props();

  const SUGGEST_DELAY_MS = 150;
  /** Escape inside the icon picker closes only the picker, even if the dialog also hears it. */
  const ESCAPE_GRACE_MS = 100;
  const HTTP_URL = /^https?:\/\/\S+$/i;

  let dialog: HTMLDialogElement;
  let nameInput: HTMLInputElement | undefined = $state();
  let fileInput: HTMLInputElement | undefined = $state();
  let choice = $state<NewCollectionStart>(initialChoice());
  let name = $state('');
  let chosenIcon = $state<string | null>(null);
  let autoIcon = $state('');
  let baseUrl = $state('');
  let remote = $state('');
  let username = $state('');
  let authorName = $state('');
  let authorEmail = $state('');
  let pickingIcon = $state(false);
  let pickerClosedAt = 0;
  let busy = $state(false);
  let error = $state<string | null>(null);

  const icon = $derived(chosenIcon ?? autoIcon);

  function select(start: NewCollectionStart): void {
    choice = start;
  }
  const baseUrlError = $derived(
    baseUrl.trim() !== '' && !HTTP_URL.test(baseUrl.trim())
      ? 'Use an address that starts with http:// or https://'
      : null,
  );
  const gitMissing = $derived(team.setup?.gitVersion === null);

  function initialChoice(): NewCollectionStart {
    return start;
  }

  onMount(() => {
    dialog.showModal();
    nameInput?.focus();
  });

  // The automatic icon follows the name as it is typed.
  $effect(() => {
    const forName = name;
    const run = { cancelled: false };
    const timer = setTimeout(() => {
      suggestCollectionIcon(forName).then(
        (suggestion) => {
          if (!run.cancelled) {
            autoIcon = suggestion.emoji;
          }
        },
        (caught: unknown) => {
          if (!run.cancelled) {
            error = toAppError(caught).message;
          }
        },
      );
    }, SUGGEST_DELAY_MS);
    return () => {
      run.cancelled = true;
      clearTimeout(timer);
    };
  });

  $effect(() => {
    const author = team.setup?.author;
    if (author && untrack(() => authorName === '' && authorEmail === '')) {
      authorName = author.name;
      authorEmail = author.email;
    }
  });

  function closePicker(): void {
    pickingIcon = false;
    pickerClosedAt = performance.now();
  }

  function cancel(event: Event): void {
    event.preventDefault();
    if (pickingIcon || performance.now() - pickerClosedAt < ESCAPE_GRACE_MS) {
      closePicker();
      return;
    }
    onClose();
  }

  async function attempt(task: () => Promise<void>): Promise<void> {
    busy = true;
    error = null;
    try {
      await task();
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }

  async function create(): Promise<void> {
    await attempt(async () => {
      await workspace.createCollection({ name, icon: chosenIcon, baseUrl });
      onCreated();
    });
  }

  async function join(): Promise<void> {
    await attempt(async () => {
      const joined = await team.join(remote, accountFor(remote, authorName, authorEmail, username));
      onJoined(joined.id);
    });
  }

  function pickedFile(file: File | undefined): void {
    if (file) {
      onImportFile(file);
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="new-collection-title" oncancel={cancel}>
  <header class="head">
    <div>
      <h2 id="new-collection-title">New collection</h2>
      <p>A collection holds your requests as numbered steps, grouped into sections.</p>
    </div>
    <button type="button" class="close" aria-label="Close" onclick={onClose}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>
  </header>

  <div class="body">
    <fieldset>
      <legend>How do you want to start?</legend>
      <ChoiceCard
        name="start"
        value="empty"
        checked={choice === 'empty'}
        title="Empty collection"
        description="Name it and add requests yourself."
        onSelect={select}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 5v14M5 12h14" />
        </svg>
      </ChoiceCard>
      <ChoiceCard
        name="start"
        value="import"
        checked={choice === 'import'}
        title="Import a file"
        description="Use a Codestamp or Collection v2.1 file. Requests, folders, scripts and variables come along."
        onSelect={select}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 4v11M7 10l5 5 5-5M4 20h16" />
        </svg>
      </ChoiceCard>
      <ChoiceCard
        name="start"
        value="join"
        checked={choice === 'join'}
        title="Join a shared collection"
        description="Clone a collection a collaborator shared through Git."
        onSelect={select}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="6" cy="5" r="2" /><circle cx="6" cy="19" r="2" /><circle
            cx="18"
            cy="9"
            r="2"
          /><path d="M6 7v10M18 11c0 3-3 4-6 4H8" />
        </svg>
      </ChoiceCard>
    </fieldset>

    {#if choice === 'empty'}
      <div class="field">
        <label for="new-collection-name">Name</label>
        <div class="name-row">
          <div class="icon-wrap">
            <button
              type="button"
              class="icon-button emoji"
              aria-label="Icon {icon}, {chosenIcon === null
                ? 'picked from the name'
                : 'chosen by you'}. Change icon"
              title="Change icon"
              aria-haspopup="dialog"
              aria-expanded={pickingIcon}
              onclick={() => {
                if (pickingIcon) {
                  closePicker();
                } else {
                  pickingIcon = true;
                }
              }}>{icon}</button
            >
            {#if pickingIcon}
              <EmojiPicker
                current={icon}
                {name}
                custom={chosenIcon !== null}
                onPick={(picked: string | null) => (chosenIcon = picked)}
                onClose={closePicker}
              />
            {/if}
          </div>
          <input
            bind:this={nameInput}
            id="new-collection-name"
            bind:value={name}
            maxlength={MAX_NAME_LENGTH}
            placeholder="e.g. Payments API"
            autocomplete="off"
          />
        </div>
        <span class="hint">The icon follows the name until you choose one.</span>
      </div>
      <div class="field">
        <label for="new-collection-base-url"
          >Base URL <span class="optional">(optional)</span></label
        >
        <input
          id="new-collection-base-url"
          class="mono"
          bind:value={baseUrl}
          placeholder="https://api.example.com"
          spellcheck="false"
          autocomplete="off"
          aria-invalid={baseUrlError !== null}
          aria-describedby="new-collection-base-url-hint"
        />
        <span id="new-collection-base-url-hint" class="hint" class:bad={baseUrlError !== null}>
          {#if baseUrlError}
            {baseUrlError}
          {:else}
            Saved as the collection variable <span class="mono">{'{{baseUrl}}'}</span>, so each
            request can start with <span class="mono">{'{{baseUrl}}/…'}</span>
          {/if}
        </span>
      </div>
    {:else if choice === 'import'}
      <p class="info">
        Next, pick the exported <span class="mono">.json</span> file. You will see what it contains, and
        can rename the collection, before anything is imported.
      </p>
      <input
        bind:this={fileInput}
        class="visually-hidden"
        type="file"
        accept=".json,application/json"
        tabindex="-1"
        aria-hidden="true"
        onchange={(event) => {
          pickedFile(event.currentTarget.files?.[0]);
        }}
      />
    {:else if gitMissing}
      <p class="info">
        Git is not installed on this device. Install Git for Windows from git-scm.com, then reopen
        Codestamp to join a shared collection.
      </p>
    {:else}
      <div class="field">
        <label for="new-collection-remote">Repository address</label>
        <input
          id="new-collection-remote"
          class="mono"
          bind:value={remote}
          placeholder="https://github.com/team/shop-api.git"
          spellcheck="false"
          autocomplete="off"
        />
        <span class="hint"
          >Your Git account needs access to it. Git asks you to sign in the first time.</span
        >
      </div>
      <AccountFields {remote} bind:username bind:name={authorName} bind:email={authorEmail} />
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </div>

  <footer class="foot">
    <span class="note">
      {busy && choice === 'join'
        ? 'Git may open a sign-in window.'
        : 'Saved on this device. You can rename it at any time.'}
    </span>
    <button type="button" class="secondary" onclick={onClose}>Cancel</button>
    {#if choice === 'empty'}
      <button
        type="button"
        class="primary"
        disabled={busy || name.trim() === '' || baseUrlError !== null}
        onclick={() => void create()}
      >
        {busy ? 'Creating…' : 'Create collection'}
      </button>
    {:else if choice === 'import'}
      <button
        type="button"
        class="primary"
        onclick={() => {
          fileInput?.click();
        }}>Choose a file…</button
      >
    {:else}
      <button
        type="button"
        class="primary"
        disabled={busy ||
          gitMissing ||
          remote.trim() === '' ||
          authorName.trim() === '' ||
          authorEmail.trim() === ''}
        onclick={() => void join()}
      >
        {busy ? 'Joining…' : 'Join'}
      </button>
    {/if}
  </footer>
</dialog>

<style>
  dialog {
    width: min(580px, calc(100vw - 32px));
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
    /* The icon picker may reach past the dialog's edge. */
    overflow: visible;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  /* Short windows: the options scroll so Cancel and Create stay in view (the icon picker then
     scrolls with them instead of reaching past the dialog). */
  @media (max-height: 760px) {
    dialog[open] {
      display: flex;
      flex-direction: column;
      max-height: calc(100vh - 32px);
      overflow: hidden;
    }

    .body {
      min-height: 0;
      overflow-y: auto;
    }
  }

  .head,
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 16px 22px;
  }

  .head {
    align-items: flex-start;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-soft);
  }

  .foot {
    border-top: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .head p {
    margin: 4px 0 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .close {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .close:hover {
    background: var(--surface-sunken);
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 18px 22px 20px;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  legend {
    margin-bottom: 8px;
    padding: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .field label {
    font-weight: 500;
  }

  .optional {
    font-weight: 400;
    color: var(--text-muted);
  }

  .field input {
    height: 40px;
    box-sizing: border-box;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 14px;
  }

  .field input:focus {
    border-color: var(--focus);
    outline: none;
  }

  .field input[aria-invalid='true'] {
    border-color: var(--err);
  }

  .name-row {
    display: flex;
    gap: 8px;
  }

  .name-row input {
    flex: 1;
    min-width: 0;
  }

  .icon-wrap {
    position: relative;
    flex: none;
  }

  .icon-button {
    width: 40px;
    height: 40px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 20px;
  }

  .icon-button:hover,
  .icon-button[aria-expanded='true'] {
    border-color: var(--text);
  }

  .hint {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .hint.bad {
    color: var(--err);
  }

  .info {
    margin: 0;
    padding: 12px 14px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-2);
  }

  .error {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--err-bg);
    color: var(--err);
    font-size: 13px;
  }

  .note {
    flex: 1 1 200px;
    font-size: 12px;
    color: var(--text-muted);
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
    background: var(--surface-sunken);
    color: var(--text-muted);
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }
</style>
