import type { MethodTone } from '../request/methods';
import type { CaptureBlock, CaptureDocument, CaptureOptions, Span, Tone } from './model';

interface Palette {
  frame: string;
  card: string;
  divider: string;
  tint: string;
  shadow: string;
  tones: Map<Tone, string>;
  methods: Map<MethodTone, string>;
}

interface Fonts {
  ui: string;
  code: string;
}

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

type Op =
  | {
      kind: 'text';
      x: number;
      y: number;
      text: string;
      font: string;
      color: string;
      spacing: string;
    }
  | { kind: 'fill'; rect: Rect; color: string }
  | { kind: 'line'; x1: number; y1: number; x2: number; y2: number; color: string };

interface Plan {
  width: number;
  height: number;
  card: Rect;
  backgrounds: Op[];
  ops: Op[];
}

const LIGHT: Palette = {
  frame: '#e2e4e9',
  card: '#ffffff',
  divider: '#ecedf0',
  tint: '#f9fafb',
  shadow: 'rgba(17, 19, 24, 0.12)',
  tones: new Map<Tone, string>([
    ['text', '#111318'],
    ['muted', '#5a606c'],
    ['key', '#6d3fd0'],
    ['string', '#0a7a4b'],
    ['number', '#a44b07'],
    ['literal', '#1d5bd8'],
    ['punct', '#5a606c'],
    ['ok', '#0a7a4b'],
    ['err', '#c0261b'],
    ['accent', '#111318'],
    ['mask', '#5a606c'],
  ]),
  methods: new Map<MethodTone, string>([
    ['get', '#0a7a4b'],
    ['post', '#a44b07'],
    ['put', '#1d5bd8'],
    ['patch', '#6d3fd0'],
    ['delete', '#c0261b'],
    ['other', '#383d47'],
  ]),
};

const DARK: Palette = {
  frame: '#141413',
  card: '#232220',
  divider: '#34332f',
  tint: '#1c1b19',
  shadow: 'rgba(0, 0, 0, 0.45)',
  tones: new Map<Tone, string>([
    ['text', '#edebe6'],
    ['muted', '#a3a099'],
    ['key', '#d3a6f5'],
    ['string', '#7ed9a6'],
    ['number', '#f4b27a'],
    ['literal', '#8db8ff'],
    ['punct', '#c9c6be'],
    ['ok', '#7ed9a6'],
    ['err', '#ff9c8f'],
    ['accent', '#a5a0ff'],
    ['mask', '#a3a099'],
  ]),
  methods: new Map<MethodTone, string>([
    ['get', '#7ed9a6'],
    ['post', '#f4b27a'],
    ['put', '#8db8ff'],
    ['patch', '#c8b5ff'],
    ['delete', '#ff9c8f'],
    ['other', '#c9c6be'],
  ]),
};

// Sizes in CSS pixels; the image is drawn at up to twice this for sharp text.
const SIDE_WIDTH = 820;
const STACKED_WIDTH = 640;
const SIDE_REQUEST_SHARE = 0.46;
const FRAME = 26;
const RADIUS = 14;
const PAD_X = 16;
const HEADER_PAD_Y = 14;
const COLUMN_PAD_Y = 12;
const GROUP_GAP = 12;
const LINE_GAP = 4;
const URL_LINE_HEIGHT = 20;
const LINE_HEIGHT = 19;
const LABEL_HEIGHT = 18;
const NOTE_LINE_HEIGHT = 18;
const FOOTER_GAP = 12;
const FOOTER_HEIGHT = 16;
const LABEL_SPACING = '0.55px';
const NO_SPACING = '0px';
// Keeps very long bodies from turning into images that need hundreds of MB to draw.
const MAX_ROWS_PER_LINE = 6;
const MAX_TOTAL_ROWS = 600;
const MAX_CHARS_PER_LINE = 2000;
const MAX_PIXELS = 16_000_000;
const PREFERRED_SCALE = 2;

const fontFor = {
  url: (fonts: Fonts): string => `13px ${fonts.code}`,
  method: (fonts: Fonts): string => `600 12px ${fonts.code}`,
  code: (fonts: Fonts): string => `12px ${fonts.code}`,
  label: (fonts: Fonts): string => `600 11px ${fonts.ui}`,
  note: (fonts: Fonts): string => `12px ${fonts.ui}`,
  footer: (fonts: Fonts): string => `11px ${fonts.ui}`,
};

/** Draws the document into `canvas`, resizing it to fit; uses the app's current fonts. */
export async function renderCapture(
  canvas: HTMLCanvasElement,
  doc: CaptureDocument,
  look: Pick<CaptureOptions, 'theme' | 'layout'>,
): Promise<void> {
  const fonts = resolveFonts();
  await Promise.all([
    document.fonts.load(fontFor.code(fonts)),
    document.fonts.load(fontFor.method(fonts)),
    document.fonts.load(fontFor.label(fonts)),
  ]);
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    throw new Error('Drawing is not available in this window.');
  }
  const palette = look.theme === 'dark' ? DARK : LIGHT;
  const plan = new Planner(ctx, palette, fonts).plan(doc, look.layout);
  const scale = Math.min(PREFERRED_SCALE, Math.sqrt(MAX_PIXELS / (plan.width * plan.height)));
  canvas.width = Math.round(plan.width * scale);
  canvas.height = Math.round(plan.height * scale);
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  paint(ctx, plan, palette);
}

export function toPngBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) {
        resolve(blob);
      } else {
        reject(new Error('Could not create the image.'));
      }
    }, 'image/png');
  });
}

function resolveFonts(): Fonts {
  const style = getComputedStyle(document.documentElement);
  return {
    ui: style.getPropertyValue('--font-ui').trim() || 'sans-serif',
    code: style.getPropertyValue('--font-code').trim() || 'monospace',
  };
}

function paint(ctx: CanvasRenderingContext2D, plan: Plan, palette: Palette): void {
  const { card } = plan;
  ctx.fillStyle = palette.frame;
  ctx.fillRect(0, 0, plan.width, plan.height);

  ctx.save();
  ctx.shadowColor = palette.shadow;
  ctx.shadowBlur = 20;
  ctx.shadowOffsetY = 6;
  ctx.fillStyle = palette.card;
  ctx.beginPath();
  ctx.roundRect(card.x, card.y, card.w, card.h, RADIUS);
  ctx.fill();
  ctx.restore();

  ctx.save();
  ctx.beginPath();
  ctx.roundRect(card.x, card.y, card.w, card.h, RADIUS);
  ctx.clip();
  for (const op of [...plan.backgrounds, ...plan.ops]) {
    if (op.kind === 'fill') {
      ctx.fillStyle = op.color;
      ctx.fillRect(op.rect.x, op.rect.y, op.rect.w, op.rect.h);
    } else if (op.kind === 'line') {
      ctx.strokeStyle = op.color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(op.x1, op.y1);
      ctx.lineTo(op.x2, op.y2);
      ctx.stroke();
    }
  }
  ctx.restore();

  ctx.textBaseline = 'middle';
  for (const op of plan.ops) {
    if (op.kind === 'text') {
      ctx.font = op.font;
      ctx.letterSpacing = op.spacing;
      ctx.fillStyle = op.color;
      ctx.fillText(op.text, op.x, op.y);
    }
  }
}

/** Measures and positions everything once; painting then only replays the result. */
class Planner {
  readonly #ctx: CanvasRenderingContext2D;
  readonly #palette: Palette;
  readonly #fonts: Fonts;
  readonly #backgrounds: Op[] = [];
  readonly #ops: Op[] = [];
  #rows = 0;

  constructor(ctx: CanvasRenderingContext2D, palette: Palette, fonts: Fonts) {
    this.#ctx = ctx;
    this.#palette = palette;
    this.#fonts = fonts;
  }

  plan(doc: CaptureDocument, layout: CaptureOptions['layout']): Plan {
    const width = layout === 'side' ? SIDE_WIDTH : STACKED_WIDTH;
    const card: Rect = { x: FRAME, y: FRAME, w: width - 2 * FRAME, h: 0 };
    let y = card.y;
    const hasColumns = doc.request.length > 0 || doc.response.length > 0;

    if (doc.method || doc.url !== null) {
      y += this.#header(doc, card, y);
      if (hasColumns) {
        this.#line(card.x, y, card.x + card.w, y);
      }
    }

    const sideBySide = layout === 'side' && doc.request.length > 0 && doc.response.length > 0;
    if (sideBySide) {
      const leftWidth = Math.round(card.w * SIDE_REQUEST_SHARE);
      const leftHeight = this.#column(doc.request, card.x, y, leftWidth);
      const rightHeight = this.#column(doc.response, card.x + leftWidth, y, card.w - leftWidth);
      const rowHeight = Math.max(leftHeight, rightHeight);
      this.#fill({ x: card.x + leftWidth, y, w: card.w - leftWidth, h: rowHeight });
      this.#line(card.x + leftWidth, y, card.x + leftWidth, y + rowHeight);
      y += rowHeight;
    } else {
      if (doc.request.length > 0) {
        y += this.#column(doc.request, card.x, y, card.w);
      }
      if (doc.response.length > 0) {
        if (doc.request.length > 0) {
          this.#line(card.x, y, card.x + card.w, y);
        }
        const height = this.#column(doc.response, card.x, y, card.w);
        this.#fill({ x: card.x, y, w: card.w, h: height });
        y += height;
      }
    }

    card.h = Math.max(y - card.y, 2 * COLUMN_PAD_Y);
    let height = card.y + card.h + FRAME;
    if (doc.footer !== null) {
      const footerY = card.y + card.h + FOOTER_GAP + FOOTER_HEIGHT / 2;
      this.#text(card.x, footerY, doc.footer, fontFor.footer(this.#fonts), 'muted', NO_SPACING);
      height += FOOTER_GAP + FOOTER_HEIGHT;
    }
    return { width, height, card, backgrounds: this.#backgrounds, ops: this.#ops };
  }

  #header(doc: CaptureDocument, card: Rect, y: number): number {
    const x = card.x + PAD_X;
    let methodWidth = 0;
    if (doc.method) {
      const font = fontFor.method(this.#fonts);
      this.#ops.push({
        kind: 'text',
        x,
        y: y + HEADER_PAD_Y + URL_LINE_HEIGHT / 2,
        text: doc.method.text,
        font,
        color: this.#palette.methods.get(doc.method.tone) ?? this.#tone('text'),
        spacing: NO_SPACING,
      });
      methodWidth = this.#measure(doc.method.text, font, NO_SPACING) + 10;
    }
    const urlHeight =
      doc.url === null
        ? URL_LINE_HEIGHT
        : this.#spans(
            [{ text: doc.url, tone: 'text' }],
            x + methodWidth,
            y + HEADER_PAD_Y,
            card.w - 2 * PAD_X - methodWidth,
            fontFor.url(this.#fonts),
            URL_LINE_HEIGHT,
          );
    return 2 * HEADER_PAD_Y + urlHeight;
  }

  #column(blocks: CaptureBlock[], x: number, y: number, width: number): number {
    let cursor = y + COLUMN_PAD_Y;
    blocks.forEach((block, index) => {
      if (index > 0) {
        cursor += block.kind === 'label' ? GROUP_GAP : LINE_GAP;
      }
      cursor += this.#block(block, x + PAD_X, cursor, width - 2 * PAD_X);
    });
    return cursor + COLUMN_PAD_Y - y;
  }

  #block(block: CaptureBlock, x: number, y: number, width: number): number {
    if (block.kind === 'label') {
      let cursor = x;
      const font = fontFor.label(this.#fonts);
      for (const span of block.spans) {
        this.#text(cursor, y + LABEL_HEIGHT / 2, span.text, font, span.tone, LABEL_SPACING);
        cursor += this.#measure(span.text, font, LABEL_SPACING);
      }
      return LABEL_HEIGHT;
    }
    if (block.kind === 'note') {
      return this.#spans(
        [{ text: block.text, tone: block.tone }],
        x,
        y,
        width,
        fontFor.note(this.#fonts),
        NOTE_LINE_HEIGHT,
      );
    }
    let used = 0;
    const code = fontFor.code(this.#fonts);
    const note = fontFor.note(this.#fonts);
    for (const line of block.lines) {
      if (this.#rows >= MAX_TOTAL_ROWS) {
        const cut: Span = { text: '… cut short to keep the image small', tone: 'muted' };
        return used + this.#spans([cut], x, y + used, width, note, NOTE_LINE_HEIGHT);
      }
      used += this.#spans(line, x, y + used, width, code, LINE_HEIGHT);
    }
    if (block.more !== null) {
      const more: Span = { text: block.more, tone: 'muted' };
      used += this.#spans([more], x, y + used, width, note, NOTE_LINE_HEIGHT);
    }
    return used;
  }

  /** Places spans as wrapped rows; returns the height they take. */
  #spans(
    spans: Span[],
    x: number,
    y: number,
    width: number,
    font: string,
    lineHeight: number,
  ): number {
    const rows = this.#wrap(spans, width, font);
    rows.forEach((row, index) => {
      let cursor = x;
      for (const span of row) {
        this.#text(cursor, y + index * lineHeight + lineHeight / 2, span.text, font, span.tone);
        cursor += this.#measure(span.text, font, NO_SPACING);
      }
    });
    this.#rows += rows.length;
    return rows.length * lineHeight;
  }

  #wrap(spans: Span[], width: number, font: string): Span[][] {
    const rows: Span[][] = [[]];
    let used = 0;
    let charsLeft = MAX_CHARS_PER_LINE;
    for (const span of spans) {
      let rest = span.text.replaceAll('\t', '  ').slice(0, charsLeft);
      charsLeft -= rest.length;
      while (rest.length > 0) {
        const fit = this.#fit(rest, width - used, font, used === 0);
        if (fit === 0) {
          rows.push([]);
          used = 0;
          continue;
        }
        const piece = rest.slice(0, fit);
        rows.at(-1)?.push({ text: piece, tone: span.tone });
        used += this.#measure(piece, font, NO_SPACING);
        rest = rest.slice(fit);
        if (rest.length > 0) {
          if (rows.length >= MAX_ROWS_PER_LINE) {
            rows.at(-1)?.push({ text: ' …', tone: 'muted' });
            return rows;
          }
          rows.push([]);
          used = 0;
        }
      }
    }
    return rows;
  }

  /**
   * How many characters of `text` fit in `width`, preferring to break after a space.
   * A row that is still empty always takes at least one character.
   */
  #fit(text: string, width: number, font: string, rowIsEmpty: boolean): number {
    if (this.#measure(text, font, NO_SPACING) <= width) {
      return text.length;
    }
    let low = 0;
    let high = text.length;
    while (low < high) {
      const middle = Math.ceil((low + high) / 2);
      if (this.#measure(text.slice(0, middle), font, NO_SPACING) <= width) {
        low = middle;
      } else {
        high = middle - 1;
      }
    }
    const space = text.lastIndexOf(' ', low - 1);
    let fit = space > 0 ? space + 1 : low;
    // Never split a surrogate pair (emoji and other astral characters).
    const code = text.charCodeAt(fit - 1);
    if (fit > 1 && code >= 0xd800 && code <= 0xdbff) {
      fit -= 1;
    }
    return fit === 0 && rowIsEmpty ? 1 : fit;
  }

  #measure(text: string, font: string, spacing: string): number {
    this.#ctx.font = font;
    this.#ctx.letterSpacing = spacing;
    return this.#ctx.measureText(text).width;
  }

  #text(x: number, y: number, text: string, font: string, tone: Tone, spacing = NO_SPACING): void {
    this.#ops.push({ kind: 'text', x, y, text, font, color: this.#tone(tone), spacing });
  }

  #fill(rect: Rect): void {
    this.#backgrounds.push({ kind: 'fill', rect, color: this.#palette.tint });
  }

  #line(x1: number, y1: number, x2: number, y2: number): void {
    this.#ops.push({ kind: 'line', x1, y1, x2, y2, color: this.#palette.divider });
  }

  #tone(tone: Tone): string {
    return this.#palette.tones.get(tone) ?? '#000000';
  }
}
