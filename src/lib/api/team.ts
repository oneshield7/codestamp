import { invoke } from '@tauri-apps/api/core';
import type { CollectionSummary, RequestKind, RequestMethod, SavedRequest } from './collections';

/** Written on commits. Access to the repository comes from the Git host, not from this. */
export interface Author {
  name: string;
  email: string;
}

/**
 * Who works on one shared collection. `username` picks the account Git signs in with on https
 * hosts; null leaves the choice to Git.
 */
export interface Account {
  author: Author;
  username: string | null;
}

export interface TeamSetup {
  /** Null when Git is not installed. */
  gitVersion: string | null;
  author: Author | null;
}

export interface ConflictField {
  label: string;
  mine: string;
  theirs: string;
}

/** One file both sides changed in the same places. */
export interface ConflictView {
  file: string;
  requestId: string | null;
  title: string;
  fields: ConflictField[];
  /** Requests can keep both versions; yours becomes a copy. */
  allowBoth: boolean;
}

export interface TeamStatus {
  collectionId: string;
  remote: string;
  branch: string;
  syncedAt: number | null;
  /** Written on this collection's commits; null until a profile exists. */
  author: Author | null;
  username: string | null;
  conflicts: ConflictView[];
}

export type SyncState = 'synced' | 'conflict' | 'offline' | 'signIn' | 'readOnly' | 'error';
export type Change = 'added' | 'updated' | 'removed';
export type Choice = 'mine' | 'theirs' | 'both';

export interface ChangedRequest {
  id: string;
  name: string;
  change: Change;
}

export interface SyncReport {
  status: SyncState;
  message: string | null;
  /** Local commits not on the remote yet. */
  ahead: number;
  changed: ChangedRequest[];
  collectionChanged: boolean;
  /** An environment that belongs to the collection was added, changed or removed. */
  environmentsChanged: boolean;
  /** An access group or a request for access changed. */
  accessChanged: boolean;
  authors: string[];
  conflicts: ConflictView[];
  syncedAt: number | null;
}

export interface Decision {
  file: string;
  choice: Choice;
}

export interface Version {
  commit: string;
  author: string;
  /** Unix seconds. */
  time: number;
  message: string;
}

export interface FieldChange {
  label: string;
  then: string;
  now: string;
}

export function teamSetup(): Promise<TeamSetup> {
  return invoke('team_setup');
}

/** Changes who this device works as on one shared collection; the next sync uses it. */
export function setTeamAccount(collectionId: string, account: Account): Promise<TeamStatus> {
  return invoke('set_team_account', { collectionId, account });
}

export function listTeamLinks(): Promise<TeamStatus[]> {
  return invoke('list_team_links');
}

/** What a repository holds before a collection is shared into it. */
export interface SharePreview {
  /** No commits yet: sharing simply fills it. */
  empty: boolean;
  /** It holds a collection shared from Codestamp, which people may have joined. */
  codestamp: boolean;
  branch: string | null;
  /** The commit replacing builds on; sharing refuses if the branch moved since. */
  head: string | null;
  /** Files that replacing removes, up to 200. */
  files: string[];
  /** Files past that limit. */
  more: number;
}

/** Looks into the repository first; may open the credential manager's sign-in window. */
export function previewShare(remote: string, username: string | null): Promise<SharePreview> {
  return invoke('preview_share', { remote, username });
}

/**
 * Puts the collection into the repository; Git signs in through its credential manager. A
 * repository with content is replaced only with the `replaceHead` of a reviewed preview.
 */
export function shareCollection(
  collectionId: string,
  remote: string,
  account: Account,
  replaceHead: string | null,
): Promise<TeamStatus> {
  return invoke('share_collection', { collectionId, remote, account, replaceHead });
}

export function joinTeamCollection(remote: string, account: Account): Promise<CollectionSummary> {
  return invoke('join_team_collection', { remote, account });
}

export function syncTeamCollection(
  collectionId: string,
  decisions: Decision[],
): Promise<SyncReport> {
  return invoke('sync_team_collection', { collectionId, decisions });
}

/** Stops syncing on this device; the collection and the repository stay as they are. */
export function unlinkTeamCollection(collectionId: string): Promise<void> {
  return invoke('unlink_team_collection', { collectionId });
}

export function requestVersions(collectionId: string, requestId: string): Promise<Version[]> {
  return invoke('request_versions', { collectionId, requestId });
}

export function requestVersionChanges(
  collectionId: string,
  requestId: string,
  commit: string,
): Promise<FieldChange[]> {
  return invoke('request_version_changes', { collectionId, requestId, commit });
}

export function restoreRequestVersion(
  collectionId: string,
  requestId: string,
  commit: string,
): Promise<SavedRequest> {
  return invoke('restore_request_version', { collectionId, requestId, commit });
}

/** One row of Settings › Git accounts; `username` null is Git's default account for the host. */
export interface GitAccount {
  /** Set for accounts saved in the list; they can be removed once no collection uses them. */
  id: string | null;
  host: string;
  username: string | null;
  label: string;
  collections: { id: string; name: string }[];
  /** What Git's credential manager reports for GitHub accounts; null when it cannot tell. */
  signedIn: boolean | null;
}

export interface NewGitAccount {
  host: string;
  username: string;
  label: string;
}

export type CheckStatus = 'signedIn' | 'signInNeeded' | 'offline' | 'failed';

export interface AccountCheck {
  status: CheckStatus;
  message: string | null;
}

export function listGitAccounts(): Promise<GitAccount[]> {
  return invoke('list_git_accounts');
}

export function addGitAccount(account: NewGitAccount): Promise<GitAccount[]> {
  return invoke('add_git_account', { account });
}

export function removeGitAccount(id: string): Promise<GitAccount[]> {
  return invoke('remove_git_account', { id });
}

/** May open the credential manager's sign-in window when the account has no stored sign-in. */
export function checkGitAccount(host: string, username: string | null): Promise<AccountCheck> {
  return invoke('check_git_account', { host, username });
}

export function signInGitAccount(host: string, username: string | null): Promise<AccountCheck> {
  return invoke('sign_in_git_account', { host, username });
}

/** Signs in with the account a shared collection uses; Git's sign-in window opens if needed. */
export function signInTeamCollection(collectionId: string): Promise<AccountCheck> {
  return invoke('sign_in_team_collection', { collectionId });
}

/** Removes the stored sign-in from Git's credential manager; nothing else changes. */
export function signOutGitAccount(host: string, username: string): Promise<void> {
  return invoke('sign_out_git_account', { host, username });
}

/** A shared collection's repository in Tools › Git changes, read from its clone offline. */
export interface RepoSummary {
  collectionId: string;
  name: string;
  remote: string;
  branch: string;
  syncedAt: number | null;
  /** Saved changes the next sync commits. */
  localEdits: number;
  /** Commits the remote does not have yet. */
  toPush: number;
}

export interface CommitSummary {
  commit: string;
  author: string;
  email: string;
  /** Unix seconds. */
  time: number;
  message: string;
  merge: boolean;
  pushed: boolean;
}

export interface CommitPage {
  commits: CommitSummary[];
  more: boolean;
}

export type FileKind = 'request' | 'collection' | 'environment' | 'accessGroup' | 'accessRequest';

/** A request, the collection itself, or one of its environments, as one commit changed it. */
export interface FileChange {
  kind: FileKind;
  requestId: string | null;
  name: string;
  change: Change;
  /** For an update: each changed field, `then` before and `now` after. */
  fields: FieldChange[];
}

export interface CommitDetail {
  commit: string;
  parents: string[];
  author: string;
  email: string;
  time: number;
  message: string;
  files: FileChange[];
  /** Changed files past the shown limit. */
  hidden: number;
}

export interface RequestActivity {
  id: string;
  name: string;
  method: RequestMethod;
  kind: RequestKind;
  versions: number;
  lastAuthor: string | null;
  lastTime: number | null;
}

export function gitChangesOverview(): Promise<RepoSummary[]> {
  return invoke('git_changes_overview');
}

export function gitCommits(collectionId: string, skip: number): Promise<CommitPage> {
  return invoke('git_commits', { collectionId, skip });
}

export function gitCommitDetail(collectionId: string, commit: string): Promise<CommitDetail> {
  return invoke('git_commit_detail', { collectionId, commit });
}

export function gitRequestActivity(collectionId: string): Promise<RequestActivity[]> {
  return invoke('git_request_activity', { collectionId });
}

/** A member of an access group, or someone asking to join one. */
export interface AccessPerson {
  name: string;
  email: string;
  /** Compared out loud before granting, so access goes to the device it was meant for. */
  fingerprint: string;
  /** Who added a member; null for requests. */
  addedBy: string | null;
  thisDevice: boolean;
}

/** A restricted variable; `environment` is null for the collection's own variables. */
export interface RestrictedUse {
  key: string;
  environment: string | null;
}

export interface AccessGroup {
  id: string;
  name: string;
  keyVersion: number;
  /** This device can read and change the group's values. */
  access: boolean;
  /** Why this device does not trust the group's current version. */
  problem: string | null;
  canRepair: boolean;
  members: AccessPerson[];
  requests: AccessPerson[];
  variables: RestrictedUse[];
}

export interface AccessOverview {
  /** This device's own fingerprint; null until the collection has a group. */
  fingerprint: string | null;
  groups: AccessGroup[];
}

export function accessOverview(collectionId: string): Promise<AccessOverview> {
  return invoke('access_overview', { collectionId });
}

export function createAccessGroup(collectionId: string, name: string): Promise<void> {
  return invoke('create_access_group', { collectionId, name });
}

export function requestGroupAccess(collectionId: string, groupId: string): Promise<void> {
  return invoke('request_group_access', { collectionId, groupId });
}

export function grantGroupAccess(
  collectionId: string,
  groupId: string,
  fingerprint: string,
): Promise<void> {
  return invoke('grant_group_access', { collectionId, groupId, fingerprint });
}

export function declineGroupAccess(
  collectionId: string,
  groupId: string,
  fingerprint: string,
): Promise<void> {
  return invoke('decline_group_access', { collectionId, groupId, fingerprint });
}

/** Returns the group's variables, whose values should now be changed where they are issued. */
export function removeGroupMember(
  collectionId: string,
  groupId: string,
  fingerprint: string,
): Promise<RestrictedUse[]> {
  return invoke('remove_group_member', { collectionId, groupId, fingerprint });
}

export function repairAccessGroup(collectionId: string, groupId: string): Promise<void> {
  return invoke('repair_access_group', { collectionId, groupId });
}
