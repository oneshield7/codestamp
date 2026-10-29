# Stage 32: Share ke repository yang sudah berisi file

Suite [`stage32-e2e.mjs`](../../../e2e/stage32-e2e.mjs) · 13 cek

Join menjelaskan masalahnya bila repository tidak dibagikan dari Codestamp. Share menampilkan daftar file untuk ditinjau sebelum menggantinya, dan repository yang sudah diganti bisa di-join.

## Kebutuhan

- Git for Windows. `https://git.test/` dipetakan ke bare repository lokal di `e2e/.work/git-hosts/` (lihat [harness](../README.md#harness)).

## Alur

### 1. Join explains what is wrong with a repository Codestamp did not share

- Join says the repository was not shared from Codestamp

### 2. Share reviews the files first

- a repository with files is shown for review first
- nothing is pushed before the review is confirmed
- the review offers Cancel and Replace files and share
- Cancel goes back to the Share button
- editing the address drops the review
- a repository that moved after the review is not replaced
- checking again lists the new file too
- Replace files and share links the collection on the repository branch
- the old files are gone and the collection is in their place
- one ordinary commit on top keeps the old ones in history

### 3. The replaced repository can now be joined

- the replaced repository can be joined
- test data cleaned up

---

Sebelumnya: [Git accounts, Git changes, dan replace collection](stage-31.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Status bar, Ctrl+S, sync manual, dan environment bersama](stage-33.md)
