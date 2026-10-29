import { invoke } from '@tauri-apps/api/core';
import type { RequestContent } from './collections';

export type ImportKind = 'collection' | 'environment';

/** Files up to this size are accepted (also enforced in the Rust core). */
export const MAX_IMPORT_BYTES = 20 * 1024 * 1024;

export interface ImportPreview {
  kind: ImportKind;
  /** What was read, such as "Collection v2.1" or "OpenAPI 3.0.3". */
  source: string;
  name: string;
  requestCount: number;
  sectionCount: number;
  variableCount: number;
  /** Environments that belong to the collection; only a Codestamp file has them. */
  environmentCount: number;
  scriptCount: number;
  warnings: string[];
}

export interface ImportResult {
  kind: ImportKind;
  id: string;
}

export interface CurlImport {
  request: RequestContent;
  warnings: string[];
}

export function previewImport(content: string): Promise<ImportPreview> {
  return invoke('preview_import', { content });
}

export function runImport(content: string, name: string): Promise<ImportResult> {
  return invoke('run_import', { content, name });
}

/**
 * Replaces the sections, requests, auth, docs and variables of an existing collection with a
 * collection file. The collection keeps its id, name, icon and Collaboration link.
 */
export function replaceWithImport(content: string, collectionId: string): Promise<ImportResult> {
  return invoke('replace_with_import', { content, collectionId });
}

/** Downloads a file to import (HTTPS, or HTTP on this machine). */
export function fetchImport(url: string): Promise<string> {
  return invoke('fetch_import', { url });
}

export function parseCurl(command: string): Promise<CurlImport> {
  return invoke('parse_curl', { command });
}
