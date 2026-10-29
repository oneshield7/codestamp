# Stage 27: Indentasi None, SSL/TLS, dan Docs

Suite [`stage27-e2e.mjs`](../../../e2e/stage27-e2e.mjs) · 27 cek

Menguji indentasi editor "None" (bawaan), Settings › SSL/TLS, dan Docs: deskripsi dari collection Postman yang diimpor, Markdown yang digambar aman (HTML mentah tidak pernah jalan), menulis deskripsi, penyamaran nilai, dan duplikat. Mengimpor "E2E Docs" lalu menghapusnya.

## Alur

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

Sebelumnya: [Grup Tools dan Catalog](stage-26.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Membuka request dan area scroll](stage-28.md)
