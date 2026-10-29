import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import type { Theme } from '../api/settings';
import { THEMES } from '../appearance/presets';
import { navLabel, type NavPage, type SettingsSection, type View } from '../state/navigation';

/** What the menu shows as it is now, and what its items do. */
export interface AppMenuActions {
  sidebarCollapsed: boolean;
  /** Focus mode or a narrow window: the sidebar cannot be collapsed or expanded now. */
  sidebarLocked: boolean;
  theme: Theme;
  newCollection: () => void;
  navigate: (view: View) => void;
  openSettings: (section: SettingsSection) => void;
  search: () => void;
  toggleSidebar: () => void;
  setTheme: (theme: Theme) => void;
  exit: () => void;
}

const GO_TO: readonly NavPage[] = [
  'home',
  'flows',
  'catalog',
  'docs',
  'changes',
  'environments',
  'cookies',
  'import',
  'trash',
  'settings',
];

// Windows carries these out on the focused field itself, CodeMirror editors included.
const EDIT_ITEMS = [
  'Undo',
  'Redo',
  'Separator',
  'Cut',
  'Copy',
  'Paste',
  'Separator',
  'SelectAll',
] as const;

function entry(text: string, action: () => void, enabled = true): Promise<MenuItem> {
  return MenuItem.new({
    text,
    enabled,
    action: () => {
      action();
    },
  });
}

function separator(): Promise<PredefinedMenuItem> {
  return PredefinedMenuItem.new({ item: 'Separator' });
}

/** The application menu (File, Edit, View, Go, Help) shown from the title bar. */
export async function buildAppMenu(actions: AppMenuActions): Promise<Menu> {
  const file = await Submenu.new({
    text: 'File',
    items: [
      await entry('New collection', actions.newCollection),
      await entry('Import…', () => {
        actions.navigate('import');
      }),
      await separator(),
      await entry('Back up data…', () => {
        actions.openSettings('data');
      }),
      await entry('Settings', () => {
        actions.navigate('settings');
      }),
      await separator(),
      await entry('Exit', actions.exit),
    ],
  });
  const edit = await Submenu.new({
    text: 'Edit',
    items: await Promise.all(EDIT_ITEMS.map((item) => PredefinedMenuItem.new({ item }))),
  });
  const themes = await Submenu.new({
    text: 'Theme',
    items: await Promise.all(
      THEMES.map((option) =>
        CheckMenuItem.new({
          text: option.label,
          checked: option.key === actions.theme,
          action: () => {
            actions.setTheme(option.key);
          },
        }),
      ),
    ),
  });
  const view = await Submenu.new({
    text: 'View',
    items: [
      await entry(
        actions.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar',
        actions.toggleSidebar,
        !actions.sidebarLocked,
      ),
      await separator(),
      themes,
    ],
  });
  const go = await Submenu.new({
    text: 'Go',
    items: [
      ...(await Promise.all(
        GO_TO.map((target) =>
          entry(navLabel(target), () => {
            actions.navigate(target);
          }),
        ),
      )),
      await separator(),
      await entry('Search…', actions.search),
    ],
  });
  const help = await Submenu.new({
    text: 'Help',
    items: [
      await entry('Keyboard shortcuts and about', () => {
        actions.openSettings('about');
      }),
    ],
  });
  return Menu.new({ items: [file, edit, view, go, help] });
}
