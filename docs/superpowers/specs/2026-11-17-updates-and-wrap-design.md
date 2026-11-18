# Update otomatis untuk rilis publik, dan wrap output respons

Tanggal: 2026-11-17 · Branch: `feature/updates-and-wrap` (dari `feature/apt-repository`, karena pesan untuk pengguna .deb merujuk ke repository APT)

Dua perubahan kecil yang tidak saling bergantung, dikerjakan di satu branch.

## 1. Update otomatis

### Tujuan dan alasan

Pengguna publik yang memasang Codestamp dari GitHub Releases atau APT mendapat versi baru tanpa mengunduh ulang sendiri. Fitur update di aplikasi sudah lengkap sejak tahap 27: banner, Settings › About › Updates, pemeriksaan tanda tangan, dan mode update NSIS. Tapi fitur itu mati di semua build, karena `plugins.updater.pubkey` dan `endpoints` di `src-tauri/tauri.conf.json` kosong, dan workflow rilis tidak menandatangani update maupun menerbitkan `latest.json`. Versi 0.5.0 menulis "This build of Codestamp is not set up to receive updates."

- **Goals**
  - Push tag `vX.Y.Z` menghasilkan `latest.json` di draft release. Update sampai ke pengguna tepat saat draft dipublish.
  - Installer Windows (NSIS) dan AppImage meng-update dirinya dari dalam aplikasi.
  - Pemasangan .deb tetap tahu ada versi baru, lalu diarahkan ke `apt`.
- **Non-goals**: macOS, RPM, MSI; kanal beta; server update sendiri; .deb memasang update sendiri.
- **Anti-goals**
  - Rilis yang belum direview (masih draft) sampai ke pengguna.
  - Pemasangan .deb mencoba memasang AppImage.
  - Kunci privat masuk repository atau log.

### Batasan

- Kunci tanda tangan update dibuat pemilik dengan password, lalu disimpan sebagai secret GitHub `TAURI_SIGNING_PRIVATE_KEY` (isi file kunci) dan `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Kunci dan password wajib dicadangkan; tanpa keduanya, salinan yang terpasang tidak bisa di-update lagi.
- Pengguna 0.5.0 harus memasang manual satu kali versi pertama yang update-nya menyala.
- Alamat update: `https://github.com/oneshield7/codestamp/releases/latest/download/latest.json`. GitHub hanya menganggap rilis yang sudah dipublish (bukan draft, bukan pre-release) sebagai "latest".
- Dengan `bundle.createUpdaterArtifacts: true`, `tauri build` gagal bila kunci privat tidak ada. Workflow rilis karenanya wajib punya kedua secret. Workflow CI tidak menjalankan `tauri build`, jadi tidak terpengaruh.

### Pendekatan

|                 | Pendekatan                                                                                                               | Untung rugi                                                                                                                            |
| --------------- | ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| **A (dipilih)** | Workflow rilis yang ada ditambah langkah tanda tangan dan satu job yang menulis `latest.json` lewat `scripts/updates.ts` | Perubahan kecil. Nama target per platform bisa diatur penuh, dan langkah build `codestamp-cli` serta kompres ulang .deb tidak berubah. |
| B               | Ganti ke `tauri-action`, yang membangun dan menulis `latest.json` sendiri                                                | Script lebih sedikit, tapi seluruh workflow ditulis ulang dan nama target kurang bisa diatur.                                          |
| Dicoret         | .deb memasang update sendiri lewat `pkexec dpkg -i`                                                                      | Menduplikasi APT. Selain itu .deb dikompres ulang setelah build, jadi perlu langkah tanda tangan terpisah.                             |

Pemilik memilih A.

### Desain

**Konfigurasi.** Pemilik menjalankan `npm run updates:setup -- <alamat update>` (sudah ada). Perintah itu membuat kunci di `~/.codestamp/updater.key` bila belum ada, lalu menulis pubkey, alamat, dan `createUpdaterArtifacts: true` ke `tauri.conf.json`. Perubahan itu di-commit.

**Workflow rilis (`.github/workflows/release.yml`).**

- Langkah build Windows (bertanda tangan dan tanpa tanda tangan) dan Linux mendapat env `TAURI_SIGNING_PRIVATE_KEY` dan `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` dari secret.
- Kedua job mengunggah file `.sig` sebagai artifact workflow: `*-setup.exe.sig` dari Windows, `*.AppImage.sig` dari Linux.
- Job baru (`needs: [windows, linux]`) mengunduh kedua `.sig`, menjalankan `scripts/updates.ts` untuk menulis `latest.json`, lalu melampirkannya ke draft release yang sama.

**Isi `latest.json`.**

- `version` diambil dari `tauri.conf.json`. Script gagal bila tidak sama dengan tag tanpa awalan `v`.
- `notes` diambil dari bagian `## [X.Y.Z]` di `CHANGELOG.md` sebagai teks biasa. Bila bagian itu tidak ada, `notes` dikosongkan. Bila lebih dari 4000 byte, script gagal, sama seperti sekarang.
- Setiap URL dikunci ke tag: `https://github.com/oneshield7/codestamp/releases/download/vX.Y.Z/<file>`. Dengan begitu, rilis yang lebih baru tidak bisa menggeser file di tengah unduhan dan membuat tanda tangannya tidak cocok.
- Target platform:
  - `windows-x86_64` dan `windows-x86_64-nsis` menunjuk ke installer.
  - `linux-x86_64-appimage` menunjuk ke AppImage.
  - Tidak ada `linux-x86_64` polos. Updater mencari `{os}-{arch}-{bundle}` lalu `{os}-{arch}`; tanpa kunci polos itu, build .deb tidak akan jatuh ke AppImage.
- Mode `manifest` lama (server sendiri, membaca folder bundle lokal) diganti mode untuk rilis GitHub ini. Mode `setup` tetap.

**Aplikasi (Rust, `src-tauri/src/update/`).**

- Pada Linux, bila `tauri::utils::platform::bundle_type()` adalah `Deb`, `check_for_update` membangun updater dengan `.target("linux-x86_64-appimage")`. Updater itu hanya dipakai untuk membaca versi dan catatan rilis.
- Hasil `Available` membawa cara update-nya: lewat aplikasi atau lewat APT.
- Untuk APT, update yang ditemukan tidak disimpan sebagai pending, sehingga `install_update` menolak.
- Logika memilih cara update dibuat fungsi murni supaya bisa diuji.

**Aplikasi (UI).** Untuk cara APT, banner dan Settings › About menulis versi baru, lalu menampilkan perintah `sudo apt update && sudo apt upgrade` dengan tombol untuk menyalin perintah itu. Tidak ada tombol _Update and restart_. Tombol _What's new_ dan _Later_ tetap.

**Dokumentasi.**

- `README.md`: cara update per jenis pemasangan.
- `README.id.md`: bagian "Pembaruan otomatis" ditulis ulang untuk alur GitHub: setup kunci, secret, dan langkah rilis.
- `CHANGELOG.md`: di `[Unreleased]`, catat bahwa pengguna 0.5.0 perlu memasang manual sekali.

## 2. Wrap output respons

### Tujuan dan alasan

Output yang panjang, misalnya bearer token, harus bisa dibaca dan di-copy tanpa scroll ke kanan. Setting _Wrap long lines_ sudah ada di Settings › Editor, tapi defaultnya mati dan berlaku untuk semua editor sekaligus.

- **Goals**
  - Tampilan respons wrap secara default.
  - Satu tombol di panel untuk mematikannya, dan pilihan itu diingat.
  - Teks yang di-copy tetap teks asli, tanpa enter tambahan.
- **Non-goals**: mengubah editor yang bisa diketik; shortcut keyboard.

### Pendekatan

|                 | Pendekatan                                                                                                        | Untung rugi                                                                             |
| --------------- | ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| **A (dipilih)** | Tombol _Wrap_ di bar setiap tampilan respons, dengan satu setting bersama yang diingat dan menyala secara default | Mudah ditemukan, dan tidak mengubah editor request.                                     |
| B               | Ubah default _Wrap long lines_ menjadi menyala                                                                    | Satu baris kode, tapi editor body dan script ikut ter-wrap, dan tidak ada tombol cepat. |
| C               | Respons selalu wrap, tanpa tombol                                                                                 | Paling sederhana, tapi tidak bisa dimatikan untuk output lebar seperti tabel teks.      |

Pemilik memilih A.

### Desain

- Preference baru `editor.wrapResponses`, default `true`, di `EditorPreferences` Rust (`src-tauri/src/preferences/models.rs`), tipe TS, dan `DEFAULT_PREFERENCES`. File preferences lama yang tidak punya field ini dibaca sebagai `true`. Disimpan lewat mekanisme preferences yang sudah ada.
- `CodeEditor` mendapat prop opsional `wrap`. Bila diisi, nilainya menggantikan `wrapLines` untuk editor itu.
- Tombol _Wrap_ berupa toggle (`aria-pressed`), satu komponen kecil yang dipakai di tiga tempat:
  - body di `ResponsePanel` (dipakai HTTP, GraphQL, SOAP), hanya saat tampilan kode, bukan tampilan halaman HTML;
  - satu pesan di `GrpcReplyPanel`;
  - body yang dibuka di `ExamplesPanel`.
- Bagian yang sudah wrap tetap seperti sekarang: header, cookie, event stream, console, dan daftar beberapa pesan gRPC.
- Label setting lama menjadi "Wrap long lines while editing", supaya jelas cakupannya.

## Diserahkan ke builder

Nama fungsi dan field, teks persis (mengikuti gaya teks UI yang ada), letak tombol di dalam bar, dan susunan commit.

## Verifikasi

- Unit test Rust:
  - pemilihan cara update (.deb, AppImage, NSIS, tanpa bundle);
  - default `wrapResponses` dan pembacaan file preferences lama.
- Script manifest dijalankan lokal dengan file `.sig` palsu, untuk memeriksa isi `latest.json`, kecocokan versi dengan tag, dan pengambilan catatan dari CHANGELOG.
- `npm run check`, `npm run lint`, dan Rust lint serta test.
- Workflow rilis baru benar-benar teruji pada push tag pertama setelah secret diisi.
