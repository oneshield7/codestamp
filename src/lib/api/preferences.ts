import { invoke } from '@tauri-apps/api/core';

export type Indent = 'none' | 'two' | 'four' | 'tab';
export type CardLayout = 'stacked' | 'side';

export interface EditorPreferences {
  indent: Indent;
  closeBrackets: boolean;
  wrapLines: boolean;
  layout: CardLayout;
  variableHints: boolean;
}

export interface RequestPreferences {
  timeoutSeconds: number;
  /** `0` downloads responses of any size. */
  maxResponseMb: number;
  followRedirects: boolean;
  cookies: boolean;
  noCache: boolean;
  /** Empty sends `Codestamp/<version>`. */
  userAgent: string;
  /** Set in Settings › SSL/TLS. */
  minTlsVersion: TlsVersion;
}

/** The oldest TLS version a server may answer with. */
export type TlsVersion = '1.2' | '1.3';

export interface HistoryPreferences {
  maxEntries: number;
  keepDays: number;
}

export interface Preferences {
  editor: EditorPreferences;
  requests: RequestPreferences;
  history: HistoryPreferences;
}

export type ProxyMode = 'system' | 'off' | 'custom';
export type ProxyScheme = 'http' | 'https' | 'socks5';

export interface ProxySettings {
  mode: ProxyMode;
  scheme: ProxyScheme;
  host: string;
  port: number;
  username: string;
  bypass: string;
}

export interface ProxyView {
  settings: ProxySettings;
  hasPassword: boolean;
}

export type CertificateKind = 'ca' | 'client';

export interface CertificateInfo {
  id: string;
  kind: CertificateKind;
  name: string;
  host: string | null;
  createdAt: number;
}

export interface AboutInfo {
  version: string;
  dataDir: string;
  databaseBytes: number;
}

export function getPreferences(): Promise<Preferences> {
  return invoke('get_preferences');
}

export function savePreferences(preferences: Preferences): Promise<Preferences> {
  return invoke('save_preferences', { preferences });
}

export function getProxy(): Promise<ProxyView> {
  return invoke('get_proxy');
}

/** `password` undefined keeps the saved one; an empty text removes it. */
export function saveProxy(settings: ProxySettings, password?: string): Promise<ProxyView> {
  return invoke('save_proxy', { input: { settings, password: password ?? null } });
}

export function networkProblem(): Promise<string | null> {
  return invoke('network_problem');
}

export function listCertificates(): Promise<CertificateInfo[]> {
  return invoke('list_certificates');
}

/** Opens a file picker; `null` when the user cancels. */
export function addCaCertificate(): Promise<CertificateInfo | null> {
  return invoke('add_ca_certificate');
}

/** Opens file pickers for the certificate (and its key); `null` when the user cancels. */
export function addClientCertificate(host: string): Promise<CertificateInfo | null> {
  return invoke('add_client_certificate', { host });
}

export function deleteCertificate(id: string): Promise<void> {
  return invoke('delete_certificate', { id });
}

/** Opens a save dialog; the saved file's name, or `null` when the user cancels. */
export function backupData(): Promise<string | null> {
  return invoke('backup_data');
}

/** Opens a file picker and replaces all data; `false` when the user cancels. */
export function restoreData(): Promise<boolean> {
  return invoke('restore_data');
}

export function aboutApp(): Promise<AboutInfo> {
  return invoke('about_app');
}
