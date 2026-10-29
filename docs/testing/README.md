# Pengujian Codestamp

Codestamp diuji dari dua sisi. Pemeriksaan statis dan tes Rust berjalan tanpa jendela. Suite end-to-end (e2e) menjalankan build release yang sebenarnya, lalu mengeklik dan mengetik di jendela aplikasi lewat WebView2 DevTools Protocol (CDP).

Folder ini menjelaskan alur setiap suite e2e: apa yang disiapkan, langkah yang dijalankan, dan cek yang harus lolos di setiap langkah. Setiap stage punya satu halaman di [`stages/`](stages/). Skrip suite dan helper-nya ada di folder [`e2e/`](../../e2e/).

## Lapisan pengujian

| Lapisan         | Perintah            | Isi                                                                                       |
| --------------- | ------------------- | ----------------------------------------------------------------------------------------- |
| Tipe dan Svelte | `npm run check`     | `svelte-check` (gagal bila ada warning) dan `tsc` untuk file Node                         |
| Lint frontend   | `npm run lint`      | ESLint tanpa warning dan Prettier                                                         |
| Lint Rust       | `npm run lint:rust` | `cargo fmt --check` dan `cargo clippy -D warnings`                                        |
| Tes Rust        | `npm run test:rust` | 308 tes unit dan integrasi di inti Rust (`src-tauri/src`)                                 |
| Semua di atas   | `npm run verify`    | Menjalankan keempatnya berurutan                                                          |
| End-to-end      | `e2e/*-e2e.mjs`     | 34 suite, 727 cek terhadap build release dan kode Copy as code, dijelaskan di halaman ini |

Tes Rust mencakup validasi request, import dan export (Postman, OpenAPI, WSDL, file Codestamp), sinkron Git dan penggabungan per field, grup akses dan enkripsinya, History, Recently deleted, dan migrasi database. Tes ini memakai database di memori, jadi tidak menyentuh data aplikasi.

## Harness

Suite e2e hanya berjalan di Windows, karena memakai WebView2 dan beberapa helper PowerShell. Semua file yang dibuat saat tes berjalan (build uji, profil WebView2, bare repository Git, screenshot, dan fixture updater) disimpan di `e2e/.work/`, yang tidak ikut di-commit.

1. **Build salinan uji.** `npx tauri build --no-bundle` dengan `CARGO_TARGET_DIR` menunjuk ke `e2e/.work/target` (path absolut, karena Tauri menjalankan cargo dari `src-tauri`), supaya build uji tidak bentrok dengan Codestamp yang sedang dipakai dari `src-tauri/target`.
2. **`launch.mjs` menjalankan salinan uji.** Exe dijalankan dengan `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` dan `WEBVIEW2_USER_DATA_FOLDER` miliknya sendiri. Salinan uji lama dihentikan lebih dulu, dan yang dikenali hanya proses dari path build uji, bukan Codestamp milik pengguna. `launch.mjs` menunggu sampai daftar collection termuat dan layar pembuka hilang.
3. **Git tanpa host sungguhan.** Lewat variabel `GIT_CONFIG_COUNT`, salinan uji memetakan `https://git.test/` ke bare repository di `e2e/.work/git-hosts/`. Konfigurasi Git pengguna tidak diubah. `E2E_GIT_CONFIG` menambah pasangan konfigurasi untuk satu suite (stage35 memakainya untuk mematikan credential helper).
4. **Suite terhubung lewat CDP.** Suite mengambil halaman dari `http://127.0.0.1:9333/json`, menjalankan JavaScript dengan `Runtime.evaluate`, lalu memasang helper kecil seperti `$btn`, `$nav`, dan `$row` untuk mencari elemen. Data awal dibuat lewat perintah Tauri (`window.__TAURI_INTERNALS__.invoke`), sedangkan langkah yang diuji dijalankan dengan klik, ketikan, dan event keyboard di UI yang asli.
5. **Hasil.** Setiap cek mencetak satu baris `PASS` atau `FAIL` beserta detail yang diamati, lalu suite menutup dengan `N/M passed`. Bila suite berhenti di tengah, baris yang sudah terkumpul tetap dicetak sebelum error-nya.

Helper yang dipakai bersama:

| File                                                       | Fungsi                                                                                      |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `launch.mjs`                                               | Menjalankan salinan uji dengan CDP dan menunggu sampai siap                                 |
| `eval.mjs`                                                 | Menjalankan satu ekspresi di halaman aplikasi dan mencetak hasilnya                         |
| `run-list.sh`                                              | Menjalankan beberapa suite di satu ukuran jendela, setelah membersihkan sisa data uji       |
| `run-sweep.sh`                                             | Menjalankan semua suite (kecuali stage25, stage34, dan CLI) di satu ukuran jendela          |
| `test-process.ps1`                                         | Menemukan salinan uji dari path exe-nya                                                     |
| `answer-dialog.ps1`                                        | Mengisi dan mengonfirmasi dialog buka atau simpan file Windows lewat pesan jendela          |
| `resize-window.ps1`                                        | Mengubah ukuran jendela uji ke 720×520 atau maximize                                        |
| `window-rect.ps1`                                          | Membaca posisi jendela dan layar (stage39)                                                  |
| `test-window.ps1`                                          | Menemukan jendela salinan uji; dipakai helper jendela lain                                  |
| `hit-test.ps1`, `restore-window.ps1`, `desktop-locked.ps1` | Tepi resize, mengembalikan ukuran jendela, dan cek desktop terkunci (stage25)               |
| `activate-window.ps1`, `menu-keys.ps1`                     | Membuka menu native lewat keyboard (stage25)                                                |
| `mcp-server.mjs`                                           | Server MCP uji lewat STDIO dan Streamable HTTP (stage13)                                    |
| `grpc-shop-server.mjs`, `protos/`                          | Server gRPC uji dengan reflection dan file `.proto`-nya (stage22)                           |
| `soap-server.mjs`                                          | Server SOAP dan WSDL uji (stage24)                                                          |
| `access-mate.mjs`                                          | Rekan kerja tiruan untuk grup akses, ditulis hanya dari format file (stage37)               |
| `updater/prepare.mjs`                                      | Membuat kunci uji, konfigurasi build updater, dan installer palsu bertanda tangan (stage34) |
| `updater/fake-installer.rs`                                | Installer palsu yang hanya mencatat argumennya (stage34)                                    |

Server lain yang dibutuhkan, misalnya server echo HTTP, server OAuth, dan server GraphQL, dijalankan oleh suite masing-masing di komputer yang sama.

## Data uji

- Setiap suite membuat data uji sendiri yang namanya diawali "E2E", lalu menghapusnya di blok `finally`, juga saat suite gagal di tengah. `run-list.sh` menghapus sisa collection "E2E" dari run yang terhenti.
- Kunci tanda tangan stage34 dibuat ulang oleh `updater/prepare.mjs` setiap kali dijalankan dan hanya ada di `e2e/.work/`. Jangan pakai kunci rilis untuk tes.
- Sebelum menjalankan e2e, cadangkan `codestamp.db` beserta file `-wal` dan `-shm` dari folder data aplikasi (`%APPDATA%\dev.codestamp.desktop`). Setelah selesai, kembalikan cadangan itu dan cocokkan hash-nya. Suite mengubah tema dan beberapa pengaturan selama berjalan.
- Beberapa suite menyentuh sistem operasi. stage37 membuat kunci perangkat di Windows Credential Manager akun yang menjalankan tes, dan suite yang memakai `answer-dialog.ps1` membuka dialog file Windows yang sebenarnya.

## Menjalankan

Kebutuhan: Windows 10 atau 11 dengan WebView2, Node.js 24 atau lebih baru, Rust, Git for Windows, dan Git Bash untuk skrip `.sh`. Semua perintah di bawah dijalankan dari folder project.

Pasang dependency server uji (gRPC dan GraphQL) sekali:

```bash
npm install --prefix e2e
```

Build salinan uji:

```bash
CARGO_TARGET_DIR="$PWD/e2e/.work/target" npx tauri build --no-bundle
```

Jalankan salinan uji dan satu suite:

```bash
node e2e/launch.mjs
```

```bash
node e2e/stage40-e2e.mjs
```

Jalankan beberapa suite, atau semuanya, di satu ukuran jendela:

```bash
bash e2e/run-list.sh default stage10 stage12 stage40
```

```bash
bash e2e/run-sweep.sh minimum
```

CLI e2e memakai `codestamp-cli` hasil build debug:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin codestamp-cli
```

```bash
node e2e/cli-e2e.mjs
```

Copy as code tidak butuh aplikasinya. Suite ini mem-bundle generator lalu menjalankan kode hasilnya dengan toolchain yang terpasang. Bahasa yang toolchain-nya tidak ada ditandai SKIP:

```bash
node e2e/codegen-e2e.mjs
```

stage34 memakai build updater khusus. Siapkan fixture-nya, build, lalu jalankan suite-nya. Build biasa perlu dibuat ulang sesudahnya untuk suite lain:

```bash
node e2e/updater/prepare.mjs
```

```bash
CARGO_TARGET_DIR="$PWD/e2e/.work/target" npx tauri build --no-bundle --config e2e/.work/updater/test.conf.json
```

```bash
node e2e/stage34-e2e.mjs
```

- `run-list.sh <minimum|maximize|default> <suite…>` menjalankan suite pilihan di satu ukuran jendela, dimulai dengan salinan uji baru, tema Light, dan tanpa sisa collection "E2E".
- `run-sweep.sh <minimum|maximize>` menjalankan semua suite di satu ukuran.
- stage25 dijalankan sendirian karena menutup jendela di akhir, dan menu native-nya hanya diuji dengan `CODESTAMP_MENU_TEST=1` saat desktop tidak terkunci.
- stage34 membutuhkan build updater khusus seperti di atas (lihat juga [halamannya](stages/stage-34.md)).

## Ukuran jendela

- **Bawaan**: jendela pertama kali dibuka, dengan viewport sekitar 1280×820.
- **minimum**: 720×520, ukuran jendela terkecil yang didukung. Di bawah 900px sidebar menjadi rel ikon.
- **maximize**: resolusi monitor.

Beberapa cek hanya berjalan di ukuran tertentu. Contohnya, cek perataan tombol "+" di sidebar dilewati saat sidebar dipaksa menjadi rel ikon, dan stage36 memeriksa tata letak berdampingan hanya bila jendelanya cukup lebar.

## Cara membaca halaman stage

Setiap halaman berisi ringkasan, kebutuhan suite itu, lalu alurnya. Bagian di alur mengikuti urutan bagian di dalam skrip suite. Nama cek ditulis persis seperti yang dicetak suite (dalam bahasa Inggris), supaya mudah dicocokkan dengan output. `[tema]` berarti cek itu diulang untuk Light, Dark, dan Dark Night. Angka cek di setiap halaman menghitung cek yang ditulis di suite; cek di dalam perulangan dihitung sekali walaupun berjalan beberapa kali.

## Menambah stage

1. Salin struktur suite terbaru: koneksi CDP, helper, `check()`, dan blok `finally` yang membersihkan data.
2. Beri nama data uji dengan awalan "E2E", dan buat data awal lewat perintah Tauri, bukan lewat UI yang tidak sedang diuji.
3. Simpan file yang dibuat suite di `e2e/.work/` atau di folder sementara sistem, bukan di folder `e2e/` itu sendiri.
4. Panel collection (Variables, Auth, Collaboration, Mock server, History, Export) terbuka sebagai dialog `.panel-dialog` dan tetap terbuka di belakang dialog konfirmasi. Cari dialog lain dengan `dialog[open]:not(.panel-dialog)`, dan cari isi panel lewat `section[aria-label="…"]`. Dropdown di dalam dialog baru membuka daftarnya setelah dialog selesai muncul, jadi tunggu sampai pilihannya ada, jangan memakai jeda tetap.
5. Tambahkan suite ke daftar di `run-sweep.sh`, lalu tulis halamannya di folder ini.

## Daftar stage

| Suite                                        | Yang diuji                                                                | Cek |
| -------------------------------------------- | ------------------------------------------------------------------------- | --- |
| [stage10-e2e.mjs](stages/stage-10.md)        | Body form, cookie jar, variabel dinamis, dan Copy as code                 | 37  |
| [stage11-e2e.mjs](stages/stage-11.md)        | Collaboration lewat Git                                                   | 30  |
| [stage12-e2e.mjs](stages/stage-12.md)        | Tampilan dasar, Home, dan dialog New collection                           | 41  |
| [stage13-e2e.mjs](stages/stage-13.md)        | Tema, Settings, variabel berwarna, dan klien MCP                          | 28  |
| [stage14-e2e.mjs](stages/stage-14.md)        | Dropdown dan ikon berwarna                                                | 11  |
| [stage15-e2e.mjs](stages/stage-15.md)        | Wording, Quick search, dan simpan body response                           | 24  |
| [stage16-e2e.mjs](stages/stage-16.md)        | Path variable dan run berbasis data                                       | 15  |
| [stage17-e2e.mjs](stages/stage-17.md)        | Auth: Digest, AWS SigV4, dan OAuth 2.0                                    | 15  |
| [stage18-e2e.mjs](stages/stage-18.md)        | Import OpenAPI dan Swagger                                                | 9   |
| [stage19-e2e.mjs](stages/stage-19.md)        | Request GraphQL                                                           | 10  |
| [stage20-e2e.mjs](stages/stage-20.md)        | Example dan mock server                                                   | 14  |
| [stage21-e2e.mjs](stages/stage-21.md)        | Server-Sent Events                                                        | 7   |
| [stage22-e2e.mjs](stages/stage-22.md)        | Request gRPC                                                              | 18  |
| [stage23-e2e.mjs](stages/stage-23.md)        | Orchestration                                                             | 16  |
| [stage24-e2e.mjs](stages/stage-24.md)        | Body XML dan HTML, SOAP, dan import WSDL                                  | 20  |
| [stage25-e2e.mjs](stages/stage-25.md)        | Title bar dan sidebar yang diciutkan                                      | 16  |
| [stage26-e2e.mjs](stages/stage-26.md)        | Grup Tools dan Catalog                                                    | 30  |
| [stage27-e2e.mjs](stages/stage-27.md)        | Indentasi None, SSL/TLS, dan Docs                                         | 27  |
| [stage28-e2e.mjs](stages/stage-28.md)        | Membuka request dan area scroll                                           | 17  |
| [stage29-e2e.mjs](stages/stage-29.md)        | Find di editor, nama baru, dan Docs yang ringkas                          | 23  |
| [stage30-e2e.mjs](stages/stage-30.md)        | Akun Git per collection                                                   | 26  |
| [stage31-e2e.mjs](stages/stage-31.md)        | Git accounts, Git changes, dan replace collection                         | 20  |
| [stage32-e2e.mjs](stages/stage-32.md)        | Share ke repository yang sudah berisi file                                | 13  |
| [stage33-e2e.mjs](stages/stage-33.md)        | Status bar, Ctrl+S, sync manual, dan environment bersama                  | 16  |
| [stage34-e2e.mjs](stages/stage-34.md)        | Pembaruan dalam aplikasi                                                  | 21  |
| [stage35-e2e.mjs](stages/stage-35.md)        | Perbaikan Git: clone rusak dan sign-in hilang                             | 9   |
| [stage36-e2e.mjs](stages/stage-36.md)        | Git changes dengan riwayat panjang                                        | 14  |
| [stage37-e2e.mjs](stages/stage-37.md)        | Grup akses                                                                | 28  |
| [stage38-e2e.mjs](stages/stage-38.md)        | Layar pembuka                                                             | 9   |
| [stage39-e2e.mjs](stages/stage-39.md)        | Posisi jendela, environment aktif, dan lipatan                            | 14  |
| [stage40-e2e.mjs](stages/stage-40.md)        | Quick search, shortcut, status bar, Export, History, dan Recently deleted | 40  |
| [collections-e2e.mjs](stages/collections.md) | Alur collection dasar                                                     | 8   |
| [cli-e2e.mjs](stages/cli.md)                 | codestamp-cli                                                             | 11  |
| [codegen-e2e.mjs](stages/codegen.md)         | Kode hasil Copy as code dijalankan di 15 bahasa                           | 90  |

Total: 34 suite dan 727 cek.
