export type View =
  | 'home'
  | 'collection'
  | 'run'
  | 'flows'
  | 'catalog'
  | 'docs'
  | 'changes'
  | 'environments'
  | 'cookies'
  | 'import'
  | 'trash'
  | 'settings';

/** The places with their own coloured icon, in the sidebar and on their page title. */
export type NavIconName =
  | 'home'
  | 'flows'
  | 'catalog'
  | 'docs'
  | 'changes'
  | 'environments'
  | 'cookies'
  | 'history'
  | 'import'
  | 'trash'
  | 'settings';

/** Places that are pages. History has an icon but lives in each collection, not on a page. */
export type NavPage = Extract<NavIconName, View>;

const NAV_LABELS: ReadonlyMap<NavIconName, string> = new Map<NavIconName, string>([
  ['home', 'Home'],
  ['flows', 'Orchestration'],
  ['catalog', 'Catalog'],
  ['docs', 'Docs'],
  ['changes', 'Git changes'],
  ['environments', 'Environments'],
  ['cookies', 'Cookies'],
  ['history', 'History'],
  ['import', 'Import'],
  ['trash', 'Recently deleted'],
  ['settings', 'Settings'],
]);

/** Each place's name in the sidebar, Menu › Go, quick search, Home and its page title. */
export function navLabel(name: NavIconName): string {
  return NAV_LABELS.get(name) ?? name;
}

/** A circle as SVG path data, so icons can be stored as one path string. */
function circle(cx: number, cy: number, r: number): string {
  return `M${String(cx - r)} ${String(cy)}a${String(r)} ${String(r)} 0 1 0 ${String(2 * r)} 0a${String(r)} ${String(r)} 0 1 0 ${String(-2 * r)} 0`;
}

/** SVG path data (24×24) of each place's icon, for the sidebar, page titles and Home. */
export const NAV_ICON_PATHS: ReadonlyMap<NavIconName, string> = new Map<NavIconName, string>([
  ['home', 'M4 11l8-7 8 7M6 10v10h12V10'],
  [
    'flows',
    `${circle(6, 5, 2)}${circle(18, 19, 2)}M6 7v3a3 3 0 0 0 3 3h6a3 3 0 0 1 3 3v1M15.5 14.5L18 17l2.5-2.5`,
  ],
  ['catalog', 'M4 4h16v7H4zM4 13h16v7H4zM10 7.5h4M10 16.5h4'],
  ['docs', 'M6 3h9l4 4v14H6zM14 3v5h5M9 12h7M9 16h5'],
  ['changes', `${circle(12, 12, 3)}M3 12h6M15 12h6`],
  [
    'environments',
    `${circle(12, 12, 8)}M4 12h16M12 4c2.5 2.5 2.5 13.5 0 16M12 4c-2.5 2.5-2.5 13.5 0 16`,
  ],
  [
    'cookies',
    `M12 4a8 8 0 1 0 8 8 3 3 0 0 1-3-3 3 3 0 0 1-3-3 2 2 0 0 1-2-2z${circle(9, 11, 1)}${circle(14, 15, 1)}${circle(9, 16, 1)}`,
  ],
  ['history', `${circle(12, 12, 8)}M12 8v4l3 2`],
  ['import', 'M12 4v12M7 11l5 5 5-5M4 20h16'],
  ['trash', 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3'],
  [
    'settings',
    `${circle(12, 12, 3)}M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z`,
  ],
]);

/** Each place keeps one colour (a `data-hue` name), in the sidebar, on its page title and Home. */
export const NAV_ICON_HUES: ReadonlyMap<NavIconName, string> = new Map<NavIconName, string>([
  ['home', 'blue'],
  ['flows', 'orange'],
  ['catalog', 'violet'],
  ['docs', 'teal'],
  ['changes', 'rose'],
  ['environments', 'green'],
  ['cookies', 'amber'],
  ['history', 'violet'],
  ['import', 'teal'],
  ['trash', 'slate'],
  ['settings', 'rose'],
]);

/** The ways the New collection dialog can start. */
export type NewCollectionStart = 'empty' | 'import' | 'join';

export type SettingsSection =
  'appearance' | 'editor' | 'requests' | 'proxy' | 'tls' | 'git' | 'data' | 'about';

export interface SettingsSectionInfo {
  key: SettingsSection;
  label: string;
  hue: string;
  /** SVG path data for the section icon. */
  icon: string;
  /** Extra words quick search matches, for settings people look for by name. */
  keywords: string;
}

export const SETTINGS_SECTIONS: readonly SettingsSectionInfo[] = [
  {
    key: 'appearance',
    hue: 'violet',
    label: 'Appearance',
    icon: 'M12 3a9 9 0 1 0 0 18c1 0 1.5-.7 1.5-1.5 0-1.2-1-1.5-1-2.5 0-.8.7-1.5 1.5-1.5H16a5 5 0 0 0 5-5c0-4.1-4-7.5-9-7.5zM7.5 12h.01M10 8h.01M15 8h.01',
    keywords: 'theme light dark night system font size',
  },
  {
    key: 'editor',
    hue: 'blue',
    label: 'Editor',
    icon: 'M8 7l-5 5 5 5M16 7l5 5-5 5',
    keywords: 'indent tabs brackets wrap layout suggestions',
  },
  {
    key: 'requests',
    hue: 'green',
    label: 'Requests',
    icon: 'M5 12h14M13 6l6 6-6 6',
    keywords: 'timeout redirects response size cookies cache user agent',
  },
  {
    key: 'proxy',
    hue: 'teal',
    label: 'Proxy',
    icon: 'M4 7h16M4 12h16M4 17h10M18 15l2 2-2 2',
    keywords: 'proxy socks bypass network',
  },
  {
    key: 'tls',
    hue: 'amber',
    label: 'SSL/TLS',
    icon: 'M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6zM9 12l2 2 4-4',
    keywords: 'ssl tls version https certificates ca client mtls',
  },
  {
    key: 'git',
    hue: 'orange',
    label: 'Git accounts',
    icon: `${circle(6, 5, 2)}${circle(6, 19, 2)}${circle(18, 7, 2)}M6 7v10M18 9c0 5-12 3-12 8`,
    keywords: 'git github gitlab bitbucket account credential sign in sign out collaboration',
  },
  {
    key: 'data',
    hue: 'rose',
    label: 'Data & history',
    icon: 'M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3',
    keywords: 'backup restore history clear database',
  },
  {
    key: 'about',
    hue: 'orange',
    label: 'About & shortcuts',
    icon: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8h.01',
    keywords: 'version shortcuts keyboard data folder',
  },
];
