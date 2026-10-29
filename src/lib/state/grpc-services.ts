import type { GrpcServiceInfo, GrpcTarget } from '../api/grpc';
import { RecentCache } from './recent';

/** Services loaded this session, so reopening a card shows them without asking again. */
const services = new RecentCache<GrpcServiceInfo[]>(20);

function key(target: GrpcTarget): string {
  const where = target.source === 'proto' ? target.protoFile : target.url.trim();
  return `${target.collectionId ?? ''} ${target.source} ${where}`;
}

export function knownServices(target: GrpcTarget): GrpcServiceInfo[] {
  return services.get(key(target)) ?? [];
}

export function rememberServices(target: GrpcTarget, found: GrpcServiceInfo[]): void {
  services.set(key(target), found);
}
