import { tick } from 'svelte';

/** Scrolls `target` to the top and focuses it; instant when Windows asks for less motion.
 *  `target` needs a `tabindex` and a `scroll-margin-top` that clears the title bar. */
export function revealElement(target: HTMLElement): void {
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  target.scrollIntoView({ behavior: reduce ? 'auto' : 'smooth', block: 'start' });
  target.focus({ preventScroll: true });
}

/** Brings an open request card into view once it has rendered; it often opens off screen. */
export async function revealCard(requestId: string | null): Promise<void> {
  if (requestId === null) {
    return;
  }
  await tick();
  const card = document.querySelector<HTMLElement>(`[data-card-id="${requestId}"]`);
  if (card) {
    revealElement(card);
  }
}
