# Copy as code: the generated code is run

Suite [`codegen-e2e.mjs`](../../../e2e/codegen-e2e.mjs) · 90 checks (15 languages × 6 cases)

The Copy as code generators (`src/lib/request/codegen/`) are bundled with rolldown. Each case is then written in 15 languages, run with the real toolchain against a local HTTP server, and the request the server actually received is compared with the expected one: method, path, query (order included), headers, and body. Cases that use _Environment variables_ get their values through the process environment, the same way a user runs the code.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).
- cURL: `bash` and `curl`. On Windows the suite uses the `curl.exe` that ships with Windows, because the curl from Git for Windows sends non-ASCII arguments in the ANSI code page.
- PowerShell: `powershell` (5.1) or `pwsh`. `.ps1` files are saved as UTF-8 with a BOM, as the comment in the generated code says.
- JavaScript: Node.js 18 or later.
- Python: Python 3 with the `requests` package.
- Java: JDK 15 or later (`java Main.java`).
- C#: .NET SDK 10 (`dotnet run request.cs`).
- HTTPie: installed in `e2e/.work/venv` (`python -m venv e2e/.work/venv`, then `pip install httpie` from that venv).
- JavaScript axios: `npm install --prefix e2e` (axios is in `e2e/package.json`).
- Rust: Cargo with `reqwest` 0.13 and `serde_urlencoded` in the Cargo cache (one `cargo fetch` is enough). The suite builds offline (`cargo run --offline`) from the `Cargo.toml` line in the generated code's comment, with a shared build folder in `e2e/.work/rust-target`.
- Go, PHP, Ruby, Kotlin, Dart, and Swift: a running Docker with the images `golang:1.24-alpine`, `composer:2`, `ruby:3-alpine`, `gradle:jdk21`, `dart:stable`, and `swift:6.1`. The suite never pulls images (`--pull never`). Guzzle is installed once with Composer into `e2e/.work/php-deps` (up to 3 attempts, because Packagist sometimes times out from a container) and shared by every PHP case; Kotlin downloads OkHttp through Gradle, and Dart downloads `http` through `dart pub get`. The server is reached through `host.docker.internal`.

A language whose toolchain or image is missing is marked SKIP, not failed. Multipart in PowerShell is also skipped when only Windows PowerShell 5.1 is available, because `-Form` arrived in PowerShell 7.

## Flow

Every case runs in cURL, HTTPie, PowerShell, JavaScript `fetch` and axios, Python `requests`, PHP Guzzle, Go `net/http`, Java `HttpClient`, Kotlin OkHttp, C# `HttpClient`, Dart `http`, Swift `URLSession`, Ruby `Net::HTTP`, and Rust `reqwest`.

### 1. JSON body, query and headers from environment variables

- `{{baseUrl}}`, `{{search}}`, `{{token}}`, and `{{name}}` are read from environment variables.
- A multi-line JSON body with quotes, backslashes, `$HOME`, `%`, backticks, braces, and `é` arrives exactly as written.
- The query arrives in order and encoded (`kopi & teh`).

### 2. URL-encoded form

- Fields arrive in their original order and correctly encoded (`&`, `%`, `+`, spaces).

### 3. Basic auth encoded when the code runs

- The username and password from environment variables are Base64-encoded when the code runs; the password contains `:`.

### 4. Multipart with a file

- The text field and the file arrive as `multipart/form-data` with the right names, file name, and file content.

### 5. Binary file body

- The file content is sent as is as a `PUT` body.

### 6. Values as written, with characters each language has to escape

- Literal values containing `'`, `"`, `"""`, `'''`, `'@`, `"@`, `${x}`, `%s`, `{0}`, backticks, backslashes, and `{{literal}}` arrive unchanged in the URL, headers, and body.

---

Previous: [CLI: codestamp-cli](cli.md) · [Stage list](../README.md#stage-list)
