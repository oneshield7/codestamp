<script lang="ts">
  import { tick } from 'svelte';
  import type { RequestKind } from '../api/collections';
  import { HUBS, primaryFor, type HubAction } from '../home/hub';
  import type { NewCollectionStart, SettingsSection, View } from '../state/navigation';
  import IconButton from './IconButton.svelte';

  interface Props {
    /** The option the open New collection dialog started from; its button looks pressed. */
    activeStart: NewCollectionStart | null;
    /** The collection that adding a request and Run all act on. */
    collectionName: string | null;
    onNewCollection: (start: NewCollectionStart) => void;
    onNavigate: (view: View) => void;
    onOpenSettings: (section: SettingsSection) => void;
    onAddRequest: (kind: RequestKind) => void;
    onRunAll: () => void;
    /** Opens the current collection's History panel. */
    onOpenHistory: () => void;
    onSearch: () => void;
  }

  let {
    activeStart,
    collectionName,
    onNewCollection,
    onNavigate,
    onOpenSettings,
    onAddRequest,
    onRunAll,
    onOpenHistory,
    onSearch,
  }: Props = $props();

  let root: HTMLDivElement;
  let tabs: HTMLDivElement;
  let hubIndex = $state(0);
  // Each section remembers its chosen tile, so switching back returns to it.
  let tileIndexes = $state<number[]>(HUBS.map(() => 0));
  let commandIndex = $state(0);
  let copyStatus = $state<{ tone: 'ok' | 'err'; text: string } | null>(null);
  let indicator = $state<{ left: number; top: number; width: number; height: number } | null>(null);

  const hub = $derived(HUBS.at(hubIndex));
  const tileIndex = $derived(tileIndexes.at(hubIndex) ?? 0);
  const tile = $derived(hub?.tiles.at(tileIndex));
  const command = $derived(tile?.commands.at(commandIndex));
  const primary = $derived(tile ? primaryFor(tile, collectionName) : null);

  function selectHub(index: number): void {
    const next = (index + HUBS.length) % HUBS.length;
    if (next === hubIndex) {
      return;
    }
    hubIndex = next;
    commandIndex = 0;
    copyStatus = null;
  }

  function selectTile(index: number): void {
    const count = hub?.tiles.length ?? 0;
    if (count === 0) {
      return;
    }
    const next = (index + count) % count;
    tileIndexes = tileIndexes.map((value, position) => (position === hubIndex ? next : value));
    commandIndex = 0;
    copyStatus = null;
  }

  function selectCommand(index: number): void {
    commandIndex = index;
    copyStatus = null;
  }

  async function copyCommand(): Promise<void> {
    if (!command) {
      return;
    }
    try {
      await navigator.clipboard.writeText(command.command);
      copyStatus = { tone: 'ok', text: 'Copied to the clipboard.' };
    } catch {
      copyStatus = {
        tone: 'err',
        text: 'Could not copy here. Select the command and copy it instead.',
      };
    }
  }

  function run(action: HubAction): void {
    switch (action.kind) {
      case 'newCollection':
        onNewCollection(action.start);
        break;
      case 'navigate':
        onNavigate(action.view);
        break;
      case 'settings':
        onOpenSettings(action.section);
        break;
      case 'addRequest':
        onAddRequest(action.request);
        break;
      case 'runAll':
        onRunAll();
        break;
      case 'history':
        onOpenHistory();
        break;
      case 'search':
        onSearch();
        break;
      case 'copy':
        void copyCommand();
        break;
    }
  }

  function opensDialog(action: HubAction): 'dialog' | undefined {
    return action.kind === 'newCollection' || action.kind === 'search' ? 'dialog' : undefined;
  }

  function dialogOpen(action: HubAction): boolean | undefined {
    return action.kind === 'newCollection' ? activeStart === action.start : undefined;
  }

  function placeIndicator(index: number): void {
    const selected = tabs.querySelectorAll<HTMLButtonElement>('[role="tab"]').item(index);
    indicator = {
      left: selected.offsetLeft,
      top: selected.offsetTop,
      width: selected.offsetWidth,
      height: selected.offsetHeight,
    };
  }

  $effect(() => {
    placeIndicator(hubIndex);
  });

  $effect(() => {
    const observer = new ResizeObserver(() => {
      placeIndicator(hubIndex);
    });
    observer.observe(tabs);
    return () => {
      observer.disconnect();
    };
  });

  async function focusSelectedTab(): Promise<void> {
    await tick();
    tabs.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus();
  }

  function arrowStep(key: string): number {
    if (key === 'ArrowRight') {
      return 1;
    }
    return key === 'ArrowLeft' ? -1 : 0;
  }

  /**
   * Home's keys work while nothing else has the keyboard: the page itself or a control on it.
   * Typing a number picks a section, the arrows pick a tile, and Enter runs its main action.
   */
  function onWindowKey(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.ctrlKey || event.altKey || event.metaKey) {
      return;
    }
    const focused = document.activeElement;
    const onHome = focused === document.body || (focused !== null && root.contains(focused));
    if (!onHome || event.shiftKey || document.querySelector('dialog[open]')) {
      return;
    }
    const section = HUBS.findIndex((_, index) => String(index + 1) === event.key);
    if (section >= 0) {
      event.preventDefault();
      selectHub(section);
      return;
    }
    const step = arrowStep(event.key);
    // Radio buttons already move with the arrow keys.
    if (step !== 0 && !(focused instanceof HTMLInputElement)) {
      event.preventDefault();
      if (focused.getAttribute('role') === 'tab') {
        selectHub(hubIndex + step);
        void focusSelectedTab();
      } else {
        selectTile(tileIndex + step);
      }
      return;
    }
    if (event.key === 'Enter' && focused === document.body && primary) {
      event.preventDefault();
      run(primary.action);
    }
  }
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="home" bind:this={root}>
  <h1 class="visually-hidden">Home</h1>
  <header class="topbar">
    <div class="hubs" role="tablist" aria-label="Home sections" bind:this={tabs}>
      {#if indicator}
        <span
          class="indicator"
          aria-hidden="true"
          style:transform="translate({indicator.left}px, {indicator.top}px)"
          style:width="{indicator.width}px"
          style:height="{indicator.height}px"
        ></span>
      {/if}
      {#each HUBS as item, index (item.key)}
        <button
          type="button"
          role="tab"
          aria-selected={index === hubIndex}
          tabindex={index === hubIndex ? 0 : -1}
          onclick={() => {
            selectHub(index);
          }}>{item.label}</button
        >
      {/each}
    </div>
    <p class="context">
      {#if collectionName}
        Collection <strong>{collectionName}</strong>
      {:else}
        No collection yet
      {/if}
    </p>
  </header>

  {#if hub && tile && primary}
    {#key hub.key}
      <div class="tiles" role="radiogroup" aria-label={hub.label}>
        {#each hub.tiles as item, index (item.key)}
          <label
            class="tile"
            class:on={index === tileIndex}
            data-hue={item.hue}
            style:--order={index}
          >
            <input
              type="radio"
              name="home-tile"
              class="visually-hidden"
              aria-label={item.name}
              checked={index === tileIndex}
              onchange={() => {
                selectTile(index);
              }}
              onkeydown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  run(primary.action);
                }
              }}
            />
            <svg width="30" height="30" viewBox="0 0 24 24" aria-hidden="true">
              <path d={item.icon} />
            </svg>
            <span class="tip" aria-hidden="true">{item.name}</span>
          </label>
        {/each}
      </div>
    {/key}

    <section class="stage" data-hue={tile.hue} aria-labelledby="home-stage-title">
      {#key tile.key}
        <svg class="mark" viewBox="0 0 24 24" aria-hidden="true"><path d={tile.icon} /></svg>
        <div class="text">
          {#if tile.facts.length > 0}
            <p class="facts">{tile.facts.join(' · ')}</p>
          {/if}
          <h2 id="home-stage-title">{tile.title}</h2>
          <p class="description">{tile.description}</p>
        </div>
      {/key}

      {#if command}
        {#if tile.commands.length > 1}
          <div class="variants" role="radiogroup" aria-label="Examples">
            {#each tile.commands as item, index (item.label)}
              <label class="variant" class:on={index === commandIndex}>
                <input
                  type="radio"
                  name="home-command"
                  class="visually-hidden"
                  checked={index === commandIndex}
                  onchange={() => {
                    selectCommand(index);
                  }}
                />
                {item.label}
              </label>
            {/each}
          </div>
        {/if}
        <div class="command">
          <span class="prompt" aria-hidden="true">$</span>
          <code>{command.command}</code>
          <IconButton label="Copy command" align="end" onclick={() => void copyCommand()}>
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="8" y="8" width="12" height="12" rx="2" /><path
                d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"
              />
            </svg>
          </IconButton>
        </div>
      {/if}

      <div class="buttons">
        <button
          type="button"
          class="primary"
          aria-haspopup={opensDialog(primary.action)}
          aria-expanded={dialogOpen(primary.action)}
          onclick={() => {
            run(primary.action);
          }}>{primary.label}</button
        >
        {#each tile.extras as extra (extra.label)}
          <IconButton
            label={extra.label}
            round
            popup={opensDialog(extra.action)}
            expanded={dialogOpen(extra.action)}
            onclick={() => {
              run(extra.action);
            }}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
              <path d={extra.icon} />
            </svg>
          </IconButton>
        {/each}
      </div>
      <p class="status" class:err={copyStatus?.tone === 'err'} role="status">
        {copyStatus?.text ?? ''}
      </p>
    </section>
  {/if}
</div>

<style>
  /* Fills the window below the page padding, so the stage grows to the bottom. */
  .home {
    min-height: calc(100vh - var(--titlebar-height) - var(--statusbar-height) - 76px);
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  @media (max-width: 899px) {
    .home {
      min-height: calc(100vh - var(--titlebar-height) - var(--statusbar-height) - 60px);
    }
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .topbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .hubs {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
  }

  /* One pill slides to the chosen tab instead of each tab painting its own background. */
  .indicator {
    position: absolute;
    top: 0;
    left: 0;
    border-radius: 999px;
    background: var(--accent-soft);
    transition:
      transform 280ms cubic-bezier(0.3, 0.8, 0.3, 1),
      width 280ms cubic-bezier(0.3, 0.8, 0.3, 1);
  }

  .hubs button {
    position: relative;
    padding: 6px 16px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 500;
  }

  .hubs button:hover {
    color: var(--text);
  }

  .hubs button[aria-selected='true'] {
    color: var(--text);
  }

  .context {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .context strong {
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-weight: 600;
  }

  .tiles {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
  }

  .tile {
    position: relative;
    width: 72px;
    height: 72px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--hue-bg);
    cursor: pointer;
    transition:
      transform 200ms ease,
      box-shadow 200ms ease,
      border-color 200ms ease;
    /* A new section's tiles come in one after another. */
    animation: tile-in 320ms cubic-bezier(0.2, 0.7, 0.2, 1) backwards;
    animation-delay: calc(var(--order) * 45ms);
  }

  .tile:hover {
    border-color: var(--hue);
  }

  .tile.on {
    border-color: var(--hue);
    box-shadow:
      0 0 0 3px var(--bg),
      0 0 0 5px var(--hue);
    transform: scale(1.04);
  }

  .tile:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 8px;
  }

  .tip {
    position: absolute;
    top: calc(100% + 10px);
    left: 50%;
    z-index: 30;
    transform: translateX(-50%);
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    pointer-events: none;
    visibility: hidden;
  }

  /* The chosen tile's name is already the stage title, so only the others show it on hover. */
  .tile:not(.on):hover .tip {
    visibility: visible;
  }

  .stage {
    position: relative;
    flex: 1 0 auto;
    min-height: 340px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: flex-start;
    gap: 16px;
    padding: 32px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    /* data-hue colours its element; only the mark and the facts take the hue here. */
    color: var(--text);
    container-type: inline-size;
  }

  /* The chosen tile's icon, drawn large and faint, carries its colour into the stage. */
  .mark {
    position: absolute;
    top: 50%;
    right: 6%;
    width: clamp(200px, 30cqw, 400px);
    height: clamp(200px, 30cqw, 400px);
    transform: translateY(-62%);
    stroke-width: 1;
    color: var(--hue);
    opacity: 0.16;
    animation: mark-in 480ms cubic-bezier(0.2, 0.7, 0.2, 1);
  }

  /* The text column is 620px wide, so below this the mark would sit behind the text. */
  @container (max-width: 900px) {
    .mark {
      display: none;
    }
  }

  .text {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 620px;
  }

  /* The stage text rises line by line when the chosen tile changes. */
  .text > * {
    animation: rise 320ms cubic-bezier(0.2, 0.7, 0.2, 1) backwards;
  }

  .text > :nth-child(2) {
    animation-delay: 50ms;
  }

  .text > :nth-child(3) {
    animation-delay: 100ms;
  }

  .variants,
  .command {
    animation: rise 320ms cubic-bezier(0.2, 0.7, 0.2, 1) 120ms backwards;
  }

  .facts {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--hue);
  }

  h2 {
    margin: 0;
    font-size: 34px;
    font-weight: 600;
    letter-spacing: -0.025em;
    line-height: 1.15;
  }

  .description {
    margin: 0;
    font-size: 15px;
    line-height: 1.6;
    color: var(--text-2);
  }

  .variants {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .variant {
    padding: 5px 14px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  .variant:hover {
    color: var(--text);
  }

  .variant.on {
    border-color: var(--text);
    background: var(--accent-soft);
    color: var(--text);
  }

  .variant:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  .command {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    max-width: 620px;
    padding: 5px 5px 5px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font-family: var(--font-code);
    font-size: var(--font-size-code);
  }

  .prompt {
    color: var(--text-muted);
  }

  .command code {
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    white-space: nowrap;
    color: var(--text);
  }

  .buttons {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    max-width: 100%;
  }

  .primary {
    max-width: 100%;
    height: 40px;
    padding: 0 22px;
    overflow: hidden;
    border: 0;
    border-radius: 999px;
    background: var(--text);
    color: var(--surface);
    font-size: 14px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .primary:hover {
    background: var(--text-2);
  }

  .status {
    min-height: 18px;
    margin: -8px 0 0;
    font-size: 12px;
    color: var(--ok);
  }

  .status.err {
    color: var(--err);
  }

  @keyframes rise {
    from {
      opacity: 0;
      translate: 0 8px;
    }
  }

  @keyframes tile-in {
    from {
      opacity: 0;
      translate: 0 10px;
    }
  }

  @keyframes mark-in {
    from {
      opacity: 0;
      scale: 0.82;
      rotate: -8deg;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .indicator,
    .tile,
    .text > *,
    .variants,
    .command,
    .mark {
      transition: none;
      animation: none;
    }
  }
</style>
