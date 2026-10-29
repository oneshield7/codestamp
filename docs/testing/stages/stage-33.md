# Stage 33: Status bar, Ctrl+S, sync manual, dan environment bersama

Suite [`stage33-e2e.mjs`](../../../e2e/stage33-e2e.mjs) · 16 cek

Status bar berisi Settings, Recently deleted, Cookies, dan Environments. Ctrl+S menyimpan environment dan panel collection, tidak ada yang tersinkron sendiri tanpa Sync now, variabel collection yang ditimpa environment aktif diberi tanda, dan environment milik collection ikut dibagikan.

## Kebutuhan

- Git for Windows. `https://git.test/` dipetakan ke bare repository lokal di `e2e/.work/git-hosts/` (lihat [harness](../README.md#harness)).

## Alur

### 1. Awal

- the collection is shared over the stand-in host

### 2. Note 5, moved on 2026-10-28: Settings and data pages sit in the status bar

- the status bar holds Settings, Recently deleted, Cookies and Environments
- the status bar runs along the bottom of the window
- its buttons open their pages

### 3. Note 2: Ctrl+S saves an environment

- Ctrl+S saves the environment being edited

### 4. Note 4: collection variables the active environment replaces

- a collection variable the active environment also has is marked replaced

### 5. Notes 2 and 3: Ctrl+S in the collection panel, and nothing syncs on its own

- Ctrl+S saves collection variables too
- a saved change waits for Sync now
- Sync now sends it

### 6. Note 1: an environment that belongs to the collection is shared

- Belongs to gives the environment to the collection
- the collection then has changes to sync
- Sync now writes it to environments/ without the secret value
- a collaborator’s environments and variables arrive with Sync now
- the secret value on this device is kept
- Git changes lists the environment files of a commit
- test data cleaned up

---

Sebelumnya: [Share ke repository yang sudah berisi file](stage-32.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Pembaruan dalam aplikasi](stage-34.md)
