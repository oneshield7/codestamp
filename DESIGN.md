# DESIGN.md: panduan desain Codestamp

Panduan visual dan pola UI Codestamp: acuan saat menambah atau mengubah layar, baik untuk manusia maupun AI agent. Arah visualnya bernama **"Postmark"**. Sumber kebenaran token adalah `src/app.css`. Bila dokumen ini berbeda dengan kode, kode yang benar dan dokumen ini yang diperbarui.

## 1. Karakter

- **Alat kerja harian**, bukan halaman pemasaran: tenang, padat, mudah dipindai.
- **Monokrom "tinta"**: abu dingin dan putih. Latar tinta hampir hitam hanya untuk tombol aksi utama di dialog dan panel (Create collection, Duplicate, Save, Share), **tidak pernah untuk menu** (tile, toolbar, item sidebar). **Warna hanya untuk makna**: status (ok / warn / err), method HTTP ("cap pos"), dan ikon emoji collection.
- **Bentuk notebook**: collection adalah daftar langkah bernomor. Satu request terbuka sebagai kartu, sisanya baris ringkas.
- **Datar**: garis 1px dan latar yang sedikit berbeda lebih diutamakan daripada bayangan. Bayangan hanya untuk yang terangkat atau mengambang (item sidebar aktif, kartu terbuka, popover, dialog).
- **Tanpa animasi atau transition**; perubahan state langsung terlihat. Satu-satunya pengecualian adalah perpindahan dari layar pembuka ke aplikasi saat Codestamp dibuka (lihat _Layar pembuka_).
- **Hanya tema terang** (`color-scheme: light`). Semua warna lewat token, sehingga tema gelap kelak cukup mendefinisikan ulang token di `:root`.

## 2. Warna

### Tema

Empat pilihan di Settings › Appearance: **Light** (bawaan), **Dark** (grafit netral), **Dark Night** (hitam kebiruan untuk malam), dan **System** (mengikuti Windows, Light atau Dark). Tema dipasang lewat `data-theme` di `<html>`; elemen apa pun boleh memakai `data-theme` sendiri, seperti pratinjau di pemilih tema.

- Nilai di tabel bawah adalah tema Light. Dark dan Dark Night menimpa token yang sama di `src/app.css`; komponen tidak pernah tahu tema yang aktif.
- Kedua tema gelap memakai warna makna yang sama (status, method, variabel) dalam versi terang, dengan latar dan garis tembus pandang (`rgba`) sehingga cocok di atas kedua permukaan.
- `--on-accent` menjadi tinta gelap di tema gelap, karena `--accent` dan warna method di sana terang. Teks di atas warna method (Send, Run tool, tombol hapus) selalu memakai `--on-accent`.
- Title bar Windows ikut gelap untuk Dark dan Dark Night (`set_theme` di Rust).
- Semua pasangan teks/latar di ketiga tema lolos WCAG: teks ≥ 4,5:1, focus ≥ 3:1.

| Token             | Dark      | Dark Night |
| ----------------- | --------- | ---------- |
| `--bg`            | `#15171c` | `#070a12`  |
| `--surface`       | `#1c1f25` | `#0d121d`  |
| `--surface-muted` | `#21252c` | `#111827`  |
| `--border`        | `#2d313a` | `#1c2436`  |
| `--text`          | `#eceef2` | `#e1e7f5`  |
| `--text-muted`    | `#9ba1ac` | `#8a95ae`  |
| `--accent`        | `#eceef2` | `#e1e7f5`  |
| `--focus`         | `#6f9bff` | `#7f9fff`  |

### Permukaan dan garis

| Token              | Nilai     | Dipakai untuk                                                                                      |
| ------------------ | --------- | -------------------------------------------------------------------------------------------------- |
| `--bg`             | `#f3f4f6` | Latar aplikasi di balik sidebar dan konten                                                         |
| `--surface`        | `#ffffff` | Kartu, panel, dialog, input, item sidebar aktif                                                    |
| `--surface-muted`  | `#f9fafb` | Area sekunder di dalam surface: bingkai editor kode, nilai di diff, tile pilihan, pilihan terpilih |
| `--surface-sunken` | `#edeef1` | Hover tombol transparan, wadah segmented control, tombol nonaktif, chip netral, kotak ikon         |
| `--border-soft`    | `#ecedf0` | Pemisah di dalam kartu/panel (bawah header, baris tabel, baris tab)                                |
| `--border`         | `#e2e4e9` | Tepi kartu, panel, baris request, tombol quiet                                                     |
| `--border-strong`  | `#c9cdd4` | Tepi input, tombol secondary, kartu request terbuka, garis putus-putus                             |

### Teks

| Token          | Nilai     | Dipakai untuk                                                                    |
| -------------- | --------- | -------------------------------------------------------------------------------- |
| `--text`       | `#111318` | Judul, teks utama, tab aktif                                                     |
| `--text-2`     | `#383d47` | Teks isi dan label kontrol                                                       |
| `--text-muted` | `#5a606c` | Keterangan, metadata, header tabel (kontras ±6,3:1 di atas putih, lolos WCAG AA) |

### Tinta (aksen) dan fokus

| Token           | Nilai     | Dipakai untuk                                                                                 |
| --------------- | --------- | --------------------------------------------------------------------------------------------- |
| `--accent`      | `#111318` | Tombol aksi utama di dialog/panel (Create collection, Duplicate, Save, Share), badge langkah  |
| `--on-accent`   | `#ffffff` | Teks di atas `--accent`                                                                       |
| `--accent-soft` | `#e6e8ec` | Pilihan terpilih (versi riwayat, autocomplete), seleksi teks editor, target drop, banner info |
| `--accent-text` | `#111318` | Link (tinta, berat 500; garis bawah saat hover untuk link "Open … →")                         |
| `--focus`       | `#2f6bff` | **Hanya** penanda fokus: outline `:focus-visible` dan border input yang sedang difokus        |

### Semantik

Selalu dipakai bertiga (teks, latar, garis):

| Token                      | Nilai                             | Arti                                                                         |
| -------------------------- | --------------------------------- | ---------------------------------------------------------------------------- |
| `--ok` / `-bg` / `-line`   | `#0a7a4b` / `#e8f6ef` / `#b6e2cb` | Berhasil: status 2xx, tes lulus, tersinkron, "Created just now"              |
| `--warn` / `-bg` / `-line` | `#a44b07` / `#fdf1e6` / `#f1cba6` | Perlu perhatian: variabel belum terisi, konflik menunggu, offline, read-only |
| `--err` / `-bg` / `-line`  | `#c0261b` / `#fdecea` / `#f5c2bd` | Gagal atau destruktif: status 4xx/5xx, error, aksi hapus                     |

Warna semantik **selalu ditemani teks** ("200", "Tests 3/4", "Team · offline"). Jangan memakai warna sebagai satu-satunya penanda.

### Method HTTP ("cap pos")

Setiap method punya tiga token: `--method-<tone>` (teks), `--method-<tone>-bg`, `--method-<tone>-line`.

| Tone     | Teks      | Latar     | Garis     | Method                   |
| -------- | --------- | --------- | --------- | ------------------------ |
| `get`    | `#0a7a4b` | `#e8f6ef` | `#b6e2cb` | GET                      |
| `post`   | `#a44b07` | `#fdf1e6` | `#f1cba6` | POST                     |
| `put`    | `#1d5bd8` | `#eaf0fd` | `#bfd0f7` | PUT                      |
| `patch`  | `#6d3fd0` | `#f1ecfd` | `#d4c6f7` | PATCH                    |
| `delete` | `#c0261b` | `#fdecea` | `#f5c2bd` | DELETE                   |
| `other`  | `#383d47` | `#edeef1` | `#c9cdd4` | HEAD, OPTIONS            |
| `ws`     | `#0f7487` | `#e6f5f8` | `#b3dfe7` | Label `WS` (WebSocket)   |
| `mcp`    | `#7a5a00` | `#fbf4dc` | `#ecd99b` | Label `MCP` (server MCP) |
| `gql`    | `#a8156f` | `#fbe8f3` | `#f1bfdd` | Label `GQL` (GraphQL)    |
| `grpc`   | `#4f6b0a` | `#f1f6e1` | `#d3e2a8` | Label `gRPC`             |
| `soap`   | `#4338ca` | `#eef0fe` | `#c9cef8` | Label `SOAP`             |

Ambil tone lewat `methodTone()` / `methodColor()` (method HTTP) atau `stampTone()` (cap per jenis request) di `src/lib/request/methods.ts`, dan label cap lewat `stampLabel()` di `src/lib/request/kinds.ts`; jangan menulis warna method langsung. Semua kombinasi teks/latar di atas lolos 4,5:1, termasuk untuk label 11px.

### Variabel `{{…}}`

| Token                     | Nilai (Light)                     | Dipakai untuk                                                                                  |
| ------------------------- | --------------------------------- | ---------------------------------------------------------------------------------------------- |
| `--var` / `-bg` / `-line` | `#a3166b` / `#fbe9f4` / `#efbfdc` | Referensi `{{nama}}` yang dikenal (Globals, collection, environment aktif, variabel dinamis)   |
| `--err` / `-err-bg`       | (lihat Semantik)                  | Referensi yang tidak terdefinisi, plus garis bawah bergelombang dan tooltip "… is not defined" |

- Dipakai di kolom URL, tabel params/headers, field auth, editor body/pesan/argumen MCP, dan baris URL request yang tertutup.
- Satu sumber: `splitVariables()` / `variablePattern()` di `src/lib/format/variables.ts` (aturan nama sama dengan Rust), dan komponen `VariableText` untuk teks berwarna.
- Kolom input tidak bisa mewarnai sebagian teksnya, jadi salinan berwarna digambar di atas input (tanpa latar, `pointer-events: none`) dan teks input dibuat transparan; caret dan seleksi tetap milik input.

### Warna ikon (`--hue-*`)

Delapan warna untuk ikon judul: `blue`, `green`, `teal`, `violet`, `amber`, `orange`, `rose`, `slate`. Masing-masing punya `--hue-<nama>` (garis ikon) dan `--hue-<nama>-bg` (latar tile); tema gelap memakai versi terang dengan latar tembus pandang. Semua lolos 4,5:1 terhadap tile-nya dan permukaan di ketiga tema.

Cara pakai: beri `data-hue="blue"` pada ikon atau tile-nya. Atribut itu mengisi `--hue` dan `--hue-bg`, dan `color` mengikuti `--hue`, jadi `stroke: currentColor` ikut berwarna. Tile memakai `background: var(--hue-bg)`.

| Tempat                                                                                | Warna  |
| ------------------------------------------------------------------------------------- | ------ |
| Home, aksi "New collection", fitur Requests, tile/adder HTTP request                  | blue   |
| Environments, fitur Scripts and tests                                                 | green  |
| Import, Docs, aksi Import, fitur Environments, tile/adder WebSocket, Settings › Proxy | teal   |
| History, Catalog, aksi Join, fitur Team dan Appearance, Settings › Appearance         | violet |
| Cookies, tile/adder MCP server, Settings › Certificates                               | amber  |
| Orchestration, fitur History, adder section, Settings › About & shortcuts             | orange |
| Settings (sidebar dan judul), fitur Capture, Settings › Data & history                | rose   |
| Fitur Copy as code, panel "No account needed"                                         | slate  |

Satu tempat selalu satu warna: ikon sidebar dan ikon judul halamannya sama (`NavIcon`). Warna ikon tidak membawa arti status; status tetap memakai token semantik dan teks.

### Token pendukung

- Titik status: `--dot-ok` (environment aktif, segmen lulus Run), `--dot-err` (segmen gagal), `--dot-off` (Globals). Di pemilih environment hanya environment yang aktif yang hijau; environment lain dan "No environment" abu-abu.
- Bayangan: `--shadow-card`, `--shadow-raise`, `--shadow-segment` (segmen terpilih), `--shadow-pop` (popover, tooltip, picker), `--shadow-dialog`; backdrop dialog `--backdrop`.
- Syntax highlight: `--code-property`, `--code-keyword`, `--code-function`, `--code-string`, `--code-number`, `--code-atom`, `--code-comment`, `--code-punctuation`, `--code-active-line`.
- Pengecualian satu-satunya: gambar Capture (`src/lib/capture/render.ts`) punya palet terang dan gelap sendiri, karena gambarnya dipakai di luar aplikasi.

Warna baru ditambahkan sebagai token di `src/app.css` untuk ketiga tema lebih dulu, bukan sebagai hex di komponen. Jalankan pemeriksaan kontras setelah mengubah nilai.

## 3. Tipografi

**Font**

- UI: `--font-ui`, default **Geist Variable** (`@fontsource-variable/geist`, dibundel), fallback Segoe UI.
- Kode: `--font-code`, default **Geist Mono Variable** (`@fontsource-variable/geist-mono`, dibundel), fallback Cascadia Code dan Consolas.
- Emoji: `--font-emoji` (Segoe UI Emoji, lalu Apple/Noto Color Emoji); dipakai lewat utilitas `.emoji`.
- Font dibundel bersama aplikasi; tidak ada font dari internet (CSP ketat).
- Pengguna bisa mengganti font (Settings › Appearance), termasuk mengimpor font sendiri, jadi jangan bergantung pada metrik satu font tertentu.

**Ukuran yang bisa diatur pengguna**

- Ukuran UI (11-20, default 14) diterapkan sebagai **zoom jendela** (`uiFontSize / 14`). Komponen menulis ukuran dalam px pada basis 14px, lalu semuanya ikut membesar.
- Kode diatur terpisah: `--font-size-code` (default 13, rentang 10-24), `--code-line-height` (1.6, rentang 1.2-2.2), `--code-ligatures`.

**Skala**

| Ukuran / berat                      | Dipakai untuk                                                                      |
| ----------------------------------- | ---------------------------------------------------------------------------------- |
| 34px / 600, letter-spacing −0.025em | Judul Home                                                                         |
| 26px / 600, letter-spacing −0.01em  | Judul view dan nama collection (h1)                                                |
| 20px / 600                          | "Add your first request", nama request di mode fokus                               |
| 18px / 600                          | Judul dialog New collection                                                        |
| 17px / 600                          | Nama request di kartu, judul ConfirmDialog, "What you can do"                      |
| 16px / 600                          | Judul dialog dan panel halaman (Import)                                            |
| 15px / 600                          | Judul panel collection (Variables, Auth, Team, History, Export), brand, judul tile |
| 14px                                | Teks dasar dan input                                                               |
| 13px                                | Ukuran kerja paling umum: tombol, tab, tabel, isi panel, banner                    |
| 12px                                | Metadata, chip, hint, judul grup sidebar, URL dan nomor di baris                   |
| 11px                                | Cap method di baris request, `kbd` daftar shortcut                                 |

**Berat dan gaya**

- 400 untuk isi, 500 untuk label dan state aktif, 600 untuk judul, tombol utama, chip, dan cap method. 700 praktis tidak dipakai.
- Monospace (`.mono` atau `--font-code`) untuk URL, alamat repository, path, nilai di diff, kode, **nomor langkah** (`01`, `02`), jumlah di sidebar, durasi, dan kode status.
- Teks yang tidak muat: satu baris dengan ellipsis (nama, URL); `overflow-wrap: anywhere` untuk path atau alamat yang harus terbaca utuh.

## 4. Spasi, ukuran, dan layout

**Skala jarak (px):** 2 · 4 · 6 · 8 · 10 · 12 · 14 · 16 · 18, ditambah 20 / 24 / 28 / 40 untuk tingkat halaman. Gap yang paling sering dipakai: 10, 12, dan 8.

**Kerangka**

- Konten utama: padding `28px 24px 48px` (`20px 16px 40px` di bawah 900px), gap antarbagian 14px. Lebar maksimal 1080px, 1400px mulai jendela 1800px, dan 1720px mulai 2400px (mode fokus: 1180 / 1560 / 1880px), supaya monitor besar terpakai untuk request dan response berdampingan.
- Sidebar: lebar tetap 264px, padding `20px 14px`, selalu di samping konten (tidak pernah turun baris). Sidebar menempel (`position: sticky`, setinggi jendela) dan punya scroll sendiri.
- Header kartu/panel: padding `14px 16px`; isi panel 16px; header dan footer dialog `14px 18px` (dialog New collection `16px 22px`).

**Tinggi kontrol**

| Tinggi  | Dipakai untuk                                                         |
| ------- | --------------------------------------------------------------------- |
| 46px    | Tombol adder (garis putus-putus), URL field dan Send di mode fokus    |
| 44px    | Tile ikon collection di header                                        |
| 42px    | URL field dan tombol Send                                             |
| 38-40px | Input form, tombol ikon di dialog New collection, tombol emoji picker |
| 36px    | Tombol di dialog dan panel halaman                                    |
| 34px    | Tombol ikon di toolbar collection (34×34), input nama request         |
| 32px    | Tombol di panel collection, icon button                               |
| 28-30px | Sel tabel, select kecil, tombol "+" sidebar, tombol kecil picker      |
| 22px    | Kotak emoji collection di sidebar                                     |

**Responsif**

- Jendela didukung dari ukuran minimum aplikasi (720×520) sampai 4K. Di dalam konten, utamakan `flex-wrap` dengan `flex-basis` (mis. `flex: 1 1 280px`) supaya elemen turun baris saat jendela sempit.
- Media query hanya untuk kerangka: di bawah **900px** sidebar menjadi **rel ikon 64px** (nama item tetap untuk pembaca layar dan muncul sebagai tooltip), dan lebar konten bertambah pada 1800px dan 2400px.
- Kartu request memakai **container query** (`container-type: inline-size`): bila kartu lebih sempit dari 640px, nama request mendapat baris sendiri dan tombol header pindah ke baris kedua.
- Dialog adalah kolom flex dengan `max-height: calc(100vh - 32px)`: header dan footer (tombol aksi) selalu terlihat, hanya isi di tengah yang digulir atau menyusut. Jangan menaruh tombol aksi di dalam area yang digulir.
- Anak flex yang berisi teks panjang (`nowrap` + ellipsis) wajib diberi `min-width: 0`, termasuk panel di dalam dialog yang `display: flex`; tanpa itu teks melebarkan wadahnya.
- Dialog: `width: min(<N>px, calc(100vw - 32px))` dan `max-height: calc(100vh - 48px)` dengan isi yang bisa di-scroll. Lebar yang dipakai: 420 (konfirmasi), 580 (New collection), 760 (Copy as code), 860 (konflik), 920 (riwayat), 1180 (Capture). Popover: `min(372px, calc(100vw - 32px))`.
- Tabel lebar dibungkus `overflow-x: auto` dan diberi `min-width`.

## 5. Radius dan elevasi

| Radius               | Dipakai untuk                                                        |
| -------------------- | -------------------------------------------------------------------- |
| `--radius-sm` (6px)  | Icon button, segmen, cap method, kotak emoji sidebar, `kbd`          |
| `--radius-md` (8px)  | Tombol, input, URL field, banner, kotak status, item sidebar         |
| `--radius-lg` (12px) | Kartu, baris request, panel, dialog, tile, adder, tile ikon, popover |
| 999px                | Chip status, bar mode fokus                                          |
| 50%                  | Titik status                                                         |
| 18px                 | Kartu di mode fokus                                                  |

| Bayangan                                             | Dipakai untuk                    |
| ---------------------------------------------------- | -------------------------------- |
| `--shadow-raise` (`0 1px 2px` α 0.06)                | Item sidebar aktif ("terangkat") |
| `--shadow-card` (`0 1px 2px` + `0 12px 32px` α 0.08) | Kartu request terbuka            |
| `0 24px 64px` α 0.28                                 | Dialog                           |
| `0 16px 40px` α 0.18                                 | Picker emoji                     |
| `0 12px 32px` α 0.14                                 | Popover autocomplete             |
| `0 10px 40px` α 0.1                                  | Kartu mode fokus                 |
| `0 1px 2px` α 0.1                                    | Segmen terpilih                  |

## 6. Ikon

- **Ikon judul berwarna**: ikon sidebar, judul halaman, navigasi Settings, tile Home, tile collection kosong, dan tombol "Add …" memakai warna `--hue-*` (bagian 2). Ikon aksi kecil (toolbar, ikon kartu) tetap netral supaya warna tidak berebut perhatian.
- **Ikon antarmuka**: inline SVG, tanpa library ikon. `viewBox="0 0 24 24"`, umumnya 16px (14px di tombol kecil, 18-20px di tile, 28px di dropzone). `fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round`. Selalu `aria-hidden="true"`; tombol yang hanya berisi ikon wajib punya `aria-label` (+ `title` bila perlu penjelasan).
- **Logo**: `branding/codestamp-logo.svg` (logo 3a), ditampilkan 28px sebagai `<img alt="">` di samping nama "Codestamp". Kotak tinta `#111318` bersudut bulat berisi kurung kurawal putih, garis pemisah `#4a505c`, perangko merah `#d1432b`, dan tiga garis alamat putih. Warnanya tetap di semua tema karena logo yang sama menjadi ikon aplikasi (`src-tauri/icons`, dibuat ulang dengan `npx tauri icon branding/codestamp-logo.svg`).
- **Ikon collection**: emoji, lihat bagian 7. Emoji tidak dipakai sebagai ikon antarmuka di tempat lain.

## 7. Komponen

### Tombol

| Varian                       | Gaya                                                                                                    | Dipakai untuk                                                                                 |
| ---------------------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Primary (`.primary`)         | Latar `--accent` (tinta), teks `--on-accent`, 600, tanpa border                                         | Satu aksi utama di dialog atau panel: Save, Share, Create, Duplicate                          |
| Send (`.send`)               | Latar warna method yang dipilih (`--method-<tone>`, WebSocket `--method-ws`), teks putih, 600           | Mengirim request di kartu; semua kombinasi lolos kontras 4,5:1                                |
| Secondary (`.secondary`)     | Latar surface, border `--border-strong`                                                                 | Aksi pendamping di dialog/halaman: Cancel, Later, Choose another file                         |
| Quiet (`.quiet`)             | Latar surface, border `--border`, teks `--text-2`, 13px                                                 | Aksi bertulisan di panel dan halaman                                                          |
| Ikon toolbar (`IconButton`)  | 34×34, border `--border`, ikon 16px, tooltip; titik status opsional di pojok; varian `round` 40px bulat | Toolbar header collection, kepala section (Add …, Duplicate, Remove), tombol tambahan di Home |
| Danger                       | Konfirmasi: latar `--err`, teks putih. Pemicu: `.quiet.danger`, merah saat hover                        | Delete, Stop syncing                                                                          |
| Link (`.link`, `.go`)        | Transparan, tinta 500; `.go` ("Open … →") bergaris bawah saat hover                                     | Aksi ringan inline: Dismiss, Resolve, Copy, Open … →                                          |
| Icon (`.icon`)               | 32×32, transparan, hover `--surface-sunken`                                                             | Aksi kartu: Copy as code, History, Duplicate, Capture, Focus, Delete, Collapse                |
| Tool (`.tool`)               | Ikon + teks, border `--border-strong`                                                                   | Aksi di mode fokus (Capture)                                                                  |
| Adder (`.adder`)             | Tinggi 46px, border 1.5px dashed `--border-strong`, hover tinta                                         | Add request, Add WebSocket, Add section                                                       |
| Tile (`.tile`)               | Kartu besar berlatar surface: ikon, judul 15px, deskripsi 13px; tanpa latar tinta                       | Aksi cepat di collection kosong                                                               |
| Pill utama Home (`.primary`) | Pill 40px, latar `--text`, teks `--surface`, 600; hover `--text-2`                                      | Satu aksi utama tile Home yang dipilih                                                        |

- Disabled: latar `--surface-sunken` dengan teks/ikon `--text-muted` (di dialog lama: opacity 0.6, Send: 0.7). Jelaskan alasannya di tooltip atau `title` (mis. "Run all · add an HTTP request first").
- **Penanda "sudah diklik"**: setiap tombol yang membuka panel atau dialog memakai `aria-expanded` (plus `aria-haspopup="dialog"` untuk dialog) dan tampil **tertekan** selama yang dibukanya masih terbuka: border/garis `--text` dan latar `--accent-soft`. Berlaku untuk Variables/Auth/Team/Duplicate/Delete di toolbar collection, tombol "+" sidebar, tombol utama Home yang membuka New collection, tombol Duplicate section, dan ikon Copy as code/History/Duplicate/Capture/Delete di kartu. Jangan memakai latar tinta untuk penanda ini.
- Semua tombol memberi umpan balik saat ditekan (`:active`), termasuk aksi sekali jalan seperti Duplicate dan Export.
- Saat sibuk, label berubah menjadi kata kerja + "…": "Saving…", "Syncing…", "Creating…", "Joining…".
- Maksimal satu primary per area. Aksi destruktif tidak pernah primary dan selalu lewat ConfirmDialog.

### Input

- **Field form**: tinggi 38-40px, padding `0 12px`, border `--border-strong`, radius md. Label di atas input (13px, `--text-2`, gap 6px), hint 12px muted di bawah. Saat fokus, border `--focus`. Nilai tidak valid: `aria-invalid="true"`, border dan hint berwarna `--err`.
- **URL field**: grup 42px berisi dropdown method (warna dan latar cap method-nya, garis kanan) dan input. `:focus-within` mengubah border grup ke `--focus`.
- **Dropdown** (`Dropdown`): pengganti semua `<select>` bawaan, yang daftarnya tidak bisa diberi gaya dan tidak mengikuti tema. Tombol menampilkan pilihan, chevron, dan bila ada, titik status atau warna (method, transport MCP). Saat terbuka atau fokus keyboard, tepinya biru: border `--focus` plus cincin 1px (di URL field, cincin 2px ke dalam). Daftarnya popover (surface, border, radius md, `--shadow-pop`, maks 320px): item aktif `--accent-soft`, item terpilih tebal dengan centang biru. Membuka ke atas bila tidak muat di bawah, tertutup saat klik di luar, scroll, atau resize. Saat menampilkan pilihan aktif, hanya daftarnya yang digulir (bukan `scrollIntoView`), supaya dialog yang memuatnya tidak ikut bergeser dan menutup daftar itu. Keyboard: panah/Enter/Spasi membuka, panah/Home/End berpindah, Enter memilih, ketik huruf untuk melompat, Escape menutup tanpa memilih (tidak ikut menutup dialog atau mode fokus). Varian: `field` (40px, lebar penuh), `compact` (34px), `cell` (30px, tepi muncul saat hover), `chip` (30px, 12px), `method` (di URL field).
- **Edit di tempat** (`EditableText`, nama request, sel tabel): border transparan, muncul `--border` saat hover, lalu `--focus` dengan latar putih saat fokus.
- **Input variabel** (`VariableInput`): autocomplete setelah `{{` dalam popover (surface, radius md, bayangan popover, item aktif `--accent-soft`). Baris "Resolves to …" 12px di bawah URL, berwarna warn bila ada variabel yang kosong.
- **Secret** (`SecretInput`): nilai tampil sebagai `••••••••` sampai tombol Show ditekan.
- **Textarea kode**: latar `--surface-muted` dengan font kode.

### Segmented control, pilihan, dan tab

- Segmented: wadah `--surface-sunken`, padding 3px, radius md; segmen aktif berlatar putih, bayangan `0 1px 2px`, berat 500. Pakai `aria-pressed`.
- Jenis body: None, JSON, XML, HTML, Text, URL-encoded, Form data, Binary. JSON, XML, dan HTML punya highlight dan Beautify. XML yang rusak memberi hint `--warn` "Line N, column M: …" di bawah editor.
- **Kartu pilihan** (radio, mis. "How do you want to start?"): `<fieldset>` + `<legend>`; tiap pilihan `<label>` berisi radio native, kotak ikon 32px, judul 500, deskripsi 13px. Terpilih: border `--text`, latar `--surface-muted`.
- Pilihan yang panjang (lebih dari ±5) memakai `Dropdown`, bukan kartu radio, supaya panel tidak memanjang. Contoh: **Auth type** di tab Auth, dengan keterangan pilihan (mis. header yang ditambahkan) 12px muted di sampingnya.
- Tab: teks 13px muted, padding `10px 2px`, jarak 16px; tab aktif teks `--text`, 500, garis bawah 2px `--text` (bukan warna). Baris tab punya garis bawah `--border-soft` dan bisa di-scroll horizontal. Jumlah ditulis di label: "Cookies · 1".

### Chip, cap method, dan titik status

- **Chip**: pill, padding `2px 8px`, 12px 600, latar `--surface-sunken`. `.ok` dan `.err` memakai pasangan semantik. Contoh: kode status di baris request (mono), "Tests 3/4", "Created just now".
- **Cap method** (`.stamp`): `padding: 2px 7px`, border 1px, radius sm, font kode 11px 600, `letter-spacing: 0.04em`, warna dari tiga token tone-nya. Dipakai di baris request; select method di kartu memakai warna + latar yang sama.
- **Titik**: lingkaran 8px. Environment aktif hijau, Globals abu, status sinkron mengikuti tone (ok / warn / err).

### Toolbar ikon dan tooltip (`IconButton`)

- Header collection memakai satu baris tombol ikon dalam `role="group"` ("Collection actions"), dipisah garis vertikal per kelompok: **Run all** | **Variables, Auth, Collaboration, Mock server, History** | **Duplicate, Export, Delete**. Variables sampai History dan Export membuka panelnya sebagai dialog di atas halaman (lihat Panel collection).
- Setiap tombol: 34×34, radius md, border `--border`, ikon 16px; hover border `--border-strong`; ditekan `--surface-sunken`; terbuka (`aria-expanded`) border `--text` + latar `--accent-soft`; Delete memerah saat hover.
- **Tooltip** = label tombol (sekaligus `aria-label`): kotak putih kecil di bawah tombol (border `--border`, radius sm, bayangan `0 4px 12px` α 0.12, 12px 500), muncul saat hover dan saat fokus keyboard (bukan setelah klik mouse). Tombol paling kanan memakai `align="end"` dan tombol paling kiri `align="start"` supaya tooltip tidak terpotong tepi jendela.
- `placement="above"` membuka tooltip di atas tombol, untuk tombol di tepi bawah jendela (status bar). `current` memberi `aria-current="page"` dan tampilan tertekan (latar `--accent-soft`) pada tombol yang membuka halaman yang sedang terbuka.
- **Shortcut tidak pernah tampil di layar**: tidak di tooltip, label tombol, placeholder, petunjuk, menu, maupun `title`. Shortcut tetap bekerja; daftarnya hanya ada di Settings › About & shortcuts (`ShortcutList`). Untuk pembaca layar, shortcut ditulis di `aria-keyshortcuts`, bukan di `aria-label`.
- Status ringkas pindah ke label dan **titik 7px** di pojok tombol: Auth terpasang (titik `--text-2`), status sinkron Team (ok/busy/warn/err). Label lengkapnya ada di tooltip ("Team · synced", "Auth · on").
- **Kepala section** memakai pola yang sama (`role="group"`): satu tombol ikon per jenis request dengan ikon berwarna `data-hue` (Add request, Add WebSocket, Add GraphQL, Add gRPC, Add MCP server, Add SOAP), garis pemisah, lalu Duplicate section dan Remove section (merah saat hover). Daftar jenisnya dari `ADDABLE_KINDS` (`lib/request/kind-icons.ts`), sama dengan adder, tile collection kosong, dan Home.

### Ikon collection (emoji)

- Setiap collection punya emoji. **Otomatis dari nama** selama pengguna belum memilih: kata kunci di nama dicocokkan ke aturan (payment/billing → 💳, shop/cart/order → 🛒, user/account → 👤, auth/login/token → 🔐, tool/admin/internal → 🛠️, test/qa → 🧪, dan ±20 aturan lain). Bila tidak ada yang cocok, hash nama memilih dari kumpulan tetap (📮 🚀 🧭 🧩 🌱 🔭 🪐 🦊 🐙 🌊 🎯 🧰 📦 🛰️ 🍀 🔮), jadi nama yang sama selalu mendapat emoji yang sama.
- Pilihan pengguna disimpan di database (`collections.icon`; `NULL` = otomatis) dan **menang** atas yang otomatis. Mengganti nama hanya mengubah emoji yang otomatis.
- Di collection tim, pilihan itu ikut tersinkron lewat `collection.json` (kunci `icon`, tidak ditulis selama otomatis; format repository 2), dan konfliknya tampil sebagai "Collection icon".
- Katalog (58 emoji), aturan, dan kumpulan cadangan hanya ada di **Rust** (`src-tauri/src/storage/icons.rs`): katalog itu sekaligus whitelist; emoji lain ditolak. UI mengambil katalog lewat `collection_icons` dan saran lewat `suggest_collection_icon`.
- Tampilan: kotak 22px (`--surface-sunken`, radius sm, emoji 14px) di sidebar; tile 44px (border `--border`, radius lg, emoji 24px) di samping judul collection; tombol 40px di sebelah kolom nama pada dialog New collection.

### Picker emoji (`EmojiPicker`)

- Popover di bawah pemicunya (`role="dialog"`, `aria-label="Collection icon"`), lebar 372px, radius lg, bayangan picker.
- Isi: kolom cari (fokus otomatis; mencocokkan kata kunci emoji), baris **Suggested for “nama”** (6 emoji dari aturan yang cocok), grid **All** 8 kolom (tombol 38px, `aria-label` = nama emoji, terpilih: border `--text` + latar sunken), lalu footer: status ("Automatic: picked from the name" / "Chosen by you"), **Shuffle**, dan **Use automatic** (nonaktif bila sudah otomatis).
- Memilih emoji di grid langsung menyimpan dan menutup; Shuffle menyimpan tanpa menutup. Escape dan klik di luar menutup picker saja (tidak menutup dialog atau mode fokus di belakangnya).

### Layar pembuka (`index.html`)

- Jendela dibuka di tengah layar (`center()` di Rust).
- Ditulis langsung di `index.html`, jadi tampil sebelum script aplikasi jalan. Gayanya ada di `src/launch.css`, **bukan** `<style>` inline: style inline mendapat hash di CSP saat build, dan browser lalu menolak semua style yang disuntikkan belakangan (CodeMirror, atribut `style`). Latarnya `--bg` tema yang tersimpan: Rust membuka jendela tersembunyi, memberi jendela dan webview warna `--bg` itu (`paint_background`), memasang `data-theme` lewat _initialization script_ (`theme_script`), lalu menampilkan jendela. Tidak ada kilatan putih.
- Logo 84px di tengah, dikelilingi persegi bulat 104px (garis 3px, radius 25px): jalur `--border` dan satu ruas `--text` (22% keliling) yang berputar 1,4 detik per putaran selama aplikasi memuat.
- Aplikasi memberi tanda siap setelah daftar collection termuat (`finishLaunch` di `lib/dom/launch.ts`). Layar pembuka tampil minimal 0,5 detik sejak jendela terbuka, supaya start yang cepat tidak berkedip.
- Perpindahan: layar pembuka memudar 320ms dan logonya membesar ke 108% (260ms), sementara aplikasi masuk dari skala 96,5% dengan blur 6px dan opacity 0 (460ms, `cubic-bezier(0.16, 1, 0.3, 1)`). Hanya awal animasi yang ditahan, sehingga setelahnya `#app` tidak menyimpan transform atau filter yang bisa menggeser popover `position: fixed`. Layar pembuka dihapus dari DOM sesudahnya.
- `prefers-reduced-motion: reduce`: ruas tidak berputar (jalur berdenyut opacity), dan perpindahan hanya fade 150ms.
- Reload jendela (Ctrl+R) memakai tema yang dipilih di sesi itu (`sessionStorage` `codestamp.theme`), bukan tema saat aplikasi dibuka.
- Template peninjauannya ada di `branding/templates/launch-and-access.html`.

### Title bar (`TitleBar`)

- Menggantikan title bar Windows (`decorations: false`). Tinggi `--titlebar-height` (36px), latar `--bg`, garis bawah `--border`. Sidebar ada tepat di bawahnya.
- Jendela sendiri tidak pernah digulir (`html, body { overflow: hidden }`), karena scrollbar jendela akan memanjang sampai ke samping tombol Minimize, Maximize, dan Close.
  - `#app` adalah kolom flex setinggi jendela: title bar (`flex: none`), lalu `.scroller` (`flex: 1; overflow-y: auto; position: relative`) yang menggulir semua konten. Scrollbar-nya mulai tepat di bawah title bar. `position: relative` wajib: tanpa itu, elemen `position: absolute` di dalamnya (label `visually-hidden`) memanjangkan jendela, dan `scrollIntoView` ikut menggeser title bar ke atas.
  - `.shell` di dalamnya mengisi area itu (`min-height: 100%`). Elemen yang menempel di dalam konten memakai `top` relatif ke `.scroller` (sidebar `top: 0`, indeks Docs `top: 16px`), dan target gulir memakai `scroll-margin-top: 16px`.
- Kiri: dua `IconButton` varian `quiet` (32×28, tanpa border, hover `--accent-soft`): **Menu** (`aria-haspopup="menu"`, tertekan selama menu terbuka) dan **Collapse/Expand sidebar** (`align="start"`, supaya tooltip-nya mulai dari tepi tombol dan tidak terpotong tepi kiri jendela). Keduanya `keepFocus`: klik mouse tidak memindahkan fokus keyboard, supaya perintah Edit dari menu mengenai kolom yang sedang diedit.
- Tengah: area kosong `data-tauri-drag-region` untuk menggeser jendela; double-click untuk maximize/restore.
- Kanan: Minimize, Maximize/Restore, dan Close, digambar seperti tombol caption Windows 11 (46px, ikon garis 1px 10×10, hover `--accent-soft`). Close saat hover memakai merah Windows `--window-close-bg` dengan teks putih (5,66:1).
- Menu memakai menu native Windows (`@tauri-apps/api/menu`), sehingga keyboard, aksesibilitas, dan perintah Edit ditangani Windows. Modul menunya dimuat saat pertama dibuka. Hanya item yang benar-benar bekerja yang ditampilkan. Item menu tidak menulis accelerator; shortcut-nya tetap bekerja.
- Fokus keyboard: cincin `--focus` seperti tombol lain. Di tombol jendela yang menempel ke tepi, offset-nya ke dalam (`-2px`).

### Sidebar

- Brand (logo + "Codestamp"), item **Home**, grup **Collections** dengan tombol "+" (`aria-label="New collection"`), daftar collection (emoji, nama, jumlah mono), lalu grup **Tools** (Orchestration, Catalog, Docs, Git changes) yang didorong ke paling bawah sidebar (`margin-top: auto`; saat daftar collection panjang, Tools tepat di bawahnya). Tiap grup berupa `role="group"` dengan `aria-label` judulnya. Settings, Recently deleted, Cookies, dan Environments ada di status bar; Import dibuka dari "+" (New collection), Home, Menu › File, dan Quick search; History ada di tiap collection.
- Judul **Tools** bisa diklik untuk melipat grupnya (`FoldButton` dengan teks, `aria-expanded`, chevron ke kanan saat terlipat). Lipatan diingat per perangkat (`localStorage` `codestamp.foldedNavGroups`). Rel ikon selalu menampilkan semua item.
- Rel ikon 64px (class `rail`): dipakai di bawah 900px, dan saat pengguna menciutkan sidebar. Logo, tombol cari, ikon tiap item, dan garis pemisah antargrup; judul grup dan jumlah disembunyikan, nama item hanya untuk pembaca layar dan tooltip.
- **Tooltip rel**: nama item muncul di kanan ikon saat hover atau fokus keyboard (`:focus-visible`), dengan gaya tooltip `IconButton` (surface, border, radius sm, `--shadow-pop`, 12px 500), tengahnya sejajar tengah item. Tooltip ini `position: fixed` di luar `nav`, karena sidebar yang bisa di-scroll akan memotongnya; hilang saat pointer keluar, fokus pindah, atau sidebar di-scroll. Nama collection yang panjang dibungkus (maks 280px), tidak dipotong. Tombol "+" memakai tooltip yang sama di sidebar penuh. Tooltip hanya tampil di tata letak tempat ia muncul, jadi melebarkan atau menciutkan sidebar saat fokus masih di item menghapusnya. Item rel tidak memakai `title`, supaya tidak ada dua tooltip.
- Item aktif "terangkat": latar `--surface`, border `--border`, `--shadow-raise`, berat 500. Hover: latar `--surface-sunken`.
- Tanpa collection: teks muted "No collections yet."

### Status bar (`StatusBar`)

- Garis memanjang selebar jendela di paling bawah, di bawah sidebar dan konten (seperti status bar Bruno). Tinggi `--statusbar-height` (30px), latar `--bg`, garis atas `--border`. Tinggi sidebar, indeks Docs, Git changes, dan Home dikurangi tinggi ini.
- Kiri: empat `IconButton` `quiet` dengan ikon 15px satu warna (`currentColor`), urut **Settings** (gerigi), **Recently deleted**, **Cookies**, **Environments**. Tooltip di atas tombol (`placement="above"`; yang pertama `align="start"`). Tombol halaman yang terbuka memakai `current` (tertekan, `aria-current="page"`).
- Kanan: versi Codestamp yang sedang berjalan ("v0.3.3", 12px `--text-muted`, angka tabular) dari `getVersion()`.
- `<footer aria-label="Status bar">` dengan grup `role="group"` "Settings and data". Disembunyikan selama mode fokus, seperti sidebar.
- Bisa diciutkan ke rel ikon (tombol di title bar, View › Collapse sidebar, atau Ctrl+B), bukan disembunyikan, supaya navigasi selalu terlihat. Pilihannya diingat di `localStorage` perangkat ini; tanpa storage, toggle tetap bekerja selama aplikasi terbuka. Di bawah 900px tombolnya nonaktif dengan tooltip "Widen the window to expand the sidebar".

### Home

- Tampil saat dipilih dari sidebar, dan otomatis bila belum ada collection. Bentuknya **hub**, mengisi tinggi jendela, dengan isi dari `lib/home/hub.ts`.
- **Baris atas**: tab pill (`role="tab"`) Start, Requests, Testing, Workspace di wadah surface ber-border; tab aktif `--accent-soft`. Di kanan, collection tujuan ("Collection **nama**" atau "No collection yet").
- **Tile**: tombol 72px, radius lg, latar `--hue-bg`, ikon 30px `--hue`, nama di tooltip. Di belakangnya radio native (`aria-label` = nama tile), jadi panah keyboard berfungsi. Terpilih: border `--hue` plus cincin 2px `--hue` berjarak 3px, sedikit membesar.
- **Panel (stage)**: surface, border, radius lg, min 340px, isi rata bawah-kiri. Ikon tile digambar besar (220px, garis 1, opacity 0.16) di kanan atas, dan disembunyikan bila panel di bawah 900px (container query) agar tidak berada di belakang teks. Isinya: fakta singkat 13px berwarna `--hue`, judul 34px/600, deskripsi 15px `--text-2`, pill contoh (bila lebih dari satu perintah), kotak perintah (`$` muted, kode mono, tombol Copy), lalu pill utama dan tombol ikon bulat tambahan. Status salin 12px (`role="status"`).
- **Tooltip tile**: nama tile muncul hanya saat hover pada tile yang **belum** dipilih. Tile terpilih sudah tampil sebagai judul panel, dan karena tile itu sedikit membesar, tooltip-nya akan tertutup panel.
- **Gerak** (semua mati bila `prefers-reduced-motion`):
  - satu pill `--accent-soft` bergeser ke tab terpilih (280ms, mengikuti posisi tab lewat `ResizeObserver`);
  - tile bagian baru masuk naik 10px satu per satu (jeda 45ms);
  - teks panel naik 8px baris demi baris (fakta, judul, deskripsi; jeda 50ms), lalu pill contoh dan kotak perintah;
  - ikon besar muncul dari skala 0.82 dan rotasi −8° dalam 480ms.
  - Animasi memakai properti `translate` / `scale` / `rotate` terpisah supaya tidak bentrok dengan `transform` tile terpilih.
- **Keyboard**: ←/→ memilih tile (di tab: pindah bagian), Enter menjalankan tombol utama, 1-4 memilih bagian. Hanya aktif bila fokus ada di halaman atau di dalam Home, dan tidak saat dialog terbuka. Tidak ada baris petunjuk tombol di bawah panel.
- Tombol utama mengikuti keadaan: "Add GraphQL to Shop API", "Run Shop API", atau "Create a collection first" bila belum ada collection.
- Hanya fitur yang benar-benar ada yang boleh ditampilkan. Tidak ada login/akun di Codestamp; teamwork masuk lewat Git.

### Dialog Duplicate

- Setiap duplikat (collection, section, request) lewat dialog dulu, tidak langsung menyalin. Lebar 460px; judul "Duplicate collection/section/request", subjudul "Make a copy of “nama”".
- Kolom **Name of the copy** terisi "nama copy" (dipendekkan agar muat 120 karakter) dan langsung terseleksi. Validasi langsung: wajib diisi, maksimal 120 karakter, dan tidak boleh sama (tanpa membedakan huruf besar/kecil) dengan nama tetangganya: collection lain, section lain di collection yang sama, atau request lain di collection yang sama. Pesan salah tampil di bawah kolom dan tombol Duplicate nonaktif sampai beres. Rust memeriksa nama lagi (`clean_name`).
- Kotak info menjelaskan apa yang ikut tersalin. Enter = Duplicate, Escape = batal.

### Dialog New collection

- Dibuka dari "+" di sidebar dan tombol di Home (dengan pilihan awal sesuai tile: New collection, Import, Join).
- Kartu pilihan "How do you want to start?": **Empty collection** (Name wajib, tombol ikon emoji di sebelahnya, Base URL opsional yang disimpan sebagai variabel collection `{{baseUrl}}`), **Import from Postman** (tombol "Choose a file…" lalu pratinjau di halaman Import), **Join a team collection** (alamat repository + nama/email Git).
- Base URL divalidasi di UI (harus `http://` atau `https://`) dan diperiksa lagi di Rust; nama dan ikon juga diperiksa di Rust.

### Collection kosong (`CollectionStart`)

- Menggantikan adder saat collection belum punya request. Meta header berbunyi "No requests yet · saved automatically", plus chip "Created just now" untuk collection yang baru dibuat.
- Panel **Add your first request** dengan satu tile per jenis request (HTTP, WebSocket, GraphQL, gRPC, MCP server, SOAP) dan Paste a cURL command (→ halaman Import dengan collection ini sudah terpilih), lalu kartu **Also ready in this collection**: variabel `{{baseUrl}}` (bila ada), Sections, dan Work with your team.

### Kartu request (terbuka)

- Border 1px `--border-strong`, radius lg, `--shadow-card`.
- Header: badge langkah (26px, bulat, latar tinta), nama 17px yang bisa diedit, status simpan 12px muted ("Saved"; merah bila gagal), lalu icon button.
- Baris target: URL field dan tombol Send berwarna method yang dipilih (ikut berubah saat method diganti). Panel request dan response bisa bertumpuk (dipisah garis putus-putus) atau berdampingan (`flex-wrap`, basis 420px).
- Kartu yang dibuka dari luar daftar (Quick search, panel History, Catalog, Docs, Orchestration, Run all, Import) digulir ke atas area konten dengan `revealElement` (`scroll-margin-top: 16px`) dan menerima fokus (`tabindex="-1"`). Kartu yang dibuka dari barisnya sendiri tidak menggulir halaman.

### Baris request (tertutup)

- Tombol selebar penuh, grid `24px 64px 1fr auto`, padding `12px 16px`, border `--border`, radius lg.
- Isi: nomor langkah mono muted (`01`), cap method, nama (500) dan URL (mono 12px muted), lalu durasi mono muted + chip kode status (atau "Not run yet").
- Target drop ditandai `box-shadow: 0 -3px 0 var(--accent)`. Bisa diseret, atau dipindah dengan Alt+↑/↓.

### Kepala section

Tombol lipat (chevron, `aria-expanded`), handle titik-titik untuk drag, nama `EditableText` 14px 700 `--text` (lebih tegas dari baris request di bawahnya), jumlah request 12px muted, lalu link aksi di kanan.

- **Melipat section** menyembunyikan request-nya supaya daftar ringkas; jumlahnya tetap terlihat dan nomor langkah request lain tidak berubah. Chevron menunjuk ke kanan saat terlipat.
- Status lipat disimpan per perangkat (`localStorage`, hanya id section) dan tidak ikut sinkron tim. Chevron lipat di collection, Docs, dan sidebar adalah satu komponen (`FoldButton`), dan semua lipatan disimpan lewat `RememberedIds`.
- Section terbuka sendiri saat salah satu request-nya dibuka (dari History, duplikat, Run) atau saat request dipindah ke dalamnya dengan Alt+↑/↓. Saat terlipat, request bisa diseret ke kepala section-nya.

### Panel collection (Variables, Auth, Collaboration, Mock server, History, Export)

- Dibuka sebagai dialog modal di atas halaman (`PanelDialog`), seperti Quick search: `<dialog>` dengan `showModal()`, lebar `min(880px, 100vw - 32px)`, margin atas 10vh, tinggi maksimal `90vh - 16px`, radius 14px, `--shadow-dialog`, latar `--backdrop` tanpa blur. Muncul naik 10px dari skala 0,98 sambil memudar (240 ms, kurva yang sama dengan Quick search dan tile Home; dimatikan oleh `prefers-reduced-motion`). Halaman di belakangnya tidak bergeser.
- Tombol di header collection tetap memakai `aria-expanded` dan tampil tertekan selama dialognya terbuka. Escape, klik di latar, atau Close menutupnya; panel yang bisa diedit menanyakan dulu bila ada perubahan yang belum disimpan.
- Isi dialog di-scroll di dalamnya, dan kepala panel (judul, aksi, Close) tetap di atas (`position: sticky`). Dialog konfirmasi dan dialog lain yang dibuka dari panel tampil di atasnya.
- Header `14px 16px` berisi judul 15px, deskripsi 13px muted, dan tombol aksi, dipisah garis `--border-soft`. Bingkai panel sendiri (border, radius) dihapus di dalam dialog karena dialognya yang menjadi kartu.
- Ctrl+S di Variables dan Auth menyimpan selama fokus keyboard ada di dalam dialog panel itu, bukan di dialog lain yang terbuka di atasnya.
- Panel yang bisa diedit memakai `CollectionPanel`: tombol Save/Saved, konfirmasi buang perubahan, dan _leave guard_ yang mencegah pindah collection selama masih ada edit.
- Collaboration: akun ditampilkan sebagai baris `dl` (_Signs in as_ hanya untuk https, _Your changes as_), dengan tombol secondary **Change account** di bawahnya. Form akunnya menggantikan kedua baris itu di dalam kotak bergaris `--border-soft` (radius md, padding 14px, judul h3 13px "Account for “nama”"), lalu Cancel dan Save di kanan.
- Settings › Git accounts: satu `.panel` dengan judul, hint, dan tombol Add account di kanan. Form tambah berada di kotak bergaris `--border-soft`. Tiap akun berupa baris grid (nama mono + host, chip, tombol) dipisah garis `--border-soft`; di bawah 560px (container query) isinya bertumpuk. Chip memakai pasangan semantik dan selalu bertulisan: jumlah collection (accent-soft), _Signed in_ (ok), _Sign-in needed_ (warn), _Check failed_ (err), _Offline_ (sunken).
- Git changes: bar atas (Dropdown collection, chip warn untuk perubahan lokal dan commit belum di-push, teks muted waktu sinkron, tombol secondary), lalu tab bergaris bawah (`role="tablist"`, panah kiri/kanan berpindah tab, tab aktif bergaris `--accent` 2px). Panel tab adalah kotak surface radius lg. Bila ada collection bersama, halaman setinggi jendela (`height` dari `--titlebar-height` dan padding `main` `--main-pad-top`/`--main-pad-bottom`, minimal 480px): judul, bar, dan tab tetap di tempat, panel yang di-scroll. Di atas 760px (container query) daftar commit dan detailnya berdampingan 2:3, masing-masing dengan area scroll sendiri (padding 4px agar cincin fokus tidak terpotong); di bawahnya panel di-scroll utuh. Titik commit 10px `--accent`; merge berupa lingkaran 2px. Chip perubahan file: _Added_ (ok), _Updated_ (accent-soft), _Removed_ (err).
- `FieldChanges` adalah tabel tiga kolom (Field, sebelum, sesudah) dengan nilai mono di kotak `--surface-muted`, dipakai dialog riwayat request dan detail commit.
- Share ke repository yang sudah berisi: kotak `--warn-bg` bergaris `--warn-line` (radius md, padding 14px) dengan judul h3 13px, penjelasan, daftar file mono 12px di kotak `--surface` (tinggi maksimal 180px, bisa digulir), lalu Cancel dan tombol merah "Replace files and share" (`--err` dengan teks `--on-accent`, sama seperti tombol aksi ConfirmDialog).
- Status _sign in_ (err) di panel membawa tombol secondary **Sign in** di dalam kotak status (posisi yang sama dengan Resolve), hanya untuk alamat `https://`. Selama Git menunggu login, tombolnya nonaktif bertuliskan "Signing in…". Penolakan tampil di `.error` panel.
- Status _not synced_ (warn) tampil di header dan panel selama ada perubahan tersimpan yang belum dikirim dengan Sync now. Tidak ada sinkron otomatis.
- **Access groups** (`AccessSection`) ada di bawah teks undangan, dipisah garis `--border-soft`: judul h3 13px, hint 13px muted, lalu baris `dl` _This device_ berisi fingerprint mono 12px (`XXXX-XXXX-XXXX-XXXX`) dengan link Copy. Baris itu baru muncul setelah collection punya grup, karena membacanya membuat kunci perangkat.
  - Tiap grup berupa kotak bergaris `--border-soft` (radius md, padding 14px): nama h4 13px 600 dan chip bertulisan _You are a member_ (ok), _No access_ (sunken), atau _Locked, needs a check_ (warn). Di bawahnya teks 12px muted "Holds token, apiKey (Staging)." dan daftar anggota (nama, email muted, fingerprint mono, tombol kecil Remove atau Leave).
  - _Asking for access_ (h5) mendaftar permintaan dengan fingerprint-nya. Anggota mendapat petunjuk 12px untuk mencocokkan fingerprint lewat chat atau telepon, lalu tombol Decline (kecil) dan Grant (secondary). Permintaan perangkat ini sendiri bertombol Withdraw.
  - Grup yang gagal diperiksa menampilkan kotak `--warn-bg` dengan alasannya dan tombol secondary **Repair group** bila perangkat ini pernah memeriksa versi sebelumnya sebagai anggota.
  - Setiap aksi langsung menjalankan Sync now. Remove meminta konfirmasi (ConfirmDialog) yang menjelaskan bahwa nilai yang sudah dilihat tetap diketahui orang itu, lalu kotak `--ok-bg` mendaftar variabel yang perlu diganti di sumbernya.
  - Form **New group** (label, input 32px, tombol secondary Create group) ada di bawah daftar grup.
- `AccountFields` dipakai di Share, form akun, dan kedua form Join. Kolom _Git account (optional)_ hanya muncul untuk alamat `https://`; kata "(optional)" memakai `--text-muted`. Hint 12px di bawahnya menjelaskan arti kolom kosong.

### Halaman Import

- Judul **Import** (ikon `import`, teal) dan penjelasan yang menyebut bahwa Export ada di tiap collection. Panel: kartu dengan `flex: 1 1 400px`, padding 18px, gap 14px, judul h2 16px, hint 12px muted.
- Isinya: file (Codestamp, Collection v2.0/v2.1, environment, OpenAPI, Swagger, WSDL), Paste cURL, dan Join a shared collection. Pratinjau file Codestamp menambah statistik _environments_ bila ada.

### Panel collection: Export dan History (`ViewPanel`)

- Cangkang bersama `ViewPanel` di dalam `PanelDialog`: kepala berisi judul 15px/600, deskripsi 13px muted, aksi opsional, dan tombol **Close**; isi dengan padding 16px dan gap 12px.
- **Export** (`CollectionExport`): `Dropdown` **Format** (varian `field`, maks 520px) berisi _Codestamp_, _Postman Collection v2.1_, dan _Docs as Markdown_, dengan keterangan pilihan 12px muted di bawahnya (seperti Auth type). Tombol utama **Export…** membuka dialog simpan. Hasil (`ExportOutcome`): kotak `--ok-bg` "Exported to <path mono>" dengan catatan `--text-2` (nilai secret dikosongkan, grup akses tidak ikut), atau kotak `--err` bila gagal. Environment diekspor dari halaman Environments.
- **History** (`CollectionHistory`): hanya request yang dikirim dari collection itu, terbaru dulu. Aksi **Clear history** (quiet, merah saat hover, ConfirmDialog; nonaktif saat filter aktif atau daftar kosong) hanya menghapus history collection itu. Kolom cari (36px, border biru saat fokus) dan pill status (All, 2xx, 3xx, 4xx, 5xx, Failed). Daftar per hari (h3 13px) dalam area scroll maks 420px: jam, method mono berwarna, nama + URL mono, chip status, durasi · ukuran, **Open** (membuka kartu request dan menggulir ke sana) atau **Save to collection** bila request-nya sudah tidak ada, dan ✕. Kosong: "Nothing sent from this collection yet." / "No requests match." Settings › Data & history tetap menghapus semua history.
- Tile History di Home membuka panel History collection yang terakhir dibuka ("Open History of …"), atau "Create a collection first".
- File Codestamp: `{"codestamp": "collection", "files": {…}}`, berisi file repository Collaboration di bawah path-nya. Secret dan nilai restricted dikosongkan, path file lokal tidak ikut, grup akses tidak ikut. Saat diimpor, semua bagian mendapat id baru, jadi file bisa diimpor di samping collection asalnya.

### Dialog

- `<dialog>` native dibuka dengan `showModal()`, `aria-labelledby` menunjuk ke judul. Escape memicu `oncancel`, yang memanggil fungsi penutup komponen.
- Struktur: header (judul 16-18px, deskripsi 13px muted, tombol ✕ 32px), isi yang bisa di-scroll, lalu footer (status/catatan di kiri; aksi di kanan, secondary lalu primary). Antarbagian dipisah garis `--border-soft`.
- ConfirmDialog: lebar 420px, judul berupa pertanyaan ("Delete?", "Stop syncing?"), pesan yang menjelaskan akibat, lalu Cancel dan tombol merah berlabel aksinya.

### Copy as code (`CodeDialog`, `lib/request/codegen/`)

- Dialog 760px: header (judul "Copy “nama” as code", ✕), baris pilihan, penampil kode (`CodeEditor` read-only, tinggi bisa diubah), dan footer (keterangan di kiri, _Copy_ di kanan).
- Baris pilihan: dua kolom berlabel 12px/500 muted, **Language** dan **Values**, masing-masing `Dropdown` varian `field`. Grid `repeat(auto-fit, minmax(200px, 1fr))`, jadi di jendela sempit keduanya bertumpuk, bukan memanjang ke kanan.
- _Language_ diingat per perangkat (`localStorage` `codestamp.codeLanguage`). _Values_ selalu mulai dari **Environment variables**; _Filled in (shows secrets)_ tidak pernah diingat.
- Keterangan footer mengikuti _Values_: "Reads BASE_URL, TOKEN from environment variables.", "Variables stay as {{name}}.", atau "Values are filled in as they would be sent now.", selalu ditutup "Pre-request scripts are not run."
- Kode yang dihasilkan selalu bersusun sama, dipisah baris kosong: import, nilai dari environment (hanya yang dipakai), URL dan query, header, body, kirim, lalu cetak status dan body. Satu pasangan query, header, atau field per baris. Indentasi mengikuti bahasanya (Go memakai tab; Python, PHP, Java, Kotlin, C#, Swift, Rust, dan PowerShell 4 spasi; JavaScript, Dart, Ruby, cURL, dan HTTPie 2 spasi).
- Nama: `{{baseUrl}}` menjadi environment variable `BASE_URL`, variabel lokal `baseUrl` (atau `base_url` di Python, Ruby, dan Rust). Nama yang bentrok dengan kata kunci atau nama lokal generator (`url`, `body`, `headers`, …) mendapat akhiran `Value`. Environment variable yang sudah dipakai sistem (`USER`, `PATH`, `HOME`, …) mendapat akhiran `_VALUE`.
- Urutan _Language_: cURL, HTTPie, PowerShell, JavaScript fetch, JavaScript axios, Python requests, PHP Guzzle, Go net/http, Java HttpClient, Kotlin OkHttp, C# HttpClient, Dart http, Swift URLSession, Ruby Net::HTTP, Rust reqwest. Label selalu "bahasa + library", supaya jelas apa yang perlu dipasang.

### Banner

- Ditampilkan di puncak konten lewat `notices`. Satu pesan pada satu waktu, dengan tombol Dismiss. Pesan info boleh membawa satu tombol aksi sebelum Dismiss, misalnya _Open variables_ yang membuka panel Variables collection terkait, _Undo_ setelah menghapus collection atau request, dan _Open_ setelah memulihkannya dari Recently deleted.

### Recently deleted

- Collection dan request yang dihapus disimpan 30 hari (`deleted_items`, skema 26) sebagai salinan baris database apa adanya: section, request, example, environment milik collection, variabel (secret tetap terenkripsi vault), info Catalog, hasil Run, tautan Collaboration, grup akses, dan token sign-in. Langkah Orchestration dan History yang menunjuk ke item itu disambungkan lagi saat pulih.
- Dialog hapus berkata "You can restore it from Recently deleted for 30 days." (bukan "cannot be undone"). Setelah dihapus, banner info "“Nama” was deleted. It stays in Recently deleted for 30 days." dengan tombol **Undo**, yang memulihkan dan membuka item itu.
- Halaman **Recently deleted** (ikon tempat sampah, hue `slate`, di status bar, Menu › Go, Quick search, dan Home): judul + penjelasan, tombol quiet **Delete all** (merah saat hover, dengan ConfirmDialog). Daftar seperti History: chip _Collection_/_Request_, nama 500, baris muted (jumlah request, "In “Collection”", atau penjelasan bahwa collection-nya juga terhapus sehingga Restore membuat collection baru bernama sama), "Deleted … · N days left", tombol **Restore** (secondary kecil), dan tombol ✕ "Delete … for good" dengan ConfirmDialog. Kosong: "Nothing deleted in the last 30 days."
- Request kembali ke section-nya bila section itu masih ada; bila tidak, ke luar section. Collection yang pernah dibagikan kembali tersambung ke repository-nya, dan clone-nya dibangun ulang dari commit tersinkron terakhir.
- Error: `--err-bg` / `--err`, tepi `--err-line`, `role="alert"`.
- Info: `--accent-soft` / `--accent-text`, `role="status"`. Contoh: rekan tim menghapus request yang sedang terbuka, atau Sync now membawa akses ke grup yang diminta perangkat ini ("Mate gave this device access to Ops. Its restricted values are ready to use.").
- Banner update (`UpdateBanner`) berdiri sendiri di atas banner `notices`, dengan warna info yang sama: teks versi, lalu tombol teks _What's new_ (bila ada catatan), _Update and restart_, dan _Later_. Selama mengunduh, tombol diganti persentase dan `<progress>` 6px (isi `--accent` di atas `--surface`). Gagal: warna error, `role="alert"`, tombol _Try again_ dan _Later_.

### Kotak status inline

- Kotak radius md, padding `10px 14px`, 13px, dengan pasangan latar/teks semantik. Contoh: hasil export (ok), konflik menunggu (warn), status sinkron di panel Collaboration (ok/warn/err dengan titik).
- Error form: latar `--err-bg`, padding `10px 12px`, `role="alert"`.

### Tabel key-value

- 13px, header 500 muted, baris dipisah `--border-soft`, sel setinggi 30px.
- Baris nonaktif: teks muted dan dicoret.
- Baris file: tombol bergaris putus-putus berisi nama file.
- Variabel collection yang diganti environment aktif: catatan 12px `--warn` di bawah nama ("Replaced by “DEV”"), bukan warna saja.
- Kolom Type di tabel variabel (lebar 150px) berisi Text, Secret, dan _Restricted: nama grup_ untuk tiap grup yang bisa dipakai perangkat ini. Nilai restricted tetap secret di perangkat dan dikirim terenkripsi untuk grupnya.
- Baris yang dibatasi ke grup yang bukan milik perangkat ini terkunci: nama read-only, sel nilai berisi teks 12px muted "Restricted to Ops. Ask a member for access in Collaboration.", kolom Type bertulisan _Restricted: Ops_, dan tidak ada tombol hapus. Preview URL menulis nama seperti itu sebagai `{{opsKey}} (restricted to Ops)`, bukan sekadar tidak terdefinisi.

### Halaman Environments

- Daftar di kiri dikelompokkan dengan judul grup 12px muted: _On this device_, lalu satu grup per collection (emoji + nama).
- Di bawah nama environment ada Dropdown **Belongs to** (_This device only_ atau collection) dengan hint 12px muted yang menjelaskan akibatnya: hanya di perangkat ini, dibagikan lewat Git saat Sync now (secret tetap di tiap perangkat), atau ikut saat collection dibagikan.

### Dropzone dan target drop

- Garis 1.5px dashed `--border-strong`. Saat hover atau aktif: border tinta dan latar `--accent-soft`.
- Target drop di daftar: latar `--accent-soft` dengan outline 1.5px dashed tinta.

### Editor kode

CodeMirror dengan latar transparan di dalam bingkai `--surface-muted` (border dan radius md). Gutter tanpa garis, baris aktif `--code-active-line`, seleksi `--accent-soft`, warna sintaks dari token `--code-*`.

- Mengikuti Settings › Editor: indentasi (None sebagai bawaan, 2 spasi, 4 spasi, Tab; juga dipakai Beautify), tutup kurung/kutip otomatis, dan bungkus baris panjang. Berubah langsung tanpa membuka ulang kartu. Dengan None, Enter mulai dari kolom 0 dan Tab memindah fokus ke kontrol berikutnya.
- Editor body, pesan WebSocket, dan argumen MCP mewarnai `{{variabel}}` seperti kolom input.
- **Find** (`editor/search.ts`, Ctrl+F): panel di atas editor, latar `--surface-muted` dengan garis bawah `--border-soft`. Isinya kolom mono 28px (border `--border-strong`), jumlah temuan 12px muted dengan angka tabular ("3 of 12", "No matches", atau "Invalid pattern" dengan warna `--err`), tombol ikon 28px Previous/Next, toggle mono `Aa` `ab` `.*` (`aria-pressed`; menyala dengan `--accent-soft` dan cincin `--border-strong`), lalu Close.
  - Klik tombol tidak memindahkan fokus dari kolom.
  - Semua temuan berlatar `--warn-bg` dengan garis `--warn-line`, dan temuan yang dipilih diberi bingkai 2px `--warn`, jadi tetap terlihat tanpa membedakan warna.
  - Selama ada kartu yang terbuka, Ctrl+F di luar editor (dari mana pun di halaman collection) membuka find di body response (`data-find`), tidak memunculkan kolom cari browser di pojok atas jendela.

### Settings

- Navigasi bagian di kiri (200px, sticky): Appearance, Editor, Requests, Proxy, SSL/TLS, Data & history, About & shortcuts. Bagian aktif terangkat seperti item sidebar.
- Isi bagian berupa panel (surface, border, radius lg). Kelas bersama (`.panel`, `.field`, `.input`, `.check`, `.button`, `.primary`, `.segmented`, `.error`, `.status`) didefinisikan sekali di `SettingsView` dengan `:global`.
- Editor, Requests, dan Data & history tersimpan otomatis 400 ms setelah perubahan terakhir; Rust memeriksa nilainya, dan pesan salah tampil di panel. Proxy punya tombol Save karena alamat dan password diketik bertahap.
- Verifikasi sertifikat server tidak bisa dimatikan; panel Requests menjelaskannya dan mengarahkan ke SSL/TLS (versi TLS minimum, CA certificate, client certificate).
- Tindakan yang tidak bisa dibatalkan (Clear history, Restore from backup, Remove certificate) lewat dialog konfirmasi.
- About › Updates: kalimat status (versi baru, atau `.status` hijau "You have the latest version"), catatan rilis sebagai teks biasa di kotak `--surface-muted` (tinggi maksimal 240px, bisa digulir), lalu _Update and restart_ (`.primary`) dan _Check for updates_ (`.button`).

### Pemilih tema (`ThemePicker`)

Grid kartu (min 150px). Tiap kartu berisi pratinjau mini yang memakai `data-theme` tema itu sendiri (sidebar, baris GET dengan Send, baris POST dengan `{{baseUrl}}`), nama, dan keterangan. System menampilkan Light dan Dark berdampingan dan keterangan "Now Dark/Light". Kartu terpilih: border dan cincin `--text`. Di belakangnya radio biasa, jadi panah keyboard berfungsi.

### Kartu MCP

- Target: pilihan transport (STDIO/HTTP, warna cap `mcp`, terkunci selama terhubung), lalu perintah (`npx -y …`) atau URL server, lalu tombol Connect/Disconnect berwarna `--method-mcp`.
- Panel kiri: info server (nama, versi, versi MCP, instructions), tab Tools/Prompts/Resources dengan jumlahnya, tab Headers (HTTP) atau Environment (STDIO), dan tab Paste config untuk menempel konfigurasi `{"mcpServers": …}` dari klien lain.
- Item terpilih memakai latar `--method-mcp-bg` dan garis `--method-mcp-line`. Memilih item mengisi argumen dari skema (`default`, lalu nilai kosong sesuai tipe).
- Argumen JSON di editor kode; tombol Run tool / Get prompt / Read resource nonaktif sampai argumennya objek JSON.
- Panel kanan: tab Result (chip Result, Tool error, atau Error kode; tampilan terbaca per blok konten, atau Raw JSON) dan tab Messages (log JSON-RPC: ↑ terkirim, ↓ diterima, stderr dan logging server, info, error).
- Tool, prompt, atau resource yang dipilih tersimpan bersama request (dan ikut sinkron tim), argumennya di body.

### Quick search (`QuickSearch`)

- `Ctrl+Space` atau tombol cari di sidebar (tombolnya hanya bertuliskan "Search"). `<dialog>` modal di sepertiga atas layar yang turun 10px dari skala 0,98 sambil memudar (240 ms, `cubic-bezier(0.2, 0.7, 0.2, 1)`, kurva yang sama dengan tile Home), dengan latar `--backdrop` yang memudar 200 ms. Hanya opacity dan transform yang bergerak, jadi animasinya berjalan di compositor. Latar **tidak** di-blur: blur selebar jendela menggambar ulang seluruh aplikasi di tiap frame dan membuat pembukaan terasa berat. Animasi dimatikan oleh `prefers-reduced-motion`.
- Tidak ada petunjuk `kbd` (Tab, Esc, panah, Enter) di kolom maupun footer; footer hanya berisi contoh path atau pesan error.
- Satu kolom besar (16px). Saat yang diketik adalah awal sebuah path API, sisa path yang cocok tampil samar di belakang teks dan `Tab` melengkapinya.
- Hasil dikelompokkan (request, collection, halaman Settings) dengan cap method, nama, URL mono, dan lokasi muted; bagian yang cocok ditebalkan dengan warna `--accent-text`. Panah atas/bawah memilih, `Enter` membuka, `Esc` menutup.
- Grup **Docs**: collection (emoji), section (ikon Docs), dan request (cap method) yang nama atau deskripsinya cocok. Detailnya berupa potongan deskripsi, atau "Request docs" bila yang cocok namanya. Saat halaman Docs terbuka, grup ini tampil paling atas.

### Kartu GraphQL

- Cap `GQL` (tone `gql`) di depan endpoint; Send berwarna `--method-gql`.
- Tab Query berisi editor query (highlight + autocomplete dari schema) dan editor variabel JSON di bawahnya. Tombol _Fetch schema_ / _Fetch again_ dengan jumlah tipe yang dimuat; popup autocomplete memakai token tooltip editor.

### Kartu SOAP (`SoapPanel`)

- Cap `SOAP` (tone `soap`) di depan endpoint; tombol **Send** biasa (selalu POST). Tab pertama bernama **Envelope**.
- Isi tab Envelope:
  - Baris WSDL: kolom URL (atau nama file bila dari _Choose file…_, path lengkap tidak ditampilkan, dengan link _Use a URL_), tombol _Choose file…_ dan _Load WSDL_ / _Reload WSDL_. Enter di kolom URL juga memuat.
  - Error memuat tampil sebagai kotak `--err`. Peringatan (import yang tidak terbaca) tampil sebagai daftar `--warn`, maksimal 3 baris plus "…and N more".
  - Dropdown **Operation** (`Operasi · Binding · SOAP 1.1`), segmented **SOAP version** (1.1 / 1.2), dan kolom **SOAPAction**.
  - Di bawahnya dokumentasi operasi (atau petunjuk singkat) dan link _Reset the envelope to the sample_. Editor XML memakai `BodyEditor` dengan `fixedMode` (tanpa pilihan jenis body).
- Memilih operasi mengisi versi, action, endpoint (bila kosong), nama (bila masih bawaan atau nama operasi sebelumnya), dan envelope (bila kosong atau masih sama dengan contoh sebelumnya). Tulisan pengguna tidak pernah ditimpa diam-diam.
- Response XML dirapikan. SOAP Fault menambah chip merah _SOAP Fault_ dan kotak error `kode: alasan`.

### Kartu gRPC (`GrpcPanel`, `GrpcReplyPanel`)

- Cap `gRPC` (tone `grpc`) di depan alamat `grpc://` / `grpcs://`; tombol utama berlabel **Invoke**, nonaktif sampai method dipilih.
- Baris sumber: segmented _Server reflection_ / _.proto file_, tombol file (nama file saja, path lengkap tidak ditampilkan), lalu _Load services_ di kanan.
- Dropdown method (mono) berisi `package.Service/Method · unary / server stream / client stream / bidirectional stream`.
- Tab Message (editor JSON dengan petunjuk sesuai jenis stream), Metadata (tabel key-value), dan Auth (tanpa Digest dan AWS, yang tidak berlaku untuk gRPC).
- Panel balasan: chip status gRPC (`0 OK` hijau, lainnya merah), durasi dan jumlah pesan, pesan status sebagai kotak error, lalu tab Messages (satu pesan di editor read-only; stream sebagai daftar bernomor), Headers, dan Trailers.

### Examples dan mock server

- Tab _Examples_ di kartu request: daftar example (nama `EditableText`, chip status, editor body), dengan tombol _Save as example_ di panel response.
- Panel _Mock server_ (toolbar collection, titik status saat berjalan): kolom port (kosong = otomatis), Start/Stop, alamat yang bisa disalin, dan log permintaan (jam, chip status, method dan path, nama example yang menjawab atau "no matching example").

### Event stream (SSE)

- Tab body berganti menjadi _Events · n_. Kepala stream: titik hijau berdenyut + "Streaming · n events" + tombol Stop; setelah selesai "Stream ended", atau pesan error merah bila putus.
- Tiap event: jam mono, nama event (bila bukan `message`), id, lalu data (JSON dirapikan) dalam blok mono.

### Orchestration (`FlowsView`, `FlowEditor`, `FlowMap`, `FlowStepCard`)

- Halaman dua kolom: daftar flow (200px, item aktif seperti item sidebar) dan editor. Header editor: nama `EditableText` (title), status simpan, tombol **Run flow** (latar tinta; berubah menjadi Stop merah bergaris saat berjalan), dan Delete flow.
- **Kartu peta** (`section` "Flow map", surface, border, radius lg): baris atas berisi label _Environment_ dan pemilih environment di kiri, ringkasan run di kanan (`role="status"`, plus catatan berhenti `--err`). Di bawahnya peta langkah (`FlowMap`).
- **Peta langkah**: grid dengan kolom Start dan End selebar `--edge` (80px), kolom Add step selebar `--add` (52px), dan kolom langkah yang berbagi sisa lebar sama rata (`minmax(120px, 1fr)`). Empat langkah (692px) muat tanpa gulir di jendela 1280px, juga saat halaman punya scrollbar; di lebar minimum itu cap DELETE masih muat di node. Bila tidak muat, hanya peta yang digulir ke samping, dan node terpilih digeser ke dalam pandangan tanpa menggulir halaman.
  - `<ol aria-label="Steps from start to end">` memakai subgrid. Garis 1.5px `--border-strong` membentang dari tengah kolom Start ke tengah kolom End, dengan chevron di depan setiap titik. Setiap titik berada di tengah kolomnya.
  - **Add step**: `IconButton` bulat dengan "+" (tooltip dan nama "Add step", `aria-haspopup="dialog"`), di antara langkah terakhir dan End.
  - Node langkah adalah tombol (radius lg, surface, border `--border`): baris atas penanda status 14px, nomor mono 11px, `MethodStamp` ukuran `sm` (jarak 5px); lalu nama 13px/500 (terpotong elipsis) dan hasil singkat mono 11px (kode · waktu; `--ok`/`--err`). Gagal bergaris `--err-line`, dilewati bergaris putus-putus. Node terpilih: border `--text`, `--shadow-raise`, `aria-current="step"`. Panah kiri/kanan berpindah node dan memindahkan fokus.
  - End: pill dengan bendera; hijau `--ok` bila semua lulus, merah `--err` bila ada yang gagal. Latar tint-nya ditumpuk di atas `--surface` agar garis tidak tembus di tema gelap.
- **Garis nilai** (di atas node, hanya untuk langkah terpilih, `aria-hidden`): satu baris per nilai, dari tengah kolom asal ke tengah kolom tujuan, garis 1.5px dengan sudut atas melengkung 10px dan chevron di setiap langkah yang memakai nilai itu. Label `FlowValueChip` (`{{nama}}`, mono, pill) di tengah atas, berlatar surface. Warna: `--text-muted` sebelum run, `--ok` setelah diteruskan, `--err` putus-putus dan dicoret bila tidak ditemukan.
  - Nilai yang diterima: setiap `{{nama}}` yang dikirim request langkah itu (`StepRequest.variables`) dari langkah terakhir sebelumnya yang meneruskan nama itu, sama seperti saat run. Nilai yang diteruskan: ke setiap langkah sesudahnya yang mengirim nama itu, sampai ada langkah lain yang meneruskan nama yang sama. Satu sumber: `lib/flow/lineage.ts`.
- **Garis berhenti**: setelah run yang berhenti karena langkah gagal, garis `--err` putus-putus di bawah node dari tengah langkah itu sampai tengah End, dengan label "Stopped at step N". Karena kolom ujung lebarnya tetap, margin kirinya dihitung dari lebar kolom langkah saja: `(100% - --add - --edge) / (2 × jumlah kolom langkah)`.
- **Detail langkah** (`FlowStepCard`, selalu terbuka, hanya untuk langkah terpilih): header berisi penanda status (`FlowStepMark`: lingkaran kosong; putus-putus = dilewati; berputar `--focus` = berjalan; centang `--ok`; silang `--err`), nomor mono, `MethodStamp`, nama dan "collection · URL", hasil lengkap, lalu Move left / Move right dan Remove. Kartu gagal bergaris `--err-line`.
  - Isi: baris _Uses_ (chip nilai + "from step N", hanya bila ada), baris pengaturan (_If this step fails_, Change request, Open request), tab Checks / Pass on / Response, aturan sebagai baris dropdown + input mono, dan kotak _Last run_ (`--surface-muted`) dengan ✓/✗ per check dan test.
  - Langkah yang dihapus digantikan tetangganya; flow tanpa langkah hanya menampilkan peta (Start, Add step, End).
- Pemilih request (`FlowStepPicker`): dialog dengan kolom cari dan daftar request HTTP/GraphQL dari semua collection (cap method, nama, URL, collection).

### Catalog (`CatalogView`, `components/catalog/`)

- **Daftar** (`CatalogList`): judul dengan `NavIcon` catalog, deskripsi, tombol Refresh. Empat `StatTile` (label 13px muted, angka 26px/600, catatan 12px; catatan berwarna `--err`/`--warn` hanya saat angkanya buruk, misalnya error naik). Lalu kolom cari, `Dropdown` tag (hanya bila ada tag), dan pill status kesehatan seperti filter History.
- Tabel dalam kartu (surface, border, radius lg) dengan `container-type: inline-size`: di bawah 760px kolom p95, Owner, dan Last activity disembunyikan dan owner pindah ke baris kecil di bawah nama. Header kolom yang bisa diurutkan adalah tombol dengan panah; `th` memakai `aria-sort`. Nama API adalah tombol yang membuka halamannya.
- **`HealthChip`**: pill dengan skor mono dan label (Healthy / Needs attention / Critical / No data) memakai pasangan semantik ok / warn / err; No data netral. Warna selalu ditemani teks.
- **Halaman API** (`CatalogApi`): link "All APIs", tile emoji 44px, nama h1, chip kesehatan, owner, tag (pill bergaris), lalu Edit owner and tags (`aria-expanded`, membuka `InfoForm` inline), Run all, Open collection. Tab bergaya tab kartu request: Overview, Runtime, Tests, Quality · jumlah aturan yang gagal.
- **Overview**: skor 48px, lalu tiga baris bagian (nama + bobot, meter 8px dengan track `--surface-sunken` dan isi ok/warn/err, nilai + label). Penjelasan bobot dan ambang di bawahnya. Kartu "What it holds" (`<dl>`) dan "Used in Orchestration".
- **Grafik** (`TrafficChart`): kolom bertumpuk per hari (lebar maks 24px, ujung atas radius 4px, celah 2px antarsegmen), abu `--text-muted` untuk request yang dijawab dan `--err` untuk error, selalu di posisi yang sama dalam tumpukan. Legenda di atas, garis bantu 1px `--border-soft`, tooltip saat hover, dan tombol **Show as table** sebagai pengganti untuk keyboard dan pembaca layar. Di tema Dark pemisahan warna untuk buta warna hanya 7,5, jadi celah, posisi, dan legenda wajib tetap ada.
- **Quality** (`RuleList`): kartu per aturan dengan tanda ✓/✗ bulat (ok/err), judul, chip Error/Warning, deskripsi, "N of M fail"; temuan sebagai daftar (cap method, nama request sebagai tombol, detail muted), lima pertama lalu "Show all N".
- Loading, kosong, dan gagal selalu punya teks: "Loading the catalog…", "No APIs yet" dengan tombol Create a collection, "No API matches these filters." dengan Clear filters, dan kotak `--err` dengan Try again.

### Docs (`DocsView`, `components/docs/`)

- **Indeks + satu dokumen**: di kiri ada indeks (280px), di kanan hanya satu dokumen. Tombol **Save as Markdown** ada di kanan atas, dan hasilnya tampil sebagai kotak `--ok` berisi path, atau `--err`.
  - Hanya satu dokumen yang digambar, jadi collection dengan ratusan request tetap ringan.
- **Indeks** (`nav` "Docs index"):
  - Isinya dari atas: `Dropdown` collection (varian `field`, emoji + nama), kolom **Search docs** (gaya kolom cari Catalog: ikon kaca pembesar, border `--border`, biru saat fokus; Ctrl+F di halaman Docs), dan baris jumlah 12px muted ("321 requests" + petunjuk `Ctrl` `F`, atau "3 of 321 requests").
  - Di bawahnya: **Overview**, request di luar section, lalu tiap section (chevron `FoldButton`, nama 600 + jumlah mono) dengan request-nya menjorok 18px (`IndexEntry`: cap method kecil + nama).
  - Section **mulai terlipat**, supaya indeks collection besar tetap pendek. Chevron (Enter/Space) membuka atau melipatnya; memilih section, membuka request lewat Previous/Next, atau hasil Quick search membuka section-nya. Section yang dibuka diingat per perangkat (`localStorage` `codestamp.openDocsSections`). Selama mencari, semua section menampilkan hasilnya dan chevron nonaktif.
  - Item terpilih tampil seperti item aktif sidebar (latar `--surface`, cincin 1px `--border`, `aria-current="page"`). Panah atas/bawah berpindah item dan langsung menampilkannya.
  - Dari lebar 700px (container query), indeks menempel di bawah title bar dan punya scroll sendiri setinggi jendela. Di bawah 700px, indeks ada di atas dokumen dengan daftar maksimal 260px.
  - Tanpa hasil: kotak bergaris putus-putus berisi teks dan tombol Clear search.
- **Overview**: kartu collection (tile emoji 44px, nama 22px/600, jumlah request dan section) dan deskripsinya, lalu baris Auth.
  - **Variables** dilipat di `<details>` ("Variables" + jumlah) berisi tabel Name/Value mono. Secret ditulis "Secret, set on each device", dan nilai yang namanya mirip kredensial (token, password, key) disamarkan lewat `documentedValue`, sama seperti di Save as Markdown.
  - Daftar **Sections** (nama + jumlah request) membuka section-nya.
- **Section**: kartu dengan judul 22px, jumlah request, deskripsi, dan daftar request (cap method, nama, URL mono muted) yang membuka dokumen request.
- **Request** (`RequestDoc`): cap method, nama 16px/600, tombol **Open request**, URL mono, deskripsi, fakta (Auth, method gRPC, operasi SOAP, transport MCP), tabel `DocTable` (Path variables, Query parameters, Headers/Metadata, Body form) dengan nilai kredensial tertulis disamarkan lewat `documentedValue`, blok body `<pre>` (maks 320px, scroll), dan example di `<details>` dengan chip status.
  - Di bawahnya ada tombol **← Previous** dan **Next →** (nama request di bawah petunjuk 12px muted) yang mengikuti urutan indeks.
- Hasil Docs dari Quick search memilih item itu, lalu menggulir dan memfokuskan dokumennya (`scroll-margin-top: 16px`).
- **Deskripsi** (`DescriptionBlock`): kosong menampilkan "No description for … yet." + link "Add a description"; terisi menampilkan Markdown + link "Edit description". Mode edit: segmented Write/Preview, hitungan karakter mono (merah bila lebih dari 20.000), textarea mono (`--surface-muted`, border biru saat fokus), pesan salah `role="alert"`, petunjuk `Ctrl Enter`/`Esc`, lalu Cancel (secondary) dan Save (primary tinta). Hanya satu deskripsi yang diedit sekaligus.
- **Markdown** (`Markdown`, `MarkdownNodes`): digambar node demi node dari mdast, tanpa HTML mentah. Heading Markdown digeser di bawah heading halaman (`headingOffset`), kode inline `--surface-sunken`, blok kode `--surface-muted` bergaris, tabel bergaris `--border` dengan header `--surface-muted`, kutipan bergaris kiri netral. Link http/https bergaris bawah dan dibuka di browser; link lain hanya teks bergaris titik; gambar ditulis "Image: alt"; HTML mentah tampil sebagai kode.

### Cap method (`MethodStamp`)

Satu komponen untuk cap method/jenis request di baris request, Quick search, dan Orchestration: label dari `stampLabel()`, tone dari `stampTone()`, ukuran `md` (11px) atau `sm` (10px).

### Path variables (`PathVariablesTable`)

Tabel di atas Query parameters yang barisnya mengikuti segmen `:nama` di URL (nama tidak bisa diedit di tabel, nilai mendukung `{{variabel}}`). Hilang sendiri bila URL tidak punya path variable.

### Dialog persetujuan program (MCP)

Muncul sebelum server STDIO dijalankan pertama kali, dan setiap kali program, argumen, atau environment-nya berubah (termasuk dari sinkron tim atau variabel yang berubah). Menampilkan program, daftar argumen, dan nama environment (nilainya tidak). Tombol utama "Start program", Escape = batal.

### Mode fokus

- Sidebar dan kartu lain disembunyikan; latar konten menjadi `--surface-sunken`.
- Bar berbentuk pill di atas berisi judul, opsi "Hide secrets on screen", dan tombol Exit `Esc`.
- Kartu lebih besar: radius 18px, URL field dan Send setinggi 46px.

## 8. State interaksi

| State          | Perlakuan                                                                                                                                                                          |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hover          | Tombol bergaris: border `--border-strong` (tile/ikon: `--text`). Tombol transparan: latar `--surface-sunken`                                                                       |
| Fokus keyboard | Global `:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px }`. Input mengganti border ke `--focus`; outline hanya boleh dihapus bila border itu menggantikannya |
| Terpilih/aktif | Item sidebar: terangkat (surface + border + `--shadow-raise`). Tombol yang membuka panel/dialog, selama terbuka: border `--text` dan latar `--accent-soft`                         |
| Ditekan        | `:active`: latar `--surface-sunken` (tombol bergaris, tile) atau `--accent-soft` (tombol transparan, item sidebar)                                                                 |
| Disabled       | `--surface-sunken` dengan `--text-muted`; `cursor: not-allowed` (global)                                                                                                           |
| Sibuk          | Label kata kerja + "…"; `aria-busy` pada area response                                                                                                                             |
| Gagal          | Teks atau latar `--err` dengan pesan yang bisa ditindaklanjuti                                                                                                                     |

## 9. Tulisan di UI

- Bahasa UI adalah **Inggris**; dokumentasi proyek berbahasa Indonesia.
- **Sentence case** di semua label dan judul: "Add request", "Delete collection", "Sync now".
- Tombol berupa kata kerja (+ objek bila perlu). Tombol konfirmasi menyebut aksinya ("Delete", "Stop syncing", "Discard"), bukan "OK".
- Nama milik pengguna diapit kutip lengkung: “Login”. Proses yang berjalan memakai karakter elipsis tunggal "…".
- Pesan konfirmasi menjelaskan akibatnya: "… will be deleted. This cannot be undone."
- Pesan error menyebut apa yang terjadi dan apa yang bisa dilakukan, tanpa detail internal (path, query, stack trace), sesuai aturan keamanan proyek.
- Bentuk tunggal/jamak harus tepat: "1 request", "2 requests".
- Hint singkat (12-13px) di bawah judul panel atau kolom menjelaskan apa yang terjadi pada data, misalnya apa yang dibagikan ke tim, atau bahwa Base URL menjadi `{{baseUrl}}`.
- Jangan menjanjikan fitur yang tidak ada (mis. akun/login); jelaskan cara yang sebenarnya (sign-in lewat Git).
- Tanpa em dash (—) dan en dash (–), di UI maupun dokumen. Pemisah memakai titik dua atau titik ("status equals 200: got 201"). Rentang di UI ditulis "1 to 4"; di dokumen cukup tanda hubung (11-20 px).
- Judul menyebut apa yang dilakukan layar itu ("Requests you sent", "Run requests in order"), bukan slogan ("Make it yours") atau klaim berlebihan ("Find anything", "Everything you sent").
- Sebut pelakunya: "Codestamp keeps only what you typed", bukan "only what you typed is kept". Hindari kata kosong seperti _journey_, _seamless_, _powerful_, _effortless_.

## 10. Aksesibilitas

- Utamakan elemen native: `<button>`, `<dialog>`, `<label>` yang membungkus atau menunjuk input, `<fieldset>`/`<legend>` untuk kelompok radio, `<table>`, `<dl>` untuk daftar shortcut.
- Atribut ARIA:
  - tombol ikon: `aria-label` (tombol emoji: "Change icon", atau keterangan ikon saat ini);
  - toggle: `aria-pressed`; pemicu popover: `aria-haspopup="dialog"` + `aria-expanded`;
  - panel buka/tutup: `aria-expanded`;
  - area atau kelompok: `aria-label`, misalnya `section[aria-label="Team"]`.
- Pengumuman: `role="alert"` untuk error, `role="status"` untuk kabar dan status.
- Keyboard:
  - semua kontrol bisa dicapai dengan Tab;
  - Escape menutup lapisan paling atas saja: picker emoji, lalu dialog, lalu mode fokus (kecuali saat kursor di editor kode);
  - Alt+↑/↓ memindah request atau section, lalu fokus dikembalikan ke item yang dipindah.
- `.visually-hidden` dipakai untuk input file tersembunyi dan teks khusus pembaca layar. Svelte membuang spasi di awal dan akhir isi elemen, jadi spasi pemisah ditulis di luar span: `Open request <span class="visually-hidden">{name}</span>`. Emoji dekoratif di sidebar diberi `aria-hidden="true"` karena nama collection sudah terbaca.
- Warna tidak pernah menjadi satu-satunya penanda.

## 11. Privasi di layar

- Nilai secret selalu tampil sebagai `SECRET_MASK` (`••••••••`, dari `src/lib/privacy/secrets.ts`) sampai pengguna menekan Show.
- Mode fokus dan Capture menyamarkan token dan cookie, dengan skema auth tetap terlihat (`Bearer ••••••••`).
- Komponen baru yang menampilkan nilai sensitif wajib memakai helper tersebut, bukan logika penyamaran sendiri.

## 12. Checklist komponen baru

1. Warna, radius, dan bayangan diambil dari token. Tidak ada hex baru di komponen; warna method lewat `methodTone()`.
2. Ukuran teks dari skala di bagian 3, ditulis dalam px pada basis 14px. Teks kode memakai `--font-size-code`.
3. Tombol mengikuti varian di bagian 7: satu primary (tinta) per area, aksi destruktif lewat ConfirmDialog.
4. Layout memakai flex-wrap dan flex-basis. Uji di jendela sempit (±600px) dan pada ukuran UI 20.
5. Dialog memakai `<dialog>` native dengan lebar `min(N, calc(100vw - 32px))`, dan Escape menutupnya.
6. Ikon antarmuka inline SVG sesuai bagian 6, dengan label. Emoji hanya untuk ikon collection, dan hanya dari katalog di Rust.
7. Teks UI berbahasa Inggris, sentence case, dengan pesan error yang generik.
8. Style ditulis di blok `<style>` komponen (ter-scope oleh Svelte). Hanya token dan utilitas global (`.mono`, `.emoji`, `.visually-hidden`) yang tinggal di `src/app.css`.
9. `npm run check` dan `npm run lint` (svelte-check, ESLint, Prettier) lolos tanpa warning.

## 13. Lokasi file

| File                                            | Isi                                                 |
| ----------------------------------------------- | --------------------------------------------------- |
| `src/app.css`                                   | Token, reset, utilitas global                       |
| `src/lib/appearance/presets.ts`                 | Preset font dan rentang ukuran                      |
| `src/lib/request/methods.ts`                    | Tone dan warna method HTTP                          |
| `src/lib/request/codegen/`                      | Generator Copy as code, satu file per bahasa        |
| `src/lib/components/CodeDialog.svelte`          | Dialog Copy as code (Language, Values, kode)        |
| `src/lib/format/status.ts`                      | Tone status HTTP                                    |
| `src/lib/team/status.ts`                        | Tone dan teks status sinkron tim                    |
| `src/lib/components/AccountFields.svelte`       | Kolom akun Git, nama, dan email untuk Collaboration |
| `src/lib/editor/setup.ts`                       | Tema dan syntax highlight editor kode               |
| `src/lib/capture/render.ts`                     | Palet gambar Capture (terang dan gelap)             |
| `src/lib/privacy/secrets.ts`                    | Penyamaran nilai sensitif                           |
| `src-tauri/src/storage/icons.rs`                | Katalog emoji, aturan otomatis, whitelist ikon      |
| `src/lib/state/icons.ts`                        | Katalog emoji yang dimuat sekali dari Rust          |
| `src/lib/components/EmojiPicker.svelte`         | Picker emoji collection                             |
| `src/lib/components/IconButton.svelte`          | Tombol ikon dengan tooltip dan titik status         |
| `src/lib/components/HomeView.svelte`            | Halaman Home                                        |
| `src/lib/components/NewCollectionDialog.svelte` | Dialog New collection                               |
| `src/lib/components/ChoiceCard.svelte`          | Kartu pilihan radio (ikon 32px, judul, deskripsi)   |
| `src/lib/components/ImportView.svelte`          | Halaman Import                                      |
| `src/lib/components/StatusBar.svelte`           | Status bar: Settings, data, dan versi               |
| `src/lib/components/ViewPanel.svelte`           | Cangkang panel collection (judul, aksi, Close)      |
| `src/lib/components/PanelDialog.svelte`         | Dialog modal untuk semua panel collection           |
| `src/lib/components/CollectionExport.svelte`    | Panel Export collection dengan dropdown format      |
| `src/lib/components/CollectionHistory.svelte`   | Panel History per collection                        |
| `src/lib/components/ExportOutcome.svelte`       | Hasil export: path dan catatan, atau error          |
| `src/lib/components/DeletedView.svelte`         | Halaman Recently deleted                            |
| `src/lib/state/deleted.svelte.ts`               | Daftar Recently deleted, Undo, dan Restore          |
| `src/lib/components/CollectionStart.svelte`     | Tampilan collection kosong                          |
| `src/lib/components/ThemePicker.svelte`         | Pemilih tema dengan pratinjau                       |
| `src/lib/components/Dropdown.svelte`            | Dropdown bergaya pengganti `<select>`               |
| `src/lib/components/NavIcon.svelte`             | Ikon berwarna sidebar dan judul halaman             |
| `src/lib/components/SettingsView.svelte`        | Kerangka Settings dan gaya panel bersama            |
| `src/lib/components/settings/`                  | Panel tiap bagian Settings                          |
| `src/lib/components/VariableText.svelte`        | Teks dengan `{{variabel}}` berwarna                 |
| `src/lib/format/variables.ts`                   | Pola dan pemecah referensi variabel                 |
| `src/lib/components/McpPanel.svelte`            | Isi kartu MCP (server, argumen, hasil, log)         |
| `src/lib/components/McpApprovalDialog.svelte`   | Persetujuan sebelum program MCP dijalankan          |
| `src/lib/components/QuickSearch.svelte`         | Quick search `Ctrl+Space`                           |
| `src/lib/dom/reveal.ts`                         | Gulir ke elemen dan pindahkan fokus ke sana         |
| `src/lib/search/`                               | Pencocokan dan pengelompokan hasil pencarian        |
| `src/lib/editor/search.ts`                      | Find (Ctrl+F) di editor kode                        |
| `src/lib/docs/filter.ts`                        | Saring dokumen dan daftar collection di Docs        |
| `src/lib/components/GraphqlEditor.svelte`       | Query, variabel, dan schema GraphQL                 |
| `src/lib/components/GrpcPanel.svelte`           | Sumber, method, pesan, metadata kartu gRPC          |
| `src/lib/components/GrpcReplyPanel.svelte`      | Balasan gRPC (status, pesan, header, trailer)       |
| `src/lib/components/SoapPanel.svelte`           | WSDL, operasi, versi dan SOAPAction kartu SOAP      |
| `src/lib/components/ExamplesPanel.svelte`       | Tab Examples kartu request                          |
| `src/lib/components/MockPanel.svelte`           | Panel mock server collection                        |
| `src/lib/components/PathVariablesTable.svelte`  | Tabel path variable `:nama`                         |
| `src/lib/components/OAuthFields.svelte`         | Pengaturan dan token OAuth 2.0                      |
| `src/lib/components/FlowsView.svelte`           | Halaman Orchestration (daftar flow dan editor)      |
| `src/lib/components/FlowEditor.svelte`          | Kartu peta dan detail langkah terpilih              |
| `src/lib/components/FlowMap.svelte`             | Peta langkah, garis nilai, dan garis berhenti       |
| `src/lib/components/FlowStepCard.svelte`        | Detail langkah (Uses, Checks, Pass on, Response)    |
| `src/lib/components/FlowStepMark.svelte`        | Penanda status langkah                              |
| `src/lib/components/FlowValueChip.svelte`       | Chip `{{nama}}` untuk nilai yang diteruskan         |
| `src/lib/components/FlowStepPicker.svelte`      | Dialog pemilih request untuk langkah                |
| `src/lib/components/MethodStamp.svelte`         | Cap method/jenis request                            |
| `src/lib/components/CatalogView.svelte`         | Halaman Catalog (daftar atau satu API)              |
| `src/lib/components/catalog/`                   | Daftar, halaman API, grafik, aturan, tile           |
| `src/lib/catalog/present.ts`                    | Label, warna, format, filter, dan urutan API        |
| `src/lib/components/DocsView.svelte`            | Halaman Docs (daftar collection dan dokumen)        |
| `src/lib/components/docs/`                      | Deskripsi, Markdown, blok request, tabel            |
| `src/lib/components/settings/TlsPanel.svelte`   | Settings › SSL/TLS (versi TLS dan sertifikat)       |
| `branding/codestamp-logo.svg`                   | Logo aplikasi: sumber ikon dan logo sidebar         |
| `branding/logo-samples/`                        | Sampel logo yang dibandingkan sebelum memilih 3a    |
