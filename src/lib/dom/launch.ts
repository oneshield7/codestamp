import type { Workspace } from '../api/workspaces';
import { initial } from '../format/names';

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

/** Where the launch screen finds the workspace a reload opens (read in Rust's launch script). */
const OPENING_KEY = 'codestamp.openingWorkspace';

/**
 * Shows the workspace about to open on the launch screen of the reload that follows, in place of
 * the Codestamp logo. Resolves to false when storage refused it; the reload then shows the logo.
 */
export function showOnLaunch(workspace: Workspace): boolean {
  const icon = workspace.icon;
  const shown = {
    name: workspace.name,
    hue: workspace.hue,
    text:
      icon.kind === 'emoji' ? icon.emoji : icon.kind === 'letter' ? initial(workspace.name) : '',
    image: icon.kind === 'image' ? icon.dataUrl : null,
  };
  try {
    sessionStorage.setItem(OPENING_KEY, JSON.stringify(shown));
    return true;
  } catch {
    return false;
  }
}
