# Stage 27: None indentation, SSL/TLS, and Docs

Suite [`stage27-e2e.mjs`](../../../e2e/stage27-e2e.mjs) · 27 checks

Tests the "None" editor indentation (the default), Settings › SSL/TLS, and Docs: descriptions from an imported Postman collection, Markdown rendered safely (raw HTML never runs), writing descriptions, masked values, and duplicates. Imports "E2E Docs" and deletes it afterwards.

## Flow

### 1. Settings › Editor: None

- Editor offers None first and saves it

### 2. Settings › SSL/TLS

- Certificates is now SSL/TLS
- Requests links to SSL/TLS, which holds the TLS version and both certificate lists
- TLS 1.3 only is saved, and resetting Requests leaves it alone
- back to TLS 1.2 or newer

### 3. Docs from an imported Postman collection

- Docs sits in Tools after Orchestration and Catalog
- the collection dropdown opens a collection on its overview
- Postman descriptions arrive for the collection, the folder and the request
- a typed-in token is masked while a {{variable}} is shown
- the body and the saved example are listed
- the index lists the overview, requests outside sections, then each folded section; one doc shows at a time
- Up and Down move through the index and show each doc
- Previous and Next walk the requests in index order

### 4. Markdown is drawn, raw HTML never runs

- the preview draws Markdown before saving
- Markdown becomes headings, bold, code, a list, a table and a web link
- raw HTML is shown as text and a javascript: link is not a link
- the description is saved as written

### 5. A request description: write, preview, Ctrl+Enter, Escape

- Ctrl+Enter saves a request description
- Escape leaves the description as it was
- a description over 20,000 characters cannot be saved

### 6. Open request, duplicates

- a duplicated request keeps its description
- Open request opens the card

### 7. None in the editor: no indent on Enter, Beautify at the left edge

- Beautify with None puts each element on its own line at the left edge
- Enter starts the next line at the left edge
- Tab moves on instead of indenting
- the suite ran to the end
- test data cleaned up

---

Previous: [The Tools group and Catalog](stage-26.md) · [Stage list](../README.md#stage-list) · Next: [Opening a request and the scroll area](stage-28.md)
