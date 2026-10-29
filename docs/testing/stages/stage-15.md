# Stage 15: Wording, Quick search, dan simpan body response

Suite [`stage15-e2e.mjs`](../../../e2e/stage15-e2e.mjs) · 24 cek

Memastikan teks UI tidak menyebut "Postman" dan memakai "Collaboration", lalu menguji Quick search: saran path, pencarian dari dalam editor kode, kata method, halaman Settings dan environment, serta menutupnya. Terakhir menyimpan body response ke file. Membuat collection "E2E Search" lalu menghapusnya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).
- Dialog simpan atau buka file Windows, dijawab oleh `answer-dialog.ps1`.

## Alur

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

Sebelumnya: [Dropdown dan ikon berwarna](stage-14.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Path variable dan run berbasis data](stage-16.md)
