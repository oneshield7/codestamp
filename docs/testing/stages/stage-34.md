# Stage 34: Pembaruan dalam aplikasi

Suite [`stage34-e2e.mjs`](../../../e2e/stage34-e2e.mjs) · 21 cek

Diuji terhadap server update lokal dengan build khusus. `node updater/prepare.mjs` membuat kunci tanda tangan sekali pakai, `test.conf.json` (kunci publik uji, alamat `http://127.0.0.1:9455`, dan izin http biasa), "installer" palsu yang hanya mencatat argumennya dan ditandatangani untuk versi berikutnya, serta `release.json` berisi versi yang diumumkan suite. Versinya dihitung dari versi aplikasi, jadi suite ini tidak perlu diubah setiap rilis. Halaman ini juga mencakup `stage34-guard` dan `stage34-unset`, yang memakai build biasa.

## Kebutuhan

- `node updater/prepare.mjs`, lalu build uji dengan `--config e2e/.work/updater/test.conf.json` (lihat [Menjalankan](../README.md#menjalankan)).
- Rust (`rustc`) untuk mengompilasi installer palsu.

## Alur

### 1. The check at start-up

- a newer version shows a banner a few seconds after start
- the check reads latest.json as the Codestamp updater
- the banner offers What's new, Update and restart, and Later

### 2. Release notes in Settings › About

- What's new opens Settings › About with the notes as plain text

### 3. Later

- Later hides the banner but Settings still offers the update

### 4. Unsaved environment edits stop the update

- unsaved environment changes stop the update with a reason

### 5. A tampered download is refused

- a download that fails the signature check is not installed
- the failed update offers Try again

### 6. A release file announcing another version than the signed one

- an installer signed for another version is refused

### 7. Up to date

- the same version reads as up to date and clears the old failure

### 8. Server down

- an unreachable server gives a short reason
- Check for updates is ready again

### 9. Update and restart

- the banner shows download progress
- Codestamp closes and starts the installer in update mode
- the test copy is no longer running

## Penjaga perubahan yang belum disimpan

Suite [`stage34-guard-e2e.mjs`](../../../e2e/stage34-guard-e2e.mjs).

### 1. Awal

- unsaved collection variables still stop leaving, with the same message
- test data cleaned up

## Build tanpa alamat update

Suite [`stage34-unset-e2e.mjs`](../../../e2e/stage34-unset-e2e.mjs).

### 1. Awal

- no update banner without an update server
- the check answers notConfigured without a request
- About says this build is not set up, with no buttons
- the privacy note does not mention an update server

---

Sebelumnya: [Status bar, Ctrl+S, sync manual, dan environment bersama](stage-33.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Perbaikan Git: clone rusak dan sign-in hilang](stage-35.md)
