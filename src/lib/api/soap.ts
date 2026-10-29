import { invoke } from '@tauri-apps/api/core';
import type { SoapVersion } from './collections';
import type { PickedFile } from './http';
import type { ImportPreview, ImportResult } from './import';

/** A WSDL by URL, a file picked on this device, or the text of a dropped file. */
export type WsdlSource =
  | { kind: 'url'; url: string }
  | { kind: 'file'; path: string }
  | { kind: 'content'; content: string };

export interface SoapOperationInfo {
  name: string;
  action: string;
  documentation: string;
  /** A sample request with `?` where values go, as SoapUI writes it. */
  envelope: string;
}

/** One SOAP binding of the WSDL (SoapUI's "interface"). */
export interface SoapInterfaceInfo {
  name: string;
  version: SoapVersion;
  endpoints: string[];
  operations: SoapOperationInfo[];
}

export interface WsdlDefinition {
  name: string;
  targetNamespace: string;
  interfaces: SoapInterfaceInfo[];
  /** What could not be read, such as an import; its types show `?` instead. */
  warnings: string[];
}

export function loadWsdl(source: WsdlSource): Promise<WsdlDefinition> {
  return invoke<WsdlDefinition>('soap_load_wsdl', { source });
}

/** Only files picked here can be read. `null` if cancelled. */
export function pickWsdlFile(): Promise<PickedFile | null> {
  return invoke<PickedFile | null>('pick_wsdl_file');
}

export function previewWsdlImport(source: WsdlSource): Promise<ImportPreview> {
  return invoke<ImportPreview>('preview_wsdl_import', { source });
}

export function runWsdlImport(source: WsdlSource, name: string): Promise<ImportResult> {
  return invoke<ImportResult>('run_wsdl_import', { source, name });
}

/** A URL is fetched; anything else is a file picked on this device. */
export function wsdlSource(where: string): WsdlSource {
  const trimmed = where.trim();
  return /^https?:\/\//i.test(trimmed)
    ? { kind: 'url', url: trimmed }
    : { kind: 'file', path: trimmed };
}
