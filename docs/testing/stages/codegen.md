# Copy as code: kode yang dihasilkan dijalankan

Suite [`codegen-e2e.mjs`](../../../e2e/codegen-e2e.mjs) · 90 cek (15 bahasa × 6 kasus)

Generator Copy as code (`src/lib/request/codegen/`) di-bundle dengan rolldown. Setiap kasus lalu ditulis dalam 15 bahasa, dijalankan dengan toolchain sungguhan terhadap server HTTP lokal, dan request yang benar-benar diterima server dibandingkan dengan yang diharapkan: method, path, query (urutan ikut dicek), header, dan body. Kasus yang memakai _Environment variables_ menerima nilainya lewat environment proses, seperti saat pengguna menjalankan kode itu.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).
- cURL: `bash` dan `curl`. Di Windows dipakai `curl.exe` bawaan Windows, karena curl dari Git for Windows mengirim argumen non-ASCII dalam code page ANSI.
- PowerShell: `powershell` (5.1) atau `pwsh`. File `.ps1` disimpan sebagai UTF-8 dengan BOM, sesuai komentar di kode yang dihasilkan.
- JavaScript: Node.js 18 atau lebih baru.
- Python: Python 3 dengan paket `requests`.
- Java: JDK 15 atau lebih baru (`java Main.java`).
- C#: .NET SDK 10 (`dotnet run request.cs`).
- HTTPie: dipasang di `e2e/.work/venv` (`python -m venv e2e/.work/venv`, lalu `pip install httpie` dari venv itu).
- JavaScript axios: `npm install --prefix e2e` (axios ada di `e2e/package.json`).
- Rust: Cargo dengan `reqwest` 0.13 dan `serde_urlencoded` di cache Cargo (sekali `cargo fetch` cukup). Suite membangun offline (`cargo run --offline`) memakai baris `Cargo.toml` dari komentar kode yang dihasilkan, dengan folder build bersama di `e2e/.work/rust-target`.
- Go, PHP, Ruby, Kotlin, Dart, dan Swift: Docker yang sedang berjalan, dengan image `golang:1.24-alpine`, `composer:2`, `ruby:3-alpine`, `gradle:jdk21`, `dart:stable`, dan `swift:6.1`. Suite tidak pernah mengunduh image (`--pull never`). Guzzle dipasang sekali lewat Composer ke `e2e/.work/php-deps` (dicoba sampai 3 kali, karena Packagist kadang timeout dari container) lalu dipakai bersama semua kasus PHP; Kotlin mengunduh OkHttp lewat Gradle, dan Dart mengunduh `http` lewat `dart pub get`. Server dijangkau lewat `host.docker.internal`.

Bahasa yang toolchain atau image-nya tidak tersedia ditandai SKIP, tidak gagal. Multipart di PowerShell juga di-SKIP bila yang ada hanya Windows PowerShell 5.1, karena `-Form` baru ada di PowerShell 7.

## Alur

Setiap kasus dijalankan di cURL, HTTPie, PowerShell, JavaScript `fetch` dan axios, Python `requests`, PHP Guzzle, Go `net/http`, Java `HttpClient`, Kotlin OkHttp, C# `HttpClient`, Dart `http`, Swift `URLSession`, Ruby `Net::HTTP`, dan Rust `reqwest`.

### 1. JSON body, query and headers from environment variables

- `{{baseUrl}}`, `{{search}}`, `{{token}}`, dan `{{name}}` dibaca dari environment variable.
- Body JSON multi-baris berisi kutip, backslash, `$HOME`, `%`, backtick, kurung kurawal, dan `é` tiba persis sama.
- Query tiba berurutan dan ter-encode (`kopi & teh`).

### 2. URL-encoded form

- Field tiba dengan urutan aslinya dan ter-encode benar (`&`, `%`, `+`, spasi).

### 3. Basic auth encoded when the code runs

- Username dan password dari environment variable dienkode Base64 saat kode berjalan; password berisi `:`.

### 4. Multipart with a file

- Field teks dan file tiba sebagai `multipart/form-data` dengan nama, nama file, dan isi file yang benar.

### 5. Binary file body

- Isi file dikirim apa adanya sebagai body `PUT`.

### 6. Values as written, with characters each language has to escape

- Nilai literal berisi `'`, `"`, `"""`, `'''`, `'@`, `"@`, `${x}`, `%s`, `{0}`, backtick, backslash, dan `{{literal}}` tiba tanpa berubah di URL, header, dan body.

---

Sebelumnya: [CLI: codestamp-cli](cli.md) · [Daftar stage](../README.md#daftar-stage)
