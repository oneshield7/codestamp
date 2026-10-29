import { invoke } from '@tauri-apps/api/core';
import type { SavedRequest } from './collections';
import type { HttpMethod } from './http';

export type StatusFilter =
  'all' | 'success' | 'redirect' | 'clientError' | 'serverError' | 'failed';

export interface HistoryEntry {
  id: string;
  collectionId: string | null;
  collectionName: string | null;
  requestId: string | null;
  name: string;
  method: HttpMethod;
  url: string;
  status: number | null;
  error: string | null;
  elapsedMs: number | null;
  bodySize: number | null;
  createdAt: number;
}

export interface HistoryFilter {
  query: string | null;
  status: StatusFilter;
  /** History lives in each collection; null lists every entry. */
  collectionId: string | null;
}

export function listHistory(filter: HistoryFilter): Promise<HistoryEntry[]> {
  return invoke('list_history', { filter });
}

export function deleteHistoryEntry(id: string): Promise<void> {
  return invoke('delete_history_entry', { id });
}

/** Clears one collection's history, or all of it when `collectionId` is null. */
export function clearHistory(collectionId: string | null): Promise<void> {
  return invoke('clear_history', { collectionId });
}

export function saveHistoryToCollection(
  id: string,
  collectionId: string,
  sectionId: string | null,
): Promise<SavedRequest> {
  return invoke('save_history_to_collection', { id, collectionId, sectionId });
}
