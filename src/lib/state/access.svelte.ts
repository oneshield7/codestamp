import { SvelteMap } from 'svelte/reactivity';
import * as api from '../api/team';

/**
 * Access groups of shared collections as this device sees them. Loaded only for collections
 * that are shared, since reading them makes this device's key the first time.
 */
class AccessGroups {
  readonly overviews = new SvelteMap<string, api.AccessOverview>();

  async load(collectionId: string): Promise<void> {
    this.overviews.set(collectionId, await api.accessOverview(collectionId));
  }

  groups(collectionId: string | null): api.AccessGroup[] {
    return collectionId === null ? [] : (this.overviews.get(collectionId)?.groups ?? []);
  }

  /** The group whose value for `name` this device cannot read, for the unresolved warning. */
  lockedGroup(collectionId: string | null, name: string): string | null {
    const group = this.groups(collectionId).find(
      (candidate) => !candidate.access && candidate.variables.some((use) => use.key === name),
    );
    return group?.name ?? null;
  }

  forget(collectionId: string): void {
    this.overviews.delete(collectionId);
  }
}

/**
 * What to tell this device after a sync let it into groups it had asked for, or null. A group
 * repaired here, or one made here, is not news.
 */
export function grantNotice(before: api.AccessGroup[], after: api.AccessGroup[]): string | null {
  const granted = after.filter((group) => {
    const earlier = before.find((candidate) => candidate.id === group.id);
    return (
      group.access &&
      earlier !== undefined &&
      !earlier.access &&
      earlier.problem === null &&
      group.problem === null
    );
  });
  if (granted.length === 0) {
    return null;
  }
  const names = granted.map((group) => group.name).join(' and ');
  const granters = granted.map(
    (group) => group.members.find((member) => member.thisDevice)?.addedBy ?? null,
  );
  const granter = granters[0] ?? null;
  const sameGranter = granter !== null && granters.every((name) => name === granter);
  const values = granted.length === 1 ? 'Its restricted values are' : 'Their restricted values are';
  return sameGranter
    ? `${granter} gave this device access to ${names}. ${values} ready to use.`
    : `This device now has access to ${names}. ${values} ready to use.`;
}

export const accessGroups = new AccessGroups();
