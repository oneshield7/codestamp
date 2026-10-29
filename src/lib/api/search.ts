import { invoke } from '@tauri-apps/api/core';
import type { RequestKind, RequestMethod } from './collections';

/** One request as quick search lists it, with where it lives. */
export interface SearchEntry {
  requestId: string;
  collectionId: string;
  collectionName: string;
  collectionIcon: string;
  sectionName: string | null;
  kind: RequestKind;
  name: string;
  method: RequestMethod;
  url: string;
}

/** Every request across collections, in notebook order. */
export function searchIndex(): Promise<SearchEntry[]> {
  return invoke('search_index');
}
