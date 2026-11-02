/** Where a `{{name}}` sits on screen, so the popover opens next to it. */
export interface VariableAnchor {
  left: number;
  top: number;
  bottom: number;
}

export interface HoveredVariable {
  name: string;
  anchor: VariableAnchor;
}

const OPEN_DELAY_MS = 350;
const CLOSE_DELAY_MS = 200;

export function anchorOf(element: Element): VariableAnchor {
  const box = element.getBoundingClientRect();
  return { left: box.left, top: box.top, bottom: box.bottom };
}

/**
 * Opens the variable popover once the pointer rests on a reference, and keeps it open while the
 * pointer is on the reference or the popover, or focus is inside the popover.
 */
export class VariableHover {
  shown = $state<HoveredVariable | null>(null);

  #reference: Element | null = null;
  #onPopover = false;
  #focusInPopover = false;
  #openTimer: ReturnType<typeof setTimeout> | undefined;
  #closeTimer: ReturnType<typeof setTimeout> | undefined;

  /** The reference under the pointer, or `null` over plain text or outside the field. */
  point(reference: Element | null): void {
    const name = reference?.getAttribute('data-variable') ?? null;
    if (reference === null || name === null) {
      this.#reference = null;
      clearTimeout(this.#openTimer);
      this.#scheduleClose();
      return;
    }
    clearTimeout(this.#closeTimer);
    if (reference === this.#reference) {
      return;
    }
    this.#reference = reference;
    clearTimeout(this.#openTimer);
    // Moving from one reference to the next swaps the popover at once.
    this.#openTimer = setTimeout(
      () => {
        this.shown = { name, anchor: anchorOf(reference) };
      },
      this.shown === null ? OPEN_DELAY_MS : 0,
    );
  }

  /** From the keyboard: the caret is inside this reference. */
  openAt(reference: Element): void {
    const name = reference.getAttribute('data-variable');
    if (name === null) {
      return;
    }
    clearTimeout(this.#openTimer);
    clearTimeout(this.#closeTimer);
    this.#reference = reference;
    this.shown = { name, anchor: anchorOf(reference) };
  }

  pointerOnPopover(on: boolean): void {
    this.#onPopover = on;
    if (on) {
      clearTimeout(this.#closeTimer);
    } else {
      this.#scheduleClose();
    }
  }

  focusInPopover(inside: boolean): void {
    this.#focusInPopover = inside;
    if (!inside) {
      this.#scheduleClose();
    }
  }

  close(): void {
    clearTimeout(this.#openTimer);
    clearTimeout(this.#closeTimer);
    this.#reference = null;
    this.#onPopover = false;
    this.#focusInPopover = false;
    this.shown = null;
  }

  #scheduleClose(): void {
    clearTimeout(this.#closeTimer);
    this.#closeTimer = setTimeout(() => {
      if (this.#reference === null && !this.#onPopover && !this.#focusInPopover) {
        this.close();
      }
    }, CLOSE_DELAY_MS);
  }
}
