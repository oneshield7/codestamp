/** The parsed value, or null when the text is not complete, valid JSON. */
export function parseJson(text: string): { value: unknown } | null {
  try {
    return { value: JSON.parse(text) as unknown };
  } catch {
    return null;
  }
}

/**
 * Indented JSON, or null when the text is not complete, valid JSON. Re-indents the original
 * tokens instead of re-serialising, so large integers (64-bit ids) and duplicate keys stay
 * exactly as they were.
 */
export function prettyJson(text: string, indent: number | 'tab' = 2): string | null {
  if (!parseJson(text)) {
    return null;
  }
  const unit = indent === 'tab' ? '\t' : ' '.repeat(indent);
  const pad = (depth: number): string => `\n${unit.repeat(depth)}`;
  let output = '';
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = 0; index < text.length; index += 1) {
    const char = text.charAt(index);
    if (inString) {
      output += char;
      if (escaped) {
        escaped = false;
      } else if (char === '\\') {
        escaped = true;
      } else if (char === '"') {
        inString = false;
      }
      continue;
    }
    if (char === '"') {
      inString = true;
      output += char;
    } else if (char === '{' || char === '[') {
      const closing = char === '{' ? '}' : ']';
      const next = nextVisible(text, index + 1);
      if (next.char === closing) {
        output += char + closing;
        index = next.index;
      } else {
        depth += 1;
        output += char + pad(depth);
      }
    } else if (char === '}' || char === ']') {
      depth -= 1;
      output += pad(depth) + char;
    } else if (char === ',') {
      output += `,${pad(depth)}`;
    } else if (char === ':') {
      output += ': ';
    } else if (!isJsonSpace(char)) {
      output += char;
    }
  }
  return output;
}

/** A value written as JSON with Settings › Editor's indentation, the way Beautify writes it. */
export function stringifyJson(value: unknown, indent: number | 'tab'): string {
  const compact = JSON.stringify(value);
  return prettyJson(compact, indent) ?? compact;
}

function nextVisible(text: string, from: number): { char: string; index: number } {
  let index = from;
  while (index < text.length && isJsonSpace(text.charAt(index))) {
    index += 1;
  }
  return { char: text.charAt(index), index };
}

function isJsonSpace(char: string): boolean {
  return char === ' ' || char === '\n' || char === '\r' || char === '\t';
}
