const encoder = new TextEncoder();
const decoder = new TextDecoder();

export interface ClippedText {
  text: string;
  truncated: boolean;
}

/**
 * Keeps at most `maxBytes` of UTF-8, cut on a character boundary. A clipped result is a fresh
 * string: in V8 a `slice` of a large text keeps the whole original alive, so long-lived snippets
 * of big bodies would hold every body in memory.
 */
export function clipUtf8(text: string, maxBytes: number): ClippedText {
  // A UTF-16 unit never takes more than 3 bytes in UTF-8, so short texts fit without encoding.
  if (text.length * 3 <= maxBytes) {
    return { text, truncated: false };
  }
  const buffer = new Uint8Array(maxBytes);
  const { read, written } = encoder.encodeInto(text, buffer);
  if (read === text.length) {
    return { text, truncated: false };
  }
  return { text: decoder.decode(buffer.subarray(0, written)), truncated: true };
}
