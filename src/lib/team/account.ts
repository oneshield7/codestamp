import type { Account, AccountCheck, GitAccount } from '../api/team';

/** Only https addresses sign in with a username; SSH uses a key and a folder needs no account. */
export function usesSignIn(remote: string): boolean {
  return remote.trim().toLowerCase().startsWith('https://');
}

/** The host of an https remote in lower case, or null for other remotes and typing in progress. */
export function hostOf(remote: string): string | null {
  if (!usesSignIn(remote)) {
    return null;
  }
  try {
    return new URL(remote.trim()).hostname.toLowerCase() || null;
  } catch {
    // An address still being typed is not a URL yet.
    return null;
  }
}

/** The account to send for `remote`; a username typed for an https address is dropped for others. */
export function accountFor(remote: string, name: string, email: string, username: string): Account {
  const signIn = usesSignIn(remote) ? username.trim() : '';
  return { author: { name, email }, username: signIn === '' ? null : signIn };
}

export function accountKey(account: Pick<GitAccount, 'host' | 'username'>): string {
  return `${account.host} ${account.username ?? ''}`;
}

/** An account can be checked against a repository that uses it, or by name on GitHub. */
export function canCheck(account: GitAccount): boolean {
  return (
    account.collections.length > 0 || (account.host === 'github.com' && account.username !== null)
  );
}

export interface CheckTone {
  tone: 'ok' | 'warn' | 'err' | 'quiet';
  text: string;
}

const CHECK_TONES = new Map<AccountCheck['status'], CheckTone>([
  ['signedIn', { tone: 'ok', text: 'Signed in' }],
  ['signInNeeded', { tone: 'warn', text: 'Sign-in needed' }],
  ['offline', { tone: 'quiet', text: 'Offline' }],
  ['failed', { tone: 'err', text: 'Check failed' }],
]);

/** The latest check wins over what the credential manager listed when the page opened. */
export function checkTone(check: AccountCheck | null, signedIn: boolean | null): CheckTone | null {
  if (check) {
    return CHECK_TONES.get(check.status) ?? null;
  }
  if (signedIn === null) {
    return null;
  }
  return CHECK_TONES.get(signedIn ? 'signedIn' : 'signInNeeded') ?? null;
}
