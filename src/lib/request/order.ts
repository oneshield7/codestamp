import type { CollectionDetail, RequestSummary, SectionInfo } from '../api/collections';

export interface NotebookGroup {
  section: SectionInfo | null;
  requests: RequestSummary[];
}

/** Requests without a section come first, then each section in order: the notebook order. */
export function collectionGroups(collection: CollectionDetail): NotebookGroup[] {
  return [
    {
      section: null,
      requests: collection.requests.filter((request) => request.sectionId === null),
    },
    ...collection.sections.map((section) => ({
      section,
      requests: collection.requests.filter((request) => request.sectionId === section.id),
    })),
  ];
}

export function orderedRequests(collection: CollectionDetail): RequestSummary[] {
  return collectionGroups(collection).flatMap((group) => group.requests);
}
