# Stage 15: Wording, Quick search, and saving the response body

Suite [`stage15-e2e.mjs`](../../../e2e/stage15-e2e.mjs) · 24 checks

Makes sure the UI text never mentions "Postman" and uses "Collaboration", then tests Quick search: path suggestions, searching from inside the code editor, method words, Settings pages and environments, and closing it. Finally saves a response body to a file. Creates the collection "E2E Search" and deletes it afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).
- The Windows save or open file dialog, answered by `answer-dialog.ps1`.

## Flow

### 1. Wording

- Home no longer names Postman or teams
- Import is general

### 2. Data

- the collection header says Collaboration
- the export button is general

### 3. Open & empty state

- Ctrl+Space opens quick search with the field focused
- it rises into view

### 4. Path suggestions

- typing a path lists matching requests first
- the path is completed inline
- Tab takes the suggestion and offers the next segment
- arrows move and Enter opens the request

### 5. Inside an editor

- Ctrl+Space in a code editor opens search, not code completion
- Ctrl+Space again closes it

### 6. Method words

- a method word narrows to that method

### 7. Settings, pages, environments

- "dark" leads to Settings › Appearance
- a settings section opens directly
- an environment opens on the Environments page
- no match says so

### 8. Closing

- Esc closes
- the sidebar button opens it and a click outside closes it
- it stays shut while another dialog is open

### 9. Save response body

- a response offers Save body
- the body is saved byte for byte
- bodies over 32 MB cannot be saved, and say why
- test data cleaned up

---

Previous: [Dropdowns and colored icons](stage-14.md) · [Stage list](../README.md#stage-list) · Next: [Path variables and data-driven runs](stage-16.md)
