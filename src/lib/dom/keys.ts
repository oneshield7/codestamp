/** Ctrl+S, or Cmd+S on a Mac keyboard. */
export function isSaveShortcut(event: KeyboardEvent): boolean {
  return (event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 's';
}

/** Page-wide shortcuts wait while a dialog is open; it has its own keys. */
export function dialogIsOpen(): boolean {
  return document.querySelector('dialog[open]') !== null;
}
