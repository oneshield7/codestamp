# Stage 40: Quick search, shortcut, status bar, Export, History, dan Recently deleted

Suite [`stage40-e2e.mjs`](../../../e2e/stage40-e2e.mjs) · 40 cek

Quick search terbuka tanpa latar blur dan tanpa frame yang tersendat, tidak ada teks shortcut di layar maupun tooltip, tooltip rel dan title bar tidak terpotong, status bar menampilkan versi yang sedang berjalan, Export ada di collection dengan dropdown format (Codestamp, Postman v2.1, Docs), file Codestamp bisa diimpor, History terpisah per collection, dan Recently deleted memulihkan item lewat Undo maupun halamannya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).
- Dialog simpan atau buka file Windows, dijawab oleh `answer-dialog.ps1`.

## Alur

### 1. No shortcut text anywhere

- Home shows no shortcut hints
- the sidebar search button just says Search
- the Send button and the card show no shortcut
- quick search has no key hints and no blurred backdrop

### 2. Quick search opens smoothly

- opening quick search keeps the frame rate (no long frame)

### 3. The sidebar toggle's tooltip stays inside the window

- the Collapse sidebar tooltip is whole and has no shortcut

### 4. Rail tooltips

- hovering a collection in the rail shows its name beside it
- rail pages show their names too
- the rail tooltip goes when the pointer leaves
- keyboard focus on a rail item shows its tooltip
- the rail has no native titles on top of its tooltips
- expanding the sidebar drops a rail tooltip still on the focused item

### 5. The status bar

- a status bar across the bottom holds Settings, Recently deleted, Cookies and Environments
- the status bar ends with the running version
- the sidebar no longer has a Settings section
- status bar tooltips open above their buttons, inside the window
- a status bar button opens its page and shows it is open

### 6. Export in the collection

- Export opens a panel in the collection with a format dropdown
- the dropdown offers Codestamp, Postman Collection v2.1 and Docs as Markdown
- a Codestamp file holds the collection, its requests and its environment
- the Codestamp file leaves the secret out and says so
- Postman Collection v2.1 is there for other tools
- Docs export as Markdown from the same panel

### 7. Import of a Codestamp file

- Import is its own page again, without an Export tab
- Import reads the Codestamp file with its environment
- the imported copy has the same requests next to the original

### 8. History per collection

- History in the collection lists only what it sent
- search narrows the collection history
- Open in History opens the request card
- Clear history empties this collection only
- Home’s History tile opens the collection’s History panel

### 9. Recently deleted: Undo right after deleting

- the delete dialog says where it can be restored from
- deleting shows a banner with Undo
- Undo puts the request back and opens it

### 10. Recently deleted page

- Recently deleted lists the collection with its requests and days left
- Restore brings the collection back and offers to open it
- the restored collection keeps its secret and its environment
- a deleted request says which collection it goes back to
- Delete for good removes the copy
- quick search finds Recently deleted

---

Sebelumnya: [Posisi jendela, environment aktif, dan lipatan](stage-39.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Alur collection dasar](collections.md)
