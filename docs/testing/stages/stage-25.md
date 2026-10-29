# Stage 25: Title bar and the collapsed sidebar

Suite [`stage25-e2e.mjs`](../../../e2e/stage25-e2e.mjs) · 16 checks

Tests the window buttons, double-click to maximize, the resize edges, and collapsing the sidebar to the icon rail (button, Ctrl+B, remembered, locked in focus mode and in a narrow window). The native menu is tested only when `CODESTAMP_MENU_TEST=1` is set and the desktop is unlocked. The suite closes the window at the end, so run it on its own. Creates "E2E Titlebar" and deletes it afterwards.

## Requirements

- Run it on its own: the suite closes the app at the end.

## Flow

### 1. The bar

- the title bar has the menu, the sidebar toggle and the window buttons

### 2. Sidebar toggle

- the toggle collapses the sidebar to the icon rail, navigation still in view
- a rail icon still opens its page
- a collapsed sidebar stays collapsed after a restart
- Ctrl+B expands it again
- Ctrl+B leaves the sidebar alone while a dialog is open

### 3. Focus mode locks the toggle

- focus mode hides the sidebar and locks the toggle until it ends

### 4. Narrow window

- a narrow window shows the rail and the toggle says why it waits

### 5. Window buttons

- Maximize and Restore resize the window and swap the button
- a double-click on the empty bar maximizes and restores
- the window can still be resized from its edges
- Minimize sends the window to the taskbar

### 6. The native menu (needs an unlocked desktop)

- Go › Import opens Import
- View › Collapse sidebar collapses it
- test data cleaned up
- Close quits Codestamp

---

Previous: [XML and HTML bodies, SOAP, and WSDL import](stage-24.md) · [Stage list](../README.md#stage-list) · Next: [The Tools group and Catalog](stage-26.md)
