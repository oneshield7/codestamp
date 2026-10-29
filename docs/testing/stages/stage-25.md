# Stage 25: Title bar dan sidebar yang diciutkan

Suite [`stage25-e2e.mjs`](../../../e2e/stage25-e2e.mjs) · 16 cek

Menguji tombol jendela, double-click untuk maximize, tepi resize, dan toggle sidebar ke rel ikon (tombol, Ctrl+B, diingat, terkunci di focus mode dan di jendela sempit). Menu native hanya diuji bila `CODESTAMP_MENU_TEST=1` dan desktop tidak terkunci. Suite ini menutup jendela di akhir, jadi jalankan sendirian. Membuat "E2E Titlebar" lalu menghapusnya.

## Kebutuhan

- Jalankan sendirian: suite ini menutup aplikasi di akhir.

## Alur

### 1. The bar

- the title bar has the menu, the sidebar toggle and the window buttons

### 2. Sidebar toggle

- the toggle collapses the sidebar to the icon rail, navigation still in view
- a rail icon still opens its page
- a collapsed sidebar stays collapsed after a restart
- Ctrl+B expands it again
- Ctrl+B leaves the sidebar alone while a dialog is open

### 3. Focus mode locks the toggle

- focus mode hides the sidebar and locks the toggle until it ends

### 4. Narrow window

- a narrow window shows the rail and the toggle says why it waits

### 5. Window buttons

- Maximize and Restore resize the window and swap the button
- a double-click on the empty bar maximizes and restores
- the window can still be resized from its edges
- Minimize sends the window to the taskbar

### 6. The native menu (needs an unlocked desktop)

- Go › Import opens Import
- View › Collapse sidebar collapses it
- test data cleaned up
- Close quits Codestamp

---

Sebelumnya: [Body XML dan HTML, SOAP, dan import WSDL](stage-24.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Grup Tools dan Catalog](stage-26.md)
