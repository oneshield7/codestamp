import { invoke } from '@tauri-apps/api/core';

/** Opens the system save dialog; resolves to the saved path, or null when the user cancels. */
export function saveCapture(png: Uint8Array, fileStem: string): Promise<string | null> {
  return invoke('save_capture', png, { headers: { 'x-capture-name': fileStem } });
}

/** Saves a run report (JSON text) through the system save dialog; null when cancelled. */
export function saveReport(report: string, name: string): Promise<string | null> {
  return invoke('save_report', { report, name });
}

/** Saves a kept response body byte for byte; the file type follows its Content-Type. */
export function saveResponseBody(bodyId: string, name: string): Promise<string | null> {
  return invoke('save_response_body', { bodyId, name });
}

/** Where an export went, and what was left out or emptied on the way. */
export interface ExportResult {
  path: string;
  notes: string[];
}

/** An export as the page shows it: where it went, or why it failed. */
export type ExportOutcome = ExportResult | { error: string };

/** `codestamp` is for another Codestamp; `postman` is Collection v2.1 for other API tools. */
export type CollectionFormat = 'codestamp' | 'postman';

/** Writes a collection file; secret values are left empty. Null when cancelled. */
export function exportCollection(
  id: string,
  format: CollectionFormat,
): Promise<ExportResult | null> {
  return invoke('export_collection', { id, format });
}

/** Writes an environment (or globals) file; secret values are left empty. */
export function exportEnvironment(id: string): Promise<ExportResult | null> {
  return invoke('export_environment', { id });
}
