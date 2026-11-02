# Stage 41: The HTML page view, the variable popover, environments per collection, and the sticky header

Suite [`stage41-e2e.mjs`](../../../e2e/stage41-e2e.mjs) · 19 checks

An HTML error page from a local server opens as the page in an empty sandbox: no script runs and nothing loads from other sites, and Source switches to the text. Resting the pointer on `{{baseUrl}}` shows its value and where it comes from, Save changes the collection variable and the Resolves to line, a secret stays hidden until Show, the caret opens the same popover from the keyboard, and so does the body editor. Each collection picks its own environment from its own ones and the ones on this device, never one that belongs to another collection, and keeps that choice when another collection picks a different one. On a long collection the header with the environment picker and the tools stays on top: the environment list opens and changes there without scrolling, and a card opened from Quick search lands below the header.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

### 1. An HTML response opens as a page

- an HTML response opens as the page, in an empty sandbox
- the page runs no script and loads no outside file
- Source shows the HTML text instead
- Page brings the rendered page back

### 2. The popover over a variable

- resting on {{baseUrl}} shows its value and where it comes from
- Save in the popover changes the collection variable
- the Resolves to line follows the new value
- Escape closes the popover
- a secret stays hidden until Show reads it
- moving the caret into {{baseUrl}} opens the popover from the keyboard
- the body editor shows the same popover

### 3. A collection only offers its own environments

- a collection offers its own environments and this device's
- another collection does not offer it and keeps its own choice (none yet)
- another collection's environment fills in nothing here
- an environment kept on this device works in any collection
- each collection keeps its own environment

### 4. The collection header stays on top while scrolling

- scrolled down, the header stays at the top with its controls on top
- the environment list opens, closes and changes from the stuck header without scrolling
- a card opened from Quick search lands below the header, not under it

---

Previous: [Quick search, shortcuts, status bar, Export, History, and Recently deleted](stage-40.md) · [Stage list](../README.md#stage-list) · Next: [Workspaces](stage-42.md)
