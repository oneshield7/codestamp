<script lang="ts">
  import NavIcon from './NavIcon.svelte';
  import { onMount } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import {
    clearCookies,
    deleteCookie,
    listCookies,
    revealCookie,
    type CookieKey,
    type StoredCookie,
  } from '../api/cookies';
  import { formatExpiry } from '../format/time';
  import { SECRET_MASK } from '../privacy/secrets';
  import { notices } from '../state/notices.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';

  let cookies: StoredCookie[] = $state([]);
  let loaded = $state(false);
  /** Values the user chose to show, by cookie; forgotten whenever the list reloads. */
  const revealed = new SvelteMap<string, string>();
  let pendingClear: { domain: string | null } | null = $state(null);

  // The list arrives sorted by domain, so neighbours share a group.
  const groups = $derived.by(() => {
    const list: { domain: string; cookies: StoredCookie[] }[] = [];
    for (const cookie of cookies) {
      const last = list.at(-1);
      if (last?.domain === cookie.domain) {
        last.cookies.push(cookie);
      } else {
        list.push({ domain: cookie.domain, cookies: [cookie] });
      }
    }
    return list;
  });

  function keyOf(cookie: CookieKey): string {
    return `${cookie.domain}\n${cookie.path}\n${cookie.name}`;
  }

  function keyFor(cookie: StoredCookie): CookieKey {
    return { domain: cookie.domain, path: cookie.path, name: cookie.name };
  }

  function flags(cookie: StoredCookie): string {
    return [
      cookie.hostOnly ? 'This host only' : 'Subdomains too',
      cookie.secure ? 'Secure' : '',
      cookie.httpOnly ? 'HttpOnly' : '',
    ]
      .filter(Boolean)
      .join(' · ');
  }

  async function load(): Promise<void> {
    await notices.guard(async () => {
      cookies = await listCookies();
      revealed.clear();
    });
    loaded = true;
  }

  async function toggle(cookie: StoredCookie): Promise<void> {
    const key = keyOf(cookie);
    if (revealed.has(key)) {
      revealed.delete(key);
      return;
    }
    await notices.guard(async () => {
      revealed.set(key, await revealCookie(keyFor(cookie)));
    });
  }

  async function remove(cookie: StoredCookie): Promise<void> {
    await notices.guard(async () => {
      await deleteCookie(keyFor(cookie));
    });
    await load();
  }

  async function confirmClear(): Promise<void> {
    const target = pendingClear;
    pendingClear = null;
    if (!target) {
      return;
    }
    await notices.guard(async () => {
      await clearCookies(target.domain);
    });
    await load();
  }

  onMount(() => {
    void load();
  });
</script>

<div class="view">
  <header class="top">
    <div class="title">
      <h1 class="page-title"><NavIcon name="cookies" size="lg" />Cookies</h1>
      <p>
        Cookies set by responses are kept here and sent back to matching hosts, like a browser does.
        They are stored encrypted on this device. A Cookie header you type yourself is sent instead.
      </p>
    </div>
    <button
      type="button"
      class="quiet danger"
      disabled={cookies.length === 0}
      onclick={() => (pendingClear = { domain: null })}
    >
      Clear all
    </button>
  </header>

  {#if loaded && cookies.length === 0}
    <p class="empty">No cookies yet. They show up here after a response sets one.</p>
  {/if}

  {#each groups as group (group.domain)}
    <section class="group" aria-label="Cookies for {group.domain}">
      <div class="group-head">
        <h2 class="mono">{group.domain}</h2>
        <span class="count">{group.cookies.length}</span>
        <span class="spacer"></span>
        <button
          type="button"
          class="link danger"
          onclick={() => (pendingClear = { domain: group.domain })}
        >
          Delete all
        </button>
      </div>
      <div class="scroll">
        <table>
          <thead>
            <tr>
              <th scope="col">Name</th>
              <th scope="col">Value</th>
              <th scope="col">Path</th>
              <th scope="col">Expires</th>
              <th scope="col">Flags</th>
              <th scope="col" class="actions"><span class="visually-hidden">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {#each group.cookies as cookie (keyOf(cookie))}
              {@const value = revealed.get(keyOf(cookie))}
              <tr>
                <td class="mono">{cookie.name}</td>
                <td class="value">
                  <span class="mono">{value ?? SECRET_MASK}</span>
                  <button type="button" class="link" onclick={() => void toggle(cookie)}>
                    {value === undefined ? 'Show' : 'Hide'}
                  </button>
                </td>
                <td class="mono">{cookie.path}</td>
                <td>{formatExpiry(cookie.expires)}</td>
                <td class="flags">{flags(cookie)}</td>
                <td class="actions">
                  <button
                    type="button"
                    class="remove"
                    aria-label="Delete cookie {cookie.name}"
                    onclick={() => void remove(cookie)}
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M6 6l12 12M18 6L6 18" />
                    </svg>
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/each}
</div>

<ConfirmDialog
  open={pendingClear !== null}
  title={pendingClear?.domain ? `Delete cookies for ${pendingClear.domain}?` : 'Clear all cookies?'}
  message={pendingClear?.domain
    ? 'Every cookie of this domain will be removed. Logged-in sessions there end.'
    : 'Every saved cookie will be removed. Logged-in sessions end.'}
  confirmLabel="Delete"
  onConfirm={() => void confirmClear()}
  onCancel={() => (pendingClear = null)}
/>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 16px;
  }

  .title {
    flex: 1 1 360px;
  }

  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .title p {
    margin: 4px 0 0;
    max-width: 640px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .empty {
    margin: 0;
    padding: 24px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-lg);
    color: var(--text-muted);
    text-align: center;
  }

  .group {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .group-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-soft);
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .count {
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--surface-sunken);
    color: var(--text-2);
    font-size: 12px;
  }

  .spacer {
    flex: 1;
  }

  .scroll {
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 560px;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    padding: 8px 16px;
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-soft);
  }

  td {
    padding: 8px 16px;
    border-bottom: 1px solid var(--border-soft);
    vertical-align: top;
  }

  tr:last-child td {
    border-bottom: 0;
  }

  .value {
    max-width: 320px;
    overflow-wrap: anywhere;
  }

  .value .link {
    margin-left: 8px;
  }

  .flags {
    color: var(--text-muted);
    font-size: 12px;
  }

  .actions {
    width: 44px;
  }

  .quiet {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .quiet.danger:hover:not(:disabled) {
    border-color: var(--err);
    color: var(--err);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: 12px;
  }

  .link.danger {
    color: var(--err);
  }

  .remove {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .remove:hover {
    background: var(--surface-sunken);
    color: var(--err);
  }

  .remove path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
</style>
