/** Focus mode shows only the open request card, for presenting and clean screenshots. */
class FocusMode {
  active = $state(false);
  hideSecrets = $state(true);

  enter(): void {
    this.active = true;
  }

  exit(): void {
    this.active = false;
  }
}

export const focusMode = new FocusMode();
