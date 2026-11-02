/// Runs before the page on every load. When the app reloads to open another workspace, it left
/// that workspace's name, color and icon in session storage (`showOnLaunch` in
/// `src/lib/dom/launch.ts`); this puts them on the launch screen in place of the Codestamp logo,
/// as CSS custom properties that `src/launch.css` draws, so they are there at the first paint.
/// Only a known color and a base64 image `data:` URL are accepted, and the item is used once.
pub const SCRIPT: &str = r#"(() => {
  const KEY = 'codestamp.openingWorkspace';
  const HUES = ['blue', 'green', 'teal', 'violet', 'amber', 'orange', 'rose', 'slate'];
  const IMAGE = /^data:image\/(png|jpeg|gif|webp|x-icon);base64,[A-Za-z0-9+/]+=*$/;
  let opening = null;
  try {
    opening = JSON.parse(sessionStorage.getItem(KEY) ?? 'null');
    sessionStorage.removeItem(KEY);
  } catch {
    opening = null;
  }
  if (!opening || typeof opening.name !== 'string' || !HUES.includes(opening.hue)) return;
  const quote = (text) =>
    '"' + String(text).replace(/[\\"\n\r]/g, (c) => '\\' + c.charCodeAt(0).toString(16) + ' ') + '"';
  const image = typeof opening.image === 'string' && IMAGE.test(opening.image) ? opening.image : null;
  const text = image || typeof opening.text !== 'string' ? '' : opening.text;
  const apply = () => {
    const root = document.documentElement;
    root.dataset.opening = image ? 'image' : 'text';
    root.style.setProperty('--opening-name', quote(opening.name));
    root.style.setProperty('--opening-text', quote(text));
    root.style.setProperty('--opening-hue', `var(--hue-${opening.hue})`);
    root.style.setProperty('--opening-hue-bg', `var(--hue-${opening.hue}-bg)`);
    if (image) root.style.setProperty('--opening-image', `url("${image}")`);
  };
  const label = () => {
    const screen = document.getElementById('launch');
    if (screen) screen.setAttribute('aria-label', `Opening ${opening.name}`);
    return !!screen;
  };
  if (document.documentElement) {
    apply();
  } else {
    new MutationObserver((_, observer) => {
      if (document.documentElement) {
        apply();
        observer.disconnect();
      }
    }).observe(document, { childList: true });
  }
  if (!label()) {
    new MutationObserver((_, observer) => {
      if (label()) observer.disconnect();
    }).observe(document, { childList: true, subtree: true });
  }
})();"#;
