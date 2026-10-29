# Stage 36: Git changes dengan riwayat panjang

Suite [`stage36-e2e.mjs`](../../../e2e/stage36-e2e.mjs) · 14 cek

Rekan kerja menambah 120 commit lewat bare repository lokal. Halaman Git changes tidak ikut scroll; daftar commit dan detailnya scroll sendiri di bawah header yang tetap. Di jendela kecil, isinya scroll utuh dan commit lama tetap bisa dimuat.

## Kebutuhan

- Git for Windows. `https://git.test/` dipetakan ke bare repository lokal di `e2e/.work/git-hosts/` (lihat [harness](../README.md#harness)).

## Alur

### 1. Awal

- in a small window the history scrolls as a whole
- older commits load in a small window too
- a commit far down shows its detail in a small window
- the collection is shared
- the collaborator's history arrives
- a long history does not make the page scroll
- the commit list scrolls on its own
- the panel ends inside the window
- Load older commits sits at the end of the list and comes into view
- scrolling the list keeps the picker and tabs in place
- older commits load into the same scroll area
- the detail of a commit far down shows without scrolling
- keyboard focus keeps the commit in view with room for its ring
- the Requests tab keeps the same fixed header

---

Sebelumnya: [Perbaikan Git: clone rusak dan sign-in hilang](stage-35.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Grup akses](stage-37.md)
