# Stage 39: Posisi jendela, environment aktif, dan lipatan

Suite [`stage39-e2e.mjs`](../../../e2e/stage39-e2e.mjs) · 14 cek

Jendela terbuka di tengah layar, hanya environment aktif yang bertitik hijau, section Docs mulai terlipat dan lipatannya diingat, grup Tools di sidebar terlipat dan diingat setelah restart, rel ikon tetap menampilkan semua halaman, tombol "+" sejajar dengan angka jumlah request, dan gerigi di status bar membuka Settings.

## Alur

### 1. Awal

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

Sebelumnya: [Layar pembuka](stage-38.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Quick search, shortcut, status bar, Export, History, dan Recently deleted](stage-40.md)
