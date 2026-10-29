# Stage 28: Opening a request and the scroll area

Suite [`stage28-e2e.mjs`](../../../e2e/stage28-e2e.mjs) · 17 checks

A request opened from Quick search or Docs scrolls into view and receives focus, also when it sits far down, in a folded section, or on another page. A row that is already visible does not move the page. The title bar stays on top, and only the area between the title bar and the status bar scrolls. Creates "E2E Jump" and deletes it afterwards.

## Flow

### 1. Setup

- the fixture is longer than the window

### 2. The title bar stays while the page scrolls

- the title bar and its Close button stay at the top while scrolling
- only the area between the title bar and the status bar scrolls, so its scrollbar starts under the window buttons

### 3. Quick search, Enter, far below

- Enter opens the highlighted request
- a request far below scrolls into view
- the opened card takes keyboard focus

### 4. Quick search, mouse, far above

- a click on a result far above scrolls back up to it

### 5. A request inside a folded section

- a request in a folded section unfolds it and scrolls into view

### 6. From another page

- a short page fills the window without extra scroll
- from Home, the collection opens on the request

### 7. Docs › Open request

- Docs buttons name their request with spaces intact
- Docs › Open request scrolls the card into view

### 8. A row already on screen

- opening a row already on screen leaves the page where it is

### 9. Focus mode still fills the window

- focus mode fills the window below the title bar

### 10. Adding a request at the end of a long section

- a request added from a section head scrolls into view and takes focus
- the suite ran to the end
- test data cleaned up

---

Previous: [None indentation, SSL/TLS, and Docs](stage-27.md) · [Stage list](../README.md#stage-list) · Next: [Find in the editor, new names, and a compact Docs page](stage-29.md)
