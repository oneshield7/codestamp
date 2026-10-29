<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="branding/banner-dark.svg">
    <img alt="Codestamp: desktop API client. No account, no cloud." src="branding/banner-light.svg" width="100%">
  </picture>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-007ec6"></a>
  <a href="../../releases"><img alt="Release: v0.3.6" src="https://img.shields.io/badge/release-v0.3.6-007ec6"></a>
  <img alt="Platform: Windows 10 and 11" src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078d6">
</p>

<p align="center"><a href="README.md">English</a> · <strong>Bahasa Indonesia</strong></p>

API client desktop untuk HTTP, GraphQL, WebSocket, Server-Sent Events, gRPC, SOAP, dan MCP. Dibangun dengan **Tauri 2 + Rust** untuk inti jaringan dan **Svelte 5 + TypeScript** untuk UI. Aplikasi ini memakai webview bawaan Windows (WebView2), bukan browser yang dibundel, jadi installernya sekitar 6,5 MB.

## Yang Harus Anda Tahu

<table>
  <tr>
    <td width="50%" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="branding/why/local-dark.svg">
        <img alt="Komputer berisi codestamp.db, dengan kunci enkripsi di Windows Credential Manager" src="branding/why/local-light.svg" width="380">
      </picture>
      <h3>Collection tetap di komputer Anda</h3>
      <p>Tidak ada akun dan tidak ada cloud. Request tersimpan di database SQLite lokal, dan Codestamp mengenkripsi nilai secret dengan kunci yang disimpan Windows di Credential Manager.</p>
    </td>
    <td width="50%" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="branding/why/git-dark.svg">
        <img alt="Dua perubahan pada field berbeda dari satu request, digabung tanpa konflik" src="branding/why/git-light.svg" width="380">
      </picture>
      <h3>Tim berbagi lewat Git</h3>
      <p>Setiap request menjadi satu file di repository yang sudah dimiliki tim. Codestamp menggabungkan perubahan per field, jadi dua orang yang mengubah bagian berbeda dari satu request tidak mendapat konflik. Login tetap diurus credential manager Git.</p>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="branding/why/secrets-dark.svg">
        <img alt="Token tersamar yang tidak pernah ditulis oleh history, export, Copy as code, maupun grup akses" src="branding/why/secrets-light.svg" width="380">
      </picture>
      <h3>Secret tidak ikut dibagikan</h3>
      <p>History menyimpan <code>{{token}}</code>, bukan nilainya, export mengosongkan secret, dan Copy as code membacanya dari environment variable. Nilai yang hanya boleh dilihat sebagian tim masuk ke grup akses, dan repository hanya menyimpan ciphertext-nya.</p>
    </td>
    <td width="50%" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="branding/why/kinds-dark.svg">
        <img alt="Satu collection berisi request POST, SOAP, gRPC, dan MCP sebagai langkah bernomor" src="branding/why/kinds-light.svg" width="380">
      </picture>
      <h3>Berbagai jenis request dalam satu collection</h3>
      <p>Request HTTP, GraphQL, WebSocket, Server-Sent Events, gRPC, SOAP, dan MCP berada di notebook yang sama sebagai langkah bernomor.</p>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="branding/why/postman-dark.svg">
        <img alt="Collection Postman diimpor ke Codestamp lalu diekspor kembali" src="branding/why/postman-light.svg" width="380">
      </picture>
      <h3>Bawa collection Postman Anda</h3>
      <p>Codestamp mengimpor file Postman Collection v2.0 dan v2.1 beserta environment-nya, script memakai API <code>pm.*</code> bergaya Postman, dan Anda bisa mengekspor kembali ke Postman Collection v2.1.</p>
    </td>
    <td width="50%" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="branding/why/cli-dark.svg">
        <img alt="Output codestamp-cli dengan langkah yang lolos dan gagal serta laporan JUnit" src="branding/why/cli-light.svg" width="380">
      </picture>
      <h3>Collection yang sama berjalan di CI</h3>
      <p><code>codestamp-cli</code> menjalankan collection beserta script dan tesnya, menulis laporan JUnit atau JSON, dan keluar dengan kode 1 jika ada tes yang gagal.</p>
    </td>
  </tr>
</table>

Kodenya terbuka dengan lisensi Apache-2.0, jadi Anda bisa membaca cara Codestamp menangani secret sebelum mempercayakan secret Anda kepadanya.

## Status

Tahap 1 (irisan pertama MVP):

- Kartu request bergaya _Notebook_: method, URL, query params, headers, body (None / JSON / Text) dengan editor CodeMirror.
- Kirim request lewat inti Rust (`reqwest` + rustls, HTTP/1.1 & HTTP/2, gzip/brotli/deflate).
- Panel response di dalam kartu (atas-bawah atau kiri-kanan), JSON otomatis dirapikan, tab Headers.
- Response besar hanya disimpan **2 MB pertama** di memori; sisanya dihitung ukurannya tapi tidak disimpan.
- Font default Google Sans Flex (UI) dan Google Sans Code (editor), dibundel offline.

Tahap 2:

- Collection, section, dan request disimpan di SQLite (`%APPDATA%\dev.codestamp.desktop\codestamp.db`).
- Collection tampil sebagai notebook: request di luar section di atas, lalu tiap section; nomor langkah berurutan.
- Hanya satu kartu request yang terbuka (dan memuat isi lengkap + editor); kartu lain hanya ringkasan.
- Autosave 600 ms setelah berhenti mengetik, `Ctrl+S` untuk simpan langsung.
- Menghapus section tidak menghapus request di dalamnya; menghapus collection menghapus semua isinya (dengan konfirmasi).

Tahap 3:

- Environment (Globals + environment bernama) dengan satu environment aktif, dipilih dari header collection.
- Variabel `{{nama}}` di URL, query params, headers, dan body diisi oleh inti Rust saat request dikirim. Environment aktif menimpa Globals; variabel yang tidak dikenal ditolak dengan pesan jelas.
- Variabel **Secret** dienkripsi ChaCha20-Poly1305 di SQLite (terikat ke ID variabelnya); kuncinya disimpan di Windows Credential Manager. Secret tidak pernah dikirim ke UI kecuali saat pengguna menekan "Show value".

Tahap 4:

- **History**: setiap request yang benar-benar terkirim (termasuk yang gagal koneksi) dicatat 30 hari / maksimal 1.000 entri. Yang disimpan adalah request seperti yang diketik (`{{variabel}}` tetap utuh), jadi secret tidak pernah masuk history. Bisa dicari, difilter per status (2xx/3xx/4xx/5xx/Failed), dibuka kembali, atau disimpan ke collection.
- **Variabel tingkat collection** (tombol _Variables_ di header collection). Urutan prioritas: Globals → Collection → Environment aktif.
- **Autocomplete `{{`** di URL dan nilai params/headers, plus baris _Resolves to …_ di bawah URL (secret ditampilkan sebagai `••••••`).
- **Urut ulang**: drag request/section, `Alt+↑/↓` dari keyboard, atau pilih section di header kartu untuk memindahkan request.
- **Secret lintas platform**: Windows Credential Manager, macOS Keychain, Linux Secret Service.

Tahap 5:

- **Pre-request & test script** (tab _Scripts_ di kartu request) dengan API `pm.*` yang kompatibel dengan Postman untuk kebutuhan umum: `pm.environment` / `pm.collectionVariables` / `pm.globals` / `pm.variables`, `pm.request` (method, url, headers, body), `pm.response` (`code`, `json()`, `text()`, `to.have.status`, …), `pm.test`, `pm.expect` (subset Chai), dan `console.*`.
- Script berjalan di **sandbox QuickJS** yang dibuat baru setiap kali dijalankan lalu dibuang: tanpa akses file atau proses; batas memori 64 MB, stack 512 KB, dan waktu 2 detik. Satu-satunya akses jaringan adalah `pm.sendRequest` (tahap 9).
- Perubahan variabel dari script disimpan (secret tetap terenkripsi). Pre-request script yang error membatalkan pengiriman; test script yang error tetap menampilkan response.
- Hasil test dan output console tampil di tab _Tests_ dan _Console_ pada panel response.

Tahap 6:

- **Import Postman** (menu _Import_): collection v2.0/v2.1 dan file environment. Ada pratinjau (jumlah request, section, script, variabel) dan daftar hal yang perlu ditinjau sebelum benar-benar mengimpor.
  - Folder tingkat atas → section; folder bertingkat → nama `Folder / Sub / Request`.
  - Auth Bearer / API key → header atau param; Basic → header, atau pre-request script (`btoa` + `pm.variables.replaceIn`) bila memakai variabel. Auth yang diwarisi dari collection/folder ikut diterapkan.
  - Script collection/folder digabung di depan script request, sesuai urutan eksekusi Postman.
  - Body raw/JSON, urlencoded, form-data, file, dan GraphQL dikonversi. Path file upload tidak ikut diimpor (dilaporkan sebagai peringatan), begitu pula variabel dinamis yang tidak didukung.
  - Variabel bertipe secret tetap menjadi secret terenkripsi.
- **Import cURL**: tempel perintah dari "Copy as cURL" (bash maupun cmd Windows), lihat hasil parse, lalu tambahkan ke collection/section mana pun. `-F` / `--form-string` menjadi body form data (file perlu dipilih ulang).

Tahap 7:

- **WebSocket client** (tombol _Add WebSocket_ di collection atau section; baris bertanda `WS`). URL `ws://` / `wss://`, query params, dan header handshake mendukung `{{variabel}}`; `Sec-WebSocket-Protocol` boleh diisi, header handshake lain (Host, Upgrade, Sec-WebSocket-Key, …) diatur otomatis.
- **Connect / Disconnect**, kirim pesan Text atau JSON (Beautify) dengan `Ctrl+Enter`; `{{variabel}}` di pesan diisi saat dikirim, tapi log tetap menampilkan teks seperti yang diketik sehingga secret tidak muncul.
- **Log pesan** (↑ terkirim, ↓ diterima, info koneksi, error) dengan jam, ukuran, filter All/Sent/Received, dan Clear. Log dibatasi 500 entri; pesan masuk maupun terkirim di atas 64 KB hanya ditampilkan 64 KB pertamanya, pesan biner ditampilkan ukurannya. Kode & alasan close dari server ikut ditampilkan.
- URL, header, dan pesan terakhir tersimpan di collection; koneksi otomatis ditutup (close 1000) saat kartu ditutup. Sesi WebSocket tidak dicatat ke history.
- Batas: handshake 15 detik, pesan maksimal 16 MB, 20 koneksi bersamaan. `wss://` memakai rustls dengan verifikasi sertifikat dari certificate store sistem operasi.

Tahap 8:

- **Settings › Appearance & fonts** (menu _Settings_ di sidebar), perubahan langsung terlihat di pratinjau:
  - Font antarmuka: Google Sans Flex (default) atau Segoe UI; ukuran 11-20 px. Ukuran ini memperbesar/memperkecil seluruh jendela (zoom WebView), jadi jarak dan tombol ikut menyesuaikan.
  - Font editor: Google Sans Code (default), Cascadia Code, atau Consolas; ukuran 10-24 px, line height 1.2-2.2, ligature on/off.
  - **Font sendiri**: import file .ttf / .otf / .woff2 (maks. 10 MB, maks. 20 font) lewat dialog file sistem. Format dicek dari isi file, bukan dari ekstensinya. File disimpan di folder data aplikasi dan hanya font yang sedang dipakai yang dimuat ke memori.
  - Nilai di luar rentang ditolak dengan pesan (tidak dibetulkan diam-diam) dan divalidasi ulang di Rust.
- **Mode fokus** (ikon ⤢ di kartu): sidebar dan kartu lain disembunyikan, kartu tampil besar dengan request dan response bersebelahan. Opsi _Hide secrets on screen_ (aktif secara default) menyamarkan nilai Authorization, Cookie/Set-Cookie, token, api_key, password, dsb. di tabel, response, dan log WebSocket. `Esc` untuk keluar; response dan koneksi WebSocket tetap utuh.
- **Capture** (ikon kamera di kartu, atau tombol _Capture_ di mode fokus): membuat PNG rapi dari request + response, atau URL + log pesan untuk WebSocket.
  - Bisa memilih isi: method & URL, header request, body request, status & waktu, header response, dan body (8/20/50 baris pertama atau semua).
  - _Hide secrets_ (aktif secara default) menyamarkan secret dan menampilkan daftar apa saja yang disamarkan. URL bisa ditampilkan dengan `{{variabel}}` atau versi yang sudah di-resolve (secret tetap tersamar).
  - Tema terang/gelap, tata letak bersebelahan/bertumpuk, serta tanggal & nama environment di bagian bawah (opsional).
  - **Copy image** (`Ctrl+Shift+C`) ke clipboard atau **Save PNG…** lewat dialog simpan sistem. Gambar dibuat di perangkat ini dengan canvas, tanpa library tambahan, dan tidak ada yang diunggah.

Tahap 9:

- **Tab Auth** di kartu request: _Inherit from collection_ (default), _No auth_, _Bearer token_ (prefix bisa diganti), _Basic auth_, dan _API key_ (di header atau query). Semua kolom mendukung `{{variabel}}`.
  - Auth tingkat collection diatur dari tombol _Auth_ di header collection dan dipakai oleh setiap request yang memilih _Inherit_.
  - Header/param auth dibuat oleh inti Rust tepat sebelum dikirim. Header atau param dengan nama yang sama yang diketik sendiri tetap menang.
  - History menyimpan auth seperti yang diketik (`{{variabel}}` utuh), jadi secret tidak masuk history.
- **`pm.sendRequest`** di pre-request maupun test script, dengan URL string atau objek `{ url, method, header, body }` (body raw atau urlencoded) dan callback `(err, response)`. Berjalan sinkron lewat HTTP client yang sama dan aturan validasi yang sama dengan kartu request. Batasnya 10 panggilan per script dan 10 detik per panggilan; waktu tunggu jaringan tidak memakan jatah 2 detik script.
- **Run all** (tombol di header collection): menjalankan semua request HTTP sesuai urutan notebook.
  - Lulus bila response diterima, script tidak error, tidak ada test yang gagal, dan (ada test, atau status < 400). Request WebSocket dilewati.
  - Opsi berhenti di kegagalan pertama dan jeda antar-request (0 / 250 / 1000 ms); bisa dihentikan kapan saja.
  - Per request: _Run only this_; hasil bisa diekspor sebagai laporan JSON (tanpa body) lewat dialog simpan sistem.
  - Request dari runner tidak dicatat ke history.
- **Perbaikan keamanan & ketahanan**:
  - Redirect hanya diikuti ke host+port yang sama atau upgrade `http` → `https` di host yang sama. Redirect ke host lain dikembalikan apa adanya (status 3xx), supaya header Authorization/Cookie tidak bocor.
  - Batas waktu script tetap aktif saat hasilnya dibaca. Perubahan variabel dari script dibatasi 200 per eksekusi, dan jumlah variabel per scope tetap dibatasi.
  - Kirim/close WebSocket punya batas waktu, dan semua koneksi ditutup saat jendela dimuat ulang.
  - SQLite memakai `secure_delete`, jadi data yang dihapus ditimpa nol di file.
  - Angka JSON besar (di atas 2⁵³) tidak lagi dibulatkan saat response dirapikan.
  - Perpindahan collection yang cepat tidak lagi bisa menampilkan atau menyimpan ke collection yang salah.
  - Run all tidak lagi menahan body response di memori (sebelumnya 2.000 request × 2 MB membuat UI membengkak ~4,6 GB lalu crash; kini memori kembali ke posisi awal setelah run).

Tahap 10:

- **Body baru** (tab _Body_): _URL-encoded_ dan _Form data_ memakai tabel baris yang sama (key, value, aktif/nonaktif, `{{variabel}}`), plus _File_ untuk mengirim satu file sebagai seluruh body.
  - Form data bisa berisi field teks dan file (`multipart/form-data`). File dipilih lewat dialog sistem dan di-_stream_ dari disk saat dikirim (maks. 100 MB total), jadi file besar tidak dimuat ke memori.
  - **Hanya file yang dipilih lewat dialog di perangkat ini yang boleh di-upload**: inti Rust menolak path lain, sehingga collection hasil impor (atau isi yang diubah) tidak bisa membuat Codestamp mengirim file lain dari disk. Import dan export tidak pernah membawa path lengkap.
  - Content-Type diisi otomatis. Untuk form data, header `multipart/form-data` yang diketik diganti dengan versi ber-boundary; Content-Type lain ditolak dengan pesan jelas. Nama field dengan baris baru ditolak (anti injeksi header di bagian multipart).
- **Cookie jar**: cookie dari response disimpan dan dikirim kembali ke host yang cocok (aturan domain, path, Secure, dan kedaluwarsa RFC 6265), termasuk dari `pm.sendRequest`. Header `Cookie` yang diketik sendiri tetap menang.
  - Tab _Cookies_ di panel response menampilkan cookie yang di-set response itu (tersamar di mode fokus).
  - Halaman **Cookies** di sidebar: daftar per domain, nilai tersembunyi sampai _Show_, hapus satu cookie, satu domain, atau semuanya.
  - Jar disimpan **terenkripsi** (ChaCha20-Poly1305, kunci yang sama dengan secret di Windows Credential Manager) dan bertahan setelah restart, termasuk cookie sesi. Maksimal 1.000 cookie.
- **Variabel dinamis** `{{$guid}}`, `{{$randomUUID}}`, `{{$timestamp}}`, `{{$isoTimestamp}}`, `{{$randomInt}}` (0-1000), `{{$randomBoolean}}`, `{{$randomAlphaNumeric}}`. Nilainya dibuat baru di setiap referensi saat dikirim, dengan sumber acak yang aman dari sistem operasi.
  - Muncul di autocomplete (`{{$`) dengan penjelasan singkat. Baris _Resolves to_ membiarkannya apa adanya.
  - Juga berlaku di pesan WebSocket dan `pm.variables.replaceIn()`. Variabel dinamis yang tidak dikenal (mis. `{{$randomFirstName}}`) ditolak dengan pesan jelas.
- **Duplikat**:
  - Request: tombol ikon di kartu; salinannya diletakkan tepat di bawah aslinya dan langsung dibuka.
  - Section: tombol _Duplicate_ di kepala section; salinan berisi semua request di dalamnya.
  - Collection: tombol _Duplicate_ di header; menyalin auth, section, request, dan variabel collection. Variabel secret dienkripsi ulang untuk salinannya.
- **Copy as code** (ikon `</>` di kartu), 15 pilihan: cURL, HTTPie, PowerShell, JavaScript `fetch` dan axios, Python `requests`, PHP Guzzle, Go `net/http`, Java `HttpClient`, Kotlin OkHttp, C# `HttpClient`, Dart `http`, Swift `URLSession`, Ruby `Net::HTTP`, dan Rust `reqwest` (Tahap 36 dan 37), termasuk query, header, body JSON/teks, form URL-encoded, multipart dengan file, file biner, dan auth.
  - _Values_: **Environment variables** (bawaan; setiap `{{nama}}` dibaca dari environment variable `NAMA`), _As written_ (`{{variabel}}` apa adanya), atau _Filled in (shows secrets)_ (diisi inti Rust persis seperti saat dikirim). Pre-request script tidak dijalankan. Tombol _Copy_ menyalin ke clipboard.
- **Export ke format Postman**:
  - Collection: tombol _Export_ di header → Postman Collection v2.1. Section menjadi folder, auth/script/variabel ikut, dan request WebSocket dilewati.
  - Environment atau Globals: tombol _Export_ di halaman Environments → file environment/globals Postman.
  - Nilai secret dikosongkan dan file upload hanya disimpan namanya. Codestamp memberi catatan bila ada yang dilewati atau dikosongkan.
  - File hasil export bisa diimpor kembali ke Codestamp maupun Postman.

Tahap 11: **collection tim lewat Git** (gratis, tanpa server Codestamp):

- **Satu collection = satu repository Git** (GitHub, GitLab, Bitbucket, Azure DevOps, atau bare repository di folder bersama). Tombol _Collaboration_ (dulu _Team_) di header collection membuka panelnya.
  - _Share_: tempel alamat repository **kosong** (repository yang sudah berisi juga bisa, dengan konfirmasi; lihat Tahap 25), isi nama & email (disarankan dari `git config` perangkat ini), dan untuk alamat https bisa juga username akun Git (lihat Tahap 24). Codestamp menulis collection lalu push.
  - _Join_ (halaman Import › _Join a shared collection_): tempel alamat yang sama. Collection masuk dengan ID yang sama seperti di repository.
  - Mengundang anggota = menambahkan mereka ke repository di host Git. Akses read-only bisa menerima perubahan tetapi tidak bisa mengirim.
- **Format workspace satu file per request**: `codestamp.json` (versi format), `collection.json` (nama, auth, ikon emoji yang dipilih, section, urutan, variabel), `requests/<id>.json`, dan `environments/<id>.json` untuk environment milik collection (Tahap 26). File JSON rapi dengan akhir baris LF, jadi diff dan review di host Git mudah dibaca.
  - **Tetap di perangkat**: nilai variabel secret (hanya namanya yang dibagikan; tiap anggota mengisi sendiri), cookie, history, dan path file yang dipilih untuk upload (baris file tetap ada, path-nya kosong). Nilai yang dibatasi ke grup akses dikirim terenkripsi untuk anggota grup itu saja (Tahap 29).
- **Sinkron lewat Git** yang terpasang di sistem:
  - Hanya saat tombol _Sync now_ ditekan (sejak Tahap 26). Perubahan yang tersimpan menunggu di perangkat ini, dan header menampilkan _not synced_ sampai dikirim.
  - Satu siklus: perubahan lokal di-commit dengan pesan yang menjelaskan isinya (mis. `Update “Login”`) atas nama pengguna → fetch → gabungkan perubahan tim → push. Push yang ditolak karena tim lebih dulu push diulang otomatis setelah digabung.
  - Saat offline, commit lokal disimpan dan dikirim pada sinkron berikutnya.
  - Login sepenuhnya diurus Git (Git Credential Manager di Windows): **Codestamp tidak pernah melihat atau menyimpan password/token**. Alamat yang berisi akun atau token ditolak.
  - Bila login tersimpan sudah tidak ada, Git Credential Manager membuka jendela login-nya saat Sync now. Bila login itu ditutup atau ditolak, panel Collaboration menampilkan tombol **Sign in** (Tahap 28).
- **Status sinkron** di tombol header (_Collaboration · synced / not synced / offline / read-only / sign in / N to resolve_) dan di panel: waktu sinkron terakhir, siapa yang mengubah apa, dan jumlah perubahan yang menunggu dikirim.
- **Merge per field** (3-way, dengan versi dasar dari Git):
  - Perubahan di field berbeda pada request yang sama digabung otomatis (mis. A mengubah URL, B menambah header).
  - Baris variabel dan section digabung per kunci. Urutan request digabung tanpa kehilangan request baru. Edit menang atas hapus.
  - Hanya bila dua orang mengubah field yang sama, sinkron berhenti dan muncul **dialog konflik**: nilai milik saya vs milik rekan per field, lalu pilih _Keep mine_, _Take my teammate's_, atau _Keep both_ (versi saya menjadi salinan “… (my copy)”).
  - Kartu yang sedang terbuka dimuat ulang bila rekan mengubahnya. Bila ada edit yang belum tersimpan, edit itu dipertahankan dan ada pemberitahuan.
- **Riwayat per request** (ikon jam di kartu, khusus collection tim):
  - Daftar versi dari commit (pesan, penulis, waktu), perbandingan per field antara versi itu dan sekarang, dan _Restore this version_.
  - Restore tidak menulis ulang riwayat: isi lama dibagikan sebagai perubahan baru.
- _Stop syncing_ melepas tautan di perangkat ini saja; collection tetap ada dan repository tidak disentuh.
- Batasan: environment hanya dibagikan bila menjadi milik collection (Tahap 26); satu repository untuk satu collection; Git for Windows harus terpasang.

Tahap 12: **tampilan "Postmark"**, Home, collection baru, dan ikon emoji (panduan lengkap di [DESIGN.md](DESIGN.md)):

- **Arah visual Postmark**: antarmuka monokrom "tinta" di atas abu dingin; warna hanya untuk makna (status dan method HTTP).
  - Font **Geist** dan **Geist Mono** (dibundel lewat Fontsource, lisensi OFL) menggantikan Google Sans.
  - Method HTTP tampil sebagai "cap pos" berwarna di baris request dan di URL bar. Nomor langkah dan URL memakai font mono.
  - Fokus keyboard memakai satu warna biru khusus (`--focus`).
- **Home**: halaman awal saat belum ada collection, dan selalu bisa dibuka dari sidebar. Isinya:
  - tiga aksi cepat: buat collection, import collection atau cURL, join collection bersama;
  - kartu "What you can do" berisi fitur yang memang ada;
  - daftar shortcut keyboard;
  - penjelasan bahwa Codestamp tidak memakai akun (teamwork login lewat Git).
- **Dialog New collection** (tombol "+" di sidebar atau tile di Home), dengan pilihan:
  - _Empty collection_: nama, ikon, dan Base URL opsional yang langsung disimpan sebagai variabel collection `{{baseUrl}}`;
  - _Import a file_: pilih file, lalu pratinjau di halaman Import;
  - _Join a shared collection_: alamat repository Git.
- **Collection kosong**: panel "Add your first request" (HTTP, WebSocket, Paste cURL dengan collection sudah terpilih) dan kartu "Also ready" (variabel `{{baseUrl}}`, section, tim).
- **Ikon emoji collection**:
  - Dipilih otomatis dari nama (kata kunci seperti payment → 💳, shop → 🛒; selain itu pilihan tetap dari hash nama).
  - Bisa diganti lewat picker: saran, pencarian, Shuffle, dan Use automatic.
  - Pilihan disimpan di database (kolom `collections.icon`, migrasi skema 10; `NULL` = otomatis) dan ikut saat collection diduplikat.
  - Di collection tim, emoji yang dipilih ikut tersinkron lewat `collection.json` (kunci `icon`; tidak ditulis selama ikonnya otomatis). Ganti ikon di satu perangkat, dan anggota lain menerimanya pada sinkron berikutnya; kembali ke "Use automatic" juga ikut tersinkron. Bila dua orang mengganti ikon bersamaan, muncul konflik "Collection icon" di dialog konflik.
  - Karena itu format repository naik ke **2**. Repository format 1 tetap terbaca dan diperbarui otomatis saat sinkron berikutnya. Codestamp versi lama menolak format 2 dengan pesan "Update Codestamp", sehingga tidak diam-diam membuang ikon. Ikon dari repository diperiksa terhadap katalog yang sama.
  - Katalog emoji ada di Rust dan sekaligus menjadi whitelist; nilai lain ditolak.

Tahap 13: **tema, Settings, warna variabel, dan MCP**:

- **Tema**: Light, Dark, **Dark Night**, dan System (mengikuti Windows), dipilih di Settings › Appearance dengan pratinjau. Title bar Windows ikut gelap. Semua warna berasal dari token, dan kontras ketiga tema lolos WCAG.
- **Menu Settings** (dibandingkan dengan Settings Postman), dengan tujuh bagian:
  - _Appearance_: tema, font antarmuka dan editor, impor font.
  - _Editor_: indentasi (juga untuk Beautify), tutup kurung otomatis, bungkus baris, tata letak kartu baru, saran variabel.
  - _Requests_: timeout default, batas ukuran respons, ikuti redirect di situs yang sama, cookie jar on/off, header `Cache-Control: no-cache`, User-Agent.
  - _Proxy_: proxy sistem (seperti sebelumnya), tanpa proxy, atau proxy kustom HTTP/HTTPS/SOCKS5 dengan login dan daftar bypass.
  - _Certificates_: CA certificate tambahan, dan client certificate (mTLS) per host.
  - _Data & history_: batas riwayat (jumlah dan hari), hapus riwayat, backup ke satu file `.codestamp` dan restore.
  - _About & shortcuts_: versi, lokasi data, ukuran database, daftar shortcut.
  - Verifikasi sertifikat server tidak bisa dimatikan; untuk server internal, tambahkan CA-nya.
  - Pengaturan disimpan di database (migrasi skema 11) dan diterapkan ke klien HTTP tanpa restart; nilai yang tidak bisa dipakai ditolak sebelum disimpan.
- **Warna `{{variabel}}`**: referensi yang dikenal berwarna magenta, yang tidak terdefinisi merah dengan garis bawah bergelombang, di kolom URL, tabel, field auth, editor kode, dan baris request.
- **MCP client** (seperti MCP request di Postman):
  - Tambah lewat _Add MCP server_; transport **STDIO** (perintah seperti `npx -y …`, dijalankan tanpa shell) atau **Streamable HTTP**, dengan header atau environment variable. Konfigurasi `mcpServers` dari klien lain bisa ditempel.
  - Setelah terhubung: daftar tools, prompts, dan resources; pilih satu, isi argumen JSON (template dari skema, `{{variabel}}` diisi saat dijalankan), lalu Run tool / Get prompt / Read resource. Hasil tampil terbaca atau sebagai JSON, dan semua pesan JSON-RPC ada di tab Messages.
  - Request MCP tersimpan di collection beserta tool yang dipilih (migrasi skema 12), bisa diduplikat, dan ikut sinkron tim. Karena itu format repository naik ke **3**. Run all dan export Postman melewatinya dengan keterangan.

Tahap 14: **paket fitur lanjutan** (Oktober 2026):

- **Tulisan umum**: kata "Postman" dihapus dari UI (Import tetap membaca format Postman, dan Export menulis Collection v2.1), dan "Team" diganti **Collaboration** di seluruh aplikasi.
- **Quick search** dengan `Ctrl+Space` (atau tombol cari di sidebar): kotak pencarian yang muncul dengan animasi zoom ala Spotlight, mencari nama request, method, URL, collection, dan halaman Settings sekaligus. Saat mengetik path API, sisa path yang cocok tampil samar dan `Tab` melengkapinya. `Enter` membuka request, `Esc` menutup. Shortcut diabaikan saat dialog lain terbuka.
  - Catatan: di Windows dengan IME Tionghoa/Jepang, `Ctrl+Space` juga dipakai untuk mengganti metode input; gunakan tombol cari di sidebar bila bentrok.
- **Save body** di panel response: menyimpan body persis seperti yang diterima ke file (maks. 32 MB per response, total 64 MB disimpan di memori), dengan ekstensi sesuai jenisnya.
- **Path variable** `:nama` di path URL (mis. `/users/:id`): tabel _Path variables_ di tab Params mengikuti URL, nilainya mendukung `{{variabel}}` dan di-percent-encode saat dikirim. Nilai kosong ditolak dengan pesan jelas.
- **Data run**: Run all bisa memakai file **CSV** (baris pertama = nama kolom) atau **JSON** (array objek), maks. 5 MB dan 5.000 baris. Satu iterasi per baris; kolom menjadi `{{variabel}}` yang menimpa semua scope, plus `pm.iterationData`, `pm.info.iteration`, dan `pm.info.iterationCount`. Tanpa file data, jumlah iterasi bisa diatur 1-100.
- **Auth tambahan**:
  - **OAuth 2.0**: authorization code (dengan PKCE, login di jendela Codestamp sendiri), client credentials, dan password. Token disimpan terenkripsi per request (atau per collection bila diwarisi), di-refresh otomatis 30 detik sebelum kedaluwarsa, dan client credentials diperbarui sendiri. Endpoint wajib HTTPS (kecuali localhost).
  - **Digest** (RFC 7616, dijawab setelah tantangan 401) dan **AWS Signature v4** (ditandatangani tepat sebelum dikirim, setelah validasi).
- **Import OpenAPI 3 / Swagger 2** (JSON atau YAML) dari file atau URL: tag menjadi section, parameter path menjadi path variable, contoh body dan response ikut. YAML dibaca dengan batas alias (anti "billion laughs"); URL harus HTTPS (atau localhost).
- **Request GraphQL** (_Add GraphQL_): editor query dengan highlight dan **autocomplete dari schema endpoint** (introspection, tombol _Fetch schema_), variabel JSON, dikirim sebagai POST JSON. Ikut Run all, export, dan Copy as code.
- **Examples & mock server**:
  - _Save as example_ menyimpan response ke tab _Examples_ request (maks. 2 MB per body, 50 per request); bisa diedit dan ikut import/export serta sinkron Collaboration.
  - Tombol _Mock server_ di header collection menjalankan server HTTP di `127.0.0.1` (port pilihan 1024-65535 atau otomatis) yang menjawab dengan examples: dicocokkan per method dan path (`:id` dan `{{variabel}}` menjadi wildcard). Header `x-mock-response-name` / `x-mock-response-code` memilih example tertentu. Log permintaan tampil di panel. Hanya bisa diakses dari perangkat ini.
- **Server-Sent Events**: response `text/event-stream` tampil event demi event (nama, id, data JSON dirapikan) tanpa terkena timeout request, dan bisa dihentikan dengan _Stop_. Run all tidak menunggu stream.
- **Request gRPC** (_Add gRPC_): alamat `grpc://` (plaintext) atau `grpcs://` (TLS, CA dan client certificate dari Settings ikut dipakai).
  - Service dan method diambil lewat **server reflection** (v1, lalu v1alpha) atau dari **file `.proto`** yang dipilih lewat dialog (import dibaca dari folder yang sama; tipe bawaan Google sudah tersedia). Deskriptor server berbasis protobufjs yang tidak lengkap diperbaiki otomatis.
  - Keempat jenis method: unary, server stream, client stream, dan bidirectional. Memilih method mengisi template pesan JSON dengan semua field (array untuk client stream).
  - Metadata, auth (Bearer, Basic, API key di header, OAuth 2.0), dan `{{variabel}}` di alamat, metadata, dan pesan. Balasan menampilkan status gRPC (mis. `5 NOT_FOUND`), pesan, header, dan trailer secara terpisah.
  - Batas: maks. 200 pesan dari stream server, pesan maks. 32 MB, metadata biner (`-bin`) belum didukung, dan proxy tidak dipakai untuk gRPC. Run all dan export melewati gRPC dengan keterangan.
- **CLI runner untuk CI** (`codestamp-cli`): menjalankan collection tanpa jendela aplikasi; lihat bagian [CLI untuk CI](#cli-untuk-ci).
- **Data**: migrasi skema 13-16 (path variable, variabel GraphQL, tabel `request_examples`, pengaturan gRPC). Format repository Collaboration naik ke **4** (path variable, GraphQL, examples, gRPC); path file `.proto` tetap di perangkat seperti path upload.

Tahap 15: **Orchestration**, dulu bernama PostFlow (menjalankan API berurutan dari Start sampai End):

- Halaman **Orchestration** di sidebar (bagian Workspace, juga lewat Quick search). Satu flow berisi langkah-langkah di antara node **Start** dan **End** yang selalu ada, dengan **panah** di setiap sambungan. Sejak Tahap 35 langkah-langkahnya tampil sebagai peta dari kiri ke kanan.
  - Pemilih environment dan ringkasan run (jumlah lulus, gagal, dan dilewati, durasi, dan langkah tempat flow berhenti) ada di baris atas peta.
  - Setiap langkah memanggil request HTTP atau GraphQL yang sudah ada di collection mana pun. Langkah itu referensi, bukan salinan, jadi mengubah request di collection ikut mengubah flow. Request WebSocket, MCP, dan gRPC tidak bisa dipilih.
- **Pass on**: nilai dari body response (path JSON seperti `data.items[0].id`) atau dari header diteruskan sebagai `{{nama}}` ke langkah-langkah sesudahnya. Nama nilai tampil sebagai garis nilai di peta. Selama run, nilai ini menang atas environment, dan tidak disimpan di mana pun.
- **Checks tanpa script**: status (equals / is not / is less than), body (path: equals / is not / contains / exists / is less than; tanpa path = seluruh body), header, dan waktu respons. Script `pm.*` milik request tetap berjalan, dan hasil `pm.test`-nya ikut dihitung.
- Sebuah langkah lulus bila response diterima, script dan Pass on tidak error, serta semua check dan test lulus. Tanpa check maupun test, langkah lulus bila status di bawah 400.
- **Jika langkah gagal**: _Stop the flow_ (default; langkah sesudahnya dilewati) atau _Continue with the next step_.
- Hasil tampil langsung per langkah selama run: penanda status, kode dan waktu, tab _Response_ (header dan 64 KB pertama body), dan bagian _Last run_ (check, test, nilai yang diteruskan, error). Nilai yang tampak rahasia disamarkan. Setelah run, langkah yang menghentikan flow terpilih sendiri.
- Request yang dihapus meninggalkan langkah "Request deleted" (bukan hilang diam-diam), dan bisa diganti lewat _Change request_. Nama request yang berubah ikut diperbarui saat halaman dibuka lagi.
- Disimpan otomatis. `Ctrl+Enter` menjalankan flow, dan _Stop_ menghentikannya setelah langkah yang sedang berjalan.
- Batas: 100 langkah per flow, 20 check dan 20 nilai per langkah, nilai maks. 8 KB. Event stream dihentikan setelah header-nya tiba. Run flow tidak dicatat ke history. Flow belum ikut sinkron Collaboration dan belum bisa dijalankan lewat CLI.
- Data: migrasi skema 17 (tabel `flows` dan `flow_steps`).

Tahap 16: **tampilan responsif** (diuji dari jendela minimum 720×520 sampai 3840×2160, termasuk 1366×768 dan 1920×1080 dengan scaling 125%/150%):

- Konten selalu di samping sidebar. Sebelumnya, di jendela sempit konten turun ke bawah sidebar dan layar pertama terlihat kosong.
- Di bawah 900px, sidebar menjadi rel ikon (nama tetap muncul sebagai tooltip dan untuk pembaca layar). Sidebar kini menempel saat konten digulir dan punya scroll sendiri.
- Header kartu request pindah ke dua baris bila kartu sempit, sehingga nama request tidak terhimpit.
- Monitor besar: lebar konten naik menjadi 1400px (mulai 1800px) dan 1720px (mulai 2400px).
- Tombol aksi dialog selalu terlihat di jendela pendek: Copy di "Copy as code", Save PNG dan Copy image di Capture (kini di footer dialog), serta Cancel dan Create di New collection. Yang digulir hanya isinya.
- Quick search tidak lagi melebar karena nama atau URL panjang (berakhir dengan "…"), dan muat di jendela pendek. Tombol aksi section tidak lagi terpecah per kata.

Tahap 17: **Home sebagai hub, tombol ikon section, dan pilihan auth ringkas**:

- **Home** kini berbentuk hub dengan empat bagian: Start, Requests, Testing, dan Workspace. Bagian dipilih lewat tab atau tombol 1-4.
  - Tiap bagian berisi deretan tile ikon. Tile yang dipilih tampil di panel besar: ikon besar samar, fakta singkat, judul, penjelasan, contoh perintah yang bisa disalin, tombol utama, dan tombol ikon tambahan.
  - _Requests_ menambah request ke collection yang sedang terbuka ("Add GraphQL to Shop API"). Tanpa collection, tombolnya menjadi "Create a collection first".
  - _Testing_ berisi Run all, Orchestration, dan contoh perintah `codestamp-cli` (Folder, Collection file, Data file).
  - Keyboard: ←/→ memilih tile, Enter menjalankan tombol utama, 1-4 pindah bagian, Ctrl+Space membuka pencarian. Daftar shortcut lengkap dibuka dari tile Settings (tombol "Keyboard shortcuts").
- **Kepala section**: link teks "Add request / Add WebSocket / Add GraphQL / Add gRPC / Add MCP / Duplicate / Remove section" diganti tombol ikon berwarna dengan tooltip. Label lengkapnya tetap menjadi nama tombol untuk pembaca layar.
  - Adder di bawah daftar, tile collection kosong, kepala section, dan Home memakai satu daftar jenis request yang sama (`lib/request/kind-icons.ts`).
- **Tab Auth**: daftar kartu radio yang memanjang ke bawah diganti satu dropdown "Auth type" dengan keterangan singkat di sampingnya.

Tahap 18: **body XML/HTML/Binary dan SOAP ala SoapUI**:

- **Home**:
  - Tooltip nama tile tidak lagi tertinggal di atas panel setelah tile diklik. Nama hanya muncul saat kursor di atas tile yang belum dipilih, karena tile terpilih sudah menjadi judul panel.
  - Animasi baru: pill tab bergeser ke tab terpilih, tile masuk satu per satu saat bagian berganti, teks panel naik baris demi baris, dan ikon besar muncul membesar sedikit. Semuanya mati bila Windows diatur untuk mengurangi gerakan.
- **Body request**:
  - Mode baru _XML_ dan _HTML_: highlight, Beautify (merapikan indentasi tanpa mengubah isi teks, komentar, CDATA, dan script), serta pesan "Line 3, column 7: …" bila XML rusak.
  - Content-Type otomatis `application/xml` / `text/html`. Mode ini juga dikenali saat import cURL, Postman (raw language `xml`/`html`), dan OpenAPI, serta saat request dibuka lagi dari History.
  - Mode file kini berlabel _Binary_: isi file dikirim apa adanya sebagai `application/octet-stream`, kecuali Content-Type diisi sendiri.
- **Response**: XML dan HTML dirapikan dan diberi highlight seperti JSON.
- **SOAP** (jenis request baru, tombol _Add SOAP_; cap `SOAP`). Alurnya mirip SoapUI:
  - Isi alamat WSDL (URL) atau _Choose file…_, lalu _Load WSDL_. Import `wsdl:import`, `xsd:import`, dan `xsd:include` ikut dibaca.
  - Pilih operasi dari dropdown (`Operasi · Binding · SOAP 1.1/1.2`). Endpoint, SOAPAction, versi, nama request, dan contoh envelope (`?` di setiap nilai, komentar _Optional_ / _repetitions_ / _CHOICE_ seperti SoapUI) terisi otomatis.
  - SOAP 1.1 dikirim sebagai `text/xml` dengan header `SOAPAction`; SOAP 1.2 sebagai `application/soap+xml; action="…"`. Header yang diketik sendiri tetap menang.
  - Envelope yang sudah diedit tidak ditimpa saat operasi berganti. Tombol _Reset the envelope to the sample_ mengembalikannya ke contoh.
  - Response yang berisi SOAP Fault ditandai chip _SOAP Fault_, dengan kode dan alasannya.
  - Test script mendapat `xml2Json()` seperti di Postman. Ada juga snippet "SOAP: no fault, read the body".
  - SOAP ikut Run all, CLI, dan Orchestration. Export Collection v2.1 menulisnya sebagai POST biasa beserta header SOAP.
- **Import WSDL** (halaman Import, file atau URL): menjadi collection baru dengan satu section per binding dan satu request SOAP per operasi, seperti "New SOAP project" di SoapUI.
- **Keamanan**:
  - WSDL boleh dimuat lewat http biasa maupun https ke server mana pun, sama seperti request SOAP-nya, karena banyak layanan SOAP internal hanya menyediakan WSDL lewat http. Isi WSDL lewat http tidak terenkripsi. Import collection atau spec lain dari URL tetap wajib HTTPS (http hanya untuk `localhost`).
  - Di halaman Import, alamat yang jelas WSDL (`?wsdl`, `?singleWsdl`, `.wsdl`) langsung dibaca pembaca WSDL beserta import-nya.
  - File WSDL hanya bisa dibaca bila dipilih lewat dialog, dan import relatifnya tidak boleh keluar dari folder file itu.
  - DTD ditolak, sehingga trik entitas XML (XXE, "billion laughs") tidak berlaku.
  - Batas: 64 dokumen dan 20 MB per WSDL.
- **Collaboration**: format repository naik ke **5** (body XML/HTML dan request SOAP). Alamat WSDL berupa URL ikut dibagikan, path file lokal tidak.
- **Data**: migrasi skema 18 (kolom `requests.soap`).
- **Belum didukung**: WSDL 2.0, WS-Security (UsernameToken), mock service SOAP, dan WSDL yang bukan UTF-8.

Tahap 19: **title bar sendiri dengan menu aplikasi**

- Title bar bawaan Windows (ikon dan tulisan "Codestamp") diganti title bar Codestamp yang mengikuti tema (Light, Dark, Dark Night):
  - kiri: tombol **Menu** dan tombol **ciutkan/lebarkan sidebar**;
  - tengah: area kosong untuk menggeser jendela (double-click untuk maximize/restore);
  - kanan: Minimize, Maximize/Restore, Close.
- **Menu** membuka menu native Windows dengan submenu:
  - _File_: New collection, Import…, Back up data…, Settings, Exit.
  - _Edit_: Undo, Redo, Cut, Copy, Paste, Select all. Bekerja pada kolom yang sedang diedit, termasuk editor kode, karena tombol Menu tidak mengambil fokus keyboard.
  - _View_: Collapse/Expand sidebar (Ctrl+B) dan Theme (Light, Dark, Dark Night, System).
  - _Go_: Home, Orchestration, Environments, Cookies, History, Import, Settings, Search… (Ctrl+Space).
  - _Help_: Keyboard shortcuts and about.
- **Sidebar** bisa diciutkan menjadi rel ikon 64px lewat tombol di title bar, menu View, atau **Ctrl+B**. Ikon Home, collection, Orchestration, Environments, dan lainnya tetap terlihat dan bisa diklik, dengan nama sebagai tooltip. Pilihan ini diingat di perangkat ini. Tombolnya nonaktif selama mode fokus (sidebar memang tersembunyi) dan di jendela yang lebih sempit dari 900px (sidebar sudah berupa rel ikon).
- Jendela tetap bisa di-resize dari tepi dan sudutnya.
- Batas: tombol Maximize buatan sendiri tidak memunculkan pilihan Snap Layouts saat kursor diam di atasnya. Snap tetap jalan lewat Win + panah atau dengan menyeret jendela ke tepi layar.

Tahap 20: **sidebar Tools dan Catalog** (dulu bernama API Catalog)

- Grup sidebar **Workspace** diganti **Tools** dan hanya berisi **Orchestration** dan **Catalog**. Environments, Cookies, History, Import, dan Settings tetap bisa dibuka dari Home (tab Workspace dan Start), Menu › Go, dan Quick search (`Ctrl+Space`). Menu › Go, Quick search, dan Home › Testing juga mendapat Catalog.
- **Catalog** meniru API Catalog Postman (fitur khusus paket Enterprise), disesuaikan untuk aplikasi lokal tanpa server: setiap collection tampil sebagai satu API.
  - **Daftar**: total di atas (jumlah API, request terkirim dan error 7 hari terakhir dibanding minggu sebelumnya, API yang perlu perhatian), lalu tabel API dengan skor kesehatan, hasil Run all terakhir, error rate, p95, owner, dan aktivitas terakhir. Bisa dicari, difilter per tag dan status kesehatan, dan diurutkan per kolom. Di jendela sempit kolom p95, owner, dan aktivitas disembunyikan; owner pindah ke bawah nama.
  - **Skor kesehatan** 0-100 dari tiga bagian: _Runtime_ 40% (error rate dan p95 dari History 7 hari kalender terakhir; error = tanpa jawaban atau 5xx, 4xx dihitung terpisah), _Tests_ 35% (cakupan test script dan pass rate Run all terakhir), _Quality_ 25% (aturan kualitas). Bagian tanpa data tidak ikut dihitung. 80 ke atas Healthy, 50-79 Needs attention, di bawah 50 Critical.
  - **Halaman API** dengan tab _Overview_ (skor per bagian, isi collection, flow Orchestration yang memakainya), _Runtime_ (angka 7 hari, grafik request per hari yang bisa dilihat sebagai tabel, request dengan error rate tertinggi), _Tests_ (cakupan, pass rate, run terbaru per request, 20 run terakhir), dan _Quality_.
  - **Aturan kualitas** (seperti governance rules Postman): secret tidak ditulis langsung di header, parameter, atau auth (Error); koneksi terenkripsi kecuali ke localhost; URL diawali variabel seperti `{{baseUrl}}`; request punya test; request punya example; nama request unik. Tiap temuan bisa diklik untuk membuka request-nya.
  - **Owner dan tag** per API, disimpan di perangkat ini saja (belum ikut Collaboration).
  - Run all kini menyimpan hasilnya (per request, iterasi dijumlahkan; 20 run terakhir per collection, schema V19) sebagai sumber angka Tests.
- Yang tidak ada di Codestamp karena butuh server atau akun: discovery dari API gateway, Kubernetes, atau Git; metrik produksi dari agent; skor kustom (scorecards) dan governance group. Run Orchestration dan `codestamp-cli` belum masuk angka catalog.

Tahap 21: **Docs, SSL/TLS, indentasi None, dan sampel logo**

- **Docs** (Tools › Docs): dokumentasi tiap collection.
  - Deskripsi Markdown untuk collection, section, dan request, ditulis dan dipratinjau langsung di halaman Docs (Write/Preview, `Ctrl+Enter` simpan, `Esc` batal, maksimal 20.000 karakter). Deskripsi disimpan di kolom sendiri (schema V20), jadi autosave kartu request tidak menimpanya.
  - Di samping deskripsi: URL, auth, path variable, query parameter, header, body, dan example tersimpan. Nilai kredensial yang ditulis langsung disamarkan (`Bearer ••••••••`); referensi `{{variabel}}` tetap terlihat. Daftar isi melompat ke tiap request.
  - Markdown diurai menjadi pohon node (`mdast-util-from-markdown` + GFM) lalu digambar komponen Svelte: tidak ada `{@html}`/`innerHTML`, HTML mentah tampil sebagai teks, dan link `javascript:` tidak menjadi link. Link http/https dibuka di browser lewat plugin resmi `tauri-plugin-opener` (capability hanya http/https).
  - **Save as Markdown** menyimpan satu file `.docs.md` (secret tidak pernah dibaca, file lokal hanya namanya).
  - Deskripsi ikut Collaboration (format repository 6), import Postman (deskripsi collection, folder, request), import OpenAPI (info, tag, operasi), import WSDL (dokumentasi operasi), export Postman, dan duplikat collection/section/request.
- **Settings › SSL/TLS** menggantikan Certificates: versi TLS minimum (TLS 1.2 atau lebih baru / TLS 1.3 saja) untuk HTTP, GraphQL, SOAP, SSE, WebSocket, token OAuth, dan import dari URL, plus CA certificate dan client certificate. Verifikasi sertifikat tetap selalu aktif. gRPC selalu menerima TLS 1.2 ke atas. Reset di Requests tidak mengubah versi TLS.
- **Indentasi editor None** menjadi bawaan: baris baru mulai dari kiri, Tab pindah ke kolom berikutnya, dan Beautify (serta contoh pesan gRPC/argumen MCP) menaruh tiap elemen di barisnya sendiri tanpa indent. Pilihan yang sudah tersimpan sebelumnya tidak diubah.
- **Sampel logo**: lima konsep di `branding/logo-samples/` (SVG, plus `preview.html`).
- Batas: WebSocket memakai trust store Windows saja (CA certificate dari Settings belum dipakai untuk WebSocket).

Tahap 22: **nama Codestamp dan logo baru**

- Aplikasi berganti nama dari Postcard menjadi **Codestamp**: judul jendela, sidebar, About, pesan di aplikasi, CLI (`codestamp-cli`), User-Agent bawaan (`Codestamp/<versi>`), header mock (`X-Codestamp-Mock`), dan identifier aplikasi (`dev.codestamp.desktop`).
- Logo 3a (`branding/codestamp-logo.svg`): kurung kurawal kode di kiri, perangko merah dan garis alamat di kanan. Logo ini menjadi ikon aplikasi (`src-tauri/icons`, dibuat dengan `npx tauri icon branding/codestamp-logo.svg`) dan logo sidebar.
- Data dari versi bernama Postcard pindah otomatis saat Codestamp pertama kali dibuka. Data dipindah, bukan disalin:
  - folder `%APPDATA%\dev.postcard.desktop` menjadi `dev.codestamp.desktop`, dan `postcard.db` menjadi `codestamp.db`. Cache webview di `%LOCALAPPDATA%` ikut pindah;
  - kunci enkripsi secret di Windows Credential Manager pindah ke nama baru;
  - repository Collaboration yang masih berisi `postcard.json` tetap terbuka, lalu memakai `codestamp.json` sejak sinkron berikutnya;
  - backup `.postcard` lama tetap bisa di-restore. Backup baru memakai `.codestamp`.
- Nama-nama lama itu hanya tersisa di `src-tauri/src/legacy.rs`, satu-satunya kode yang membaca data versi Postcard.
- Tutup Postcard sebelum membuka Codestamp pertama kali. Folder data yang sedang dipakai tidak bisa dipindah, dan Codestamp tidak akan terbuka sampai Postcard ditutup.
- Status ciut sidebar dan section kembali ke bawaan satu kali, karena kunci penyimpanannya ikut berganti nama.

Perbaikan setelah Tahap 22:

- Request yang dipilih di Quick search kini langsung terlihat. Codestamp menggulir halaman sampai kartunya berada tepat di bawah title bar, lalu memindahkan fokus keyboard ke kartu itu, juga untuk request yang jauh di bawah collection panjang atau di dalam section yang diciutkan. Sebelumnya kartu terbuka di tempatnya tanpa digulir, jadi pengguna harus mencarinya sendiri.
- Hal yang sama berlaku untuk request yang dibuka dari History, Catalog, Docs, Orchestration, hasil Run all, dan Import cURL. Membuka baris yang sudah terlihat di layar tidak menggulir halaman.
- Title bar (Menu, sidebar, Minimize, Maximize, Close) kini tetap di atas sejauh apa pun halaman digulir. Sebelumnya title bar ikut tergulir hilang setelah sekitar satu layar.
- Hanya area di bawah title bar yang digulir, jadi scrollbar-nya tidak lagi memanjang sampai ke samping tombol Close. Jendelanya sendiri tidak pernah digulir.
- Pembaca layar kini membaca tombol di Docs dengan spasi yang benar ("Open request Login", "Edit description of this request"), bukan "Open requestLogin".

Tahap 23: **Ctrl+F, nama baru, dan pencarian Docs**

- **Ctrl+F di editor**: body request, body response, script, dan editor kode lain punya pencarian sendiri di atas editor (paket resmi `@codemirror/search`).
  - Isinya kolom Find, jumlah temuan ("3 of 12"), Previous/Next, serta opsi Match case, Whole word, dan Regular expression. Teks dicari apa adanya, jadi `\n` di body JSON cocok dengan tulisan `\n`.
  - Enter dan F3 ke temuan berikutnya, Shift+Enter dan Shift+F3 ke sebelumnya, Esc menutup dan kembali ke editor. Semua temuan diberi latar, dan temuan yang dipilih diberi bingkai tebal.
  - Selama ada kartu request yang terbuka, Ctrl+F dari luar editor (misalnya setelah menekan Send, dari tabel form, atau dari sidebar) membuka pencarian tepat di atas body response, bukan kolom cari bawaan browser di pojok atas jendela. Tanpa response, pencarian dibuka di editor pertama yang terlihat.
- **Nama baru**: PostFlow menjadi **Orchestration**, dan API Catalog menjadi **Catalog**, di sidebar, Menu › Go, Home, Quick search, dan judul halaman. Mengetik nama lama di Quick search tetap menemukan halamannya. Nama semua halaman kini berasal dari satu tempat (`navLabel` di `lib/state/navigation.ts`).
- **Docs: indeks + satu dokumen** (seperti tab dokumentasi Postman):
  - Di kiri ada indeks yang menempel dan punya scroll sendiri. Isinya pemilih collection (dropdown), kolom **Search docs** (Ctrl+F di halaman Docs), Overview, request di luar section, lalu tiap section beserta request-nya. Panah atas/bawah berpindah item.
  - Di kanan hanya ada satu dokumen: Overview collection, satu section, atau satu request dengan tombol Previous/Next. Collection dengan ratusan request jadi tetap ringan dan rapi.
  - Pencarian menyaring indeks menurut nama, alamat, method, dan deskripsi. Section yang namanya atau deskripsinya cocok tetap tampil bersama semua request-nya. Jumlahnya tampil ("1 of 3 requests"), dan bila tidak ada yang cocok tersedia tombol Clear search.
  - Variables di Overview dilipat menjadi tabel. Nilai variabel biasa yang namanya mirip kredensial (misalnya `baseTokenBotTele`) kini disamarkan, di layar maupun di Save as Markdown.
  - Di jendela sempit, indeks ada di atas dokumen dengan daftar setinggi beberapa baris.
- **Docs di Quick search** (`Ctrl+Space`): grup baru **Docs** mencari nama dan deskripsi collection, section, dan request. Pencocokannya berjalan di Rust (`search_docs`, maksimal 200 karakter dan 8 kata), dan setiap hasil menampilkan potongan deskripsi di sekitar kata yang ditemukan. Memilih hasil membuka Docs pada item itu. Saat halaman Docs terbuka, grup Docs tampil paling atas.
- **Add request di section**: request baru yang ditambahkan dari kepala section, dari tombol di bawah daftar, dari collection kosong, dari Home, atau lewat Duplicate kini langsung digulir ke layar dan mendapat fokus. Sebelumnya kartu baru terbuka di ujung section yang panjang, jadi harus digulir sendiri.

Tahap 24: **akun Git per collection**

- Setiap collection yang dibagikan kini punya akunnya sendiri, jadi collection A bisa memakai akun kantor dan collection B akun pribadi, walaupun keduanya di host yang sama (mis. github.com).
  - **Git account** (opsional, khusus alamat `https://`): username yang dipakai Git untuk login, misalnya username GitHub. Codestamp meneruskannya sebagai `credential.username` hanya pada perintah Git yang menghubungi repository (fetch, push, clone), sehingga Git Credential Manager memilih atau meminta login akun itu. Password dan token tetap dipegang Credential Manager. Kosong berarti akun bawaan Git untuk host itu, seperti sebelumnya.
  - **Nama dan email commit** kini disimpan per collection. Profil terakhir yang dipakai hanya mengisi form Share dan Join berikutnya. Tautan lama yang belum punya penulis sendiri tetap memakai profil itu.
  - Alamat SSH tidak memakai username: akun ditentukan oleh kunci SSH. Untuk dua akun SSH di host yang sama, pakai alias host di `~/.ssh/config` (mis. `git@github-kantor:org/repo.git`).
- Di panel Collaboration, collection yang sudah dibagikan menampilkan _Signs in as_ (untuk https) dan _Your changes as_. Tombol **Change account** membuka form akun untuk collection itu. Akun baru dipakai pada _Sync now_ berikutnya.
- Form Share dan dua form Join (halaman Import dan dialog New collection) memakai komponen yang sama, `AccountFields`. Kolom Git account baru muncul setelah alamat yang diketik diawali `https://`.
- Validasi di Rust: username maksimal 100 karakter, hanya huruf, angka, `.`, `_`, `-`, `@`, `+`, tidak boleh diawali `-`, dan ditolak untuk alamat SSH atau folder. Migrasi skema 21 menambah kolom `username`, `author_name`, dan `author_email` di `team_links`. Perintah `set_team_author` diganti `set_team_account`.

Tahap 25: **Git accounts, Git changes, dan replace collection**

- **Settings › Git accounts**: daftar akun Git yang dipakai collection bersama. Isinya akun yang ditambahkan sendiri (host, username, label opsional), akun yang dipakai collection (otomatis masuk daftar saat Share, Join, atau Change account), dan baris "Git's default account" per host untuk collection https tanpa username.
  - Tiap baris menampilkan collection yang memakainya. Status _Signed in_ / _Sign-in needed_ dibaca dari `git credential-manager github list` untuk github.com. Untuk host lain, **Check** menjalankan `git ls-remote` pada repository collection yang memakai akun itu; bila belum ada login, Git Credential Manager membuka jendela sign-in-nya.
  - **Sign in** untuk akun GitHub memakai `git credential-manager github login --username`. **Sign out** memakai `github logout` atau `git credential reject` (host lain), dengan konfirmasi. Codestamp tidak pernah menerima password atau token.
  - **Remove** hanya untuk akun yang tidak dipakai collection mana pun.
  - Di Collaboration (Share, Join, Change account), kolom Git account menjadi pilihan: "Git's default account", akun yang tersimpan untuk host itu, atau "Another account…" untuk mengetik username baru.
- **Tools › Git changes** (gaya GitLens: daftar commit + detail, ditambah tab Requests):
  - Pilih collection bersama. Bar atasnya menampilkan jumlah perubahan lokal yang belum di-commit, commit yang belum di-push, waktu sinkron terakhir, serta tombol Open collection dan Sync now.
  - Tab **Commits**: commit dikelompokkan per hari (Today, Yesterday, tanggal), titik untuk commit biasa dan lingkaran untuk merge, chip _Not pushed_, dan "Load older commits" (50 per halaman). Detail commit menampilkan pesan, penulis, email, waktu, hash (Copy hash), lalu tiap request yang _Added_, _Updated_, atau _Removed_ beserta field yang berubah (sebelum dan sesudah, berdampingan). Perubahan collection tampil sebagai "Collection settings": section per nama dan urutan request per jumlah. Merge dibandingkan dengan parent pertamanya, dan detail dibatasi 200 file.
  - Tab **Requests**: tiap request beserta siapa yang terakhir mengubahnya, kapan, dan jumlah versinya. History membuka dialog riwayat yang sama dengan ikon jam di kartu.
  - Semua dibaca dari salinan repository di perangkat ini (tanpa jaringan), dan dimuat ulang setiap kali sinkron selesai.
- **Import › The content of an existing collection**: file collection (Postman, OpenAPI) bisa menggantikan isi collection yang sudah ada. Section, request, auth, docs, dan variabelnya diganti, sedangkan id, nama, ikon, dan tautan Collaboration tetap. Sebelum mengganti ada konfirmasi berisi jumlah request lama dan baru. Collection bersama mengirim penggantian itu sebagai commit biasa pada sinkron berikutnya, jadi versi lama tetap ada di riwayat Git.
- **Share ke repository yang sudah berisi** (mis. repository GitLab berisi export Postman):
  - Share selalu melihat isi repository dulu (`preview_share`: clone dangkal tanpa working tree yang langsung dihapus lagi). Repository kosong langsung diisi seperti sebelumnya.
  - Bila sudah ada file, panel menampilkan kotak peringatan berisi branch utama repository (mis. `master`), daftar file yang akan dihapus (maksimal 200 nama, sisanya dihitung), dan catatan bila isinya collection Codestamp lain yang mungkin sudah di-Join orang. Tombol **Replace files and share** menghapus semua file itu dan menaruh collection di tempatnya dalam **satu commit biasa di branch utama repository, tanpa force push**, jadi file lama tetap ada di riwayat.
  - Pengaman: Share hanya mengganti isi bila menerima hash commit yang sudah dilihat di pratinjau. Bila ada yang push setelah pratinjau, Share berhenti dengan pesan "The repository changed after you checked it" dan meminta melihat ulang. Mengubah alamat repository juga membatalkan pratinjau.
  - Join ke repository yang bukan hasil Share Codestamp kini menjelaskan masalahnya ("This repository was not shared from Codestamp…"), bukan lagi "file Codestamp cannot use: codestamp.json".
- Data: migrasi skema 22 (`git_accounts`: id, host, username, label; tanpa rahasia apa pun). Perintah baru: `list_git_accounts`, `add_git_account`, `remove_git_account`, `check_git_account`, `sign_in_git_account`, `sign_out_git_account`, `git_changes_overview`, `git_commits`, `git_commit_detail`, `git_request_activity`, `replace_with_import`.
- Tabel perbandingan field (`FieldChanges.svelte`) kini dipakai bersama oleh dialog riwayat request dan Git changes.

Tahap 26: **sinkron manual, environment milik collection, dan section Settings di sidebar**

- **Sync now saja**: collection bersama tidak lagi sinkron sendiri (dulu 4 detik setelah perubahan tersimpan dan setiap 30 detik), supaya tidak ada sinkron latar yang bisa terkena timeout. Perubahan yang tersimpan menunggu di perangkat ini. Header dan panel Collaboration menampilkan _not synced_ dengan penjelasan "Your saved changes are not in the repository yet", dan status itu dibaca ulang dari repository saat aplikasi dibuka. Mengganti akun Git juga tidak memicu sinkron.
- **Environment milik collection**: halaman Environments punya pilihan **Belongs to** (_This device only_ atau sebuah collection). Daftar environment dikelompokkan: _On this device_, lalu satu grup per collection.
  - Environment milik collection bersama ikut dikirim pada _Sync now_ sebagai `environments/<id>.json` (nama dan variabel). Nilai variabel secret tidak pernah dikirim: hanya namanya, dan setiap perangkat menyimpan nilainya sendiri.
  - Environment atau variabel yang ditambahkan rekan muncul setelah _Sync now_. Environment yang dihapus rekan ikut terhapus. Variabel digabung per nama; bila dua orang mengubah hal yang sama, muncul di dialog konflik sebagai `Environment “nama”`.
  - Pemilih environment aktif menulis pemiliknya di belakang nama (`DEV · Shop`). Git changes menampilkan file environment sebagai `Environment “nama”`.
  - Format repository naik ke **7**. Codestamp versi lama menolak format 7 dengan pesan "Update Codestamp". Maksimal 50 environment per repository; isi file diperiksa seperti file request (id cocok dengan nama file, nama valid, batas jumlah dan panjang variabel).
- **Variabel collection yang diganti environment**: saat sebuah environment aktif, variabel collection dengan nama yang sama diberi catatan "Replaced by “DEV”" di panel Variables. Urutan prioritas tidak berubah: environment menimpa Globals dan variabel collection yang namanya sama, variabel lain tetap terpakai.
- **Ctrl+S** menyimpan editor variabel di halaman Environments dan panel Variables collection, seperti di kartu request. Shortcut dari dialog yang terbuka diabaikan (`lib/dom/keys.ts`).
- **Section Settings di sidebar**: Environments, Cookies, History, dan Import pindah ke section baru **Settings** di paling bawah sidebar, di bawah Tools. Halaman Settings sendiri tetap dibuka dari Menu › Go, Home, dan Quick search.
- Data: migrasi skema 23 (`environments.owner_collection_id`, terhapus bersama collection-nya). Perintah baru: `set_environment_collection`.

Tahap 27: **pembaruan otomatis**

- Codestamp mengecek versi baru beberapa detik setelah dibuka, tanpa memperlambat start. Bila ada, muncul banner "Codestamp 0.3.0 is available. You have 0.2.0." dengan tombol _What's new_, _Update and restart_, dan _Later_. Pengecekan saat start yang gagal (offline, server mati) tidak menampilkan apa-apa.
- **Settings › About › Updates**: tombol _Check for updates_, versi dan tanggal rilis, catatan rilis (teks biasa, maksimal 4000 byte), dan _Update and restart_. Di sini kegagalan cek ditampilkan dengan alasan singkat.
- _Update and restart_ mengunduh installer dengan progres di banner, memeriksa tanda tangannya, lalu menjalankannya dalam mode update (`/P /UPDATE /R`): halaman "Already Installed" dilewati, tidak ada uninstall, file ditimpa di tempat, data tidak disentuh, lalu Codestamp terbuka lagi.
  - Sebelum menutup, Codestamp menyimpan edit request dan Orchestration yang tertunda. Update tidak dimulai selama ada panel collection atau editor environment yang belum disimpan, atau Orchestration yang sedang berjalan; alasannya ditulis di banner.
- **Keamanan**: installer hanya dipasang bila tanda tangannya cocok dengan kunci publik di dalam Codestamp dan dibuat untuk versi yang diumumkan (`requireSignedVersion`). Installer yang diubah, atau `latest.json` yang memasangkan nomor versi baru dengan installer lama, ditolak dengan pesan "failed its signature check". Alamat update wajib `https://`. Pesan error tidak memuat alamat server atau detail kunci.
- Build tanpa alamat update dan kunci publik tidak pernah mengecek; About menulis "This build of Codestamp is not set up to receive updates."
- Rust: `src-tauri/src/update/` dengan `tauri-plugin-updater` 2.14. Perintah baru: `check_for_update`, `install_update` (progres lewat Channel). Panel yang belum disimpan kini dilaporkan lewat `Workspace.unsavedPanel()`.
- Cara menyiapkan server dan merilis versi baru: lihat **Pembaruan otomatis** di bagian Installer Windows.

Tahap 28: **salinan repository yang rusak dan login ulang**

- **Perbaikan otomatis**: Codestamp menyimpan salinan tiap collection bersama di `%APPDATA%\dev.codestamp.desktop\repos\<id collection>`. Bila salinan itu hilang atau `.git`-nya rusak, Sync now dulu gagal dengan "Git failed: not a git repository". Sekarang Sync now meng-clone ulang repository ke folder sementara (di sinilah Git Credential Manager meminta login bila akun belum tersimpan), lalu menggantikan salinan lama dan melanjutkan sinkron.
  - Clone baru dimulai dari **commit terakhir yang tersinkron**, jadi perubahan yang tersimpan di perangkat sejak itu dikirim sebagai perubahan biasa dan perubahan rekan tetap digabung per field. Bila keduanya mengubah field yang sama, dialog konflik muncul seperti biasa.
  - Commit itu kini disimpan di setiap Share, Join, dan sinkron yang berhasil (migrasi skema 24, kolom `team_links.synced_commit`). Untuk collection yang dibagikan sebelum skema 24, Codestamp memakai commit terbaru sebelum waktu sinkron terakhir, atau commit pertama bila tidak ada, sehingga perbedaan muncul sebagai konflik dan tidak hilang diam-diam.
  - Bila repository tidak bisa dijangkau saat clone ulang, sinkron dilaporkan seperti biasa (offline, sign in) dan folder sementaranya dihapus.
- **Sign in di Collaboration**: bila sinkron gagal karena login, status panel berbunyi "Git could not sign in. … Sign in opens Git's sign-in window." dengan tombol **Sign in**. Tombol itu meminta daftar branch dari repository dengan akun collection tersebut (`git ls-remote`), sehingga Git Credential Manager membuka jendela login-nya. Akun GitHub dengan username memakai `git credential-manager github login`. Setelah login berhasil, sinkron langsung dijalankan. Untuk alamat SSH tidak ada tombol, karena aksesnya ditentukan kunci SSH.
- Settings › Git accounts › Check kini juga bertanya langsung ke alamat repository, sehingga tetap berjalan walau salinan collection rusak.
- Perintah baru: `sign_in_team_collection`.

Tahap 29: **variabel terbatas untuk grup akses**

- **Grup akses** di Collaboration › _Access groups_: nilai variabel bisa dibatasi ke satu grup (misalnya "Prod"). Nilainya dikirim terenkripsi, dan hanya perangkat anggota grup yang bisa membacanya. Orang lain yang punya akses ke repository hanya melihat nama variabelnya.
  - Di tabel variabel (collection dan environment milik collection bersama), kolom Type kini berisi Text, Secret, dan _Restricted: nama grup_.
  - Perangkat tanpa akses melihat baris itu terkunci ("Restricted to Prod. Ask a member for access in Collaboration."), tidak bisa mengubah atau menghapusnya, dan preview URL menulis `{{token}} (restricted to Prod)`. Nilai itu tidak ikut dikirim di request, jadi tidak pernah terkirim kosong diam-diam.
- **Alur**: buat grup (pembuatnya menjadi anggota pertama), pilih _Restricted: Prod_ di variabel, lalu Sync now. Rekan menekan **Request access**; anggota mencocokkan fingerprint perangkatnya (`XXXX-XXXX-XXXX-XXXX`, tampil di _This device_) lewat chat atau telepon, lalu menekan **Grant**. Setiap aksi langsung menjalankan Sync now.
  - **Remove** memberi grup kunci baru dan mengenkripsi ulang semua nilainya. Codestamp lalu mendaftar variabel yang perlu diganti di sumbernya, karena orang yang dikeluarkan sudah pernah melihat nilai lama (dan riwayat Git masih menyimpan versi lama).
  - Semua anggota boleh memberi akses. Perangkat baru atau Windows yang diinstal ulang meminta akses lagi; tidak ada pemulihan tanpa anggota lain.
- **Kriptografi**: tiap perangkat punya pasangan kunci X25519; kunci privatnya disimpan di Windows Credential Manager (`access-device-key`) dan tidak pernah keluar. Kunci grup dibungkus untuk kunci publik tiap anggota dengan HPKE (RFC 9180: DHKEM X25519, HKDF-SHA256, ChaCha20-Poly1305). Nilai dienkripsi ChaCha20-Poly1305 dengan kunci turunan HKDF dan terikat ke collection, set variabel, nama variabel, grup, dan versi kunci, sehingga ciphertext yang dipindah tidak bisa dibuka.
- **Perlindungan file grup**: tiap perubahan file grup membawa stempel HMAC-SHA256 yang hanya bisa dibuat pemegang kunci grup. Kunci baru harus membawa semua kunci lama (terenkripsi dengan kunci baru), dan setiap perangkat memeriksa setiap versi yang di-commit sejak sinkron terakhirnya secara berurutan. Jadi orang yang bisa push tapi bukan anggota tidak bisa menambahkan diri atau mengganti kunci, dan anggota yang sudah dikeluarkan tidak bisa menyelipkan kunci baru. Perubahan yang gagal diperiksa membuat grup terkunci di perangkat itu dengan tombol **Repair group**, yang mengembalikan anggota versi terakhir yang lolos pemeriksaan dengan kunci baru.
- **Batasan**: anggota tetap bisa menyalin nilai yang mereka lihat. URL, header, body, dan script tetap terlihat dan bisa diubah oleh siapa pun yang bisa push, termasuk alamat tujuan sebuah token, jadi tinjau perubahan rekan di Git changes. Token yang diketik langsung di Auth atau header tidak terlindungi dan perlu dipindah ke variabel. Penghapusan file grup tidak bisa dicegah (gunakan protected branch); Codestamp anggota menulisnya kembali pada sinkron berikutnya. CLI belum bisa membuka nilai terbatas.
- **Format repository 8**: `access/<id grup>.json` (anggota, kunci yang dibungkus, stempel) dan `access/requests/<id grup>.<fingerprint>.json`. Variabel terbatas ditulis dengan `restricted` dan `sealed`. Format 8 hanya dipakai setelah collection punya grup, jadi repository lain tetap terbuka di versi lama; Codestamp 0.2.1 menolak repository format 8 dengan pesan "Update Codestamp".
- Sync now yang membawa perubahan akses langsung memuat ulang halaman Environments, jadi nilai yang baru boleh dibaca terbuka tanpa menutup aplikasi (perbaikan setelah 0.3.0).
- Versi aplikasi naik ke **0.3.0**, karena format repository 8 tidak bisa dibuka 0.2.x. Semua anggota collection yang memakai grup perlu memasang installer 0.3.0.
- Data: migrasi skema 25 (`team_access_groups`, `team_access_requests`, kolom `variables.restricted` dan `variables.sealed`). Crate baru: `hpke` 0.14.1 dan `hkdf` 0.13. Perintah baru: `access_overview`, `create_access_group`, `request_group_access`, `grant_group_access`, `decline_group_access`, `remove_group_member`, `repair_access_group`.

Tahap 30: **layar pembuka, kabar akses, dan versi 0.3.1**

- **Layar pembuka**: saat dibuka, Codestamp langsung menampilkan logo di atas warna tema yang tersimpan (Light, Dark, atau Dark Night), dengan garis loading yang berjalan mengelilingi tepi logo. Tidak ada lagi layar putih sebelum konten muncul.
  - Setelah daftar collection termuat (minimal 0,5 detik), logo memudar dan aplikasi muncul dari ukuran sedikit lebih kecil dengan blur yang menghilang, 0,46 detik, mirip jendela macOS. Setelah itu tidak ada animasi lain.
  - Bila Windows diatur untuk mengurangi animasi, garisnya diam dan aplikasi hanya memudar 0,15 detik.
  - Rust membuka jendela tersembunyi, memberi warna latar tema (juga saat tema diganti di Settings), memasang tema di halaman sebelum script aplikasi jalan, lalu menampilkan jendela.
- **Kabar saat akses diberikan**: setelah Sync now yang membuat perangkat ini menjadi anggota grup yang dimintanya, muncul banner "Mate gave this device access to Ops. Its restricted values are ready to use." dengan tombol **Open variables** (membuka panel Variables collection itu) dan Dismiss. Grup yang dibuat atau diperbaiki di perangkat ini tidak memicu banner.
- **Perbaikan**: Sync now yang membawa perubahan akses langsung memuat ulang halaman Environments (lihat Tahap 29).
- Versi aplikasi naik ke **0.3.1**. Tidak ada perubahan format repository atau skema database.

Tahap 31: **perbaikan tampilan setelah 0.3.1, versi 0.3.2**

- **Pemilih environment**: hanya environment yang aktif yang bertitik hijau. Sebelumnya semua environment bernama tampak hijau, sehingga terlihat aktif semua.
- **Jendela di tengah layar** saat Codestamp dibuka (sebelumnya mengikuti posisi bawaan Windows dan bisa menepi).
- **Docs**: section di indeks bisa dilipat dengan chevron dan mulai terlipat, jadi daftar tidak memanjang terus. Memilih section, Previous/Next, dan Quick search membuka section yang dituju; section yang dibuka diingat di perangkat ini. Selama mencari, semua hasil tampil.
- **Sidebar**: judul **Tools** dan **Settings** bisa diklik untuk melipat grupnya (diingat per perangkat), dan di samping **Settings** ada tombol gerigi yang langsung membuka halaman Settings. Saat sidebar diciutkan menjadi rel ikon, semua item tetap tampil.
- **Perbaikan 0.3.1**: editor kode (body, response, script) tampil berantakan di 0.3.1 karena gaya layar pembuka ditulis sebagai `<style>` inline. Tauri memberinya hash di CSP, sehingga browser mengabaikan `'unsafe-inline'` dan menolak gaya CodeMirror. Gaya itu kini ada di `src/launch.css`, dan stage38 memeriksa bahwa gaya yang disuntikkan saat berjalan tetap berlaku.
- Ikon Settings (judul halaman, Home, Quick search, dan tombol di sidebar) kini berupa gerigi.
- Versi aplikasi naik ke **0.3.2**. Installer 0.3.1 sebaiknya tidak dipakai lagi karena editor kodenya rusak.
- Kode: satu komponen `FoldButton` untuk chevron lipat di collection, Docs, dan sidebar; satu penyimpan `RememberedIds` untuk semua lipatan.

Tahap 32: **Export / Import, Recently deleted, dan tampilan yang lebih tenang**

- **Quick search lebih ringan dibuka**: latar tidak lagi di-blur (blur selebar jendela menggambar ulang seluruh aplikasi di setiap frame). Dialog turun dan memudar dengan kurva yang sama dengan tile Home, dan hanya opacity serta transform yang bergerak.
- **Tanpa teks shortcut di layar**: tombol Send, Invoke, Run flow, Run tool, Send message, Copy image, Exit focus mode, tombol cari di sidebar, Quick search, Home, Docs, editor deskripsi, tombol Find di editor, tooltip, dan menu tidak lagi menulis Ctrl+…, Esc, atau Enter. Shortcut tetap bekerja, dan daftar lengkapnya ada di Settings › About & shortcuts (kini juga Tab di Quick search dan Ctrl+F).
- **Tooltip**: tooltip Collapse/Expand sidebar tidak lagi terpotong di tepi kiri jendela. Di rel ikon, nama tiap item muncul di samping ikonnya saat kursor diarahkan atau saat fokus keyboard ada di item itu.
- **Export / Import** (dulu Import): dua tab. Tab Export menyimpan collection dalam format **Codestamp** (untuk sesama Codestamp: semua jenis request, example, Docs, dan environment milik collection), **Postman Collection v2.1** (untuk Postman, Insomnia, Bruno, dan alat lain), atau **Docs as Markdown**, serta environment sebagai file environment Postman. Secret dan nilai restricted tidak ikut, begitu juga path file lokal. Tombol Export di collection membuka tab ini dengan collection itu terpilih.
  - File Codestamp berisi file repository Collaboration dalam satu JSON (`"codestamp": "collection"`) dan diperiksa seperti repository saat diimpor. Semua bagiannya mendapat id baru, jadi bisa diimpor di samping collection asalnya.
- **Recently deleted**: collection atau request yang dihapus disimpan 30 hari. Setelah menghapus, banner menawarkan **Undo**. Halaman Recently deleted (grup Settings di sidebar, Menu › Go, Quick search, Home) memulihkan item dengan section, example, Docs, variabel, environment, tautan Collaboration, dan langkah Orchestration-nya, atau menghapusnya untuk selamanya.
  - Salinannya berupa baris database apa adanya (secret tetap terenkripsi). Saat dipulihkan, nama tabel dan kolom dicocokkan ke daftar yang diizinkan, jadi salinan yang diubah tidak bisa menyentuh tabel lain.
  - Request kembali ke section-nya bila masih ada. Bila collection-nya juga sudah dihapus, request dipulihkan ke collection baru dengan nama yang sama.
- Skema database naik ke **26** (tabel `deleted_items`). Format repository tetap 8.
- Versi aplikasi naik ke **0.3.3**.

Tahap 33: **status bar, Export dan History di collection**

- **Status bar**: garis memanjang di paling bawah jendela, seperti Bruno. Kiri: ikon Settings, Recently deleted, Cookies, dan Environments (tooltip muncul di atas tombol). Kanan: versi Codestamp yang sedang dibuka. Section Settings di sidebar dihapus, jadi sidebar kini berisi Home, Collections, dan Tools.
- **Export di collection**: tombol Export di header collection membuka panel Export di collection itu, tidak lagi pindah ke halaman lain. Format dipilih dari dropdown (Codestamp, Postman Collection v2.1, Docs as Markdown) dengan keterangan di bawahnya. Halaman Import kembali hanya untuk import; environment tetap diekspor dari halaman Environments.
- **History per collection**: tombol History di header collection membuka daftar request yang dikirim dari collection itu saja, dengan pencarian, filter status, Open, Save to collection, dan Clear history untuk collection itu. Halaman History global dihapus; tile History di Home membuka History collection yang terakhir dibuka, dan Settings › Data & history tetap bisa menghapus semua history.
- Import dibuka dari "+" (New collection), Home, Menu › File, dan Quick search.
- Versi aplikasi naik ke **0.3.4**. Skema database dan format repository tidak berubah.

Tahap 34: **panel collection sebagai dialog, nama section tebal, Tools di bawah**

- **Panel sebagai dialog**: Variables, Auth, Collaboration, Mock server, History, dan Export kini terbuka sebagai dialog di atas halaman, seperti Quick search, tidak lagi menyisip di antara header dan daftar request. Escape, klik di luar, atau Close menutupnya; Variables dan Auth tetap menanyakan dulu bila ada perubahan yang belum disimpan, dan Ctrl+S tetap menyimpan.
- **Nama section** di collection lebih tegas (14px, tebal, warna teks utama).
- **Grup Tools** di sidebar didorong ke paling bawah, di atas status bar.
- Perbaikan: dropdown di dalam dialog (misalnya Format di Export) tidak lagi menutup sendiri saat dibuka, karena kini hanya daftar pilihannya yang digulir.

Tahap 35: **peta langkah Orchestration**

- Timeline vertikal diganti **peta langkah** dari kiri ke kanan: Start, setiap langkah sebagai node ringkas (penanda status, nomor, cap method, nama, kode dan waktu), tombol bulat "+" (Add step), lalu End. Empat langkah muat tanpa gulir di jendela 1280px. Flow yang panjang digulir ke samping di dalam peta saja.
- Klik node (atau panah kiri/kanan saat fokus di node) untuk memilih langkah. Detail langkah terpilih tampil di bawah peta: _Uses_, _If this step fails_, tab Checks / Pass on / Response, dan _Last run_. Urutan diubah dengan _Move left_ / _Move right_.
- **Garis nilai**: di atas peta, garis menunjukkan nilai `{{nama}}` yang diterima langkah terpilih dan dari langkah mana, serta ke langkah mana nilai yang ia teruskan dipakai. Warnanya mengikuti run terakhir: abu sebelum run, hijau bila diteruskan, merah putus-putus bila tidak ditemukan.
- Setelah run yang berhenti karena langkah gagal, garis merah putus-putus di bawah peta menunjukkan langkah itu sampai End, dan langkah itu langsung terpilih.
- Codestamp membaca `{{nama}}` yang dikirim tiap request (URL, query, header, path variable, body, auth) saat flow dibuka atau disimpan, dengan fungsi yang sama yang dipakai saat mengirim. Skema database dan format repository tidak berubah.
- Peta dibuat dari HTML dan SVG biasa, tanpa canvas, dan hanya satu detail langkah yang dirender sekaligus.
- Versi aplikasi naik ke **0.3.5**.

Tahap 36: **Copy as code untuk delapan bahasa**

- Bahasa baru: **PowerShell** (`Invoke-WebRequest`, jalan di Windows PowerShell 5.1 dan PowerShell 7), **PHP Guzzle**, **Go `net/http`**, **Java `HttpClient`** (JDK 15+), dan **C# `HttpClient`** (.NET 7+). cURL, JavaScript `fetch`, dan Python `requests` ditulis ulang dengan susunan yang sama.
- Bahasa dan nilai dipilih lewat dua dropdown (_Language_, _Values_), tidak lagi berupa tombol berderet ke kanan. Bahasa terakhir diingat di perangkat ini; _Values_ selalu dimulai dari _Environment variables_ supaya secret tidak tampil tanpa diminta.
- **Secret dari environment variable**: `{{token}}` menjadi `os.environ["TOKEN"]`, `$env:TOKEN`, `getenv('TOKEN')`, `System.getenv("TOKEN")`, dan seterusnya, jadi kode yang disalin aman ditempel ke repository atau chat. Hanya variabel yang benar-benar dipakai yang dibaca.
- **Kode terstruktur**: import, nilai dari environment, URL dan query (satu pasangan per baris), header, body, kirim, lalu cetak status dan body. Body JSON dirapikan dan ditulis sebagai string multi-baris bawaan tiap bahasa (here-string, text block, raw string, heredoc).
- Basic auth dienkode saat kode berjalan, path variable `:id` diisi, urutan query dan form dipertahankan, dan karakter khusus tiap bahasa di-escape. Yang tidak didukung sebuah bahasa ditulis sebagai komentar: Digest di Go dan Java, AWS Signature di luar cURL, dan `-Form` yang butuh PowerShell 7.
- Diuji dengan menjalankan kode yang dihasilkan: `node e2e/codegen-e2e.mjs` menjalankan 6 kasus di 8 bahasa terhadap server lokal dan memeriksa request yang benar-benar diterima (Go dan PHP lewat Docker).

Tahap 37: **Copy as code untuk 15 bahasa**

- Bahasa baru: **HTTPie**, **JavaScript axios** (Node), **Kotlin OkHttp** (OkHttp 5), **Dart `http`** (Dart CLI dan Flutter), **Swift `URLSession`** (async/await; Linux lewat FoundationNetworking), **Ruby `Net::HTTP`** (tanpa gem), dan **Rust `reqwest`** (baris `Cargo.toml` di komentar pertama menyebut fitur yang dipakai: `blocking`, ditambah `query`, `form`, atau `multipart` bila perlu, karena reqwest 0.13 memisahkannya).
- Dropdown _Language_ diurutkan: alat shell, JavaScript, lalu bahasa lain.
- Variabel yang namanya sama dengan environment variable sistem (`{{user}}`, `{{path}}`, `{{home}}`, …) dibaca dari `USER_VALUE`, `PATH_VALUE`, dan seterusnya, supaya kode tidak diam-diam memakai nama login atau PATH.
- Hal khusus tiap bahasa yang ditangani: axios mengirim body teks apa adanya (tanpa `transformRequest`, axios merapikan atau meng-encode ulang JSON), HTTPie memakai `--ignore-stdin` agar tidak menunggu stdin, Kotlin menulis `$` sebagai `${'$'}` di raw string, Swift dan Java menulis multipart sendiri karena tidak punya builder-nya, dan Rust memakai `as_str()` di array query supaya tipenya seragam.
- `node e2e/codegen-e2e.mjs` kini menjalankan 15 bahasa. Image Docker tidak pernah diunduh oleh suite (`--pull never`); bahasa yang toolchain atau image-nya tidak ada ditandai SKIP.
- Versi aplikasi naik ke **0.3.6**.

Berikutnya: build untuk macOS dan Linux.

## Prasyarat (Windows)

- Node.js 24+
- Rust stable (via [rustup](https://rustup.rs)) dengan komponen `clippy` dan `rustfmt`
- Visual Studio Build Tools (MSVC) dengan workload "Desktop development with C++"
- WebView2 Runtime (sudah ada di Windows 11)

## Menjalankan

```bash
npm install
npm run tauri dev
```

Build release tanpa installer (hanya `codestamp-desktop.exe` di `src-tauri/target/release`):

```bash
npx tauri build --no-bundle
```

## Installer Windows

```bash
npm run installer
```

Perintah ini membuild `codestamp-cli` lalu Codestamp; bundler Tauri menyertakan semua binary di crate, jadi CLI ikut terpasang di samping aplikasi. Hasilnya `src-tauri/target/release/bundle/nsis/Codestamp_<versi>_x64-setup.exe`.

- **Kompresi LZMA**: installer jauh lebih kecil daripada ukuran terpasang.
- **Per pengguna, tanpa hak admin**: terpasang di `%LOCALAPPDATA%\Codestamp`, dengan shortcut di Start Menu (folder Codestamp) dan pilihan shortcut desktop.
- **Bahasa Inggris saja**: installer dan uninstaller selalu berbahasa Inggris, tanpa dialog pilih bahasa, apa pun bahasa Windows-nya. Sampai versi 0.3.6 installer juga punya bahasa Indonesia.
- **WebView2** diunduh dan dipasang diam-diam hanya bila belum ada (Windows 10/11 umumnya sudah punya).
- **codestamp-cli** dipasang di `%LOCALAPPDATA%\Codestamp` bersama `codestamp-desktop.exe`, dan folder itu ditambahkan ke PATH pengguna (`src-tauri/windows/hooks.nsh`). Terminal yang dibuka setelah instalasi bisa langsung menjalankan `codestamp-cli`. Saat uninstall, folder itu dihapus lagi dari PATH. Entri PATH lain, termasuk yang memakai `%VARIABEL%`, tidak diubah.
- Uninstall bisa sekalian menghapus data aplikasi (pilihan "Delete the application data"). Tanpa pilihan itu, data di `%APPDATA%\dev.codestamp.desktop` tetap ada untuk instalasi berikutnya.
- **Tanda tangan (self-signed)**: `npm run installer` menandatangani `codestamp-desktop.exe`, `codestamp-cli.exe`, uninstaller, dan `setup.exe` lewat `bundle.windows.signCommand` (`src-tauri/windows/sign.ps1`, signtool dari Windows SDK, SHA-256, timestamp RFC 3161 dari `http://timestamp.digicert.com`).
  - Sertifikatnya `CN=Codestamp` di store _Personal_ akun Windows yang membuild, dibuat sekali dengan `src-tauri/windows/new-signing-certificate.ps1`. Kunci privatnya tidak bisa diekspor dan tidak pernah masuk repository; bagian publiknya diekspor ke `src-tauri/windows/codestamp.cer`.
  - Tanpa sertifikat itu, `npm run installer` berhenti dengan pesan cara membuatnya.
  - `CODESTAMP_SIGNING_THUMBPRINT` memilih sertifikat lain (misalnya dari CA kantor nanti), `CODESTAMP_TIMESTAMP_URL` mengganti server timestamp (`none` untuk build offline), dan `SIGNTOOL` menunjuk signtool lain.
  - Self-signed tidak dipercaya Windows secara bawaan: di PC lain, nama penerbit tampil tidak terverifikasi dan SmartScreen tetap memperingatkan (_More info_ › _Run anyway_). Agar dipercaya di PC tim, IT memasang `codestamp.cer` ke _Trusted Root Certification Authorities_ dan _Trusted Publishers_ (lewat GPO atau Intune, atau `Import-Certificate` per PC). Distribusi lewat file share internal atau Intune juga tidak memicu SmartScreen.
- Naikkan versi di tiga tempat sekaligus sebelum membuat installer baru: `src-tauri/tauri.conf.json`, `package.json`, dan `src-tauri/Cargo.toml`.
- Installer dibuat dari folder build bawaan `src-tauri/target`, jadi jangan setel `CARGO_TARGET_DIR` saat menjalankan `npm run installer`. Git for Windows tidak ikut installer; Collaboration tetap membutuhkannya.
- **Rilis lewat GitHub Actions**: push tag `vX.Y.Z` menjalankan workflow _Release_ (`.github/workflows/release.yml`), yang membuild installer dan melampirkannya ke _draft release_ untuk ditinjau dulu. Bila secret `WINDOWS_CERTIFICATE` (file `.pfx` dalam base64) dan `WINDOWS_CERTIFICATE_PASSWORD` diisi, sertifikatnya diimpor dan installer ditandatangani; tanpa keduanya installer dibuat tanpa tanda tangan lewat `src-tauri/tauri.unsigned.conf.json`. Workflow _CI_ menjalankan `npm run verify` dan `npm audit` di setiap push ke `main` dan pull request.

### Pembaruan otomatis

Codestamp membaca `latest.json` dari server update, lalu mengunduh installer yang alamatnya tertulis di sana. Keduanya harus bisa diunduh lewat **HTTPS tanpa login** (updater tidak membawa akun atau token). Folder share Windows tidak bisa dipakai.

Sekali saja, setelah server siap:

```bash
npm run updates:setup -- https://server-anda/codestamp/latest.json
```

- Membuat kunci tanda tangan update di `%USERPROFILE%\.codestamp\updater.key` (password ditanyakan oleh Tauri CLI), kecuali file itu sudah ada. Lokasi lain: setel `CODESTAMP_UPDATER_KEY`.
- Menulis kunci publik dan alamat itu ke `plugins.updater` di `src-tauri/tauri.conf.json`, dan menyalakan `bundle.createUpdaterArtifacts`. File itu ditulis ulang dengan format `JSON.stringify` (indentasi 2 spasi).
- **Cadangkan `updater.key` beserta passwordnya.** Tanpa keduanya, Codestamp yang sudah terpasang tidak bisa menerima update lagi dan harus dipasang ulang manual. Kunci ini terpisah dari sertifikat code signing dan tidak boleh masuk repository.
- Satu kali lagi, pasang installer hasil build setelah langkah ini secara manual di setiap PC (versi sebelumnya belum punya alamat update). Sesudah itu update datang dari dalam aplikasi.

Setiap rilis:

1. Naikkan versi di tiga tempat (lihat di atas).
2. Build dengan kunci update (PowerShell):
   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.codestamp\updater.key"
   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Net.NetworkCredential]::new('', (Read-Host 'Update key password' -AsSecureString)).Password
   npm run installer
   ```
   Selain `setup.exe`, bundler menulis `setup.exe.sig` (tanda tangan yang terikat ke nomor versi).
3. Tulis `latest.json` di folder `src-tauri/target/release/bundle/nsis`:
   ```bash
   npm run updates:manifest -- --notes "Ringkasan perubahan"
   ```
   (`--notes-file catatan.txt` untuk catatan panjang, maksimal 4000 byte.)
4. Unggah `Codestamp_<versi>_x64-setup.exe` dan `latest.json` ke folder yang sama di server. Codestamp yang sudah terpasang menemukannya saat dibuka berikutnya, atau lewat _Check for updates_.

## CLI untuk CI

`codestamp-cli` menjalankan request HTTP dan GraphQL sebuah collection sesuai urutan notebook, lengkap dengan script dan test, seperti Run all. Database-nya hanya ada di memori selama run berjalan (dengan kunci enkripsi acak sementara), jadi data aplikasi di mesin itu tidak dibaca maupun diubah.

```bash
cargo build --release --manifest-path src-tauri/Cargo.toml --bin codestamp-cli
```

```bash
codestamp-cli run ./api-collaboration-repo --environment staging.postman_environment.json --var API_TOKEN --junit report.xml
```

- `<collection>`: folder repository Collaboration, atau file collection (Codestamp/Postman Collection v2.1, OpenAPI 3, Swagger 2).
- `-e, --environment <file>`: file environment hasil export Codestamp atau Postman.
- `--var <nama>=<nilai>` mengisi variabel environment; `--var <nama>` mengambil nilainya dari environment variable proses, sehingga secret dari CI tidak muncul di baris perintah.
- `-d, --data <file>` (CSV/JSON, satu iterasi per baris) atau `-n, --iterations <1-100>`.
- `--bail` berhenti setelah request pertama yang gagal; `--delay <ms>`; `--timeout <detik>` (1-300).
- `--junit <file>` (JUnit XML untuk CI) dan `--json <file>` (bentuk sama dengan Export report di aplikasi).
- Exit code: `0` semua lulus, `1` ada yang gagal, `2` run tidak bisa dimulai atau laporan gagal ditulis. Warna hanya untuk terminal dan dimatikan oleh `NO_COLOR`.
- WebSocket, MCP, dan gRPC dilewati dengan keterangan. Saat ini CLI dibangun bersama aplikasi Windows; build macOS/Linux menyusul bersama aplikasinya.

## Pemeriksaan kualitas

```bash
npm run verify
```

Menjalankan `svelte-check` (TypeScript strict), ESLint (+ typescript-eslint strict, plugin security), Prettier, `cargo fmt`, `cargo clippy -D warnings`, dan unit/integration test Rust.

Pengujian end-to-end (34 suite, 727 cek terhadap build release dan kode hasil Copy as code) ada di folder [e2e](e2e/README.md), dan alur setiap stage-nya dijelaskan di [docs/testing](docs/testing/README.md).

Audit dependency sebelum rilis:

```bash
npm audit
cargo install cargo-audit
cargo audit --file src-tauri/Cargo.lock
```

## Struktur

Panduan desain UI (token, komponen, aturan tulisan, aksesibilitas): [DESIGN.md](DESIGN.md).

```
src/                      UI (Svelte 5)
  lib/api/http.ts         Pintu ke command pengiriman request (tipe + invoke)
  lib/api/collections.ts  Pintu ke command penyimpanan collection/section/request
  lib/state/              State workspace (collection aktif, kartu terbuka, autosave)
  lib/request/            Model draft request, pembentukan input, warna method
  lib/format/             Helper format ukuran, durasi, JSON, dan pemotongan teks per byte UTF-8
  lib/editor/setup.ts     Konfigurasi CodeMirror (tema, bahasa, keymap)
  lib/editor/search.ts    Find (Ctrl+F) di editor: panel, jumlah temuan, opsi, dan pembuka dari luar editor
  lib/dom/reveal.ts       Gulir ke sebuah elemen dan pindahkan fokus ke sana (kartu dari Quick search, Docs)
  lib/dom/keys.ts         Shortcut simpan (Ctrl+S) dan pengecekan dialog terbuka
  lib/state/updates.svelte.ts  Cek dan pasang update (banner dan Settings › About)
  lib/components/         Komponen UI (kartu request, panel response, tabel, editor)
  lib/request/auth.ts     Pilihan auth, auth efektif (inherit), dan pratinjau header/param auth
  lib/request/body.ts     Mode body dan labelnya, nama file dari path
  lib/request/codegen/     Copy as code: model request, nama environment variable, dan satu generator per bahasa
  lib/api/cookies.ts      Pintu ke command cookie jar (daftar, tampilkan nilai, hapus)
  lib/request/order.ts    Urutan request sesuai notebook (dipakai daftar & runner)
  lib/state/runner.svelte.ts  Runner "Run all": urutan, aturan lulus, stop, laporan
  lib/capture/            Model isi gambar Capture (sudah tersamar) dan renderer canvas
  lib/privacy/secrets.ts  Satu tempat untuk mengenali dan menyamarkan nilai sensitif di layar & gambar
  lib/appearance/         Preset font dan rentang ukuran
  lib/api/team.ts         Pintu ke command collection tim (share, join, sync, riwayat, akun Git, Git changes)
  lib/state/team.svelte.ts  Sinkron manual (antrean per collection), status per collection, dan collection yang belum dikirim
  lib/team/status.ts      Ringkasan status sinkron untuk header dan panel Team
  lib/team/account.ts     Akun Git: kapan kolom Git account dipakai, isi yang dikirim, dan status cek akun
  lib/state/icons.ts      Katalog emoji collection, dimuat sekali dari Rust
  lib/state/preferences.svelte.ts  Settings Editor, Requests, Data: diterapkan langsung, disimpan setelah jeda
  lib/state/variable-names.svelte.ts  Nama variabel yang dikenal per collection, untuk mewarnai {{…}}
  lib/state/mcp.svelte.ts  Sesi MCP kartu yang terbuka: koneksi, persetujuan, daftar, panggilan, log
  lib/request/mcp-items.ts  Membaca daftar tools/prompts/resources dan membentuk argumen MCP
  lib/request/mcp-config.ts  Membaca konfigurasi `mcpServers` yang ditempel
  lib/state/flows.svelte.ts  Orchestration: daftar flow, flow terbuka (autosave), hasil run per langkah
  lib/state/catalog.svelte.ts  Catalog: daftar API, filter & urutan, API yang dibuka, simpan owner & tag
  lib/catalog/present.ts  Label & warna status kesehatan, format persen dan tren, filter, urutan, tag
  lib/state/docs.svelte.ts  Docs: collection yang dibuka, deskripsi yang sedang diedit, simpan
  lib/docs/               Markdown → pohon mdast (tanpa HTML mentah), baris dokumentasi request (nilai rahasia disamarkan), saringan pencarian Docs
  lib/format/plural.ts    "1 request", "2 requests": satu helper bentuk jamak
  lib/home/hub.ts         Isi Home: bagian, tile, contoh perintah, dan aksi tiap tile
  lib/request/kind-icons.ts  Jenis request yang bisa ditambahkan: label, warna, ikon, dan keterangan
  lib/format/markup.ts    Merapikan dan memeriksa XML/HTML, dan mengenali response XML/HTML
  lib/request/soap.ts     Content-Type SOAP, deteksi WSDL, dan pembacaan SOAP Fault
  lib/state/soap.svelte.ts  WSDL yang dimuat kartu SOAP (operasi per binding), di-cache per sesi
  lib/titlebar/app-menu.ts  Menu aplikasi native (File, Edit, View, Go, Help) dari title bar
  lib/state/sidebar.svelte.ts  Sidebar tersembunyi atau tidak, diingat di perangkat ini
  lib/flow/rules.ts       Pilihan Checks dan Pass on yang valid, placeholder, nilai awal
src-tauri/
  windows/hooks.nsh       Hook NSIS: menambah dan menghapus folder instalasi di PATH pengguna
  windows/sign.ps1        signCommand Tauri: menandatangani file dengan sertifikat CN=Codestamp
  windows/new-signing-certificate.ps1  Membuat sertifikat self-signed dan mengekspor codestamp.cer
src-tauri/src/
  request/validate.rs     Validasi whitelist: method, skema URL, header (anti header injection), ukuran body, timeout
  request/client.rs       Satu HTTP client bersama (cookie jar, multipart, file di-stream), baca response bertahap dengan batas preview
  request/uploads.rs      Daftar file yang dipilih lewat dialog; hanya file ini yang boleh di-upload
  cookies/                Cookie jar (RFC 6265 via cookie_store), simpan terenkripsi, command kelola cookie
  environment/dynamic.rs  Variabel dinamis `{{$guid}}` dkk.
  request/commands.rs     Command Tauri `send_http_request`
  request/rules.rs        Batas bersama (method, panjang URL, jumlah baris, ukuran body)
  storage/db.rs           Koneksi SQLite, migrasi skema (PRAGMA user_version)
  storage/repo.rs         Semua query SQL (selalu parameterized)
  storage/validate.rs     Validasi nama, ID (UUID), dan isi request sebelum disimpan
  storage/icons.rs        Ikon emoji collection: katalog (whitelist), aturan otomatis dari nama, hash cadangan
  storage/commands.rs     Command Tauri untuk collection, section, request
  environment/resolve.rs  Parser & substitusi {{variabel}} (tanpa ekspansi berulang)
  environment/vault.rs    Enkripsi secret (ChaCha20-Poly1305), kunci di Credential Manager
  environment/repo.rs     Query environment, variabel, environment aktif
  environment/lookup.rs   Mengisi variabel ke request sebelum divalidasi & dikirim; preview URL
  history/repo.rs         Mencatat, mencari (LIKE ter-escape), memangkas, dan menyimpan ulang history
  request/pipeline.rs     Alur kirim: auth efektif → pre-request script → variabel → auth → validasi → kirim → history → test script
  auth/                   Model auth (inherit/none/bearer/basic/API key), validasi, dan penerapan ke request
  script/pm.js            API pm.* (prelude JavaScript), hasil keluar hanya lewat JSON
  script/runner.rs        Sandbox QuickJS per eksekusi dengan batas memori, stack, dan waktu
  script/network.rs       Jembatan pm.sendRequest ke HTTP client bersama (timeout & validasi sama)
  import/postman.rs       Konversi Postman collection/environment menjadi rencana import (tanpa menulis)
  import/curl.rs          Tokenizer shell (bash, $'…', cmd ^) dan parser opsi cURL
  import/run.rs           Menulis rencana import secara transaksional
  websocket/connect.rs    Handshake ws/wss (TCP + rustls), validasi URL & header, batas waktu dan ukuran
  websocket/manager.rs    Satu task per koneksi: kirim/terima, close, event ke UI lewat Channel
  appearance/             Tema dan font & ukuran (validasi whitelist), zoom & tema jendela, import font (cek signature)
  export/                 Menyimpan PNG Capture, laporan JSON runner, dan export Postman (collection v2.1, environment) lewat dialog simpan sistem
  request/network.rs      Konfigurasi klien HTTP dari Settings (proxy, CA & client certificate per host, redirect, cookie, batas respons)
  preferences/            Settings Editor/Requests/Data, proxy (password terenkripsi), sertifikat, backup & restore, info aplikasi
  mcp/                    MCP client: protokol JSON-RPC, transport STDIO (tanpa shell) dan Streamable HTTP, sesi, persetujuan program
  files.rs                Membaca file pilihan pengguna dengan batas ukuran, nama tampilan dari nama file
  dialogs.rs              Dialog buka/simpan file native (plugin dialog resmi Tauri)
  encoding.rs             Base64 bersama (auth Basic, import)
  team/files.rs           Format repository (satu file per request dan per environment): tulis dari database, baca & validasi file dari repository
  team/git.rs             Menjalankan Git sistem (argumen array, tanpa prompt, batas waktu), validasi alamat & author, klasifikasi error
  team/merge.rs           Merge 3-way per field untuk collection.json dan file request
  team/sync.rs            Share, join, sinkron (commit → fetch → merge → push), stop syncing, pembersihan folder clone
  team/apply.rs           Menerapkan isi repository ke database dalam satu transaksi (path file & secret lokal dipertahankan)
  team/history.rs         Versi per request dari `git log`, perbandingan per field, restore
  update/                 Pembaruan otomatis: cek latest.json, unduh, periksa tanda tangan, jalankan installer dalam mode update
  error.rs                Format error tunggal ke UI: { code, message }
  request/path_vars.rs    Path variable `:nama`: nama di path URL dan pengisian ter-encode
  request/streams.rs      Server-Sent Events: parser WHATWG dan pembaca stream per response
  request/bodies.rs       Body response utuh yang disimpan sementara untuk Save body
  request/saved.rs        Request tersimpan → input kirim (dipakai CLI), sama dengan kartu request
  run_data/               File data CSV/JSON untuk data run
  catalog/                Catalog: metrik 7 hari dari history, aturan kualitas, skor kesehatan, run tersimpan, owner & tag
  docs/                   Docs: memuat dokumentasi satu collection, menyimpan deskripsi, render Markdown untuk export, pencarian nama dan deskripsi untuk Quick search
  storage/descriptions.rs Deskripsi Markdown collection/section/request: baca, tulis, validasi, salin
  sensitive.rs            Satu tempat untuk mengenali nama header/parameter yang membawa kredensial
  auth/oauth.rs           OAuth 2.0: grant, PKCE, token terenkripsi, refresh
  auth/digest.rs, aws.rs  Digest (RFC 7616) dan AWS Signature v4
  import/openapi.rs       OpenAPI 3 / Swagger 2 → rencana import
  storage/examples.rs     Examples per request
  mock/                   Mock server lokal dari examples (pencocokan method + path)
  grpc/                   Klien gRPC: deskriptor (reflection / .proto), codec dinamis, panggilan
  cli/                    codestamp-cli: argumen, memuat collection, menjalankan, laporan JUnit/JSON
  bin/codestamp-cli.rs    Titik masuk CLI
  flow/                   Orchestration: penyimpanan flow & langkah, validasi, Pass on & Checks, run berurutan dengan event per langkah
  text.rs                 Pemotongan teks UTF-8 bersama (pratinjau pesan, body langkah flow)
  legacy.rs               Nama lama dari versi Postcard dan pemindahan folder data serta database-nya
```

## Keamanan

- Semua input dari UI divalidasi ulang di Rust (whitelist method & skema `http`/`https` atau `ws`/`wss`, header tanpa karakter kontrol, batas ukuran body 10 MB, timeout 1-300 detik).
- Verifikasi sertifikat TLS selalu aktif (rustls + certificate store Windows) dan tidak ada pengaturan untuk mematikannya. CA certificate tambahan hanya menambah kepercayaan; file CA yang berisi private key ditolak.
- Password proxy dan private key client certificate disimpan terenkripsi seperti variabel secret; UI tidak pernah menerima isinya kembali.
- MCP STDIO: program dijalankan langsung tanpa shell (argumen sebagai array, jendela konsol disembunyikan) dan berhenti bersama sesinya. Sebelum dijalankan, Codestamp meminta persetujuan untuk program, argumen, dan environment persis yang akan berjalan (setelah variabel diisi); yang disimpan hanya hash SHA-256-nya. Perubahan apa pun, termasuk dari sinkron tim atau variabel yang diubah rekan, meminta persetujuan lagi. UI hanya boleh memanggil method MCP yang ada di whitelist (list, call, get, read, ping); permintaan dari server selain `ping` ditolak, dan pesan dibatasi 16 MB.
- Backup berisi semua data dengan nilai secret tetap terenkripsi; restore memeriksa bahwa file adalah database Codestamp yang utuh dan tidak lebih baru dari aplikasi.
- CSP ketat di `tauri.conf.json`; UI tidak memuat skrip atau font dari luar.
- Body dan header request/response tidak ditulis ke log.
- Dialog file dibuka dari sisi Rust; UI tidak diberi izin plugin dialog/fs sama sekali. File font, PNG, dan laporan JSON hanya ditulis ke lokasi yang dipilih pengguna (PNG, laporan) atau folder data aplikasi dengan nama UUID (font).
- Redirect lintas host tidak diikuti, sehingga kredensial tidak terkirim ke host lain.
- Upload hanya bisa membaca file yang dipilih pengguna lewat dialog sistem; path dari import, script, atau isi yang diubah ditolak sebelum file dibaca.
- Cookie jar disimpan terenkripsi seperti variabel secret; export tidak pernah menyertakan nilai secret maupun path file lengkap.
- OAuth 2.0: endpoint harus HTTPS (kecuali localhost), PKCE untuk authorization code, token dan refresh token disimpan terenkripsi, dan jendela login hanya menangkap redirect ke alamat callback yang diatur.
- Mock server hanya mendengar di `127.0.0.1`, dengan batas ukuran, waktu, dan jumlah koneksi; port di bawah 1024 ditolak.
- gRPC: file `.proto` hanya dibaca bila dipilih lewat dialog di perangkat ini; deskriptor dari server dibatasi jumlah filenya, dan pesan dibatasi 32 MB.
- Orchestration: semua aturan flow divalidasi ulang di Rust (ID, nama variabel, panjang path, pasangan check yang valid), langkah hanya bisa memanggil request HTTP/GraphQL yang ada, dan nilai yang diteruskan hanya hidup selama run.
- CLI: memakai database sementara di memori, tidak pernah menyentuh database aplikasi maupun Credential Manager; secret dari `--var <nama>` dibaca dari environment proses.
- Collection tim: Git dipanggil dengan argumen array (tanpa shell), `protocol.ext.allow=never`, tanpa prompt terminal, dan dengan batas waktu. Alamat repository dibatasi ke `https://`, SSH, atau path folder; `ext::`, `file://`, `http://`, dan alamat berisi kredensial ditolak. Semua file dari repository divalidasi seperti import (path dan ID dikenal, batas ukuran & jumlah, aturan request yang sama), dan ID milik collection lain tidak bisa diambil alih. Nilai secret dan path file tidak pernah masuk repository. Username akun Git per collection divalidasi dengan whitelist karakter lalu dikirim sebagai satu argumen `-c credential.username=…` hanya ke perintah jaringan; Codestamp tidak menyimpan password atau token apa pun untuknya.
