import type { NameValue } from '../../api/http';
import type { CodeValues, CodeVariable } from './values';

/** Part of a string the generated code builds: literal text, or an expression in that language. */
export type Segment = { text: string; code?: never } | { code: string; text?: never };

/** Adjacent text is joined, so each language writes as few literals as it can. */
export function joined(segments: Segment[]): Segment[] {
  const result: Segment[] = [];
  for (const segment of segments) {
    const last = result.at(-1);
    if (segment.text !== undefined && last?.text !== undefined) {
      last.text += segment.text;
    } else if (segment.text !== '') {
      result.push({ ...segment });
    }
  }
  return result;
}

export function textSegments(
  values: CodeValues,
  text: string,
  variable: (item: CodeVariable) => string,
): Segment[] {
  return values
    .pieces(text)
    .map((piece) => (piece.variable ? { code: variable(piece.variable) } : { text: piece.text }));
}

/**
 * `name=value&…` with plain text encoded now and variables encoded when the code runs, for
 * languages without a query builder.
 */
export function encodedPairs(
  values: CodeValues,
  pairs: NameValue[],
  variable: (item: CodeVariable) => string,
  encode: (expression: string) => string,
): Segment[] {
  const encoded = (text: string): Segment[] =>
    values
      .pieces(text)
      .map((piece) =>
        piece.variable
          ? { code: encode(variable(piece.variable)) }
          : { text: encodeURIComponent(piece.text) },
      );
  return pairs.flatMap((pair, index) => [
    ...(index === 0 ? [] : [{ text: '&' }]),
    ...encoded(pair.name),
    { text: '=' },
    ...encoded(pair.value),
  ]);
}

/** The URL with its query appended the way a browser would. */
export function urlSegments(
  values: CodeValues,
  url: string,
  query: NameValue[],
  variable: (item: CodeVariable) => string,
  encode: (expression: string) => string,
): Segment[] {
  if (query.length === 0) {
    return joined(textSegments(values, url, variable));
  }
  return joined([
    ...textSegments(values, url, variable),
    { text: url.includes('?') ? '&' : '?' },
    ...encodedPairs(values, query, variable, encode),
  ]);
}

/** The text's lines, for languages whose multi-line strings are laid out line by line. */
export function lines(text: string): string[] {
  return text.replaceAll('\r\n', '\n').split('\n');
}
