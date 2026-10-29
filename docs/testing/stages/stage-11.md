# Stage 11: Collaboration lewat Git

Suite [`stage11-e2e.mjs`](../../../e2e/stage11-e2e.mjs) · 30 cek

Membagikan collection ke bare repository lokal, lalu mensimulasikan rekan kerja dengan `git` CLI. Yang diuji: perubahan rekan masuk lewat Sync now, edit lokal menunggu Sync now, ikon ikut tersinkron, konflik saat dua orang mengganti nama request yang sama (pilihan keep both), riwayat dan restore versi request, Join yang ditolak untuk alamat buruk atau collection yang sudah ada, dan Stop syncing.

## Kebutuhan

- Git for Windows. `https://git.test/` dipetakan ke bare repository lokal di `e2e/.work/git-hosts/` (lihat [harness](../README.md#harness)).

## Alur

### 1. CDP

Terhubung ke salinan uji lewat CDP dan memasang helper.

### 2. A local collection

Menyiapkan data atau keadaan untuk langkah berikutnya, tanpa cek sendiri.

### 3. Share

- Team panel suggests the Git identity of this device
- Share pushes the collection and shows its status
- the header shows the team state
- the repository has one file per request
- secret values stay on this device
- the first commit is written as the user
- the local clone lives in the app data folder

### 4. A teammate's changes arrive (open card reloads)

- a teammate's edit reloads the open card
- a teammate's new request appears
- the panel says who changed what

### 5. Local edits wait for Sync now

- a saved edit is not pushed on its own and the header says so
- Sync now pushes the edit
- the commit says what changed and who changed it

### 6. A chosen icon syncs both ways

- a chosen icon is pushed in collection.json
- the repository is written in format 7
- a teammate's icon arrives

### 7. Both rename the same request: conflict, keep both

- overlapping edits wait for a choice, in the panel and above the collection
- the conflict dialog shows both sides
- keep both: their name wins and mine becomes a copy
- the merge and the copy reach the teammate
- nothing is left to resolve
- an idle collection does not sync on its own

### 8. History and restore

- history lists versions and compares one with now
- restore brings the old version back and shares it
- a sync without changes adds no commit

### 9. Join: refused for bad addresses and for a collection already here

- join refuses an ext:: address
- join refuses a collection that is already on this device
- the refused join leaves no folder behind

### 10. Stop syncing

- stop syncing keeps the collection and removes the clone
- test data cleaned up

---

Sebelumnya: [Body form, cookie jar, variabel dinamis, dan Copy as code](stage-10.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Tampilan dasar, Home, dan dialog New collection](stage-12.md)
