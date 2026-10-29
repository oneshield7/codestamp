# Stage 28: Membuka request dan area scroll

Suite [`stage28-e2e.mjs`](../../../e2e/stage28-e2e.mjs) · 17 cek

Request yang dibuka dari Quick search atau Docs digulir ke tampilan dan menerima fokus, juga saat letaknya jauh di bawah, di section terlipat, atau dari halaman lain. Baris yang sudah terlihat tidak menggeser halaman. Title bar tetap di atas, dan hanya area di antara title bar dan status bar yang di-scroll. Membuat "E2E Jump" lalu menghapusnya.

## Alur

### 1. Awal

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

Sebelumnya: [Indentasi None, SSL/TLS, dan Docs](stage-27.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Find di editor, nama baru, dan Docs yang ringkas](stage-29.md)
