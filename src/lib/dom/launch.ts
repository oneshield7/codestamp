/** The launch screen stays at least this long from the window opening, so a quick start does
 * not flicker. */
const MIN_LAUNCH_MS = 500;
/** The longest exit animation in index.html; the screen is removed after it. */
const EXIT_MS = 480;

/** Hands over from the launch screen in index.html to the app once its first content is in. */
export function finishLaunch(): void {
  const screen = document.getElementById('launch');
  if (!screen) {
    return;
  }
  setTimeout(
    () => {
      document.documentElement.classList.add('launched');
      setTimeout(() => {
        screen.remove();
      }, EXIT_MS);
    },
    Math.max(0, MIN_LAUNCH_MS - performance.now()),
  );
}
