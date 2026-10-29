import { invoke } from '@tauri-apps/api/core';

/** A cookie in the jar. Values are left out; read one with `revealCookie`. */
export interface StoredCookie {
  domain: string;
  path: string;
  name: string;
  /** Unix time in seconds; null until the end of the session. */
  expires: number | null;
  /** Sent to this exact host only, not to its subdomains. */
  hostOnly: boolean;
  secure: boolean;
  httpOnly: boolean;
}

export interface CookieKey {
  domain: string;
  path: string;
  name: string;
}

export function listCookies(): Promise<StoredCookie[]> {
  return invoke('list_cookies');
}

export function revealCookie(key: CookieKey): Promise<string> {
  return invoke('reveal_cookie', { key });
}

export function deleteCookie(key: CookieKey): Promise<void> {
  return invoke('delete_cookie', { key });
}

/** Clears every cookie, or only those of `domain`. */
export function clearCookies(domain: string | null): Promise<void> {
  return invoke('clear_cookies', { domain });
}
