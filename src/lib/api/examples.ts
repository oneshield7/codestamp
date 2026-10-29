import { invoke } from '@tauri-apps/api/core';
import type { NameValue } from './http';

export interface ExampleInput {
  name: string;
  status: number;
  headers: NameValue[];
  body: string;
}

export interface Example extends ExampleInput {
  id: string;
  requestId: string;
}

/** What a list shows; the body loads when an example is opened. */
export interface ExampleSummary {
  id: string;
  name: string;
  status: number;
  /** Body size in bytes. */
  size: number;
}

export function listExamples(requestId: string): Promise<ExampleSummary[]> {
  return invoke('list_examples', { requestId });
}

export function getExample(id: string): Promise<Example> {
  return invoke('get_example', { id });
}

export function saveExample(requestId: string, example: ExampleInput): Promise<Example> {
  return invoke('save_example', { requestId, example });
}

export function renameExample(id: string, name: string): Promise<void> {
  return invoke('rename_example', { id, name });
}

export function deleteExample(id: string): Promise<void> {
  return invoke('delete_example', { id });
}
