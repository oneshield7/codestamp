import {
  describeGrpc,
  invokeGrpc,
  type GrpcMethodInfo,
  type GrpcReply,
  type GrpcServiceInfo,
  type GrpcTarget,
} from '../api/grpc';
import { toAppError } from '../api/http';
import { knownServices, rememberServices } from './grpc-services';

/** The open gRPC card: the services it can call and its last call. */
export class GrpcCall {
  services = $state<GrpcServiceInfo[]>([]);
  loading = $state(false);
  /** Why the services could not be loaded. */
  loadError = $state<string | null>(null);
  invoking = $state(false);
  reply = $state<GrpcReply | null>(null);
  error = $state<string | null>(null);

  /** Shows services loaded earlier for the same server or file, if any. */
  recall(target: GrpcTarget): void {
    this.services = knownServices(target);
  }

  async load(target: GrpcTarget): Promise<void> {
    if (this.loading) {
      return;
    }
    this.loading = true;
    this.loadError = null;
    try {
      const services = await describeGrpc(target);
      this.services = services;
      rememberServices(target, services);
    } catch (caught) {
      this.loadError = toAppError(caught).message;
    } finally {
      this.loading = false;
    }
  }

  async invoke(target: GrpcTarget, method: string, message: string): Promise<void> {
    if (this.invoking) {
      return;
    }
    this.invoking = true;
    this.error = null;
    try {
      this.reply = await invokeGrpc(target, method, message);
    } catch (caught) {
      this.reply = null;
      this.error = toAppError(caught).message;
    } finally {
      this.invoking = false;
    }
  }

  find(fullName: string): GrpcMethodInfo | null {
    for (const service of this.services) {
      const found = service.methods.find((method) => method.fullName === fullName);
      if (found) {
        return found;
      }
    }
    return null;
  }
}
