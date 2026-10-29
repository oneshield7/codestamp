import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
  insertNewline,
} from '@codemirror/commands';
import { javascript } from '@codemirror/lang-javascript';
import { json } from '@codemirror/lang-json';
import {
  bracketMatching,
  foldGutter,
  foldKeymap,
  HighlightStyle,
  indentOnInput,
  indentUnit,
  syntaxHighlighting,
} from '@codemirror/language';
import { EditorState, Facet, Prec, type Extension } from '@codemirror/state';
import {
  Decoration,
  drawSelection,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  MatchDecorator,
  ViewPlugin,
  type DecorationSet,
  type ViewUpdate,
} from '@codemirror/view';
import { tags } from '@lezer/highlight';
import type { EditorPreferences, Indent } from '../api/preferences';
import { variablePattern } from '../format/variables';
import { findExtension } from './search';

export type EditorLanguage = 'json' | 'javascript' | 'text' | 'graphql' | 'xml' | 'html';

// Colours come from the theme tokens so the editor follows Light, Dark and Dark Night.
const highlightStyle = HighlightStyle.define([
  { tag: tags.propertyName, color: 'var(--code-property)' },
  {
    tag: [tags.keyword, tags.controlKeyword, tags.definitionKeyword],
    color: 'var(--code-keyword)',
  },
  {
    tag: [tags.function(tags.variableName), tags.function(tags.propertyName)],
    color: 'var(--code-function)',
  },
  { tag: tags.comment, color: 'var(--code-comment)', fontStyle: 'italic' },
  { tag: tags.string, color: 'var(--code-string)' },
  { tag: tags.number, color: 'var(--code-number)' },
  { tag: [tags.bool, tags.null], color: 'var(--code-atom)' },
  {
    tag: [tags.punctuation, tags.brace, tags.squareBracket, tags.separator, tags.angleBracket],
    color: 'var(--code-punctuation)',
  },
  { tag: tags.tagName, color: 'var(--code-function)' },
  { tag: tags.attributeName, color: 'var(--code-property)' },
  { tag: tags.attributeValue, color: 'var(--code-string)' },
  {
    tag: [tags.processingInstruction, tags.documentMeta],
    color: 'var(--code-comment)',
  },
]);

const theme = EditorView.theme({
  '&': {
    height: '100%',
    fontSize: 'var(--font-size-code)',
    backgroundColor: 'transparent',
  },
  '.cm-scroller': {
    fontFamily: 'var(--font-code)',
    lineHeight: 'var(--code-line-height)',
    fontVariantLigatures: 'var(--code-ligatures)',
  },
  '.cm-content': { caretColor: 'var(--text)' },
  '.cm-gutters': {
    backgroundColor: 'transparent',
    border: 'none',
    color: 'var(--text-muted)',
  },
  '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'var(--code-active-line)' },
  '&.cm-focused': { outline: 'none' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
    backgroundColor: 'var(--accent-soft)',
  },
  '.cm-variable': {
    borderRadius: '3px',
    backgroundColor: 'var(--var-bg)',
    color: 'var(--var)',
  },
  '.cm-variable-missing': {
    backgroundColor: 'var(--err-bg)',
    color: 'var(--err)',
    textDecoration: 'underline wavy var(--err)',
    textDecorationThickness: '1px',
    textUnderlineOffset: '3px',
  },
  // Completion and diagnostic popups (GraphQL) follow the theme like the app's own popovers.
  '.cm-tooltip': {
    border: '1px solid var(--border)',
    borderRadius: 'var(--radius-md)',
    backgroundColor: 'var(--surface)',
    color: 'var(--text)',
    boxShadow: 'var(--shadow-pop)',
    overflow: 'hidden',
  },
  '.cm-tooltip.cm-tooltip-autocomplete > ul': {
    fontFamily: 'var(--font-code)',
    maxHeight: '240px',
  },
  '.cm-tooltip.cm-tooltip-autocomplete > ul > li': { padding: '3px 10px' },
  '.cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected]': {
    backgroundColor: 'var(--accent-soft)',
    color: 'var(--text)',
  },
  '.cm-completionMatchedText': {
    textDecoration: 'none',
    color: 'var(--accent-text)',
    fontWeight: '600',
  },
  '.cm-completionDetail': { color: 'var(--text-muted)', marginLeft: '8px' },
  '.cm-diagnostic': { padding: '6px 10px' },
  '.cm-diagnostic-error': { borderLeft: '3px solid var(--err)' },
  '.cm-diagnostic-warning': { borderLeft: '3px solid var(--warn)' },
  '.cm-lintRange-error': {
    backgroundImage: 'none',
    textDecoration: 'underline wavy var(--err)',
    textUnderlineOffset: '3px',
  },
});

export function languageExtension(language: EditorLanguage): Extension {
  switch (language) {
    case 'json':
      return json();
    case 'javascript':
      return javascript();
    // GraphQL, XML and HTML support is loaded on demand by the editor (see CodeEditor).
    case 'text':
    case 'graphql':
    case 'xml':
    case 'html':
      return [];
  }
}

export function baseExtensions(label: string, readOnly: boolean): Extension[] {
  const shared: Extension[] = [
    lineNumbers(),
    foldGutter(),
    drawSelection(),
    bracketMatching(),
    syntaxHighlighting(highlightStyle),
    theme,
    findExtension(label),
    EditorView.contentAttributes.of({ 'aria-label': label }),
  ];
  if (readOnly) {
    return [...shared, EditorState.readOnly.of(true), keymap.of(foldKeymap)];
  }
  return [
    ...shared,
    history(),
    highlightActiveLine(),
    highlightActiveLineGutter(),
    keymap.of([...defaultKeymap, ...historyKeymap, ...foldKeymap]),
  ];
}

const TAB = '\t';
const INDENT_UNITS = new Map([
  ['two', '  '],
  ['four', '    '],
  ['tab', TAB],
]);

/** Settings › Editor: indentation, closing brackets and line wrapping. */
export function preferenceExtensions(
  preferences: EditorPreferences,
  readOnly: boolean,
): Extension[] {
  // CodeMirror needs a unit even with no indentation, for the explicit indent commands.
  const unit = INDENT_UNITS.get(preferences.indent) ?? '  ';
  const extensions: Extension[] = [
    indentUnit.of(unit),
    EditorState.tabSize.of(unit === TAB ? 4 : unit.length),
  ];
  if (!readOnly) {
    extensions.push(...indentKeys(preferences.indent));
  }
  if (preferences.wrapLines) {
    extensions.push(EditorView.lineWrapping);
  }
  if (preferences.closeBrackets && !readOnly) {
    extensions.push(closeBrackets(), keymap.of(closeBracketsKeymap));
  }
  return extensions;
}

/**
 * With no indentation, Enter starts the next line at the left edge and Tab moves on to the next
 * control; otherwise lines keep their indent and Tab indents.
 */
function indentKeys(indent: Indent): Extension[] {
  if (indent === 'none') {
    return [Prec.high(keymap.of([{ key: 'Enter', run: insertNewline }]))];
  }
  return [indentOnInput(), keymap.of([indentWithTab])];
}

/** Names that are defined; `null` marks none as missing. */
const knownVariables = Facet.define<ReadonlySet<string> | null, ReadonlySet<string> | null>({
  combine: (values) => values.at(-1) ?? null,
});

const variableMark = Decoration.mark({ class: 'cm-variable' });
const missingMark = Decoration.mark({ class: 'cm-variable cm-variable-missing' });

const variableMatcher = new MatchDecorator({
  regexp: variablePattern(),
  decoration: (match, view) => {
    const known = view.state.facet(knownVariables);
    const name = match[1] ?? '';
    return known !== null && !known.has(name) ? missingMark : variableMark;
  },
});

const variablePlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = variableMatcher.createDeco(view);
    }

    update(update: ViewUpdate): void {
      // New names can turn any reference known or missing, not just the edited ones.
      this.decorations =
        update.startState.facet(knownVariables) === update.state.facet(knownVariables)
          ? variableMatcher.updateDeco(update, this.decorations)
          : variableMatcher.createDeco(update.view);
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

/** Colours `{{name}}` references, marking names that are not defined. */
export function variableHighlight(known: ReadonlySet<string> | null): Extension[] {
  return [knownVariables.of(known), variablePlugin];
}
