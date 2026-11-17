# Update otomatis untuk rilis publik, dan wrap output respons: rencana implementasi

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menyalakan update otomatis di rilis publik GitHub (NSIS dan AppImage update dari aplikasi; .deb diarahkan ke `apt`), dan membuat tampilan respons wrap secara default dengan tombol untuk mematikannya.

**Architecture:**
- `latest.json` ditulis job baru di workflow rilis, dari file `.sig` hasil build, lewat modul murni `scripts/release-manifest.ts`. File itu dilampirkan ke draft release, dan aplikasi membacanya dari `releases/latest/download/latest.json`.
- Aplikasi memilih cara update dari jenis bundle (`tauri::utils::platform::bundle_type()`).
- Wrap respons adalah satu preference editor baru (`wrapResponses`). Preference itu diteruskan ke `CodeEditor` lewat prop `wrap`, dan diubah lewat komponen `WrapToggle`.

**Tech Stack:** Tauri 2.12 + `tauri-plugin-updater` 2.14 (Rust 2024), Svelte 5 + CodeMirror 6, Node 24 (menjalankan `.ts` langsung, `node:test`), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-17-updates-and-wrap-design.md`

## Global Constraints

- Alamat update: `https://github.com/oneshield7/codestamp/releases/latest/download/latest.json`
- URL unduhan dikunci ke tag: `https://github.com/oneshield7/codestamp/releases/download/vX.Y.Z/<file>`
- Target di `latest.json`:
  - `windows-x86_64` dan `windows-x86_64-nsis` menunjuk ke `<productName>_<version>_x64-setup.exe`.
  - `linux-x86_64-appimage` menunjuk ke `<productName>_<version>_amd64.AppImage`.
  - Jangan pernah menulis `linux-x86_64` polos.
- Catatan rilis maksimal 4000 byte (`MAX_NOTES_BYTES`, sama dengan Rust `settings.rs`).
- Secret rilis: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Perintah APT yang ditampilkan: `sudo apt update && sudo apt upgrade`
- `wrapResponses` default `true`. File preferences lama tanpa field ini dibaca sebagai `true`.
- Versi action:
  - Yang sudah dipakai repo: `actions/checkout@v7`, `actions/setup-node@v7` (Node 24), `softprops/action-gh-release@v3`.
  - Yang baru: `actions/upload-artifact@v7`, `actions/download-artifact@v8` (rilis terbaru per 2026-11-17).
- Teks UI dalam bahasa Inggris, gaya kalimat biasa. Tanpa em dash, tanpa shortcut di layar (DESIGN.md › Icon toolbar). Label sibuk memakai "…".
- Komentar code mengikuti `antislop:antislop-code`: hanya "kenapa", bukan "apa".
- Task UI memuat `antislop:antislop`, `antislop:antislop-ui`, `antislop:antislop-copywriting`, dan `antislop:antislop-human` (mode during).

## Review Focus

1. **Pemasangan .deb saat `latest.json` tidak punya target deb.** Aplikasi harus melaporkan versi baru dengan cara APT dan tidak pernah memasang apa pun. Diuji di Task 3 (`route_for`) dan Task 4 (guard `install`).
2. **Tag di-push sebelum versi dinaikkan** (`v0.6.0`, sementara `tauri.conf.json` masih `0.5.0`). Job manifest harus gagal dengan pesan yang jelas, bukan menerbitkan `latest.json` yang salah. Diuji di Task 1.
3. **CHANGELOG belum punya bagian untuk versi itu.** Rilis tetap jalan dan `notes` dikosongkan. Diuji di Task 1.
4. **Alamat update bukan alamat "latest" GitHub** (fork atau server sendiri). `manifest` harus gagal dan menyebut bentuk alamat yang diharapkan. Diuji di Task 1.
5. **File preferences dari versi sebelum fitur ini.** Respons harus langsung wrap, dan setting lain tidak berubah. Diuji di Task 5.

---

### Task 1: Modul manifest rilis dan perintah `manifest`

**Files:**
- Create: `scripts/release-manifest.ts`
- Create: `scripts/release-manifest.test.ts`
- Modify: `scripts/updates.ts`: ganti `manifest()` dan `readNotes()`; usage di header dan di `fail(...)` terakhir.
- Modify: `package.json`: script `"test:scripts": "node --test scripts/"`, lalu sisipkan `npm run test:scripts` di `verify` setelah `npm run lint`.

**Interfaces:**
- Produces (dipakai Task 2):
  - `node scripts/updates.ts manifest --tag <vX.Y.Z> --signatures <dir> [--out <file>]`. Default `--out`: `latest.json` di folder kerja.
  - `<dir>` berisi `<installer>.sig` dan `<appimage>.sig`.
- Produces di `scripts/release-manifest.ts`:
  - `export const MAX_NOTES_BYTES = 4000`
  - `export function downloadBase(endpoint: string, tag: string): string`
    - Endpoint harus berbentuk `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`.
    - Hasilnya `https://github.com/<owner>/<repo>/releases/download/<tag>/`.
    - Bentuk lain melempar `Error` yang pesannya memuat `releases/latest/download/latest.json`.
  - `export function releaseNotes(changelog: string, version: string): string | undefined`
    - Isi di antara `## [<version>]` dan heading `## [` berikutnya, di-trim.
    - Baris `#`/`##`/`###` kehilangan tanda `#` dan spasinya.
    - `undefined` bila bagiannya tidak ada atau kosong. Melempar `Error` bila hasilnya lebih dari `MAX_NOTES_BYTES` byte.
  - `export interface ReleaseInput { version: string; tag: string; productName: string; endpoint: string; changelog: string; signatures: { installer: string; appImage: string }; pubDate: string }`
  - `export function buildManifest(input: ReleaseInput): Manifest`
    - Melempar `Error` bila `tag !== \`v${version}\``.
    - `Manifest` = `{ version, notes?, pub_date, platforms: Record<string, { signature: string; url: string }> }`.
    - Kunci platform persis seperti di Global Constraints.
    - Signature di-trim, dan nama file di URL di-`encodeURIComponent`.

- [ ] **Step 1: Tulis test yang gagal** di `scripts/release-manifest.test.ts` (`node:test` + `node:assert/strict`, import `./release-manifest.ts`):
  - `downloadBase('https://github.com/oneshield7/codestamp/releases/latest/download/latest.json', 'v0.6.0')` sama dengan `'https://github.com/oneshield7/codestamp/releases/download/v0.6.0/'`.
  - `downloadBase('https://updates.example.com/codestamp/latest.json', 'v0.6.0')` melempar, dan pesannya cocok dengan `/releases\/latest\/download\/latest\.json/`.
  - `releaseNotes(changelog, '0.5.0')`, dengan contoh berisi `## [Unreleased]`, `## [0.5.0] - 2026-11-03`, `### Added`, `- Linux packages.`, `## [0.4.0]`:
    - hasilnya tidak memuat `'## [0.5.0]'` maupun `'2026-11-03'`, karena baris heading versi tidak ikut;
    - hasilnya memuat `'Added\n\n- Linux packages.'`;
    - hasilnya tidak memuat `'0.4.0'`.
  - `releaseNotes(changelog, '9.9.9')` sama dengan `undefined`.
  - `releaseNotes('## [1.0.0]\n\n' + 'x'.repeat(4001), '1.0.0')` melempar.
  - Dengan input valid (version `0.6.0`, tag `v0.6.0`, productName `Codestamp`, signatures `' sigA\n'`/`'sigB'`), `buildManifest`:
    - menghasilkan kunci platform `windows-x86_64`, `windows-x86_64-nsis`, `linux-x86_64-appimage` (diurutkan), tanpa `linux-x86_64`;
    - menghasilkan URL `windows-x86_64` = `.../releases/download/v0.6.0/Codestamp_0.6.0_x64-setup.exe`;
    - menghasilkan URL appimage = `.../releases/download/v0.6.0/Codestamp_0.6.0_amd64.AppImage`;
    - menghasilkan signature windows `'sigA'`.
  - `buildManifest` dengan tag `v0.5.9` dan version `0.6.0` melempar, dan pesannya memuat `0.6.0`.

- [ ] **Step 2: Jalankan dan pastikan gagal**
  Run: `/c/laragon/bin/nodejs/node-v24/node.exe --test scripts/`
  Expected: FAIL (modul `./release-manifest.ts` tidak ada)

- [ ] **Step 3: Implementasikan `scripts/release-manifest.ts`** sesuai Interfaces. Tanpa dependensi, hanya JS standar. `Buffer.byteLength` untuk batas byte boleh lewat `node:buffer`.

- [ ] **Step 4: Ganti `manifest` di `scripts/updates.ts`**
  - Baca flag `--tag`, `--signatures`, `--out`. `fail(usage)` bila flag tidak dikenal atau `--tag`/`--signatures` tidak ada.
  - Baca `tauri.conf.json` (`version`, `productName`, `plugins.updater.endpoints[0]`) dan `CHANGELOG.md` di root.
  - Baca kedua `.sig`. Bila file hilang, `fail` dengan menyebut nama filenya.
  - Panggil `buildManifest` dan ubah `Error` menjadi `fail(message)`. Tulis JSON berindentasi 2 dengan baris baru di akhir.
  - Hapus `readNotes`, `bundleDir`, dan konstanta lokal `MAX_NOTES_BYTES` (pakai yang di modul).

- [ ] **Step 5: Jalankan test dan pemeriksaan tipe**
  Run: `/c/laragon/bin/nodejs/node-v24/node.exe --test scripts/` lalu `npx tsc -p tsconfig.node.json` (dengan folder Node di `PATH`)
  Expected: semua test PASS, `tsc` tanpa error

- [ ] **Step 6: Coba perintahnya dengan file `.sig` palsu**
  - Script mencari root di `scripts/..`. Endpoint di config asli masih kosong sampai Task 8, jadi bangun salinan kecil di scratchpad:
    - `scripts/updates.ts` dan `scripts/release-manifest.ts`;
    - `src-tauri/tauri.conf.json` dengan `endpoints` = alamat update GitHub;
    - `CHANGELOG.md`;
    - folder `sigs/` berisi `Codestamp_0.5.0_x64-setup.exe.sig` dan `Codestamp_0.5.0_amd64.AppImage.sig`.
  - Jalankan `node scripts/updates.ts manifest --tag v0.5.0 --signatures sigs --out latest.json` di salinan itu.
  - Expected: `latest.json` berisi tiga target dengan URL `.../download/v0.5.0/...`, dan `notes` = bagian 0.5.0 CHANGELOG.
  - Jalankan juga dengan `--tag v0.5.1`. Expected: gagal dan menyebut 0.5.0.
  - Di repo asli, `manifest` gagal dengan "Updates are not set up yet…".

- [ ] **Step 7: Commit**
  ```bash
  git add scripts/release-manifest.ts scripts/release-manifest.test.ts scripts/updates.ts package.json
  git commit -m "Write latest.json for GitHub releases from the build signatures"
  ```

### Task 2: Workflow rilis menandatangani update dan menerbitkan `latest.json`

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: perintah `manifest` dari Task 1.

- [ ] **Step 1: Tanda tangan di build**
  - Tambahkan `TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}` dan `TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}` sebagai `env` di level job `windows` dan `linux`.
  - Komentar satu baris: build gagal tanpa keduanya karena `createUpdaterArtifacts` menyala.
- [ ] **Step 2: Unggah signature**
  - `windows`: tambahkan `actions/upload-artifact@v7`, `name: updater-windows`, `path: src-tauri/target/release/bundle/nsis/*-setup.exe.sig`, `if-no-files-found: error`.
  - `linux`: sama dengan `name: updater-linux` dan `path: src-tauri/target/release/bundle/appimage/*.AppImage.sig`. Letakkan sebelum langkah recompress .deb atau sesudahnya; keduanya tidak menyentuh AppImage.
- [ ] **Step 3: Job `updates`** (`name: Update manifest`, `needs: [windows, linux]`, `runs-on: ubuntu-latest`). Langkahnya:
  - `checkout@v7`, `setup-node@v7` (node 24, tanpa `npm ci`; script hanya memakai modul bawaan Node);
  - `actions/download-artifact@v8` dengan `pattern: updater-*`, `merge-multiple: true`, `path: signatures`;
  - `run: node scripts/updates.ts manifest --tag "$GITHUB_REF_NAME" --signatures signatures --out latest.json`;
  - `softprops/action-gh-release@v3` dengan `draft: true` dan `files: latest.json`.
- [ ] **Step 4: Perbarui komentar header workflow**: `latest.json` ikut di draft, dan update sampai ke pengguna saat draft dipublish.
- [ ] **Step 5: Periksa YAML**
  Run: `/c/laragon/bin/nodejs/node-v24/node.exe -e "require('node:fs').readFileSync('.github/workflows/release.yml','utf8')"`, lalu baca ulang indentasi dan `needs` secara manual. Repo tidak punya parser YAML atau actionlint, dan tidak boleh mengunduh alat baru.
  Expected: `needs` job `updates` = `[windows, linux]`, nama artifact cocok dengan `pattern`.
- [ ] **Step 6: Commit** `git commit -m "Sign updates and attach latest.json to the draft release"`

### Task 3: Rust memilih cara update per jenis bundle

**Files:**
- Modify: `src-tauri/src/update/settings.rs`: tambah `UpdateRoute`, `route_for`, `APT_TARGET`, dan test.
- Modify: `src-tauri/src/update/models.rs`: field `via` di `Available`.
- Modify: `src-tauri/src/update/commands.rs`: `check_for_update`.

**Interfaces:**
- Produces (dipakai Task 4, lewat JSON):
  - `UpdateCheck::Available` mendapat `via: UpdateRoute`, diserialisasi sebagai `"via": "app"` atau `"via": "apt"`.
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "lowercase")] pub enum UpdateRoute { App, Apt }` (di `settings.rs`, re-export dari `models.rs` atau sebaliknya; pilih satu tempat).
  - `pub fn route_for(bundle: Option<tauri::utils::config::BundleType>) -> UpdateRoute`: `Deb` menjadi `Apt`, selain itu `App`.
  - `pub const APT_TARGET: &str = "linux-x86_64-appimage";`

- [ ] **Step 1: Tulis test yang gagal** di `settings.rs` `mod tests`:
  ```rust
  #[test]
  fn deb_installs_update_through_apt() {
      assert_eq!(route_for(Some(BundleType::Deb)), UpdateRoute::Apt);
      for other in [Some(BundleType::AppImage), Some(BundleType::Nsis), Some(BundleType::Rpm), None] {
          assert_eq!(route_for(other), UpdateRoute::App);
      }
  }
  ```
  Tambahkan juga di `models.rs` (`mod tests` baru) bahwa `serde_json::to_value(UpdateCheck::Available { …, via: UpdateRoute::Apt })` memuat `"via": "apt"` dan `"kind": "available"`.
- [ ] **Step 2: Jalankan dan pastikan gagal**: lihat "Verifikasi Rust" di bawah. Expected: compile error, `route_for` tidak ada.
- [ ] **Step 3: Implementasikan**
  - `check_for_update` menghitung `let route = route_for(bundle_type());`.
  - Builder mendapat `.target(APT_TARGET)` hanya untuk `Apt`.
  - `Available` diisi `via: route`.
  - `*pending.lock()` = `found` untuk `App`, `None` untuk `Apt`.
  - Komentar satu baris: target AppImage dipakai hanya untuk membaca versi, karena .deb dipasang ulang oleh apt.
  - `install_update` tidak berubah. Ia sudah menolak bila tidak ada pending ("Check for updates first.").
- [ ] **Step 4: Jalankan test dan pastikan lolos**: lihat "Verifikasi Rust". Expected: PASS, clippy bersih.
- [ ] **Step 5: Commit** `git commit -m "Point .deb installs at apt instead of installing updates in the app"`

### Task 4: UI update: cara APT di banner dan About

Muat `antislop:antislop`, `antislop:antislop-ui`, `antislop:antislop-copywriting`, `antislop:antislop-human`, dan `antislop:antislop-code`.

**Files:**
- Modify: `src/lib/api/update.ts`: `via: 'app' | 'apt'` di varian `available`, plus `export const APT_UPGRADE_COMMAND = 'sudo apt update && sudo apt upgrade';`
- Modify: `src/lib/state/updates.svelte.ts`: `install()` keluar bila `this.available?.via !== 'app'`.
- Create: `src/lib/components/CopyCommand.svelte`
- Modify: `src/lib/components/UpdateBanner.svelte`
- Modify: `src/lib/components/settings/AboutPanel.svelte`

**Interfaces:**
- Consumes: JSON `via` dari Task 3.
- Produces: `CopyCommand` dengan props `{ command: string }`.
  - Isinya `<code class="mono">` yang memuat perintah, lalu `<button type="button">` "Copy" yang berubah menjadi "Copied" setelah berhasil.
  - `aria-label` tombol: "Copy the apt command" (tetap memuat kata "Copy").
  - Menyalin lewat `navigator.clipboard.writeText` di dalam `notices.guard`, seperti `copyFolder` di AboutPanel.

- [ ] **Step 1: Ubah tipe dan guard**, lalu jalankan `npm run check`. Expected: error di tempat yang belum menangani `via`, atau lolos bila tidak ada. Catat hasilnya.
- [ ] **Step 2: Banner** untuk `update.via === 'apt'` (bukan sedang memasang dan bukan error):
  - Teks: `Codestamp {version} is available. You have {current}. Update it from a terminal:`, lalu `<CopyCommand command={APT_UPGRADE_COMMAND} />`.
  - Tombol: *What's new* (bila ada notes) dan *Later*. Tanpa *Update and restart*.
  - Varian `app` tidak berubah.
- [ ] **Step 3: About** untuk `via === 'apt'`:
  - Baris berita dan catatan tetap.
  - Lalu `<p>Codestamp was installed from the APT repository, so apt installs the update:</p>` dan `<CopyCommand …/>`.
  - Tombol *Update and restart* hanya untuk `via === 'app'`. *Check for updates* tetap.
- [ ] **Step 4: Teks privasi About**: `' and contacts the update server only to look for a newer version'` menjadi `' and contacts GitHub only to look for a newer version'`.
- [ ] **Step 5: Gaya**
  - `code`: `--surface-muted`, border `--border-soft`, radius sm, padding `2px 6px`, `overflow-wrap: anywhere`, supaya perintah tidak melebar di jendela 720px.
  - Tombol Copy: kelas `.link` di banner (DESIGN.md: "Light inline actions: … Copy") dan `.button small` di About (sama dengan tombol Copy data folder).
  - Fokus dan cursor memakai gaya global di `app.css`.
- [ ] **Step 6: Jalankan** `npm run check` dan `npm run lint`. Expected: lolos tanpa warning.
- [ ] **Step 7: Commit** `git commit -m "Show the apt command when a .deb install has an update"`

### Task 5: Preference `wrapResponses` dan prop `wrap` di CodeEditor

**Files:**
- Modify: `src-tauri/src/preferences/models.rs`: field `wrap_responses: bool` di `EditorPreferences`, default `true`; `mod tests` baru.
- Modify: `src/lib/api/preferences.ts`: `wrapResponses: boolean` di `EditorPreferences`.
- Modify: `src/lib/state/preferences.svelte.ts`: `wrapResponses: true` di `DEFAULT_PREFERENCES.editor`.
- Modify: `src/lib/editor/setup.ts`: `preferenceExtensions(preferences: EditorPreferences, readOnly: boolean, wrap?: boolean): Extension[]`. Wrap menyala bila `wrap ?? preferences.wrapLines`.
- Modify: `src/lib/components/CodeEditor.svelte`: prop `wrap?: boolean` (komentar: untuk tampilan respons, menggantikan *Wrap long lines*), diteruskan ke kedua panggilan `preferenceExtensions`.
- Modify: `src/lib/components/settings/EditorPanel.svelte`: label `Wrap long lines while editing`.

**Interfaces:**
- Produces (dipakai Task 6): `preferenceSettings.editor.wrapResponses: boolean`, `preferenceSettings.updateEditor({ wrapResponses })`, `<CodeEditor wrap={…} />`.

- [ ] **Step 1: Tulis test yang gagal** di `models.rs`:
  ```rust
  #[test]
  fn responses_wrap_unless_turned_off() {
      assert!(EditorPreferences::default().wrap_responses);
      let saved: EditorPreferences = serde_json::from_str(
          r#"{"indent":"two","closeBrackets":false,"wrapLines":true,"layout":"side","variableHints":false}"#,
      ).expect("preferences saved before wrapResponses");
      assert!(saved.wrap_responses);
      assert_eq!(saved.indent, Indent::Two);
      assert!(saved.wrap_lines && !saved.close_brackets);
  }
  ```
- [ ] **Step 2: Jalankan dan pastikan gagal**: lihat "Verifikasi Rust". Expected: compile error, field tidak ada.
- [ ] **Step 3: Implementasikan** perubahan di semua file di atas. `validate.rs` tidak perlu diubah karena boolean selalu valid.
- [ ] **Step 4: Jalankan** "Verifikasi Rust", lalu `npm run check` dan `npm run lint`. Expected: semua lolos.
- [ ] **Step 5: Commit** `git commit -m "Add a wrap setting for response viewers, on by default"`

### Task 6: Tombol Wrap di tampilan respons

Muat paket antislop UI yang sama dengan Task 4.

**Files:**
- Create: `src/lib/components/WrapToggle.svelte`
- Modify: `src/lib/components/ResponsePanel.svelte`
- Modify: `src/lib/components/GrpcReplyPanel.svelte`
- Modify: `src/lib/components/ExamplesPanel.svelte`

**Interfaces:**
- Consumes: `preferenceSettings.editor.wrapResponses` dan `updateEditor` dari Task 5.
- Produces: `WrapToggle` tanpa props.
  - Satu `<button type="button" aria-pressed={on}>` berisi ikon 14px (`aria-hidden`) dan teks `Wrap lines`.
  - Klik memanggil `preferenceSettings.updateEditor({ wrapResponses: !on })`.

- [ ] **Step 1: `WrapToggle.svelte`**
  - Gaya dasar sama dengan `.save` di ResponsePanel: 28px, border `--border`, radius md, 12px, `--text-2`.
  - Saat `aria-pressed="true"`: background `--accent-soft`, border `--border-strong`, teks `--text`. Ini meniru toggle `Aa`/`.*` di panel Find (DESIGN.md › Code editor), jadi status tidak bergantung pada warna saja: border dan background ikut berubah.
  - `:hover` border `--border-strong`, `:active` `--surface-sunken`.
  - Ikon: panah baris balik (garis lurus, lalu baris kedua yang berbelok ke bawah), stroke 2 seperti ikon `.save`.
- [ ] **Step 2: ResponsePanel**
  - Bungkus `div.tabs[role=tablist]` dan toggle dalam `div.tab-row` (flex, `align-items: center`, border bawah pindah dari `.tabs` ke `.tab-row`, padding kanan 16px). Toggle tidak boleh menjadi anak `tablist`, karena tablist hanya boleh berisi tab.
  - Toggle tampil hanya bila `tab === 'body' && !stream` dan bukan tampilan halaman HTML (`!(pageHtml !== null && htmlView === 'page')`).
  - `CodeEditor` body mendapat `wrap={preferenceSettings.editor.wrapResponses}`.
- [ ] **Step 3: GrpcReplyPanel**: pola yang sama. Toggle tampil hanya saat tab messages dengan tepat satu pesan (yang memakai `CodeEditor`). Editor itu mendapat prop `wrap`.
- [ ] **Step 4: ExamplesPanel**: `CodeEditor` body example mendapat `wrap={preferenceSettings.editor.wrapResponses}` tanpa tombol sendiri.
  - Ini menyimpang dari spec, yang menyebut tombol di tiga tempat. Body example terbuka di dalam baris daftar, dan bar tambahan di sana hanya menambah kepadatan.
  - Example tetap mengikuti pilihan yang sama. Catat ini sebagai keputusan builder di laporan.
- [ ] **Step 5: Jalankan** `npm run check` dan `npm run lint`. Expected: lolos tanpa warning.
- [ ] **Step 6: Commit** `git commit -m "Wrap response viewers with a toggle next to the response tabs"`

### Task 7: Dokumentasi

**Files:**
- Modify: `README.md`
  - Di bagian Install › Windows dan Linux, tambahkan satu kalimat tentang update: installer dan AppImage meng-update dari aplikasi, .deb lewat `sudo apt update && sudo apt upgrade`.
  - Tambahkan catatan bahwa pengguna 0.5.0 memasang manual sekali.
- Modify: `README.id.md`: ganti isi bagian `### Pembaruan otomatis` dengan alur GitHub:
  - setup sekali: `npm run updates:setup -- https://github.com/oneshield7/codestamp/releases/latest/download/latest.json`, cadangan kunci, dan dua secret;
  - setiap rilis: naikkan versi, isi bagian CHANGELOG, push tag, review draft (cek `latest.json` ada), lalu publish;
  - perilaku .deb.
  Hapus langkah manual `updates:manifest` dan upload ke server.
- Modify: `CHANGELOG.md`: di `[Unreleased]`:
  - Added: update otomatis dari GitHub (catat bahwa 0.5.0 perlu dipasang manual sekali);
  - Added: tombol *Wrap lines* di tampilan respons, menyala secara default;
  - Changed: *Wrap long lines* di Settings › Editor kini hanya berlaku saat mengedit.
- Modify: `DESIGN.md`:
  - `### Code editor`: tampilan respons mengikuti *Wrap lines*, bukan setting editor;
  - `### Banner` atau bagian update: varian APT dengan `CopyCommand`.

- [ ] **Step 1: Tulis perubahan di atas** dengan gaya dokumen yang ada: kalimat pendek, tanpa em dash.
- [ ] **Step 2: Jalankan** `npx prettier --check .` (dengan folder Node di `PATH`). Expected: lolos.
- [ ] **Step 3: Commit** `git commit -m "Document updates from GitHub releases and response wrapping"`

### Task 8 (pemilik): kunci update dan secret

Langkah ini butuh password pemilik, jadi tidak dikerjakan agent. Agent menyiapkan perintahnya, dan pemilik menjalankannya di terminal.

- [ ] **Step 1: Buat kunci dan tulis config.** Di folder project:
  ```bash
  npm run updates:setup -- https://github.com/oneshield7/codestamp/releases/latest/download/latest.json
  ```
  Tauri CLI menanyakan password. Hasilnya: `%USERPROFILE%\.codestamp\updater.key` dan `.key.pub`, plus `pubkey`, `endpoints`, dan `createUpdaterArtifacts` di `tauri.conf.json`.
- [ ] **Step 2: Cadangkan** `updater.key` dan passwordnya di tempat yang aman, di luar repository.
- [ ] **Step 3: Isi secret** di GitHub › Settings › Secrets and variables › Actions:
  - `TAURI_SIGNING_PRIVATE_KEY` = isi file `updater.key`;
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = passwordnya.
- [ ] **Step 4: Agent memeriksa diff** `tauri.conf.json`: hanya `plugins.updater` dan `bundle.createUpdaterArtifacts` yang berubah, dan tidak ada kunci privat. Lalu commit: `git commit -m "Turn on updates from GitHub releases"`.

## Verifikasi Rust

Mesin ini tidak punya toolchain Rust di Windows. WSL Debian punya `cargo`, tapi tidak punya `libwebkit2gtk-4.1-dev`, jadi crate Tauri tidak bisa dibangun di sana. Urutan pilihan:

1. Bila pemilik memasang library build Linux di WSL, jalankan dari folder project:
   ```bash
   wsl -d Debian -- bash -lc 'cd "/mnt/c/Users/ridwannw/Documents/Ridwan/New Working/New Working/postcard" && CARGO_TARGET_DIR=$HOME/codestamp-target cargo test --manifest-path src-tauri/Cargo.toml && CARGO_TARGET_DIR=$HOME/codestamp-target cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings'
   ```
   Perintah instalasinya sama dengan di `ci.yml`: `sudo apt-get install -y libwebkit2gtk-4.1-dev libxdo-dev libssl-dev librsvg2-dev`.
2. Bila tidak, `cargo fmt --check` dijalankan di WSL (tidak perlu build). Test dan clippy dijalankan oleh workflow CI saat branch di-push atau PR dibuka. Laporan akhir menyebut dengan jelas bahwa test Rust belum dijalankan lokal.

## Pemeriksaan akhir

- `npm run check`, `npm run lint`, dan `npm run test:scripts` dengan Node dari `C:\laragon\bin\nodejs\node-v24` di `PATH`.
- Rust sesuai bagian di atas.
- Tampilan tidak bisa dijalankan sebagai aplikasi Tauri di mesin ini, karena tidak ada Rust di Windows. Delivery Gate antislop memeriksa lewat kode, dan laporan menyebutnya.
- Bila `npm run dev` bisa membuka halaman di browser pane, cek tombol Wrap dan banner di sana dengan data tiruan hanya bila komponen bisa tampil tanpa backend. Bila tidak bisa, jangan dipaksakan.
