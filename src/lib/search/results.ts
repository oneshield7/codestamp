import type { CollectionSummary } from '../api/collections';
import type { DocHit } from '../api/docs';
import type { EnvironmentSummary } from '../api/environments';
import type { SearchEntry } from '../api/search';
import { plural } from '../format/plural';
import { urlPath } from '../format/url';
import { stampLabel } from '../request/kinds';
import { stampTone, type StampTone } from '../request/methods';
import {
  navLabel,
  SETTINGS_SECTIONS,
  type NavIconName,
  type NavPage,
  type SettingsSection,
  type View,
} from '../state/navigation';
import {
  highlight,
  matchFields,
  queryWords,
  type Range,
  type SearchField,
  type Segment,
} from './match';

export type SearchTarget =
  | { kind: 'request'; collectionId: string; requestId: string }
  | { kind: 'collection'; collectionId: string }
  /** A collection, section or request in Docs; `itemId` is the one to scroll to. */
  | { kind: 'docs'; collectionId: string; itemId: string }
  | { kind: 'environment'; environmentId: string }
  | { kind: 'page'; view: View }
  | { kind: 'settings'; section: SettingsSection };

export type SearchIcon =
  | { kind: 'stamp'; label: string; tone: StampTone }
  | { kind: 'emoji'; emoji: string }
  | { kind: 'nav'; name: NavIconName }
  | { kind: 'path'; path: string; hue: string };

export interface SearchResult {
  key: string;
  target: SearchTarget;
  icon: SearchIcon;
  title: Segment[];
  /** The URL of a request, or what a page holds. */
  detail: Segment[];
  /** Where a request lives, e.g. "Shop API › Orders". */
  place: string | null;
  /** A request URL; for paths it is shown in the code font. */
  mono: boolean;
}

export interface SearchGroup {
  label: string;
  results: SearchResult[];
}

export interface SearchSources {
  requests: readonly SearchEntry[];
  collections: readonly CollectionSummary[];
  environments: readonly EnvironmentSummary[];
  /** Found by the core in names and descriptions as the query changes. */
  docs: readonly DocHit[];
}

/** A place to go; its name and icon come from the navigation it shares with the sidebar. */
interface Page {
  view: NavPage;
  detail: string;
  keywords: string;
}

const PAGES: readonly Page[] = [
  { view: 'home', detail: 'Start here', keywords: 'welcome start' },
  {
    view: 'flows',
    detail: 'Run requests in order, start to end',
    // The old names still find the renamed pages.
    keywords: 'postflow flow sequence chain journey scenario end to end',
  },
  {
    view: 'catalog',
    detail: 'Health, owner and tags of every API',
    keywords: 'api catalog apis services health score quality rules owner tags governance',
  },
  {
    view: 'docs',
    detail: 'Descriptions of each collection and request',
    keywords: 'documentation describe markdown readme reference',
  },
  {
    view: 'changes',
    detail: 'Commits and who changed each request',
    keywords: 'git commits log blame gitlens report diff push pull collaboration',
  },
  {
    view: 'environments',
    detail: 'Globals and environment variables',
    keywords: 'variables globals secrets',
  },
  { view: 'cookies', detail: 'Cookies kept from responses', keywords: 'cookie jar' },
  {
    view: 'import',
    detail: 'Collections, environments and cURL',
    keywords: 'curl file join shared collaboration postman',
  },
  {
    view: 'trash',
    detail: 'Restore deleted collections and requests',
    keywords: 'trash bin undo restore deleted recover',
  },
  {
    view: 'settings',
    detail: 'Appearance, requests, proxy and data',
    keywords: 'preferences options',
  },
];

const GROUP_LIMITS = { requests: 40, docs: 8, collections: 6, environments: 6, goTo: 8 } as const;

const DOC_DETAILS = new Map<DocHit['item'], string>([
  ['collection', 'Collection docs'],
  ['section', 'Section docs'],
  ['request', 'Request docs'],
]);

interface Scored {
  result: SearchResult;
  score: number;
  /** Shorter matches win ties, then the order the items came in. */
  length: number;
}

/**
 * The quick search list for a query, best group first (Docs first while Docs is open); with no
 * query, places to go.
 */
export function searchResults(
  sources: SearchSources,
  query: string,
  docsFirst: boolean,
): SearchGroup[] {
  const words = queryWords(query);
  if (words.length === 0) {
    return [
      { label: 'Go to', results: PAGES.map((page) => pageResult(page, null)) },
      {
        label: 'Collections',
        results: sources.collections
          .slice(0, GROUP_LIMITS.collections)
          .map((collection) => collectionResult(collection, null)),
      },
    ].filter((group) => group.results.length > 0);
  }
  const groups = [
    {
      label: 'Requests',
      items: requestMatches(sources.requests, words),
      limit: GROUP_LIMITS.requests,
    },
    {
      label: 'Collections',
      items: scoreAll(sources.collections, (collection) => {
        const match = matchFields([{ text: collection.name, weight: 1 }], words);
        return (
          match && {
            score: match.score,
            length: collection.name.length,
            result: collectionResult(collection, match.ranges[0] ?? []),
          }
        );
      }),
      limit: GROUP_LIMITS.collections,
    },
    {
      label: 'Environments',
      items: scoreAll(sources.environments, (environment) => {
        const name = environmentName(environment);
        const match = matchFields([{ text: name, weight: 1 }], words);
        return (
          match && {
            score: match.score,
            length: name.length,
            result: environmentResult(environment, name, match.ranges[0] ?? []),
          }
        );
      }),
      limit: GROUP_LIMITS.environments,
    },
    { label: 'Docs', items: docMatches(sources.docs, words), limit: GROUP_LIMITS.docs },
    { label: 'Go to', items: placeMatches(words), limit: GROUP_LIMITS.goTo },
  ];
  const rank = (group: (typeof groups)[number]): number => {
    const best = group.items[0]?.score ?? 0;
    if (group.label !== 'Docs') {
      return best;
    }
    // Docs repeat names the other groups list, so they lead only while Docs is open.
    return docsFirst ? Infinity : best / 2;
  };
  return groups
    .filter((group) => group.items.length > 0)
    .sort((a, b) => rank(b) - rank(a))
    .map((group) => ({
      label: group.label,
      results: group.items.slice(0, group.limit).map((item) => item.result),
    }));
}

function scoreAll<T>(items: readonly T[], score: (item: T) => Scored | null): Scored[] {
  const scored: { item: Scored; order: number }[] = [];
  for (const [order, item] of items.entries()) {
    const result = score(item);
    if (result) {
      scored.push({ item: result, order });
    }
  }
  return scored
    .sort(
      (a, b) => b.item.score - a.item.score || a.item.length - b.item.length || a.order - b.order,
    )
    .map((entry) => entry.item);
}

function requestMatches(entries: readonly SearchEntry[], words: readonly string[]): Scored[] {
  return scoreAll(entries, (entry) => {
    const place = requestPlace(entry);
    const fields: SearchField[] = [
      { text: entry.name, weight: 1 },
      { text: entry.url, weight: 1 },
      { text: stampLabel(entry.kind, entry.method), weight: 1, exact: true },
      { text: place, weight: 0.5 },
    ];
    const match = matchFields(fields, words);
    if (!match) {
      return null;
    }
    return {
      score: match.score,
      length: entry.url.length,
      result: {
        key: `request:${entry.requestId}`,
        target: { kind: 'request', collectionId: entry.collectionId, requestId: entry.requestId },
        icon: {
          kind: 'stamp',
          label: stampLabel(entry.kind, entry.method),
          tone: stampTone(entry.kind, entry.method),
        },
        title: highlight(entry.name, match.ranges[0]),
        detail: highlight(entry.url, match.ranges[1]),
        place: `${entry.collectionIcon} ${place}`,
        mono: true,
      },
    };
  });
}

function requestPlace(entry: { collectionName: string; sectionName: string | null }): string {
  return entry.sectionName === null
    ? entry.collectionName
    : `${entry.collectionName} › ${entry.sectionName}`;
}

/** Kept in the core's order: name matches, then description matches. */
function docMatches(hits: readonly DocHit[], words: readonly string[]): Scored[] {
  return hits.map((hit) => {
    const match = matchFields(
      [
        { text: hit.name, weight: 1 },
        { text: hit.snippet ?? '', weight: 0.4 },
      ],
      words,
    );
    return {
      // A word found only beyond the snippet still counts; the core matched the whole text.
      score: match?.score ?? 1,
      length: hit.name.length,
      result: docResult(hit, match?.ranges ?? []),
    };
  });
}

function docResult(hit: DocHit, ranges: readonly (readonly Range[])[]): SearchResult {
  const place =
    hit.item === 'request' ? requestPlace(hit) : hit.item === 'section' ? hit.collectionName : null;
  return {
    key: `docs:${hit.id}`,
    target: { kind: 'docs', collectionId: hit.collectionId, itemId: hit.id },
    icon: docIcon(hit),
    title: highlight(hit.name, ranges[0]),
    detail:
      hit.snippet === null
        ? highlight(DOC_DETAILS.get(hit.item) ?? '')
        : highlight(hit.snippet, ranges[1]),
    place: place === null ? null : `${hit.collectionIcon} ${place}`,
    mono: false,
  };
}

function docIcon(hit: DocHit): SearchIcon {
  if (hit.item === 'collection') {
    return { kind: 'emoji', emoji: hit.collectionIcon };
  }
  if (hit.kind !== null && hit.method !== null) {
    return {
      kind: 'stamp',
      label: stampLabel(hit.kind, hit.method),
      tone: stampTone(hit.kind, hit.method),
    };
  }
  return { kind: 'nav', name: 'docs' };
}

function placeMatches(words: readonly string[]): Scored[] {
  const pages = scoreAll(PAGES, (page) => {
    const label = navLabel(page.view);
    const match = matchFields(
      [
        { text: label, weight: 1 },
        { text: page.keywords, weight: 0.6 },
      ],
      words,
    );
    return (
      match && {
        score: match.score,
        length: label.length,
        result: pageResult(page, match.ranges[0] ?? []),
      }
    );
  });
  const sections = scoreAll(SETTINGS_SECTIONS, (section) => {
    const title = `Settings › ${section.label}`;
    const match = matchFields(
      [
        { text: title, weight: 1 },
        { text: section.keywords, weight: 0.6 },
      ],
      words,
    );
    if (!match) {
      return null;
    }
    return {
      score: match.score,
      length: title.length,
      result: {
        key: `settings:${section.key}`,
        target: { kind: 'settings', section: section.key },
        icon: { kind: 'path', path: section.icon, hue: section.hue },
        title: highlight(title, match.ranges[0]),
        detail: highlight(section.keywords, match.ranges[1]),
        place: null,
        mono: false,
      },
    };
  });
  return [...pages, ...sections].sort((a, b) => b.score - a.score || a.length - b.length);
}

function pageResult(page: Page, ranges: readonly Range[] | null): SearchResult {
  return {
    key: `page:${page.view}`,
    target: { kind: 'page', view: page.view },
    icon: { kind: 'nav', name: page.view },
    title: highlight(navLabel(page.view), ranges ?? []),
    detail: highlight(page.detail),
    place: null,
    mono: false,
  };
}

function collectionResult(
  collection: CollectionSummary,
  ranges: readonly Range[] | null,
): SearchResult {
  return {
    key: `collection:${collection.id}`,
    target: { kind: 'collection', collectionId: collection.id },
    icon: { kind: 'emoji', emoji: collection.icon },
    title: highlight(collection.name, ranges ?? []),
    detail: highlight(plural(collection.requestCount, 'request')),
    place: null,
    mono: false,
  };
}

function environmentName(environment: EnvironmentSummary): string {
  return environment.isGlobal ? 'Globals' : environment.name;
}

function environmentResult(
  environment: EnvironmentSummary,
  name: string,
  ranges: readonly Range[],
): SearchResult {
  return {
    key: `environment:${environment.id}`,
    target: { kind: 'environment', environmentId: environment.id },
    icon: { kind: 'nav', name: 'environments' },
    title: highlight(name, ranges),
    detail: highlight(plural(environment.variableCount, 'variable')),
    place: null,
    mono: false,
  };
}

/** The rest of a request path that begins with a typed path, for the inline suggestion. */
export function pathCompletion(
  typed: string,
  groups: readonly SearchGroup[],
  sources: SearchSources,
): string | null {
  if (!typed.startsWith('/') || /\s/.test(typed)) {
    return null;
  }
  const lower = typed.toLowerCase();
  const urls = new Map(sources.requests.map((entry) => [entry.requestId, entry.url]));
  for (const result of groups.find((group) => group.label === 'Requests')?.results ?? []) {
    if (result.target.kind !== 'request') {
      continue;
    }
    const path = urlPath(urls.get(result.target.requestId) ?? '');
    if (path.length > typed.length && path.toLowerCase().startsWith(lower)) {
      return path.slice(typed.length);
    }
  }
  return null;
}
