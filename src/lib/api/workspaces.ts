import { invoke } from '@tauri-apps/api/core';

/** The title icon colors a workspace can have (DESIGN.md, `--hue-*`). */
export const WORKSPACE_HUES = [
  'blue',
  'green',
  'teal',
  'violet',
  'amber',
  'orange',
  'rose',
  'slate',
] as const;

export type WorkspaceHue = (typeof WORKSPACE_HUES)[number];

/** An image is a `data:` URL; `source` is the website a found logo came from. */
export type WorkspaceIcon =
  | { kind: 'letter' }
  | { kind: 'emoji'; emoji: string }
  | { kind: 'image'; dataUrl: string; source: string | null };

export interface Workspace {
  id: string;
  name: string;
  /** Colors the letter and emoji icons. */
  hue: WorkspaceHue;
  icon: WorkspaceIcon;
  collectionCount: number;
  /** Named environments, Globals not included. */
  environmentCount: number;
  flowCount: number;
}

export interface WorkspaceList {
  workspaces: Workspace[];
  currentId: string;
}

export function listWorkspaces(): Promise<WorkspaceList> {
  return invoke('list_workspaces');
}

/** How a workspace looks: what the New and Edit workspace dialog sets. */
export interface WorkspaceLook {
  name: string;
  hue: WorkspaceHue;
  icon: WorkspaceIcon;
}

export interface LogoCandidate {
  website: string;
  dataUrl: string;
}

export function createWorkspace(look: WorkspaceLook): Promise<Workspace> {
  return invoke('create_workspace', { ...look });
}

export function updateWorkspace(id: string, look: WorkspaceLook): Promise<Workspace> {
  return invoke('update_workspace', { id, ...look });
}

/** Logos for the websites the name suggests; sends those website names to Google. */
export function findWorkspaceLogos(name: string): Promise<LogoCandidate[]> {
  return invoke('find_workspace_logos', { name });
}

/** The logo of one website, when Google's icon service has it. */
export function findWebsiteLogo(website: string): Promise<LogoCandidate[]> {
  return invoke('find_website_logo', { website });
}

/** Swaps the cookie jar and remembers the choice; the app reloads afterwards. */
export function openWorkspace(id: string): Promise<void> {
  return invoke('open_workspace', { id });
}

/** Deleting the open workspace opens the first other one. */
export function deleteWorkspace(id: string): Promise<WorkspaceList> {
  return invoke('delete_workspace', { id });
}

export function moveCollectionToWorkspace(
  collectionId: string,
  workspaceId: string,
): Promise<void> {
  return invoke('move_collection_to_workspace', { collectionId, workspaceId });
}
