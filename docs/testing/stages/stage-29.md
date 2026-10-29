# Stage 29: Find di editor, nama baru, dan Docs yang ringkas

Suite [`stage29-e2e.mjs`](../../../e2e/stage29-e2e.mjs) · 23 cek

Menguji Ctrl+F di editor response dan body, nama Orchestration dan Catalog, halaman Docs yang tetap pendek dengan banyak collection dan bisa dicari, serta hasil Docs di Quick search. Membuat "E2E Find" dan delapan collection "E2E Docs list" lalu menghapusnya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

### 1. Fixture

Menyiapkan data atau keadaan untuk langkah berikutnya, tanpa cek sendiri.

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

Sebelumnya: [Membuka request dan area scroll](stage-28.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Akun Git per collection](stage-30.md)
