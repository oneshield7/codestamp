const NAMED = new Map([
  ['\\', '\\\\'],
  ['\n', '\\n'],
  ['\r', '\\r'],
  ['\t', '\\t'],
]);

/**
 * The inside of a double-quoted literal for languages whose escapes differ from JSON's: line
 * breaks and tabs by name, other control characters through `control`, and `special` for the
 * characters the language treats specially (the quote, `$`, `#`).
 */
export function escaped(
  text: string,
  control: (code: number) => string,
  special: ReadonlyMap<string, string>,
): string {
  return Array.from(text)
    .map((character) => {
      const named = NAMED.get(character) ?? special.get(character);
      if (named !== undefined) {
        return named;
      }
      const code = character.codePointAt(0) ?? 0;
      return code < 0x20 || code === 0x7f ? control(code) : character;
    })
    .join('');
}

export function hex(code: number, width: number): string {
  return code.toString(16).toUpperCase().padStart(width, '0');
}
