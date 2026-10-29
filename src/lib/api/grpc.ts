import { invoke } from '@tauri-apps/api/core';
import type { Auth, GrpcSource } from './collections';
import type { NameValue, PickedFile } from './http';

/** `{{variables}}` stay in place; the Rust core fills them in. */
export interface GrpcTarget {
  /** `grpc://host:port` (plaintext) or `grpcs://host:port` (TLS). */
  url: string;
  source: GrpcSource;
  protoFile: string;
  metadata: NameValue[];
  auth: Auth;
  collectionId: string | null;
  requestId: string | null;
}

export interface GrpcMethodInfo {
  name: string;
  /** `package.Service/Method`, as requests store it. */
  fullName: string;
  clientStreaming: boolean;
  serverStreaming: boolean;
  /** An input message with every field (JSON), as a starting point. */
  template: string;
}

export interface GrpcServiceInfo {
  name: string;
  methods: GrpcMethodInfo[];
}

export interface GrpcReply {
  /** The gRPC status code; 0 is OK. */
  code: number;
  status: string;
  message: string;
  /** Each answer as proto3 JSON. */
  messages: string[];
  headers: NameValue[];
  trailers: NameValue[];
  elapsedMs: number;
  /** Set when a long server stream was cut short. */
  warning: string | null;
}

export function describeGrpc(target: GrpcTarget): Promise<GrpcServiceInfo[]> {
  return invoke('grpc_describe', { target });
}

export function invokeGrpc(
  target: GrpcTarget,
  method: string,
  message: string,
): Promise<GrpcReply> {
  return invoke('grpc_invoke', { input: { target, method, message } });
}

export function pickProtoFile(): Promise<PickedFile | null> {
  return invoke('pick_proto_file');
}
