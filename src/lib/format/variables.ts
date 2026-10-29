/** A run of text, or a `{{name}}` reference with the name it refers to. */
export interface TextPart {
  text: string;
  variable: string | null;
}

/**
 * The same names the Rust core fills in: letters, digits, `_ . -`, up to 100 characters, with
 * `$` for dynamic variables and spaces allowed around the name. A new expression each time,
 * since a global one keeps its position between uses.
 */
export function variablePattern(): RegExp {
  return /\{\{\s*(\$?[A-Za-z0-9_.-]{1,100})\s*\}\}/g;
}

export function splitVariables(text: string): TextPart[] {
  const parts: TextPart[] = [];
  let copied = 0;
  for (const match of text.matchAll(variablePattern())) {
    if (match.index > copied) {
      parts.push({ text: text.slice(copied, match.index), variable: null });
    }
    parts.push({ text: match[0], variable: match[1] ?? null });
    copied = match.index + match[0].length;
  }
  if (copied < text.length) {
    parts.push({ text: text.slice(copied), variable: null });
  }
  return parts;
}
