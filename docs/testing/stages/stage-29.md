# Stage 29: Find in the editor, new names, and a compact Docs page

Suite [`stage29-e2e.mjs`](../../../e2e/stage29-e2e.mjs) · 23 checks

Tests Ctrl+F in the response and body editors, the names Orchestration and Catalog, a Docs page that stays short with many collections and can be searched, and Docs results in Quick search. Creates "E2E Find" and eight "E2E Docs list" collections and deletes them afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

### 1. Fixture

Prepares data or state for the next steps, with no checks of its own.

### 2. New names

- the Tools group reads Orchestration, Catalog, Docs and Git changes
- the pages carry the new names
- quick search still finds Catalog by its old name

### 3. Ctrl+F in the response body

- Ctrl+F outside an editor opens find in the response body
- typing finds every match, Enter and Shift+Enter step through them
- typing letter by letter keeps the whole word, and after Enter the next letter adds on
- every match is marked in the body
- Match case narrows the matches
- a broken regular expression says so
- Escape closes find and returns to the body, the card stays open
- Ctrl+F with the focus outside the card still opens find above the response body

### 4. Ctrl+F in the request body

- Ctrl+F in the request body searches the body itself

### 5. Docs: an index and one doc at a time, searchable

- the dropdown lists every collection and the index fits the window, one doc at a time
- variables fold into a table and a token-like value is masked
- Ctrl+F on the Docs page goes to its search
- the docs search keeps the requests that match, description included
- a matching section keeps all of its requests
- no match says so, and Clear search brings the whole index back

### 6. Docs in quick search

- quick search finds words written in Docs
- picking it opens Docs on that request, selected in the index and in view
- Docs results lead while Docs is open, requests lead elsewhere
- the suite ran to the end
- test data cleaned up

---

Previous: [Opening a request and the scroll area](stage-28.md) · [Stage list](../README.md#stage-list) · Next: [A Git account per collection](stage-30.md)
