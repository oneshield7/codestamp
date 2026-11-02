<script lang="ts">
  import { onMount } from 'svelte';
  import { toAppError } from '../api/http';
  import {
    findWebsiteLogo,
    findWorkspaceLogos,
    WORKSPACE_HUES,
    type LogoCandidate,
    type Workspace,
    type WorkspaceHue,
    type WorkspaceIcon,
  } from '../api/workspaces';
  import { ICON_IMAGE_TYPES, shrinkImage } from '../dom/image';
  import { MAX_NAME_LENGTH } from '../format/names';
  import { workspaces } from '../state/workspaces.svelte';
  import EmojiPicker from './EmojiPicker.svelte';
  import WorkspaceTile from './WorkspaceTile.svelte';

  interface Props {
    /** The workspace to rename or restyle; without it the dialog creates one. */
    editing?: Workspace | null;
    onClose: () => void;
  }

  let { editing = null, onClose }: Props = $props();

  type IconChoice = 'letter' | 'logo' | 'image' | 'emoji';
  type Search =
    | { state: 'idle' }
    | { state: 'searching'; for: string }
    | { state: 'done'; for: string }
    | { state: 'failed'; message: string };

  const CHOICES: { value: IconChoice; label: string }[] = [
    { value: 'letter', label: 'Letter' },
    { value: 'logo', label: 'Logo' },
    { value: 'image', label: 'Image' },
    { value: 'emoji', label: 'Emoji' },
  ];
  /** Typing settles before the name is looked up. */
  const SEARCH_DELAY_MS = 600;
  /** Uploaded images are shrunk to this many pixels on the longer edge. */
  const IMAGE_SIDE = 128;
  /** Escape inside the emoji picker closes only the picker, even if the dialog also hears it. */
  const ESCAPE_GRACE_MS = 100;

  let dialog: HTMLDialogElement;
  let nameInput: HTMLInputElement;
  let fileInput: HTMLInputElement | undefined = $state();
  const start = initial();
  let name = $state(start.name);
  let hue = $state<WorkspaceHue>(start.hue);
  let choice = $state<IconChoice>(start.choice);
  let candidates = $state.raw<LogoCandidate[]>(start.candidates);
  let logo = $state<LogoCandidate | null>(start.logo);
  let image = $state<string | null>(start.image);
  let emoji = $state<string | null>(start.emoji);
  let search = $state<Search>(start.search);
  /** The name last looked up; a lookup runs again only once the name changes. */
  let lookedUp = $state<string | null>(start.lookedUp);
  /** Only the newest lookup may fill the list. */
  let lookups = 0;
  let website = $state('');
  let imageError = $state<string | null>(null);
  let pickingEmoji = $state(false);
  let pickerClosedAt = 0;
  let busy = $state(false);
  let error = $state<string | null>(null);

  const creating = $derived(editing === null);
  const icon = $derived(chosenIcon());
  const ready = $derived(name.trim() !== '' && icon !== null);

  function initial(): {
    name: string;
    hue: WorkspaceHue;
    choice: IconChoice;
    candidates: LogoCandidate[];
    logo: LogoCandidate | null;
    image: string | null;
    emoji: string | null;
    search: Search;
    lookedUp: string | null;
  } {
    const icon = editing?.icon ?? { kind: 'letter' };
    const name = editing?.name ?? '';
    const found =
      icon.kind === 'image' && icon.source !== null
        ? { website: icon.source, dataUrl: icon.dataUrl }
        : null;
    return {
      name,
      hue: editing?.hue ?? unusedHue(),
      choice:
        icon.kind === 'image'
          ? found
            ? 'logo'
            : 'image'
          : icon.kind === 'emoji'
            ? 'emoji'
            : 'letter',
      candidates: found ? [found] : [],
      logo: found,
      image: icon.kind === 'image' && !found ? icon.dataUrl : null,
      emoji: icon.kind === 'emoji' ? icon.emoji : null,
      search: found ? { state: 'done', for: name.trim() } : { state: 'idle' },
      // The logo it already has came from this name; it is not looked up again until it changes.
      lookedUp: found ? name.trim() : null,
    };
  }

  // A new workspace takes a color no other one has yet, so the switcher tells them apart.
  function unusedHue(): WorkspaceHue {
    const taken = new Set(workspaces.list.map((item) => item.hue));
    return (
      WORKSPACE_HUES.find((candidate) => !taken.has(candidate)) ??
      WORKSPACE_HUES[workspaces.list.length % WORKSPACE_HUES.length] ??
      'blue'
    );
  }

  /** The icon to save, or null while the chosen kind still needs a pick. */
  function chosenIcon(): WorkspaceIcon | null {
    switch (choice) {
      case 'letter':
        return { kind: 'letter' };
      case 'logo':
        return logo ? { kind: 'image', dataUrl: logo.dataUrl, source: logo.website } : null;
      case 'image':
        return image ? { kind: 'image', dataUrl: image, source: null } : null;
      case 'emoji':
        return emoji ? { kind: 'emoji', emoji } : null;
    }
  }

  function hueLabel(value: WorkspaceHue): string {
    return value.charAt(0).toUpperCase() + value.slice(1);
  }

  onMount(() => {
    dialog.showModal();
    nameInput.focus();
  });

  // With Logo chosen, the name is looked up once typing settles.
  $effect(() => {
    const wanted = name.trim();
    if (choice !== 'logo' || wanted === '' || wanted === lookedUp) {
      return;
    }
    const timer = setTimeout(() => void lookUp(wanted), SEARCH_DELAY_MS);
    return () => {
      clearTimeout(timer);
    };
  });

  async function lookUp(wanted: string): Promise<void> {
    lookedUp = wanted;
    const run = ++lookups;
    search = { state: 'searching', for: wanted };
    try {
      const found = await findWorkspaceLogos(wanted);
      if (run !== lookups) {
        return;
      }
      candidates = found;
      logo = found.find((item) => item.website === logo?.website) ?? found.at(0) ?? null;
      search = { state: 'done', for: wanted };
    } catch (caught) {
      if (run === lookups) {
        search = { state: 'failed', message: toAppError(caught).message };
      }
    }
  }

  async function findWebsite(): Promise<void> {
    const typed = website.trim();
    if (typed === '') {
      return;
    }
    const run = ++lookups;
    search = { state: 'searching', for: typed };
    try {
      const found = await findWebsiteLogo(typed);
      if (run !== lookups) {
        return;
      }
      const first = found.at(0);
      if (first) {
        candidates = [first, ...candidates.filter((item) => item.website !== first.website)];
        logo = first;
        website = '';
        search = { state: 'done', for: typed };
      } else {
        search = { state: 'failed', message: `No logo found for ${typed}.` };
      }
    } catch (caught) {
      if (run === lookups) {
        search = { state: 'failed', message: toAppError(caught).message };
      }
    }
  }

  async function pickImage(file: File | undefined): Promise<void> {
    if (!file) {
      return;
    }
    imageError = null;
    const shrunk = await shrinkImage(file, IMAGE_SIDE);
    if ('error' in shrunk) {
      imageError = shrunk.error;
    } else {
      image = shrunk.dataUrl;
    }
  }

  function closePicker(): void {
    pickingEmoji = false;
    pickerClosedAt = performance.now();
  }

  function cancel(event: Event): void {
    event.preventDefault();
    if (pickingEmoji || performance.now() - pickerClosedAt < ESCAPE_GRACE_MS) {
      closePicker();
      return;
    }
    onClose();
  }

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!icon) {
      return;
    }
    busy = true;
    error = null;
    try {
      const look = { name, hue, icon };
      if (editing) {
        await workspaces.update(editing.id, look);
      } else {
        await workspaces.create(look);
      }
      onClose();
    } catch (caught) {
      error = toAppError(caught).message;
    } finally {
      busy = false;
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="workspace-dialog-title" oncancel={cancel}>
  <form onsubmit={(event) => void save(event)}>
    <header class="head">
      <h2 id="workspace-dialog-title">{creating ? 'New workspace' : 'Edit workspace'}</h2>
      <button type="button" class="close" aria-label="Close" onclick={onClose}>
        <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </header>

    <div class="body">
      <div class="field">
        <label for="workspace-name">Name</label>
        <input
          bind:this={nameInput}
          id="workspace-name"
          bind:value={name}
          maxlength={MAX_NAME_LENGTH}
          placeholder="Client demo"
          autocomplete="off"
        />
      </div>

      <fieldset>
        <legend>Icon</legend>
        <div class="icon-row">
          <WorkspaceTile name={name || 'W'} {hue} icon={icon ?? { kind: 'letter' }} size="xl" />
          <div class="segmented">
            {#each CHOICES as option (option.value)}
              <label class:on={choice === option.value}>
                <input
                  type="radio"
                  name="workspace-icon"
                  class="visually-hidden"
                  value={option.value}
                  bind:group={choice}
                />
                {option.label}
              </label>
            {/each}
          </div>
        </div>
      </fieldset>

      {#if choice === 'logo'}
        <div class="kind-panel">
          <p class="lookup" role="status">
            {#if search.state === 'searching'}
              Looking for a logo of “{search.for}”…
            {:else if search.state === 'failed'}
              {search.message}
            {:else if candidates.length > 0}
              Logos found. Pick the right one.
            {:else if search.state === 'done'}
              No logo found for “{search.for}”. Type its website, or keep the letter.
            {:else}
              Type a name to look for its logo.
            {/if}
          </p>
          {#if candidates.length > 0}
            <div class="candidates" role="group" aria-label="Logos found">
              {#each candidates as candidate (candidate.website)}
                <button
                  type="button"
                  class="candidate"
                  aria-pressed={logo?.website === candidate.website}
                  onclick={() => (logo = candidate)}
                >
                  <WorkspaceTile
                    {name}
                    {hue}
                    icon={{ kind: 'image', dataUrl: candidate.dataUrl, source: candidate.website }}
                    size="lg"
                  />
                  <span class="website">{candidate.website}</span>
                </button>
              {/each}
            </div>
          {/if}
          <div class="find">
            <input
              bind:value={website}
              aria-label="Website"
              placeholder="Or type the website, like example.com"
              spellcheck="false"
              autocomplete="off"
              onkeydown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  void findWebsite();
                }
              }}
            />
            <button type="button" class="secondary" onclick={() => void findWebsite()}>Find</button>
          </div>
          <p class="note">
            Looking up a logo sends the name or website to Google's icon service. The logo you pick
            is saved on this device.
          </p>
        </div>
      {:else if choice === 'image'}
        <div class="kind-panel">
          <div class="find">
            <button type="button" class="secondary" onclick={() => fileInput?.click()}>
              {image ? 'Choose another image…' : 'Choose image…'}
            </button>
            <input
              bind:this={fileInput}
              type="file"
              accept={ICON_IMAGE_TYPES}
              class="visually-hidden"
              tabindex="-1"
              aria-hidden="true"
              onchange={(event) => {
                void pickImage(event.currentTarget.files?.[0]);
                event.currentTarget.value = '';
              }}
            />
          </div>
          {#if imageError}
            <p class="lookup bad" role="alert">{imageError}</p>
          {/if}
          <p class="note">PNG, JPG or WebP, up to 1 MB. It is shrunk to 128 pixels.</p>
        </div>
      {:else if choice === 'emoji'}
        <div class="kind-panel">
          <div class="emoji-wrap">
            <button
              type="button"
              class="secondary"
              aria-haspopup="dialog"
              aria-expanded={pickingEmoji}
              onclick={() => {
                if (pickingEmoji) {
                  closePicker();
                } else {
                  pickingEmoji = true;
                }
              }}
            >
              {emoji ? `Change emoji ${emoji}` : 'Choose emoji'}
            </button>
            {#if pickingEmoji}
              <EmojiPicker
                current={emoji ?? ''}
                {name}
                custom={emoji !== null}
                label="Workspace icon"
                automatic={false}
                onPick={(picked: string | null) => (emoji = picked)}
                onClose={closePicker}
              />
            {/if}
          </div>
        </div>
      {/if}

      {#if choice === 'letter' || choice === 'emoji'}
        <fieldset>
          <legend>Color</legend>
          <div class="swatches">
            {#each WORKSPACE_HUES as value (value)}
              <label class="swatch" title={hueLabel(value)}>
                <input
                  type="radio"
                  name="workspace-hue"
                  class="visually-hidden"
                  {value}
                  aria-label={hueLabel(value)}
                  bind:group={hue}
                />
                <WorkspaceTile
                  name={name || 'W'}
                  hue={value}
                  icon={choice === 'emoji' && emoji ? { kind: 'emoji', emoji } : { kind: 'letter' }}
                  size="lg"
                />
              </label>
            {/each}
          </div>
        </fieldset>
      {/if}

      {#if creating}
        <p class="info">
          Collections, environments, Globals, flows, cookies and Recently deleted belong to one
          workspace. Settings, Git accounts and certificates are shared by every workspace.
        </p>
      {/if}

      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>

    <footer class="foot">
      <span class="hint">
        {#if icon === null}
          {choice === 'logo'
            ? 'Pick a logo, or choose another icon.'
            : choice === 'image'
              ? 'Choose an image, or another icon.'
              : 'Choose an emoji, or another icon.'}
        {/if}
      </span>
      <button type="button" class="secondary" onclick={onClose}>Cancel</button>
      <button type="submit" class="primary" disabled={busy || !ready}>
        {#if creating}
          {busy ? 'Creating…' : 'Create workspace'}
        {:else}
          {busy ? 'Saving…' : 'Save'}
        {/if}
      </button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(500px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-dialog);
    /* The emoji picker may reach past the dialog's edge. */
    overflow: visible;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  form {
    display: flex;
    flex-direction: column;
  }

  /* Short windows: the options scroll so Cancel and Save stay in view (the emoji picker then
     scrolls with them instead of reaching past the dialog). */
  @media (max-height: 760px) {
    dialog[open] {
      overflow: hidden;
    }

    form {
      max-height: calc(100vh - 34px);
    }

    .body {
      min-height: 0;
      overflow-y: auto;
    }
  }

  .head,
  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 16px 22px;
  }

  .head {
    justify-content: space-between;
    border-bottom: 1px solid var(--border-soft);
  }

  .foot {
    border-top: 1px solid var(--border-soft);
  }

  .hint {
    flex: 1 1 160px;
    font-size: 12px;
    color: var(--text-muted);
  }

  h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
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
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 18px 22px 20px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .field label,
  legend {
    font-weight: 500;
  }

  .field input,
  .find input {
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 14px;
  }

  .field input:focus,
  .find input:focus {
    border-color: var(--focus);
    outline: none;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-bottom: 8px;
    padding: 0;
    font-size: 13px;
    color: var(--text-2);
  }

  .icon-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
  }

  .segmented {
    display: inline-flex;
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .segmented label {
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    font-size: 13px;
    cursor: pointer;
  }

  .segmented label.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 500;
    box-shadow: var(--shadow-segment);
  }

  .segmented label:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }

  .kind-panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-md);
  }

  .lookup {
    margin: 0;
    font-size: 13px;
    color: var(--text-2);
  }

  .lookup.bad {
    color: var(--err);
  }

  .candidates {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(124px, 1fr));
    gap: 8px;
  }

  .candidate {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 10px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
  }

  .candidate:hover {
    border-color: var(--border-strong);
  }

  /* The chosen logo is ringed, so it reads without relying on color. */
  .candidate[aria-pressed='true'] {
    border-color: var(--text);
    box-shadow: inset 0 0 0 1px var(--text);
    color: var(--text);
    font-weight: 500;
  }

  .website {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .find {
    display: flex;
    gap: 8px;
  }

  .find input {
    flex: 1;
    min-width: 0;
  }

  .note {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .emoji-wrap {
    position: relative;
    align-self: flex-start;
  }

  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .swatch {
    display: inline-flex;
    padding: 3px;
    border: 2px solid transparent;
    border-radius: 11px;
    cursor: pointer;
  }

  /* The chosen color is ringed, not only tinted, so it reads without seeing the hue. */
  .swatch:has(input:checked) {
    border-color: var(--text);
  }

  .swatch:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
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

  .secondary:hover {
    border-color: var(--text-muted);
  }
</style>
