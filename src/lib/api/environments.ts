import { invoke } from '@tauri-apps/api/core';
import type { NameValue } from './http';

export interface EnvironmentSummary {
  id: string;
  name: string;
  isGlobal: boolean;
  /** The collection it belongs to and is shared with; null keeps it on this device. */
  collectionId: string | null;
  variableCount: number;
}

export interface EnvironmentList {
  environments: EnvironmentSummary[];
  activeId: string | null;
}

/** Secret variables arrive with an empty `value`; read one with `revealSecret`. */
export interface VariableView {
  id: string;
  key: string;
  value: string;
  isSecret: boolean;
  enabled: boolean;
  /** The access group of a restricted variable; its value is a secret here. */
  restricted: string | null;
  /** Restricted to a group this device cannot read: empty here and not editable. */
  locked: boolean;
}

/** `value: null` on a secret keeps the stored secret unchanged. */
export interface VariableInput {
  id: string | null;
  key: string;
  value: string | null;
  isSecret: boolean;
  enabled: boolean;
  restricted: string | null;
}

export function listEnvironments(): Promise<EnvironmentList> {
  return invoke('list_environments');
}

export function createEnvironment(name: string): Promise<EnvironmentSummary> {
  return invoke('create_environment', { name });
}

export function renameEnvironment(id: string, name: string): Promise<void> {
  return invoke('rename_environment', { id, name });
}

export function deleteEnvironment(id: string): Promise<void> {
  return invoke('delete_environment', { id });
}

/** Gives an environment to a collection, which shares it, or back to this device only. */
export function setEnvironmentCollection(id: string, collectionId: string | null): Promise<void> {
  return invoke('set_environment_collection', { id, collectionId });
}

export function setActiveEnvironment(id: string | null): Promise<void> {
  return invoke('set_active_environment', { id });
}

export function getEnvironmentVariables(id: string): Promise<VariableView[]> {
  return invoke('get_environment_variables', { id });
}

export function saveEnvironmentVariables(
  id: string,
  variables: VariableInput[],
): Promise<VariableView[]> {
  return invoke('save_environment_variables', { id, variables });
}

export function revealSecret(id: string): Promise<string> {
  return invoke('reveal_secret', { id });
}

export type VariableScope = 'global' | 'collection' | 'environment';

/** `dynamic` marks built-ins such as `$guid`, made fresh on every send. */
export type HintSource = VariableScope | 'dynamic';

/** For autocomplete. `preview` is masked for secrets and describes dynamic variables. */
export interface VariableHint {
  name: string;
  scope: HintSource;
  isSecret: boolean;
  preview: string;
}

export interface UrlPreview {
  url: string;
  missing: string[];
}

export function getCollectionVariableSet(collectionId: string): Promise<string> {
  return invoke('get_collection_variable_set', { collectionId });
}

export function variableHints(collectionId: string | null): Promise<VariableHint[]> {
  return invoke('variable_hints', { collectionId });
}

export function previewUrl(
  url: string,
  params: NameValue[],
  pathVariables: NameValue[],
  collectionId: string | null,
): Promise<UrlPreview> {
  return invoke('preview_url', { url, params, pathVariables, collectionId });
}
