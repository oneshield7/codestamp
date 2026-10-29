import {
  closeSearchPanel,
  findNext,
  findPrevious,
  getSearchQuery,
  openSearchPanel,
  search,
  SearchQuery,
  setSearchQuery,
} from '@codemirror/search';
import { EditorSelection, type EditorState, type Extension } from '@codemirror/state';
import { EditorView, keymap, type Command, type Panel, type ViewUpdate } from '@codemirror/view';

/** Counting stops here, so a pattern over a 2 MB response still answers at once. */
const MAX_COUNTED = 9999;

type FindOption = 'caseSensitive' | 'wholeWord' | 'regexp';

const OPTIONS: readonly { key: FindOption; text: string; label: string }[] = [
  { key: 'caseSensitive', text: 'Aa', label: 'Match case' },
  { key: 'wholeWord', text: 'ab', label: 'Whole word' },
  { key: 'regexp', text: '.*', label: 'Regular expression' },
];

const ICONS = {
  previous: 'M6 15l6-6 6 6',
  next: 'M6 9l6 6 6-6',
  close: 'M6 6l12 12M18 6L6 18',
} as const;

const SVG_NS = 'http://www.w3.org/2000/svg';

function icon(path: string): SVGSVGElement {
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('width', '16');
  svg.setAttribute('height', '16');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('aria-hidden', 'true');
  const shape = document.createElementNS(SVG_NS, 'path');
  shape.setAttribute('d', path);
  svg.append(shape);
  return svg;
}

function panelButton(
  label: string,
  content: Node | string,
  onPress: () => void,
): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'cm-find-button';
  button.setAttribute('aria-label', label);
  button.title = label;
  button.append(content);
  // A mouse click leaves the caret in the field, so typing carries on.
  button.addEventListener('mousedown', (event) => {
    event.preventDefault();
  });
  button.addEventListener('click', onPress);
  return button;
}

function countText(state: EditorState): { text: string; invalid: boolean } {
  const query = getSearchQuery(state);
  if (query.search === '') {
    return { text: '', invalid: false };
  }
  if (!query.valid) {
    return { text: 'Invalid pattern', invalid: true };
  }
  const main = state.selection.main;
  let total = 0;
  let current = 0;
  const cursor = query.getCursor(state);
  for (let match = cursor.next(); !match.done && total <= MAX_COUNTED; match = cursor.next()) {
    total += 1;
    if (match.value.from === main.from && match.value.to === main.to) {
      current = total;
    }
  }
  if (total === 0) {
    return { text: 'No matches', invalid: false };
  }
  const shownTotal = total > MAX_COUNTED ? `${String(MAX_COUNTED)}+` : String(total);
  if (current === 0) {
    return { text: `${shownTotal} ${total === 1 ? 'match' : 'matches'}`, invalid: false };
  }
  return { text: `${String(current)} of ${shownTotal}`, invalid: false };
}

class FindPanel implements Panel {
  readonly dom: HTMLElement;
  readonly top = true;
  private readonly field: HTMLInputElement;
  private readonly count: HTMLElement;
  private readonly toggles = new Map<FindOption, HTMLButtonElement>();

  constructor(
    private readonly view: EditorView,
    label: string,
  ) {
    this.dom = document.createElement('div');
    this.dom.className = 'cm-find';
    this.dom.setAttribute('role', 'search');
    this.dom.setAttribute('aria-label', `Find in ${label.toLowerCase()}`);

    this.field = document.createElement('input');
    this.field.className = 'cm-find-field';
    this.field.type = 'text';
    this.field.placeholder = 'Find';
    this.field.spellcheck = false;
    this.field.autocomplete = 'off';
    this.field.setAttribute('aria-label', 'Find');
    this.field.setAttribute('main-field', 'true');
    this.field.addEventListener('input', () => {
      this.commit(true);
    });
    this.field.addEventListener('keydown', (event) => {
      this.onKey(event);
    });

    this.count = document.createElement('span');
    this.count.className = 'cm-find-count';
    this.count.setAttribute('aria-live', 'polite');

    const options = OPTIONS.map((option) => {
      const toggle = panelButton(option.label, option.text, () => {
        toggle.setAttribute('aria-pressed', String(toggle.getAttribute('aria-pressed') !== 'true'));
        this.commit(true);
      });
      toggle.classList.add('cm-find-option');
      toggle.setAttribute('aria-pressed', 'false');
      this.toggles.set(option.key, toggle);
      return toggle;
    });

    this.dom.append(
      this.field,
      this.count,
      panelButton('Previous match', icon(ICONS.previous), () => {
        this.step(findPrevious);
      }),
      panelButton('Next match', icon(ICONS.next), () => {
        this.step(findNext);
      }),
      ...options,
      panelButton('Close find', icon(ICONS.close), () => {
        this.close();
      }),
    );
    this.show(getSearchQuery(view.state));
  }

  mount(): void {
    this.field.focus();
    this.field.select();
  }

  update(update: ViewUpdate): void {
    const query = getSearchQuery(update.state);
    if (!query.eq(getSearchQuery(update.startState))) {
      this.show(query);
    }
    if (update.docChanged || update.selectionSet) {
      this.showCount(update.state);
    }
  }

  private show(query: SearchQuery): void {
    if (this.field.value !== query.search) {
      this.field.value = query.search;
    }
    const values = new Map<FindOption, boolean>([
      ['caseSensitive', query.caseSensitive],
      ['wholeWord', query.wholeWord],
      ['regexp', query.regexp],
    ]);
    for (const [key, toggle] of this.toggles) {
      toggle.setAttribute('aria-pressed', String(values.get(key) ?? false));
    }
    this.showCount(this.view.state);
  }

  private showCount(state: EditorState): void {
    const { text, invalid } = countText(state);
    this.count.textContent = text;
    this.count.classList.toggle('invalid', invalid);
  }

  private pressed(key: FindOption): boolean {
    return this.toggles.get(key)?.getAttribute('aria-pressed') === 'true';
  }

  /** Applies the field and options; with `jump`, selects the first match from the caret on. */
  private commit(jump: boolean): void {
    const query = new SearchQuery({
      search: this.field.value,
      caseSensitive: this.pressed('caseSensitive'),
      wholeWord: this.pressed('wholeWord'),
      regexp: this.pressed('regexp'),
      // Bodies often hold escaped text such as \n, which should match as typed.
      literal: true,
    });
    if (!query.eq(getSearchQuery(this.view.state))) {
      this.view.dispatch({ effects: setSearchQuery.of(query) });
    }
    if (jump && query.valid) {
      this.selectMatchFrom(this.view.state.selection.main.from);
    }
  }

  /** Selects the first match at or after `from`, wrapping to the top, without touching the field. */
  private selectMatchFrom(from: number): void {
    const state = this.view.state;
    const query = getSearchQuery(state);
    const after = query.getCursor(state, from).next();
    const match = after.done ? query.getCursor(state).next() : after;
    if (match.done) {
      return;
    }
    this.view.dispatch({
      selection: EditorSelection.single(match.value.from, match.value.to),
      effects: EditorView.scrollIntoView(match.value.from, { y: 'center' }),
      userEvent: 'select.search',
    });
  }

  /** CodeMirror selects the field text after a jump, so the next keystroke would replace the
   *  whole query; the caret goes back to the end instead. */
  private step(command: Command): void {
    command(this.view);
    const end = this.field.value.length;
    this.field.setSelectionRange(end, end);
  }

  private onKey(event: KeyboardEvent): void {
    const withModifier = event.ctrlKey || event.metaKey;
    if (event.key === 'Enter' && !withModifier) {
      event.preventDefault();
      this.step(event.shiftKey ? findPrevious : findNext);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      this.close();
    } else if (withModifier && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      this.field.select();
    }
  }

  private close(): void {
    closeSearchPanel(this.view);
    this.view.focus();
  }
}

const findTheme = EditorView.theme({
  '.cm-panels': {
    backgroundColor: 'var(--surface-muted)',
    color: 'var(--text)',
  },
  '.cm-panels-top': { borderBottom: '1px solid var(--border-soft)' },
  '.cm-find': {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '2px',
    padding: '6px 8px',
    fontFamily: 'var(--font-ui)',
    fontSize: '13px',
  },
  '.cm-find-field': {
    flex: '1 1 160px',
    minWidth: '0',
    height: '28px',
    marginRight: '6px',
    padding: '0 8px',
    border: '1px solid var(--border-strong)',
    borderRadius: 'var(--radius-sm)',
    backgroundColor: 'var(--surface)',
    color: 'var(--text)',
    fontFamily: 'var(--font-code)',
    fontSize: '13px',
  },
  '.cm-find-field::placeholder': { color: 'var(--text-muted)' },
  '.cm-find-count': {
    minWidth: '84px',
    marginRight: '4px',
    color: 'var(--text-muted)',
    fontSize: '12px',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  },
  '.cm-find-count.invalid': { color: 'var(--err)' },
  '.cm-find-button': {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    minWidth: '28px',
    height: '28px',
    padding: '0 6px',
    border: '0',
    borderRadius: 'var(--radius-sm)',
    backgroundColor: 'transparent',
    color: 'var(--text-2)',
    cursor: 'pointer',
  },
  '.cm-find-button:hover': { backgroundColor: 'var(--accent-soft)', color: 'var(--text)' },
  '.cm-find-button svg': {
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: '2',
    strokeLinecap: 'round',
    strokeLinejoin: 'round',
  },
  '.cm-find-option': { fontFamily: 'var(--font-code)', fontSize: '12px' },
  '.cm-find-option[aria-pressed="true"]': {
    backgroundColor: 'var(--accent-soft)',
    color: 'var(--text)',
    boxShadow: 'inset 0 0 0 1px var(--border-strong)',
  },
  // Every match is tinted; the selected one also gets a solid frame, so it shows without colour.
  '.cm-searchMatch': {
    borderRadius: '2px',
    backgroundColor: 'var(--warn-bg)',
    boxShadow: 'inset 0 0 0 1px var(--warn-line)',
  },
  '.cm-searchMatch.cm-searchMatch-selected': { boxShadow: 'inset 0 0 0 2px var(--warn)' },
});

/** Ctrl+F opens find above the editor; Enter and F3 step through the matches. */
export function findExtension(label: string): Extension[] {
  return [
    search({ top: true, literal: true, createPanel: (view) => new FindPanel(view, label) }),
    keymap.of([
      { key: 'Mod-f', run: openSearchPanel, preventDefault: true },
      { key: 'F3', run: findNext, shift: findPrevious, preventDefault: true },
      { key: 'Mod-g', run: findNext, shift: findPrevious, preventDefault: true },
      { key: 'Escape', run: closeSearchPanel },
    ]),
    findTheme,
  ];
}

/** Keys pressed in a code editor (find field included) are the editor's to handle. */
export function isInEditor(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest('.cm-editor') !== null;
}

/** Opens find in the editor marked `data-find` inside `root`, else in its first editor. */
export function openFindIn(root: Element): boolean {
  const host = root.querySelector('[data-find] .cm-editor') ?? root.querySelector('.cm-editor');
  const view = host instanceof HTMLElement ? EditorView.findFromDOM(host) : null;
  return view !== null && openSearchPanel(view);
}
