import { invoke } from '@tauri-apps/api/core';
import type { Auth } from './collections';

/** What is known about a stored token; the token itself never leaves the Rust core. */
export interface TokenStatus {
  /** Milliseconds since 1970; null when the server gave no lifetime. */
  expiresAt: number | null;
  scope: string | null;
  refreshable: boolean;
  obtainedAt: number;
}

/**
 * Gets a token for a request, or for a collection whose requests inherit its auth. The
 * Authorization Code grant opens a sign-in window first.
 */
export function getOAuthToken(
  ownerId: string,
  collectionId: string | null,
  auth: Auth,
): Promise<TokenStatus> {
  return invoke('oauth_get_token', { ownerId, collectionId, auth });
}

export function oauthTokenStatus(ownerId: string): Promise<TokenStatus | null> {
  return invoke('oauth_token_status', { ownerId });
}

export function clearOAuthToken(ownerId: string): Promise<void> {
  return invoke('oauth_clear_token', { ownerId });
}
