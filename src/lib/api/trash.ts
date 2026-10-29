import { invoke } from '@tauri-apps/api/core';

/** How long Recently deleted keeps a copy; the Rust core forgets older ones. */
export const KEEP_DAYS = 30;

export type DeletedKind = 'collection' | 'request';

export interface DeletedItem {
  id: string;
  kind: DeletedKind;
  name: string;
  /** The collection itself, or the one the request was in. */
  collectionId: string;
  collectionName: string;
  requestCount: number;
  deletedAt: number;
  /** For a request: its collection is still here, so it goes back into it. */
  collectionExists: boolean;
}

/** What a restore put back. */
export interface RestoredItem {
  kind: DeletedKind;
  collectionId: string;
  requestId: string | null;
}

/** Newest first. */
export function listDeleted(): Promise<DeletedItem[]> {
  return invoke('list_deleted');
}

export function restoreDeleted(id: string): Promise<RestoredItem> {
  return invoke('restore_deleted', { id });
}

/** Deletes the kept copy for good. */
export function forgetDeleted(id: string): Promise<void> {
  return invoke('forget_deleted', { id });
}

export function forgetAllDeleted(): Promise<void> {
  return invoke('forget_all_deleted');
}
