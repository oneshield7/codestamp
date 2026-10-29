import { invoke } from '@tauri-apps/api/core';
import type { Auth, RequestKind, RequestMethod, SavedRequest } from './collections';

export type Described = 'collection' | 'section' | 'request';

export interface DocVariable {
  key: string;
  /** Empty for a secret, whose value never leaves the vault. */
  value: string;
  secret: boolean;
}

export interface SectionDocs {
  id: string;
  name: string;
  description: string;
}

export interface DocExample {
  name: string;
  status: number;
  contentType: string | null;
  body: string;
  /** The body was cut short for Docs. */
  cut: boolean;
}

export interface RequestDocs extends SavedRequest {
  description: string;
  examples: DocExample[];
}

export interface CollectionDocs {
  id: string;
  name: string;
  icon: string;
  description: string;
  auth: Auth;
  variables: DocVariable[];
  sections: SectionDocs[];
  /** In notebook order: requests outside sections, then each section's. */
  requests: RequestDocs[];
}

/** The longest description the core keeps. */
export const MAX_DESCRIPTION_CHARS = 20_000;

export function collectionDocs(collectionId: string): Promise<CollectionDocs> {
  return invoke('collection_docs', { collectionId });
}

/** Resolves to the description as it was stored. */
export function saveDescription(item: Described, id: string, text: string): Promise<string> {
  return invoke('save_description', { item, id, text });
}

/** A collection, section or request whose name or description holds every searched word. */
export interface DocHit {
  item: Described;
  id: string;
  collectionId: string;
  collectionName: string;
  collectionIcon: string;
  /** The section a request sits in. */
  sectionName: string | null;
  name: string;
  /** Requests only. */
  kind: RequestKind | null;
  method: RequestMethod | null;
  /** Part of the description around the first word found there; null for a name match. */
  snippet: string | null;
}

export function searchDocs(query: string): Promise<DocHit[]> {
  return invoke('search_docs', { query });
}

/** Opens the save dialog for a Markdown file; null when the user cancels. */
export function exportDocs(collectionId: string): Promise<string | null> {
  return invoke('export_docs', { collectionId });
}
