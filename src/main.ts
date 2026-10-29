import '@fontsource-variable/geist';
import '@fontsource-variable/geist-mono';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { mcpCloseAll } from './lib/api/mcp';
import { wsCloseAll } from './lib/api/websocket';
import { appearanceSettings } from './lib/state/appearance.svelte';
import { notices } from './lib/state/notices.svelte';
import { preferenceSettings } from './lib/state/preferences.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('Missing #app root element');
}

// Theme and fonts are applied before the first paint so the UI does not flash in the defaults.
await Promise.all([
  appearanceSettings.load(),
  preferenceSettings.load(),
  notices.guard(wsCloseAll),
  notices.guard(mcpCloseAll),
]);

export default mount(App, { target });
