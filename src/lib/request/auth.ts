import type { Auth, AuthType } from '../api/collections';
import type { NameValue } from '../api/http';
import { SECRET_MASK } from '../privacy/secrets';

export interface AuthOption {
  type: AuthType;
  label: string;
  hint: string;
}

/** A header or query parameter that the auth adds when the request is sent. */
export interface AuthPair {
  name: string;
  value: string;
  location: 'header' | 'query';
}

const AUTHORIZATION = 'Authorization';
const ONLY_VARIABLE = /^\{\{[\w.-]+\}\}$/;
const SHORT_LABELS = new Map<AuthType, string>([
  ['inherit', 'Inherit'],
  ['none', 'None'],
  ['bearer', 'Bearer'],
  ['basic', 'Basic'],
  ['apiKey', 'API key'],
  ['digest', 'Digest'],
  ['awsSigV4', 'AWS Signature'],
  ['oauth2', 'OAuth 2.0'],
]);
/** Shown where the real token goes, since it never leaves the Rust core. */
export const ACCESS_TOKEN_PLACEHOLDER = '<access token>';

export function blankAuth(type: AuthType): Auth {
  switch (type) {
    case 'inherit':
    case 'none':
      return { type };
    case 'bearer':
      return { type, token: '', prefix: 'Bearer' };
    case 'basic':
      return { type, username: '', password: '' };
    case 'apiKey':
      return { type, key: 'X-API-Key', value: '', location: 'header' };
    case 'digest':
      return { type, username: '', password: '' };
    case 'awsSigV4':
      return { type, accessKey: '', secretKey: '', sessionToken: '', region: '', service: '' };
    case 'oauth2':
      return {
        type,
        grant: 'authorizationCode',
        authUrl: '',
        tokenUrl: '',
        clientId: '',
        clientSecret: '',
        scope: '',
        redirectUri: '',
        usePkce: true,
        username: '',
        password: '',
        clientAuth: 'header',
        headerPrefix: '',
      };
  }
}

export function authLabel(auth: Auth): string {
  return SHORT_LABELS.get(auth.type) ?? '';
}

export function authOptions(allowInherit: boolean, collectionAuth: Auth | null): AuthOption[] {
  const options: AuthOption[] = [
    { type: 'bearer', label: 'Bearer token', hint: 'Authorization: Bearer …' },
    { type: 'basic', label: 'Basic auth', hint: 'Username and password' },
    { type: 'apiKey', label: 'API key', hint: 'In a header or query param' },
    { type: 'oauth2', label: 'OAuth 2.0', hint: 'Gets and refreshes an access token' },
    { type: 'digest', label: 'Digest auth', hint: 'Answers the server’s challenge' },
    { type: 'awsSigV4', label: 'AWS Signature', hint: 'Signs the request (SigV4)' },
    { type: 'none', label: 'No auth', hint: 'Nothing is added' },
  ];
  if (!allowInherit) {
    return options;
  }
  const inherited = collectionAuth ? authSummary(collectionAuth) : 'No auth';
  return [{ type: 'inherit', label: 'Inherit from collection', hint: inherited }, ...options];
}

/** The auth a request really uses: its own, or the collection's when it inherits. */
export function effectiveAuth(auth: Auth, collectionAuth: Auth | null): Auth {
  if (auth.type !== 'inherit') {
    return auth;
  }
  return collectionAuth && collectionAuth.type !== 'inherit' ? collectionAuth : { type: 'none' };
}

/**
 * What the auth adds, with values as typed (variables are not filled in here).
 * Null when nothing is added or the settings are still incomplete.
 */
export function authPair(auth: Auth): AuthPair | null {
  switch (auth.type) {
    case 'inherit':
    case 'none':
    case 'digest':
    case 'awsSigV4':
      return null;
    case 'oauth2':
      return {
        name: AUTHORIZATION,
        value: `${oauthPrefix(auth.headerPrefix)} ${ACCESS_TOKEN_PLACEHOLDER}`,
        location: 'header',
      };
    case 'bearer': {
      // Named "credential" so the linter does not mistake an emptiness check for a secret compare.
      const credential = auth.token.trim();
      const prefix = auth.prefix.trim();
      if (credential.length === 0) {
        return null;
      }
      return {
        name: AUTHORIZATION,
        value: prefix ? `${prefix} ${credential}` : credential,
        location: 'header',
      };
    }
    case 'basic':
      if (auth.username === '') {
        return null;
      }
      return {
        name: AUTHORIZATION,
        value: basicValue(auth.username, auth.password),
        location: 'header',
      };
    case 'apiKey': {
      const key = auth.key.trim();
      return key === '' ? null : { name: key, value: auth.value, location: auth.location };
    }
  }
}

/** Adds the auth's header, like the Rust core does: a header typed by the user wins. */
export function addAuthHeader(pair: AuthPair | null, headers: NameValue[]): void {
  const typed = (name: string): boolean =>
    headers.some((header) => header.name.toLowerCase() === name.toLowerCase());
  if (pair?.location === 'header' && !typed(pair.name)) {
    headers.push({ name: pair.name, value: pair.value });
  }
}

/** Adds the auth's query parameter unless one with that exact name was typed. */
export function addAuthQuery(pair: AuthPair | null, params: NameValue[]): void {
  if (pair?.location === 'query' && !params.some((param) => param.name === pair.name)) {
    params.push({ name: pair.name, value: pair.value });
  }
}

/** A one-line preview for the Auth tab; secret values are masked unless they are a variable. */
export function authPreview(auth: Auth): string {
  switch (auth.type) {
    case 'inherit':
      return 'Uses the collection’s auth.';
    case 'none':
      return 'No auth header or parameter is added.';
    case 'bearer':
      return auth.token.trim() === ''
        ? 'Add a token to send the Authorization header.'
        : `Authorization: ${[auth.prefix.trim(), shown(auth.token)].filter(Boolean).join(' ')}`;
    case 'basic':
      return auth.username === ''
        ? 'Add a username to send the Authorization header.'
        : `Authorization: Basic ${SECRET_MASK}`;
    case 'apiKey':
      if (auth.key.trim() === '') {
        return 'Add the key name.';
      }
      return auth.location === 'header'
        ? `${auth.key.trim()}: ${shown(auth.value)}`
        : `?${auth.key.trim()}=${shown(auth.value)}`;
    case 'digest':
      return auth.username === ''
        ? 'Add a username to answer the server’s Digest challenge.'
        : 'Authorization: Digest … (answers the server’s 401 challenge)';
    case 'awsSigV4':
      return 'Authorization: AWS4-HMAC-SHA256 … (signed when sending)';
    case 'oauth2':
      return `Authorization: ${oauthPrefix(auth.headerPrefix)} ${ACCESS_TOKEN_PLACEHOLDER}`;
  }
}

export function oauthPrefix(prefix: string): string {
  return prefix.trim() === '' ? 'Bearer' : prefix.trim();
}

/** The kind of auth and what it adds, such as "Bearer · Authorization", never its value. */
export function authSummary(auth: Auth): string {
  if (auth.type === 'none' || auth.type === 'inherit') {
    return 'No auth';
  }
  if (auth.type === 'digest' || auth.type === 'awsSigV4') {
    return authLabel(auth);
  }
  const pair = authPair(auth);
  return pair ? `${authLabel(auth)} · ${pair.name}` : `${authLabel(auth)} (incomplete)`;
}

function shown(value: string): string {
  return ONLY_VARIABLE.test(value.trim()) ? value.trim() : SECRET_MASK;
}

// Templates cannot be encoded before their variables are known, so they are shown as typed.
function basicValue(username: string, password: string): string {
  const credentials = `${username}:${password}`;
  if (credentials.includes('{{')) {
    return `Basic ${credentials}`;
  }
  const bytes = new TextEncoder().encode(credentials);
  return `Basic ${btoa(String.fromCharCode(...bytes))}`;
}
