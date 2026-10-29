import type { CollectionDocs } from '../api/docs';
import { stampLabel } from '../request/kinds';
import { queryWords } from '../search/match';

/** The requests and sections a docs search keeps. */
export interface DocsFilter {
  requestIds: ReadonlySet<string>;
  sectionIds: ReadonlySet<string>;
}

function holdsAll(texts: readonly string[], words: readonly string[]): boolean {
  const text = texts.join('\n').toLowerCase();
  return words.every((word) => text.includes(word));
}

/** Names, addresses and descriptions holding every word; a section that matches keeps all of its requests. */
export function filterDocs(docs: CollectionDocs, query: string): DocsFilter | null {
  const words = queryWords(query);
  if (words.length === 0) {
    return null;
  }
  const sectionIds = new Set(
    docs.sections
      .filter((section) => holdsAll([section.name, section.description], words))
      .map((section) => section.id),
  );
  const requestIds = new Set<string>();
  for (const request of docs.requests) {
    const inMatchedSection = request.sectionId !== null && sectionIds.has(request.sectionId);
    const fields = [
      request.name,
      request.url,
      stampLabel(request.kind, request.method),
      request.description,
    ];
    if (inMatchedSection || holdsAll(fields, words)) {
      requestIds.add(request.id);
      if (request.sectionId !== null) {
        sectionIds.add(request.sectionId);
      }
    }
  }
  return { requestIds, sectionIds };
}
