import { invoke } from '@tauri-apps/api/core';
import type { DeletedItem } from './trash';
import type { HttpMethod } from './http';

export type BodyMode =
  'none' | 'json' | 'xml' | 'html' | 'text' | 'urlencoded' | 'formdata' | 'binary';
export type RequestKind = 'http' | 'websocket' | 'mcp' | 'graphql' | 'grpc' | 'soap';
/** MCP requests keep their transport where HTTP requests keep the method. */
export type McpTransportName = 'HTTP' | 'STDIO';
export type RequestMethod = HttpMethod | McpTransportName;
export type McpOperationKind = 'tool' | 'prompt' | 'resource';

/** What an MCP request calls: a tool or prompt by name, or a resource by URI. */
export interface McpOperation {
  kind: McpOperationKind;
  name: string;
}
export type FormFieldKind = 'text' | 'file';
export type ApiKeyLocation = 'header' | 'query';

export type OAuthGrant = 'authorizationCode' | 'clientCredentials' | 'password';
/** Where the client id and secret go when asking for a token. */
export type ClientAuth = 'header' | 'body';

/** OAuth 2.0 settings; the token itself stays encrypted in the Rust core. */
export interface OAuth2Settings {
  grant: OAuthGrant;
  authUrl: string;
  tokenUrl: string;
  clientId: string;
  clientSecret: string;
  scope: string;
  redirectUri: string;
  usePkce: boolean;
  username: string;
  password: string;
  clientAuth: ClientAuth;
  /** Empty means `Bearer`. */
  headerPrefix: string;
}

/** Values may contain `{{variables}}`; the Rust core fills them in and adds the header. */
export type Auth =
  | { type: 'inherit' }
  | { type: 'none' }
  | { type: 'bearer'; token: string; prefix: string }
  | { type: 'basic'; username: string; password: string }
  | { type: 'apiKey'; key: string; value: string; location: ApiKeyLocation }
  | { type: 'digest'; username: string; password: string }
  | {
      type: 'awsSigV4';
      accessKey: string;
      secretKey: string;
      sessionToken: string;
      region: string;
      service: string;
    }
  | ({ type: 'oauth2' } & OAuth2Settings);

export type AuthType = Auth['type'];

export interface CollectionSummary {
  id: string;
  name: string;
  requestCount: number;
  /** The chosen emoji, or the one picked from the name. */
  icon: string;
}

export interface SectionInfo {
  id: string;
  name: string;
}

export interface RequestSummary {
  id: string;
  sectionId: string | null;
  kind: RequestKind;
  name: string;
  method: RequestMethod;
  url: string;
}

export interface CollectionDetail {
  id: string;
  name: string;
  auth: Auth;
  icon: string;
  /** False while the icon follows the name. */
  iconCustom: boolean;
  sections: SectionInfo[];
  requests: RequestSummary[];
}

export interface StoredKeyValue {
  enabled: boolean;
  key: string;
  value: string;
}

/** The value of a `:name` path segment; `{{variables}}` in it are filled in when sending. */
/** Where a gRPC request learns the services: by asking the server, or from a `.proto` file. */
export type GrpcSource = 'reflection' | 'proto';

export interface GrpcSettings {
  /** `package.Service/Method`; empty until one is chosen. */
  method: string;
  source: GrpcSource;
  /** The `.proto` file picked on this device; it is not shared with collaborators. */
  protoFile: string;
}

export type SoapVersion = '1.1' | '1.2';

/** What a SOAP request calls; the endpoint is `url` and the envelope `body` (XML). */
export interface SoapSettings {
  /** Where the WSDL is read from: a URL, or a file on this device (not shared). */
  wsdl: string;
  version: SoapVersion;
  /** The WSDL binding the operation belongs to (SoapUI's "interface"). */
  binding: string;
  operation: string;
  /** Sent as the SOAPAction header (1.1) or the Content-Type action (1.2). */
  action: string;
}

export interface PathVariable {
  key: string;
  value: string;
}

/** A form row; for a file row `value` is the path picked in the system dialog. */
export interface StoredFormField extends StoredKeyValue {
  kind: FormFieldKind;
}

/**
 * `body` holds raw text (JSON or plain), `form` the rows of URL-encoded and form-data bodies,
 * and `bodyFile` the file of a binary body. Each keeps its content when the mode changes.
 */
export interface RequestContent {
  kind: RequestKind;
  name: string;
  method: RequestMethod;
  url: string;
  params: StoredKeyValue[];
  headers: StoredKeyValue[];
  bodyMode: BodyMode;
  body: string;
  form: StoredFormField[];
  bodyFile: string;
  preRequestScript: string;
  testScript: string;
  auth: Auth;
  /** MCP requests only; the arguments are `body` (JSON). */
  mcpOperation: McpOperation | null;
  pathVariables: PathVariable[];
  /** GraphQL requests only: the variables (JSON); the query is `body`. */
  graphqlVariables: string;
  /** gRPC requests only; the address is `url`, the metadata `headers`, the message `body`. */
  grpc: GrpcSettings | null;
  /** SOAP requests only. */
  soap: SoapSettings | null;
}

export interface SavedRequest extends RequestContent {
  id: string;
  collectionId: string;
  sectionId: string | null;
}

export function listCollections(): Promise<CollectionSummary[]> {
  return invoke('list_collections');
}

export interface NewCollection {
  name: string;
  /** Null follows the name. */
  icon: string | null;
  /** Saved as the collection variable `baseUrl`; empty for none. */
  baseUrl: string;
}

export function createCollection(collection: NewCollection): Promise<CollectionSummary> {
  return invoke('create_collection', {
    name: collection.name,
    icon: collection.icon,
    baseUrl: collection.baseUrl,
  });
}

/** The summary comes back because an automatic icon follows the new name. */
export function renameCollection(id: string, name: string): Promise<CollectionSummary> {
  return invoke('rename_collection', { id, name });
}

export interface IconEntry {
  emoji: string;
  label: string;
  /** Space-separated words the picker's search matches. */
  keywords: string;
}

export interface IconSuggestion {
  /** The icon the name gets automatically. */
  emoji: string;
  related: string[];
}

/** Every icon a collection can have; the Rust core accepts only these. */
export function collectionIcons(): Promise<IconEntry[]> {
  return invoke('collection_icons');
}

export function suggestCollectionIcon(name: string): Promise<IconSuggestion> {
  return invoke('suggest_collection_icon', { name });
}

/** Null goes back to the icon picked from the name. */
export function setCollectionIcon(id: string, icon: string | null): Promise<CollectionSummary> {
  return invoke('set_collection_icon', { id, icon });
}

/** The copy Recently deleted keeps. */
export function deleteCollection(id: string): Promise<DeletedItem> {
  return invoke('delete_collection', { id });
}

export function getCollection(id: string): Promise<CollectionDetail> {
  return invoke('get_collection', { id });
}

export function setCollectionAuth(id: string, auth: Auth): Promise<void> {
  return invoke('set_collection_auth', { id, auth });
}

export function createSection(collectionId: string, name: string): Promise<SectionInfo> {
  return invoke('create_section', { collectionId, name });
}

export function renameSection(id: string, name: string): Promise<void> {
  return invoke('rename_section', { id, name });
}

export function deleteSection(id: string): Promise<void> {
  return invoke('delete_section', { id });
}

export function createRequest(
  collectionId: string,
  sectionId: string | null,
  content: RequestContent,
): Promise<SavedRequest> {
  return invoke('create_request', { collectionId, sectionId, content });
}

export function getRequest(id: string): Promise<SavedRequest> {
  return invoke('get_request', { id });
}

export function updateRequest(id: string, content: RequestContent): Promise<RequestSummary> {
  return invoke('update_request', { id, content });
}

export interface RequestGroup {
  sectionId: string | null;
  requestIds: string[];
}

export function reorderCollection(
  collectionId: string,
  sections: string[],
  groups: RequestGroup[],
): Promise<CollectionDetail> {
  return invoke('reorder_collection', { collectionId, sections, groups });
}

/** The copy Recently deleted keeps. */
export function deleteRequest(id: string): Promise<DeletedItem> {
  return invoke('delete_request', { id });
}

/** The copy, named `name`, is placed right after the original, in the same section. */
export function duplicateRequest(id: string, name: string): Promise<SavedRequest> {
  return invoke('duplicate_request', { id, name });
}

export function duplicateSection(id: string, name: string): Promise<CollectionDetail> {
  return invoke('duplicate_section', { id, name });
}

/** Copies auth, sections, requests and collection variables (secrets included). */
export function duplicateCollection(id: string, name: string): Promise<CollectionSummary> {
  return invoke('duplicate_collection', { id, name });
}
