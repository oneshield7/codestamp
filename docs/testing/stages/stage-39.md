# Stage 39: Window position, the active environment, and folding

Suite [`stage39-e2e.mjs`](../../../e2e/stage39-e2e.mjs) · 14 checks

The window opens in the center of the screen, only the active environment has a green dot, Docs sections start folded and the folding is remembered, the Tools group in the sidebar is folded and remembered after a restart, the icon rail still shows every page, the "+" button lines up with the request count, and the gear in the status bar opens Settings.

## Flow

### 1. Setup

- the window opens in the middle of its screen

### 2. Only the active environment is green

- only the active environment has a green dot

### 3. Docs sections start folded and remember what was opened

- Docs sections start folded, so the index stays short
- the chevron opens a section from the keyboard
- an opened section stays open after a restart
- picking a section shows its doc and its requests
- while searching, matches show even in folded sections
- clearing the search folds them again

### 4. The sidebar's Tools and Settings fold

- in a narrow window the icon rail and the status bar show every page
- Tools folds to its heading
- a folded group stays folded after a restart
- the icon rail still shows every page

### 5. The heading buttons line up with the counts of the rows

- the + icon ends where the row counts end

### 6. The status bar's gear opens Settings

- the status bar gear opens the Settings page

---

Previous: [Launch screen](stage-38.md) · [Stage list](../README.md#stage-list) · Next: [Quick search, shortcuts, status bar, Export, History, and Recently deleted](stage-40.md)
