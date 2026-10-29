import { collectionIcons, type IconEntry } from '../api/collections';

let catalogue: Promise<IconEntry[]> | null = null;

/** The icon catalogue from the Rust core, loaded once; a failed load is tried again next time. */
export function iconCatalogue(): Promise<IconEntry[]> {
  catalogue ??= collectionIcons().catch((error: unknown) => {
    catalogue = null;
    throw error;
  });
  return catalogue;
}
