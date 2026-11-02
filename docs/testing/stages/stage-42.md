# Stage 42: Workspaces

Suite [`stage42-e2e.mjs`](../../../e2e/stage42-e2e.mjs) · 24 checks

The switcher sits in the middle of the title bar and opens a menu of workspaces from the keyboard. A new workspace opens empty, with only its own Globals and no cookies from the other one, and Quick search offers to switch back. Moving a collection takes the environment it owns along and leaves the one kept on this device behind. Settings › Workspaces counts what each holds, deletes only an empty workspace after a confirmation, and renames and recolors one. A workspace icon can be a logo looked up by name or by a typed website (only after Logo is chosen), an image file shrunk to 128 pixels, or an emoji, and the launch screen shows the icon of the workspace being opened.

## Requirements

- A local HTTP server started by the suite itself (Node `http`). It also stands in for Google's icon service through `CODESTAMP_ICON_SERVICE`, which Codestamp follows only when it points at 127.0.0.1.

## Flow

### 1. The switcher in the title bar

- the switcher sits in the middle of the title bar and names the open workspace
- the keyboard opens a menu of workspaces with the open one checked, then New and Manage
- Escape closes the menu and gives the keyboard back to the switcher

### 2. A new workspace starts empty

- New workspace asks for a name and offers 8 colors, starting on one no workspace uses yet
- creating a workspace opens it, empty: no collections and only its own Globals
- cookies stay with the workspace that received them
- Quick search finds nothing of the other workspace, and offers switching to it
- switching back from Quick search shows the first workspace again, cookies included

### 3. Moving a collection to another workspace

- Move to workspace lists the other workspaces, one already chosen
- the moved collection leaves the sidebar, with a note offering to open its workspace
- its own environment went with it, and this device’s one stayed
- in the new workspace the collection has its own environment and no longer uses the device one

### 4. Settings › Workspaces

- Settings › Workspaces marks the open one and counts what each holds
- only an empty workspace can be deleted
- deleting the empty workspace removes it after a confirmation
- Edit renames and recolors the workspace, and the switcher follows

### 5. Workspace icons

- choosing Logo looks the name up, and nothing is sent before
- logos found for the name are offered, without repeats, the first one chosen
- a typed website adds its logo first and chooses it
- the logo is saved with its website and shows in the switcher and Settings
- an image file is shrunk to 128 pixels before it is saved
- the image is saved without a website
- an emoji from the picker becomes the icon, with no automatic choice to go back to
- the launch screen shows the workspace being opened: its letter, or its logo

---

Previous: [The HTML page view, the variable popover, environments per collection, and the sticky header](stage-41.md) · [Stage list](../README.md#stage-list) · Next: [Basic collection flow](collections.md)
