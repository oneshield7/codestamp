export type MarkupLanguage = 'xml' | 'html';

/** HTML elements that never have content, so they do not open a level. */
const VOID_ELEMENTS = new Set([
  'area',
  'base',
  'br',
  'col',
  'embed',
  'hr',
  'img',
  'input',
  'link',
  'meta',
  'source',
  'track',
  'wbr',
]);
/** HTML elements whose content is kept exactly as written. */
const RAW_ELEMENTS = new Set(['script', 'style', 'pre', 'textarea']);
/** Larger texts are shown as they are: re-indenting them would make typing lag. */
const MAX_FORMAT_CHARS = 2_000_000;
const MAX_CHECK_CHARS = 500_000;

type TokenKind = 'open' | 'close' | 'single' | 'text' | 'other';

interface Token {
  kind: TokenKind;
  text: string;
  /** Lower-cased for HTML; the tag name of open, close and single tokens. */
  name: string;
}

/** Where a construct that starts at `from` ends (exclusive), or -1 when it never closes. */
function endOf(text: string, from: number, closer: string): number {
  const found = text.indexOf(closer, from);
  return found === -1 ? -1 : found + closer.length;
}

/** The end of a tag, skipping `>` inside quoted attribute values. */
function tagEnd(text: string, from: number): number {
  let quote = '';
  for (let index = from; index < text.length; index += 1) {
    const char = text.charAt(index);
    if (quote !== '') {
      if (char === quote) {
        quote = '';
      }
    } else if (char === '"' || char === "'") {
      quote = char;
    } else if (char === '>') {
      return index + 1;
    }
  }
  return -1;
}

function tagName(tag: string, language: MarkupLanguage): string {
  const name = /^<\/?\s*([^\s/>]+)/.exec(tag)?.[1] ?? '';
  return language === 'html' ? name.toLowerCase() : name;
}

function tokenize(text: string, language: MarkupLanguage): Token[] | null {
  const tokens: Token[] = [];
  let index = 0;
  while (index < text.length) {
    if (text.charAt(index) !== '<') {
      const next = text.indexOf('<', index);
      const end = next === -1 ? text.length : next;
      tokens.push({ kind: 'text', text: text.slice(index, end), name: '' });
      index = end;
      continue;
    }
    let end: number;
    let kind: TokenKind;
    if (text.startsWith('<!--', index)) {
      end = endOf(text, index + 4, '-->');
      kind = 'other';
    } else if (text.startsWith('<![CDATA[', index)) {
      end = endOf(text, index + 9, ']]>');
      kind = 'text';
    } else if (text.startsWith('<?', index)) {
      end = endOf(text, index + 2, '?>');
      kind = 'other';
    } else if (text.startsWith('<!', index)) {
      end = tagEnd(text, index + 2);
      kind = 'other';
    } else if (text.startsWith('</', index)) {
      end = tagEnd(text, index + 2);
      kind = 'close';
    } else {
      end = tagEnd(text, index + 1);
      kind = 'open';
    }
    if (end === -1) {
      return null;
    }
    const tag = text.slice(index, end);
    const name = kind === 'open' || kind === 'close' ? tagName(tag, language) : '';
    if (
      kind === 'open' &&
      (tag.endsWith('/>') || (language === 'html' && VOID_ELEMENTS.has(name)))
    ) {
      kind = 'single';
    }
    tokens.push({ kind, text: tag, name });
    index = end;
    if (kind === 'open' && language === 'html' && RAW_ELEMENTS.has(name)) {
      // Scripts, styles and preformatted text are copied through untouched, closing tag included.
      const close = text.toLowerCase().indexOf(`</${name}`, index);
      if (close === -1) {
        return null;
      }
      const closeEnd = tagEnd(text, close + 2);
      if (closeEnd === -1) {
        return null;
      }
      const last = tokens.pop();
      tokens.push({
        kind: 'single',
        text: `${last?.text ?? ''}${text.slice(index, closeEnd)}`,
        name,
      });
      index = closeEnd;
    }
  }
  return tokens;
}

/** An element holding only text (or nothing) stays on one line: `<name>Ada</name>`. */
function inlineElement(tokens: Token[], at: number): { line: string; next: number } | null {
  const open = tokens.at(at);
  const middle = tokens.at(at + 1);
  if (!open || !middle) {
    return null;
  }
  if (middle.kind === 'close' && middle.name === open.name) {
    return { line: `${open.text}${middle.text}`, next: at + 2 };
  }
  const close = tokens.at(at + 2);
  if (middle.kind === 'text' && close?.kind === 'close' && close.name === open.name) {
    return { line: `${open.text}${middle.text.trim()}${close.text}`, next: at + 3 };
  }
  return null;
}

/**
 * The text re-indented one element per line, or null when it is not complete markup. Text,
 * attributes, comments and CDATA keep their exact content; only whitespace between tags moves.
 */
export function formatMarkup(
  text: string,
  language: MarkupLanguage,
  indent: number | 'tab' = 2,
): string | null {
  if (text.length > MAX_FORMAT_CHARS) {
    return null;
  }
  const tokens = tokenize(text.trim(), language);
  if (!tokens) {
    return null;
  }
  const unit = indent === 'tab' ? '\t' : ' '.repeat(indent);
  const lines: string[] = [];
  let depth = 0;
  let index = 0;
  while (index < tokens.length) {
    const token = tokens.at(index);
    if (!token) {
      break;
    }
    if (token.kind === 'open') {
      const inline = inlineElement(tokens, index);
      if (inline) {
        lines.push(unit.repeat(depth) + inline.line);
        index = inline.next;
        continue;
      }
      lines.push(unit.repeat(depth) + token.text);
      depth += 1;
    } else if (token.kind === 'close') {
      depth = Math.max(0, depth - 1);
      lines.push(unit.repeat(depth) + token.text);
    } else {
      const trimmed = token.kind === 'text' ? token.text.trim() : token.text;
      if (trimmed !== '') {
        lines.push(unit.repeat(depth) + trimmed);
      }
    }
    index += 1;
  }
  return lines.join('\n');
}

/** Where XML is broken, as "Line 3, column 7: …", or null when it is well-formed. */
export function xmlProblem(text: string): string | null {
  if (text.trim() === '' || text.length > MAX_CHECK_CHARS) {
    return null;
  }
  const parsed = new DOMParser().parseFromString(text, 'application/xml');
  const error = parsed.getElementsByTagName('parsererror').item(0);
  if (!error) {
    return null;
  }
  const message = error.textContent;
  const found = /line (\d+) at column (\d+): ([^\n]+)/.exec(message);
  if (!found) {
    return 'This is not well-formed XML.';
  }
  const [, line, column, detail] = found;
  return `Line ${line ?? ''}, column ${column ?? ''}: ${(detail ?? '').trim()}`;
}

/** The markup language a response is written in, from its Content-Type or, without one, its text. */
export function markupLanguage(contentType: string, text: string): MarkupLanguage | null {
  const media = (contentType.split(';').at(0) ?? '').trim().toLowerCase();
  if (media === 'text/html' || media === 'application/xhtml+xml') {
    return 'html';
  }
  if (media.endsWith('/xml') || media.endsWith('+xml')) {
    return 'xml';
  }
  const start = text.trimStart();
  if (media !== '' || !start.startsWith('<')) {
    return null;
  }
  return /^<(!doctype html|html)/i.test(start) ? 'html' : 'xml';
}
