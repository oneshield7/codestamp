/** A highlighted stretch of text, from `start` up to (not including) `end`. */
export type Range = readonly [start: number, end: number];

export interface Segment {
  text: string;
  hit: boolean;
}

export interface FieldMatch {
  score: number;
  ranges: Range[];
}

/** A text an item can be found by; `exact` fields (such as the method) only match whole. */
export interface SearchField {
  text: string;
  weight: number;
  exact?: boolean;
}

export interface ItemMatch {
  score: number;
  /** Highlights per field, in the order the fields were given. */
  ranges: Range[][];
}

const EXACT = 120;
const PREFIX = 100;
const WORD_START = 70;
const INSIDE = 40;
const FUZZY = 12;
/** Shorter words would match almost anything letter by letter. */
const MIN_FUZZY_LENGTH = 3;
/** Letters spread wider than this many times the word length are not a match. */
const MAX_FUZZY_SPREAD = 3;
const WORD_CHAR = /[\p{L}\p{N}]/u;

export function queryWords(query: string): string[] {
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter((word) => word !== '');
}

/**
 * Scores one lowercase query word against a text. The start of the text ranks highest, then
 * the start of a word or path segment, then anywhere inside, then the letters in order.
 */
export function matchWord(text: string, word: string): FieldMatch | null {
  const lower = text.toLowerCase();
  const first = lower.indexOf(word);
  if (first === 0) {
    return { score: PREFIX, ranges: [[0, word.length]] };
  }
  if (first > 0) {
    for (let at = first; at !== -1; at = lower.indexOf(word, at + 1)) {
      if (!WORD_CHAR.test(lower.charAt(at - 1))) {
        return { score: WORD_START, ranges: [[at, at + word.length]] };
      }
    }
    return { score: INSIDE, ranges: [[first, first + word.length]] };
  }
  return matchLetters(lower, word);
}

function matchLetters(lower: string, word: string): FieldMatch | null {
  if (word.length < MIN_FUZZY_LENGTH) {
    return null;
  }
  const ranges: [number, number][] = [];
  let from = 0;
  for (const char of word) {
    const at = lower.indexOf(char, from);
    if (at === -1) {
      return null;
    }
    const last = ranges.at(-1);
    if (last?.[1] === at) {
      last[1] = at + char.length;
    } else {
      ranges.push([at, at + char.length]);
    }
    from = at + char.length;
  }
  const spread = (ranges.at(-1)?.[1] ?? 0) - (ranges[0]?.[0] ?? 0);
  if (spread > word.length * MAX_FUZZY_SPREAD) {
    return null;
  }
  // Fewer separate pieces read as a closer match.
  return { score: FUZZY - ranges.length, ranges };
}

function matchField(field: SearchField, word: string): FieldMatch | null {
  if (!field.exact) {
    return matchWord(field.text, word);
  }
  return field.text.toLowerCase() === word
    ? { score: EXACT, ranges: [[0, field.text.length]] }
    : null;
}

/** Every word must match one of the fields; each counts where it scores best. */
export function matchFields(
  fields: readonly SearchField[],
  words: readonly string[],
): ItemMatch | null {
  const ranges = fields.map((): Range[] => []);
  let score = 0;
  for (const word of words) {
    let best: { index: number; match: FieldMatch; weighted: number } | null = null;
    for (const [index, field] of fields.entries()) {
      const match = matchField(field, word);
      const weighted = match ? match.score * field.weight : 0;
      if (match && (best === null || weighted > best.weighted)) {
        best = { index, match, weighted };
      }
    }
    if (best === null) {
      return null;
    }
    score += best.weighted;
    ranges[best.index]?.push(...best.match.ranges);
  }
  return { score, ranges };
}

/** Splits a text into plain and highlighted parts; overlapping ranges are merged. */
export function highlight(text: string, ranges: readonly Range[] = []): Segment[] {
  const sorted = [...ranges].sort((a, b) => a[0] - b[0]);
  const segments: Segment[] = [];
  let at = 0;
  for (const [start, end] of sorted) {
    const from = Math.max(start, at);
    if (end <= from) {
      continue;
    }
    if (from > at) {
      segments.push({ text: text.slice(at, from), hit: false });
    }
    segments.push({ text: text.slice(from, end), hit: true });
    at = end;
  }
  if (at < text.length) {
    segments.push({ text: text.slice(at), hit: false });
  }
  return segments;
}
